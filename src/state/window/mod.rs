//! Owns one engine/window store for routing and page session data, with guarded commits.
mod error;
use crate::state::window::session::{Receipt, Session, SessionCommand, Ticket};
pub use error::Error;
use serde::Serialize;
use std::{
    collections::BTreeSet,
    sync::{Arc, Mutex},
};
use usher_router::{
    Catalog, History, NavigationCommand, NavigationResult, Outcome, Router, Target,
    matcher::Matchit,
};
use usher_state::{
    Snapshot, StoreReader, StoreWriter, Subscription, backends::WatchBackend, create_store,
};

#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct WindowSnapshot {
    pub history: History,
    pub session: Session,
    pub pending: Option<Ticket>,
    pub navigation_result: NavigationResult,
}
#[derive(Clone)]
struct Pending {
    replace: bool,
    id: Ticket,
    source_revision: u64,
    outcome: Outcome,
    visited: BTreeSet<String>,
    redirects: usize,
}
struct Inner {
    router: Router,
    writer: StoreWriter<WindowSnapshot>,
    pending: Option<Pending>,
    next_navigation: u64,
    closed: bool,
    retired_catalogs: BTreeSet<String>,
}
/// Application-only guard reply; no UI closure is stored inside a snapshot.
pub enum Decision {
    Allow,
    Cancel,
    Fail,
    Redirect(Target),
}
/// Owns one window lifetime and serializes navigation and session snapshot commits.
pub struct WindowService {
    inner: Mutex<Inner>,
    reader: StoreReader<WindowSnapshot>,
}
impl WindowService {
    pub fn new(catalog: Catalog) -> Result<Arc<Self>, Error> {
        Self::with_backend(catalog, &WatchBackend)
    }
    /// Injects a backend with atomic failed-publication semantics; failures can be retried.
    pub fn with_backend(
        catalog: Catalog,
        factory: &dyn usher_state::BackendFactory<WindowSnapshot>,
    ) -> Result<Arc<Self>, Error> {
        let router = Router::new(catalog, &Matchit).map_err(Error::Catalog)?;
        let history = router.initial().map_err(Error::Catalog)?;
        let (writer, reader) = create_store(
            crate::state::identity::next_store_id().map_err(Error::Identity)?,
            WindowSnapshot {
                history,
                session: Session::default(),
                pending: None,
                navigation_result: NavigationResult::Committed,
            },
            factory,
        )
        .map_err(Error::Store)?;
        Ok(Arc::new(Self {
            inner: Mutex::new(Inner {
                router,
                writer,
                pending: None,
                next_navigation: 0,
                closed: false,
                retired_catalogs: BTreeSet::new(),
            }),
            reader,
        }))
    }
    fn lock(&self) -> Result<std::sync::MutexGuard<'_, Inner>, Error> {
        let inner = self.inner.lock().map_err(|_| Error::Poisoned)?;
        if inner.closed {
            return Err(Error::Closed);
        }
        Ok(inner)
    }
    pub fn snapshot(&self) -> Result<Snapshot<WindowSnapshot>, Error> {
        let _inner = self.lock()?;
        self.reader.snapshot().map_err(Error::Store)
    }
    pub fn subscribe(&self) -> Result<Subscription<WindowSnapshot>, Error> {
        let _inner = self.lock()?;
        self.reader.subscribe().map_err(Error::Store)
    }
    /// Registration is idempotent for the exact catalog. A reused identity with
    /// different content is rejected; replacement is an explicit separate operation.
    pub fn register(&self, catalog: &Catalog) -> Result<(), Error> {
        if self.lock()?.router.catalog() == catalog {
            Ok(())
        } else {
            Err(Error::CatalogMismatch)
        }
    }
    /// Atomically replaces a catalog and fences pending guards. Old identities cannot
    /// reattach after HMR; a reused identity must retain exactly the same content.
    pub fn replace_catalog(&self, catalog: Catalog) -> Result<(), Error> {
        let mut inner = self.lock()?;
        if inner.router.catalog() == &catalog {
            return Ok(());
        }
        if inner.router.catalog().identity == catalog.identity
            || inner.retired_catalogs.contains(&catalog.identity)
        {
            return Err(Error::CatalogMismatch);
        }
        let router = Router::new(catalog, &Matchit).map_err(Error::Catalog)?;
        let current = inner.writer.snapshot().map_err(Error::Store)?;
        let mut next = current.state().clone();
        next.history = router
            .migrate_history(&next.history)
            .map_err(Error::Catalog)?;
        next.navigation_result = if next.pending.is_some() {
            NavigationResult::Superseded
        } else if next.history.current().location.id.is_none() {
            NavigationResult::NotFound
        } else {
            NavigationResult::Committed
        };
        next.pending = None;
        Self::publish(&mut inner, next)?;
        let previous = inner.router.catalog().identity.clone();
        inner.retired_catalogs.insert(previous);
        inner.router = router;
        inner.pending = None;
        Ok(())
    }
    pub fn resolve(&self, target: &Target) -> Result<usher_router::Location, Error> {
        self.lock()?.router.resolve(target).map_err(Error::Target)
    }
    /// Starts the latest navigation. The old page remains committed while guards run.
    pub fn begin(&self, command: NavigationCommand) -> Result<Ticket, Error> {
        let mut inner = self.lock()?;
        let current = inner.writer.snapshot().map_err(Error::Store)?;
        let outcome = inner
            .router
            .transition(&current.state().history, &command)
            .map_err(Error::Target)?;
        let id = inner
            .next_navigation
            .checked_add(1)
            .ok_or(Error::CapacityExhausted)?;
        let pending = Pending {
            replace: matches!(command, NavigationCommand::Replace(_)),
            id: Ticket::new(id),
            source_revision: current.state().history.revision(),
            visited: BTreeSet::from([outcome.state.current().location.url()]),
            redirects: 0,
            outcome,
        };
        let mut next = current.state().clone();
        next.pending = Some(Ticket::new(id));
        Self::publish(&mut inner, next)?;
        inner.next_navigation = id;
        inner.pending = Some(pending);
        Ok(Ticket::new(id))
    }
    /// Rejects old tickets after supersession/close. Guard cancellation never moves history.
    pub fn complete(
        &self,
        ticket: &Ticket,
        decision: Decision,
    ) -> Result<Option<NavigationResult>, Error> {
        let mut inner = self.lock()?;
        let Some(mut pending) = inner.pending.as_ref().filter(|p| &p.id == ticket).cloned() else {
            return Ok(Some(NavigationResult::Superseded));
        };
        let current = inner.writer.snapshot().map_err(Error::Store)?;
        if pending.source_revision != current.state().history.revision() {
            return Ok(Some(NavigationResult::Superseded));
        }
        if let Decision::Redirect(target) = decision {
            let command = if pending.replace {
                NavigationCommand::Replace(target)
            } else {
                NavigationCommand::Push(target)
            };
            let outcome = inner
                .router
                .transition(&current.state().history, &command)
                .map_err(Error::Target)?;
            let limit = inner.router.catalog().redirect_limit;
            if pending.redirects >= limit
                || !pending
                    .visited
                    .insert(outcome.state.current().location.url())
            {
                let mut next = current.state().clone();
                next.pending = None;
                next.navigation_result = NavigationResult::Failed;
                Self::publish(&mut inner, next)?;
                inner.pending = None;
                return Ok(Some(NavigationResult::Failed));
            }
            pending.outcome = outcome;
            pending.redirects += 1;
            inner.pending = Some(pending);
            return Ok(None);
        }
        let mut next = current.state().clone();
        let result = match decision {
            Decision::Allow => {
                next.history = pending.outcome.state.clone();
                pending.outcome.result
            }
            Decision::Cancel => NavigationResult::Cancelled,
            Decision::Fail => NavigationResult::Failed,
            Decision::Redirect(_) => unreachable!(),
        };
        next.pending = None;
        next.navigation_result = result;
        Self::publish(&mut inner, next)?;
        inner.pending = None;
        Ok(Some(result))
    }
    pub fn navigate(&self, command: NavigationCommand) -> Result<NavigationResult, Error> {
        let ticket = self.begin(command)?;
        self.complete(&ticket, Decision::Allow)?
            .ok_or(Error::Pending)
    }
    pub fn session(&self, command: SessionCommand) -> Result<Receipt, Error> {
        let mut inner = self.lock()?;
        let mut next = inner
            .writer
            .snapshot()
            .map_err(Error::Store)?
            .state()
            .clone();
        let receipt = next.session.apply(command).map_err(Error::Session)?;
        Self::publish(&mut inner, next)?;
        Ok(receipt)
    }
    fn publish(inner: &mut Inner, next: WindowSnapshot) -> Result<(), Error> {
        let current = inner.writer.snapshot().map_err(Error::Store)?;
        if *current.state() != next {
            // Both the candidate and pending metadata remain private until publication succeeds.
            inner.writer.publish(next).map_err(Error::Store)?;
        }
        Ok(())
    }
    pub fn close(&self) {
        let mut inner = self.inner.lock().unwrap_or_else(|error| error.into_inner());
        inner.closed = true;
        inner.pending = None;
        inner.writer.close();
    }
}

pub mod session;
