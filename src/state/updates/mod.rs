//! Publishes update policy through usher-state and fences one application-owned effect executor.
//!
//! Consumers borrow `reader()` and send user commands. Only the controller holding a
//! lease may claim effects or report results. No network, file, NSH or timer work runs here.
//!
//! There is no cancel and no executor handover (docs/update.md, 8.2, option B): NSH cannot cancel
//! an update operation, so the service does not model stopping one. The lease is held until the
//! application scope closes. Borrowed windows do not own the service. Destroying the executor
//! VM requires a process restart; a new view must not impersonate the previous executor.
mod execution;
pub(crate) mod transport;
pub use execution::{Claim, Completion, Lease};

use std::{collections::VecDeque, sync::Mutex};
use usher_state::{
    BackendFactory, Snapshot, StoreId, StoreReader, StoreWriter, Transition, create_store,
};
use usher_update::state::{Effect, Event, Machine, Settings, Trigger};

/// Stable application errors; transports must preserve the codes rather than format domain data.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum Error {
    Closed,
    Unavailable,
    Busy,
    Stale,
    Invalid,
    Exhausted,
    /// The service exists but its machine has not been created yet: the installed shell's
    /// version is not known until the controller reports it (docs/update.md, 8.1).
    NotConfigured,
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
            Self::NotConfigured => "updates_not_configured",
        }
    }
}
impl std::fmt::Display for Error {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(self.code())
    }
}
impl std::error::Error for Error {}
/// User intent is separate from executor completion, so consumers cannot forge a result.
#[derive(Clone, Debug)]
pub enum Command {
    /// Creates the machine. Allowed once: the shell's version does not change while the process
    /// lives, and a code package restart is a new process.
    Configure(Settings),
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
/// Process-local runtime identity and check history shared by every update view.
#[derive(Clone, Debug, Default, Eq, PartialEq)]
pub struct RuntimeInfo {
    pub shell_build: Option<String>,
    pub package_build: Option<String>,
    pub last_checked: Option<String>,
    pub last_result: Option<String>,
    pub manual_url: Option<String>,
    pub manual_check: bool,
}

/// Read-only domain, runtime metadata and delivery status; leases are never broadcast.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct State {
    pub info: RuntimeInfo,
    /// `None` until the service is configured. A consumer shows "preparing", never "up to date".
    pub domain: Option<Machine>,
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
    info: RuntimeInfo,
    domain: Option<Machine>,
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
            info: self.info.clone(),
            domain: self.domain.clone(),
            pending: self.queue.len(),
            execution: if self.active.is_some() {
                Execution::Running
            } else {
                Execution::Available
            },
        }
    }
    fn machine(&mut self) -> Result<&mut Machine, Error> {
        self.domain.as_mut().ok_or(Error::NotConfigured)
    }
    fn event(&mut self, event: Event) -> Result<(), Error> {
        if let Event::CheckRequested(trigger) = &event {
            let machine = self.domain.as_ref().ok_or(Error::NotConfigured)?;
            if !machine.candidate_locked() && (*trigger == Trigger::Manual || machine.auto_check())
            {
                if machine.phase != usher_update::state::Phase::Checking {
                    self.info.manual_check = *trigger == Trigger::Manual;
                } else if *trigger == Trigger::Manual {
                    self.info.manual_check = true;
                }
            }
        }
        let machine = self.machine()?;
        // The domain uses u64 tickets. Reject before its unchecked increment.
        if machine.ticket() == u64::MAX {
            return Err(Error::Exhausted);
        }
        let effects = machine.handle(event);
        for effect in effects {
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
    /// Creates the service with an injected backend, not yet configured: it holds no machine
    /// until `Command::Configure` arrives. Recovery from a previous process belongs to the host
    /// and must not silently replay effects, so a configured machine always starts idle.
    pub fn new(factory: &dyn BackendFactory<State>) -> Result<Self, Error> {
        let id = crate::state::identity::next_store_id().map_err(|_| Error::Exhausted)?;
        let model = Model {
            info: RuntimeInfo::default(),
            domain: None,
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
    /// Configures runtime identity and domain together before any executor can acquire.
    pub(crate) fn configure_runtime(
        &self,
        settings: Settings,
        info: RuntimeInfo,
    ) -> Result<(), Error> {
        self.change(|model| {
            if model.domain.is_some() || settings.failure_limit == 0 {
                return Err(Error::Invalid);
            }
            model.domain = Some(Machine::new(settings));
            model.info = info;
            Ok(())
        })
    }
    /// Serializes user intent without executing effects or accepting completion events.
    pub fn request(&self, command: Command) -> Result<(), Error> {
        self.change(|model| {
            let event = match command {
                Command::Configure(settings) => {
                    if model.domain.is_some() || settings.failure_limit == 0 {
                        return Err(Error::Invalid);
                    }
                    model.domain = Some(Machine::new(settings));
                    return Ok(());
                }
                Command::Check(trigger) => Event::CheckRequested(trigger),
                Command::Download => Event::DownloadRequested,
                Command::Install => Event::InstallRequested,
                Command::ConfirmFirstLaunch => Event::FirstLaunchConfirmed,
                Command::AutoCheck(on) => {
                    // Not a domain event: it only changes what the next check does.
                    model.machine()?.set_auto_check(on);
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
    /// Only the owning application/session may close this service, never a borrowed view.
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
    use usher_update::state::{Capability, Phase, Settings};

    fn settings() -> Settings {
        Settings {
            shell: "1.0.0".parse().unwrap(),
            running: "1.0.0".parse().unwrap(),
            capability: Capability::InApp,
            auto_check: true,
            failure_limit: 2,
        }
    }
    /// A service with its machine created, as it is once the controller has configured it.
    fn service() -> UpdateService {
        let service = unconfigured();
        service.request(Command::Configure(settings())).unwrap();
        service
    }
    fn unconfigured() -> UpdateService {
        UpdateService::new(&WatchBackend).unwrap()
    }
    fn phase(service: &UpdateService) -> Option<Phase> {
        service
            .snapshot()
            .unwrap()
            .state()
            .domain
            .as_ref()
            .map(|machine| machine.phase.clone())
    }

    #[test]
    fn a_new_service_is_not_configured_and_says_so_instead_of_claiming_it_is_up_to_date() {
        let service = unconfigured();
        let snapshot = service.snapshot().unwrap();
        assert!(snapshot.state().domain.is_none());
        assert_eq!(snapshot.state().pending, 0);
        // It can be read and subscribed to before it is configured: the interface needs that to
        // show "preparing".
        service.subscribe().unwrap();
        service.reader().snapshot().unwrap();
    }
    #[test]
    fn everything_that_needs_a_machine_is_refused_until_it_is_configured() {
        let service = unconfigured();
        for command in [
            Command::Check(Trigger::Manual),
            Command::Check(Trigger::Background),
            Command::Download,
            Command::Install,
            Command::ConfirmFirstLaunch,
            Command::AutoCheck(false),
        ] {
            assert_eq!(
                service.request(command.clone()),
                Err(Error::NotConfigured),
                "{command:?}"
            );
        }
        assert_eq!(Error::NotConfigured.code(), "updates_not_configured");
        // Nothing was published by the refusals.
        assert_eq!(service.snapshot().unwrap().sequence(), 0);
        assert_eq!(service.acquire(), Err(Error::NotConfigured));
        let foreign = super::tests::service().acquire().unwrap();
        assert_eq!(service.claim(foreign), Err(Error::NotConfigured));
    }
    #[test]
    fn configuring_creates_an_idle_machine_and_is_published_to_subscribers() {
        let service = unconfigured();
        let mut subscription = service.subscribe().unwrap();
        let before = service.snapshot().unwrap().sequence();
        service.request(Command::Configure(settings())).unwrap();
        let snapshot = ready(subscription.next()).unwrap();
        assert!(snapshot.sequence() > before);
        let machine = snapshot.state().domain.as_ref().expect("configured");
        assert_eq!(machine.phase, Phase::Idle);
        assert_eq!(machine.ticket(), 0);
        assert_eq!(machine.shell().to_string(), "1.0.0");
        assert!(machine.auto_check());
    }
    #[test]
    fn a_service_can_be_configured_once_and_a_second_configuration_changes_nothing() {
        let service = service();
        let sequence = service.snapshot().unwrap().sequence();
        let mut other = settings();
        other.shell = "9.0.0".parse().unwrap();
        assert_eq!(
            service.request(Command::Configure(other)),
            Err(Error::Invalid)
        );
        assert_eq!(service.snapshot().unwrap().sequence(), sequence);
        assert_eq!(
            service
                .snapshot()
                .unwrap()
                .state()
                .domain
                .as_ref()
                .unwrap()
                .shell()
                .to_string(),
            "1.0.0"
        );
    }
    #[test]
    fn the_unconfigured_service_closes_like_any_other() {
        let service = unconfigured();
        let mut subscription = service.subscribe().unwrap();
        service.close();
        assert_eq!(
            ready(subscription.next()).unwrap_err(),
            usher_state::SubscriptionEnd::Closed
        );
        assert_eq!(
            service.request(Command::Configure(settings())),
            Err(Error::Closed)
        );
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
        let sequence = service.snapshot().unwrap().sequence();
        assert_eq!(service.snapshot().unwrap().sequence(), sequence);
        assert_eq!(phase(&service), Some(Phase::Idle));
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

pub(crate) mod preferences;
