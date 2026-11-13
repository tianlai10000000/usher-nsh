//! Owns one engine/window store for routing and page session data, with guarded commits.
pub mod session;
use serde::Serialize;
use session::{Session, SessionCommand};
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
    pub pending: Option<String>,
    pub navigation_result: NavigationResult,
}
struct Pending {
    replace: bool,
    id: u64,
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
    next_operation: u64,
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
pub struct RouterService {
    inner: Mutex<Inner>,
    reader: StoreReader<WindowSnapshot>,
}
impl RouterService {
    pub fn new(catalog: Catalog) -> Result<Arc<Self>, &'static str> {
        let router = Router::new(catalog, &Matchit).map_err(|_| "router_invalid_catalog")?;
        let history = router.initial().map_err(|_| "router_invalid_catalog")?;
        let (writer, reader) = create_store(
            crate::app_state::next_store_id()?,
            WindowSnapshot {
                history,
                session: Session::default(),
                pending: None,
                navigation_result: NavigationResult::Committed,
            },
            &WatchBackend,
        )
        .map_err(|_| "router_unavailable")?;
        Ok(Arc::new(Self {
            inner: Mutex::new(Inner {
                router,
                writer,
                pending: None,
                next_navigation: 0,
                next_operation: 0,
                closed: false,
                retired_catalogs: BTreeSet::new(),
            }),
            reader,
        }))
    }
    fn lock(&self) -> Result<std::sync::MutexGuard<'_, Inner>, &'static str> {
        let inner = self.inner.lock().map_err(|_| "router_unavailable")?;
        if inner.closed {
            return Err("router_closed");
        }
        Ok(inner)
    }
    pub fn snapshot(&self) -> Result<Snapshot<WindowSnapshot>, &'static str> {
        let _inner = self.lock()?;
        self.reader.snapshot().map_err(|_| "router_unavailable")
    }
    pub fn subscribe(&self) -> Result<Subscription<WindowSnapshot>, &'static str> {
        let _inner = self.lock()?;
        self.reader.subscribe().map_err(|_| "router_unavailable")
    }
    /// Registration is idempotent for the exact catalog. A reused identity with
    /// different content is rejected; replacement is an explicit separate operation.
    pub fn register(&self, catalog: &Catalog) -> Result<(), &'static str> {
        if self.lock()?.router.catalog() == catalog {
            Ok(())
        } else {
            Err("router_catalog_mismatch")
        }
    }
    /// Atomically replaces a catalog and fences pending guards. Old identities cannot
    /// reattach after HMR; a reused identity must retain exactly the same content.
    pub fn replace_catalog(&self, catalog: Catalog) -> Result<(), &'static str> {
        let mut inner = self.lock()?;
        if inner.router.catalog() == &catalog {
            return Ok(());
        }
        if inner.router.catalog().identity == catalog.identity
            || inner.retired_catalogs.contains(&catalog.identity)
        {
            return Err("router_catalog_mismatch");
        }
        let router = Router::new(catalog, &Matchit).map_err(|_| "router_invalid_catalog")?;
        let current = inner.writer.snapshot().map_err(|_| "router_unavailable")?;
        let mut next = current.state().clone();
        next.history = router
            .migrate_history(&next.history)
            .map_err(|_| "router_invalid_catalog")?;
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
    pub fn resolve(&self, target: &Target) -> Result<usher_router::Location, &'static str> {
        self.lock()?
            .router
            .resolve(target)
            .map_err(|_| "router_invalid_target")
    }
    /// Starts the latest navigation. The old page remains committed while guards run.
    pub fn begin(&self, command: NavigationCommand) -> Result<String, &'static str> {
        let mut inner = self.lock()?;
        let current = inner.writer.snapshot().map_err(|_| "router_unavailable")?;
        let outcome = inner
            .router
            .transition(&current.state().history, &command)
            .map_err(|_| "router_invalid_target")?;
        let id = inner
            .next_navigation
            .checked_add(1)
            .ok_or("router_capacity_exhausted")?;
        let pending = Pending {
            replace: matches!(command, NavigationCommand::Replace(_)),
            id,
            source_revision: current.state().history.revision(),
            visited: BTreeSet::from([outcome.state.current().location.url()]),
            redirects: 0,
            outcome,
        };
        let mut next = current.state().clone();
        next.pending = Some(id.to_string());
        Self::publish(&mut inner, next)?;
        inner.next_navigation = id;
        inner.pending = Some(pending);
        Ok(id.to_string())
    }
    /// Rejects old tickets after supersession/close. Guard cancellation never moves history.
    pub fn complete(
        &self,
        ticket: &str,
        decision: Decision,
    ) -> Result<Option<NavigationResult>, &'static str> {
        let mut inner = self.lock()?;
        let Some(pending) = inner
            .pending
            .as_ref()
            .filter(|p| p.id.to_string() == ticket)
        else {
            return Ok(Some(NavigationResult::Superseded));
        };
        let current = inner.writer.snapshot().map_err(|_| "router_unavailable")?;
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
                .map_err(|_| "router_invalid_target")?;
            let limit = inner.router.catalog().redirect_limit;
            let pending = inner.pending.as_mut().expect("ticket was checked");
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
    pub fn navigate(&self, command: NavigationCommand) -> Result<NavigationResult, &'static str> {
        let ticket = self.begin(command)?;
        self.complete(&ticket, Decision::Allow)?
            .ok_or("router_pending")
    }
    pub fn session(&self, command: SessionCommand) -> Result<String, &'static str> {
        let mut inner = self.lock()?;
        let mut next = inner
            .writer
            .snapshot()
            .map_err(|_| "router_unavailable")?
            .state()
            .clone();
        let mut counter = inner.next_operation;
        let receipt = next.session.apply(command, &mut counter)?;
        Self::publish(&mut inner, next)?;
        inner.next_operation = counter;
        Ok(receipt)
    }
    fn publish(inner: &mut Inner, next: WindowSnapshot) -> Result<(), &'static str> {
        let current = inner.writer.snapshot().map_err(|_| "router_unavailable")?;
        if *current.state() != next && inner.writer.publish(next).is_err() {
            inner.closed = true;
            inner.writer.close();
            return Err("router_unavailable");
        }
        Ok(())
    }
    pub fn close(&self) {
        if let Ok(mut inner) = self.inner.lock() {
            inner.closed = true;
            inner.pending = None;
            inner.writer.close();
        }
    }
}
