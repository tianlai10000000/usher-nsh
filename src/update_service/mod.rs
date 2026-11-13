//! Publishes update policy through usher-state and fences one application-owned effect executor.
//!
//! Consumers borrow `reader()` and send user commands. Only the controller holding a
//! lease may claim effects or report results. No network, file, NSH or timer work runs here.
//!
//! There is no cancel and no executor handover (docs/update.md, 8.2, option B): NSH cannot cancel
//! an update operation, so the service does not model stopping one. The lease is held until the
//! service closes; an operation that outlives a rebuilt window is met by NSH's own
//! "already in progress" answer, which the machine classes as `Failure::Busy`.
mod execution;
mod registry;
pub use execution::{Claim, Completion, Lease};
pub use registry::UpdateRegistry;

use std::{collections::VecDeque, sync::Mutex};
use usher_state::{
    BackendFactory, Snapshot, StoreId, StoreReader, StoreWriter, Transition, create_store,
};
use usher_update::state::{Effect, Event, Machine, Phase, Trigger};

/// Stable application errors; transports must preserve the codes rather than format domain data.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum Error {
    Closed,
    Unavailable,
    Busy,
    Stale,
    Invalid,
    Exhausted,
    Configuration,
}
impl Error {
    pub fn code(self) -> &'static str {
        match self {
            Self::Closed => "updates_closed",
            Self::Unavailable => "updates_unavailable",
            Self::Busy => "updates_busy",
            Self::Stale => "updates_stale_execution",
            Self::Invalid => "updates_invalid_command",
            Self::Exhausted => "updates_identity_exhausted",
            Self::Configuration => "updates_configuration_mismatch",
        }
    }
}
/// User intent is separate from executor completion, so consumers cannot forge a result.
#[derive(Clone, Copy, Debug)]
pub enum Command {
    Check(Trigger),
    Download,
    Install,
    ConfirmFirstLaunch,
    /// The user turned automatic checks on or off in Settings. Applies to the next check.
    AutoCheck(bool),
}
/// Observable execution lifecycle. Leases and executable effects are never broadcast.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum Execution {
    Available,
    Running,
}
/// Immutable projection of the legacy domain and its effect delivery status. Never persisted.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct State {
    pub domain: Machine,
    pub execution: Execution,
    pub pending: usize,
}

/// Records the machine produced and nobody has collected. A logger that is off or slow must not
/// make this grow without bound, so the oldest are dropped past this many.
const LOG_BACKLOG: usize = 64;

#[derive(Clone)]
struct Job {
    id: u64,
    effect: Effect,
}
#[derive(Clone)]
struct Model {
    domain: Machine,
    queue: VecDeque<Job>,
    lease: Option<Lease>,
    active: Option<Claim>,
    /// Records the machine produced and nobody has collected yet; bounded by `LOG_BACKLOG`.
    logs: Vec<usher_update::state::Record>,
    next_identity: u64,
}
impl Model {
    fn identity(&mut self) -> Result<u64, Error> {
        self.next_identity = self.next_identity.checked_add(1).ok_or(Error::Exhausted)?;
        Ok(self.next_identity)
    }
    fn state(&self) -> State {
        State {
            domain: self.domain.clone(),
            pending: self.queue.len(),
            execution: if self.active.is_some() {
                Execution::Running
            } else {
                Execution::Available
            },
        }
    }
    fn event(&mut self, event: Event) -> Result<(), Error> {
        // The domain uses u64 tickets. Reject before its unchecked increment.
        if self.domain.ticket() == u64::MAX {
            return Err(Error::Exhausted);
        }
        for effect in self.domain.handle(event) {
            // A log record needs no executor: it is not work for the network or the disk, only
            // something to hand to the application's logger. The service keeps them in order and
            // hands them out through `take_logs`, so a caller that has no logger drops them.
            if let Effect::Log(record) = effect {
                if self.logs.len() == LOG_BACKLOG {
                    self.logs.remove(0);
                }
                self.logs.push(record);
                continue;
            }
            let id = self.identity()?;
            self.queue.push_back(Job { id, effect });
        }
        Ok(())
    }
}
struct Transaction {
    model: Model,
    writer: StoreWriter<State>,
    closed: bool,
}
/// One update domain per application scope, independent of the number of windows or views.
pub struct UpdateService {
    id: StoreId,
    transaction: Mutex<Transaction>,
    reader: StoreReader<State>,
}
impl UpdateService {
    /// Creates a fresh domain with an injected backend. Pass an idle Machine; recovery
    /// from a previous process belongs to the host and must not silently replay effects.
    pub fn new(domain: Machine, factory: &dyn BackendFactory<State>) -> Result<Self, Error> {
        if domain.phase != Phase::Idle || domain.ticket() != 0 {
            return Err(Error::Invalid);
        }
        let id = crate::app_state::next_store_id().map_err(|_| Error::Exhausted)?;
        let model = Model {
            domain,
            queue: VecDeque::new(),
            lease: None,
            active: None,
            logs: Vec::new(),
            next_identity: 0,
        };
        let (writer, reader) =
            create_store(id, model.state(), factory).map_err(|_| Error::Unavailable)?;
        Ok(Self {
            id,
            transaction: Mutex::new(Transaction {
                model,
                writer,
                closed: false,
            }),
            reader,
        })
    }
    pub fn reader(&self) -> StoreReader<State> {
        self.reader.clone()
    }
    pub fn snapshot(&self) -> Result<Snapshot<State>, Error> {
        let guard = self.transaction.lock().map_err(|_| Error::Unavailable)?;
        if guard.closed {
            return Err(Error::Closed);
        }
        self.reader.snapshot().map_err(|_| Error::Unavailable)
    }
    pub fn subscribe(&self) -> Result<usher_state::Subscription<State>, Error> {
        let guard = self.transaction.lock().map_err(|_| Error::Unavailable)?;
        if guard.closed {
            return Err(Error::Closed);
        }
        self.reader.subscribe().map_err(|_| Error::Unavailable)
    }
    /// Serializes user intent without executing effects or accepting completion events.
    pub fn request(&self, command: Command) -> Result<(), Error> {
        self.change(|model| {
            let event = match command {
                Command::Check(trigger) => Event::CheckRequested(trigger),
                Command::Download => Event::DownloadRequested,
                Command::Install => Event::InstallRequested,
                Command::ConfirmFirstLaunch => Event::FirstLaunchConfirmed,
                Command::AutoCheck(on) => {
                    // Not a domain event: it only changes what the next check does.
                    model.domain.set_auto_check(on);
                    return Ok(());
                }
            };
            model.event(event)
        })
    }
    /// Hands the application's logger what the machine has recorded since the last call, oldest
    /// first. The caller writes the records or drops them; this service does neither. Taking is
    /// not a state change, so it publishes nothing.
    pub fn take_logs(&self) -> Result<Vec<usher_update::state::Record>, Error> {
        let mut guard = self.transaction.lock().map_err(|_| Error::Unavailable)?;
        if guard.closed {
            return Err(Error::Closed);
        }
        Ok(std::mem::take(&mut guard.model.logs))
    }
    /// Closing only fences callbacks; it does not assert that native side effects stopped.
    /// Hosts must drain/reconcile in-flight work before rebuilding this application scope.
    pub fn close(&self) {
        let mut guard = self
            .transaction
            .lock()
            .unwrap_or_else(|error| error.into_inner());
        guard.closed = true;
        guard.model.queue.clear();
        guard.model.lease = None;
        guard.writer.close();
    }
    fn change<T>(&self, action: impl FnOnce(&mut Model) -> Result<T, Error>) -> Result<T, Error> {
        let mut guard = self.transaction.lock().map_err(|_| Error::Unavailable)?;
        if guard.closed {
            return Err(Error::Closed);
        }
        let mut next = guard.model.clone();
        let result = action(&mut next)?;
        let state = next.state();
        let transition: Transition<State, ()> = if state == guard.model.state() {
            Transition::Unchanged { effects: vec![] }
        } else {
            Transition::Changed {
                state,
                effects: vec![],
            }
        };
        // Neither a claimed effect nor a new lease escapes before successful publication.
        guard
            .writer
            .apply(transition)
            .map_err(|_| Error::Unavailable)?;
        guard.model = next;
        Ok(result)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use usher_state::backends::WatchBackend;
    use usher_test_support::task::ready;
    use usher_update::state::{Capability, Settings};

    fn service() -> UpdateService {
        UpdateService::new(
            Machine::new(Settings {
                shell: "1.0.0".parse().unwrap(),
                running: "1.0.0".parse().unwrap(),
                capability: Capability::InApp,
                auto_check: true,
                failure_limit: 2,
            }),
            &WatchBackend,
        )
        .unwrap()
    }
    #[test]
    fn exhausted_identity_does_not_publish_or_escape_as_a_lease() {
        let service = service();
        service.transaction.lock().unwrap().model.next_identity = u64::MAX;
        assert_eq!(service.acquire(), Err(Error::Exhausted));
        assert_eq!(
            service.request(Command::Check(Trigger::Manual)),
            Err(Error::Exhausted)
        );
        assert_eq!(service.snapshot().unwrap().sequence(), 0);
        assert_eq!(
            service.snapshot().unwrap().state().domain.phase,
            Phase::Idle
        );
    }
    #[test]
    fn dropping_last_service_owner_closes_retained_subscription() {
        let service = service();
        let mut subscription = service.subscribe().unwrap();
        drop(service);
        assert_eq!(
            ready(subscription.next()).unwrap_err(),
            usher_state::SubscriptionEnd::Closed
        );
    }
}
