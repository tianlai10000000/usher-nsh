//! Process-owned global registration state, exclusive VM leases and versioned health.
use super::{
    Command,
    catalog::{LoadedModule, ModuleKind},
    loader::ModuleLoader,
    service::{BindingHealth, Health, ModuleHealth, WireEffect},
};
use nsh_core::platform::Os;
use std::{
    collections::{BTreeMap, BTreeSet},
    sync::{Arc, Mutex},
};
use usher_shortcuts::{
    Catalog, Effect, Failure, OwnerToken, Receipt, Runtime, ScopeRelations, Status, Ticket,
};
use usher_state::{
    Snapshot, StoreReader, StoreWriter, Subscription, backends::WatchBackend, create_store,
};

#[derive(Clone)]
struct Machine {
    runtime: Runtime,
    owner: Option<String>,
    retired: bool,
    revision: u64,
    sequence: u64,
    modules: BTreeMap<String, LoadedModule>,
    tickets: BTreeMap<String, Ticket>,
    health: Health,
}
struct Inner {
    machine: Machine,
    writer: StoreWriter<Health>,
}
/// One coordinator per process, independent of window and settings-domain identities.
pub struct GlobalService {
    inner: Mutex<Inner>,
    reader: StoreReader<Health>,
    loader: Arc<ModuleLoader>,
    os: Os,
    attempts: usize,
}
impl GlobalService {
    /// Creates an idle coordinator; claiming a VM lease does not itself register keys.
    pub fn new(loader: Arc<ModuleLoader>, os: Os, attempts: usize) -> Result<Arc<Self>, String> {
        let id = crate::state::identity::next_store_id()?;
        let modules: Vec<_> = loader
            .index()
            .descriptors()
            .filter(|d| d.kind == ModuleKind::Global)
            .map(|d| ModuleHealth {
                id: d.id.clone(),
                kind: d.kind.clone(),
                page: None,
                phase: "validating".into(),
                error: None,
            })
            .collect();
        let health = Health {
            startup: "notRequested".into(),
            page: "notRequested".into(),
            global: if modules.is_empty() {
                "notRequested"
            } else {
                "validating"
            }
            .into(),
            entry: None,
            modules,
            bindings: vec![],
        };
        let (writer, reader) = create_store(id, health.clone(), &WatchBackend)
            .map_err(|_| "shortcuts_publish_failed")?;
        Ok(Arc::new(Self {
            inner: Mutex::new(Inner {
                machine: Machine {
                    runtime: Runtime::new(attempts).map_err(|_| "shortcuts_invalid_policy")?,
                    owner: None,
                    retired: false,
                    revision: 0,
                    sequence: 0,
                    modules: BTreeMap::new(),
                    tickets: BTreeMap::new(),
                    health,
                },
                writer,
            }),
            reader,
            loader,
            os,
            attempts,
        }))
    }
    /// Reads health without requiring execution ownership.
    pub fn snapshot(&self) -> Result<Snapshot<Health>, String> {
        self.reader
            .snapshot()
            .map_err(|_| "shortcuts_closed".into())
    }
    /// Every window observes the same process-global health store.
    pub fn subscribe(&self) -> Result<Subscription<Health>, String> {
        self.reader
            .subscribe()
            .map_err(|_| "shortcuts_closed".into())
    }
    fn change(
        &self,
        run: impl FnOnce(&mut Machine) -> Result<Vec<Effect>, String>,
    ) -> Result<Vec<WireEffect>, String> {
        let mut inner = self.inner.lock().map_err(|_| "shortcuts_closed")?;
        let mut next = inner.machine.clone();
        let effects = run(&mut next)?;
        Self::project(&mut next);
        let mut wire = Vec::new();
        for effect in effects {
            let (kind, ticket) = match effect {
                Effect::Install(t) => ("install", t),
                Effect::Release(t) => ("release", t),
            };
            let token = if let Some((id, _)) = next.tickets.iter().find(|(_, t)| **t == ticket) {
                id.clone()
            } else {
                next.sequence = next.sequence.checked_add(1).ok_or("shortcuts_exhausted")?;
                let id = next.sequence.to_string();
                next.tickets.insert(id.clone(), ticket.clone());
                id
            };
            let binding = &ticket.desired().binding;
            wire.push(WireEffect {
                kind: kind.into(),
                ticket: token,
                binding: binding.id.as_str().into(),
                keystroke: binding.resource.stroke.as_str().into(),
                editing: "always".into(),
                repeat: true,
                pass_through: false,
            });
        }
        let live: Vec<_> = next
            .runtime
            .snapshot()
            .into_iter()
            .map(|s| s.ticket)
            .collect();
        next.tickets.retain(|_, t| live.contains(t));
        if inner.machine.health != next.health {
            let health = next.health.clone();
            inner
                .writer
                .publish(health)
                .map_err(|_| "shortcuts_publish_failed")?;
        }
        inner.machine = next;
        Ok(wire)
    }
    /// Claims exclusive execution; other VMs may observe but cannot register or acknowledge.
    /// After VM death only a verified host reset may discard unresolved native ownership.
    pub fn claim(&self, owner: &str, reset_verified: bool) -> Result<bool, String> {
        let mut claimed = false;
        self.change(|m| {
            if m.owner.as_deref() == Some(owner) {
                claimed = true;
                return Ok(vec![]);
            }
            if m.owner.is_some() {
                return Ok(vec![]);
            }
            if m.retired && !m.runtime.is_disposed() && !reset_verified {
                return Ok(vec![]);
            }
            if m.retired {
                m.runtime = Runtime::new(self.attempts).map_err(|_| "shortcuts_invalid_policy")?;
                m.tickets.clear();
                m.modules.clear();
                for h in &mut m.health.modules {
                    h.phase = "validating".into();
                    h.error = None;
                }
            }
            m.owner = Some(owner.into());
            m.retired = false;
            claimed = true;
            Ok(vec![])
        })?;
        Ok(claimed)
    }
    fn owned(m: &Machine, owner: &str) -> Result<(), String> {
        if !m.retired && m.owner.as_deref() == Some(owner) {
            Ok(())
        } else {
            Err("shortcuts_stale_executor".into())
        }
    }
    /// Loads global declarations at application startup, never page factories.
    pub fn initialize(&self, owner: &str) -> Result<Vec<WireEffect>, String> {
        self.change(|m| {
            Self::owned(m, owner)?;
            if !self.os.is_desktop() {
                for h in &mut m.health.modules {
                    h.phase = "unsupported".into();
                }
                return Ok(vec![]);
            }
            for descriptor in self
                .loader
                .index()
                .descriptors()
                .filter(|d| d.kind == ModuleKind::Global)
            {
                let result = self
                    .loader
                    .load(&descriptor.id, self.os)
                    .and_then(|module| {
                        if module
                            .commands
                            .values()
                            .any(|c| !matches!(c, Command::Window(_) | Command::Appearance(_)))
                        {
                            return Err(super::Error::InvalidConfig(
                                "global command requires process ownership".into(),
                            ));
                        }
                        Ok(module)
                    });
                let health = m
                    .health
                    .modules
                    .iter_mut()
                    .find(|h| h.id == descriptor.id)
                    .ok_or("shortcuts_unknown_module")?;
                match result {
                    Ok(module) => {
                        m.modules.insert(descriptor.id.clone(), module);
                        health.phase = "installing".into();
                        health.error = None;
                    }
                    Err(_) => {
                        health.phase = "failed".into();
                        health.error = Some("shortcuts_module_failed".into());
                    }
                }
            }
            match self.plan(m) {
                Ok(effects) => Ok(effects),
                Err(_) => {
                    for h in &mut m.health.modules {
                        h.phase = "failed".into();
                        h.error = Some("shortcuts_conflict".into());
                    }
                    Ok(vec![])
                }
            }
        })
    }
    fn plan(&self, m: &mut Machine) -> Result<Vec<Effect>, String> {
        let bindings: Vec<_> = m
            .modules
            .values()
            .flat_map(|v| v.bindings.clone())
            .collect();
        // Startup declarations are cheap; pages remain lazy and validate before local installation.
        let mut validation = bindings.clone();
        for d in self
            .loader
            .index()
            .descriptors()
            .filter(|d| d.kind == ModuleKind::Application)
        {
            validation.extend(
                self.loader
                    .load(&d.id, self.os)
                    .map_err(|_| "shortcuts_invalid_catalog")?
                    .bindings,
            );
        }
        let scopes = BTreeSet::from_iter(validation.iter().map(|b| b.scope.clone()));
        Catalog::new(
            m.revision,
            validation,
            ScopeRelations::new(scopes, vec![]).map_err(|_| "shortcuts_conflict")?,
        )
        .map_err(|_| "shortcuts_conflict")?;
        let scopes = BTreeSet::from_iter(bindings.iter().map(|b| b.scope.clone()));
        let enabled = BTreeSet::from_iter(bindings.iter().map(|b| b.id.clone()));
        let catalog = Catalog::new(
            m.revision,
            bindings,
            ScopeRelations::new(scopes.clone(), vec![]).map_err(|_| "shortcuts_conflict")?,
        )
        .map_err(|_| "shortcuts_conflict")?;
        let plan = catalog
            .plan(
                &scopes,
                &enabled,
                OwnerToken::new(m.owner.clone().ok_or("shortcuts_stale_executor")?)
                    .map_err(|_| "shortcuts_invalid_owner")?,
            )
            .map_err(|_| "shortcuts_conflict")?;
        m.runtime
            .reconcile(m.revision, plan)
            .map_err(|_| "shortcuts_registration_failed".into())
    }
    /// Acknowledges only the current VM's exact ticket; late receipts cannot replace newer owners.
    pub fn acknowledge(
        &self,
        owner: &str,
        token: &str,
        success: bool,
        release: bool,
    ) -> Result<Vec<WireEffect>, String> {
        self.change(|m| {
            Self::owned(m, owner)?;
            let Some(ticket) = m.tickets.get(token) else {
                return Ok(vec![]);
            };
            let receipt = match (release, success) {
                (false, true) => Receipt::Installed,
                (false, false) => Receipt::InstallFailed(Failure::Rejected),
                (true, true) => Receipt::Released,
                (true, false) => Receipt::ReleaseFailed(Failure::Unknown),
            };
            m.runtime
                .acknowledge(ticket, receipt)
                .map_err(|_| "shortcuts_invalid_receipt".into())
        })
    }
    /// Verifies confirmation and VM identity before dispatching a process-owned business intent.
    pub fn trigger(&self, owner: &str, token: &str) -> Result<Option<Command>, String> {
        let inner = self.inner.lock().map_err(|_| "shortcuts_closed")?;
        let m = &inner.machine;
        if Self::owned(m, owner).is_err() {
            return Ok(None);
        }
        let Some(ticket) = m.tickets.get(token) else {
            return Ok(None);
        };
        if !m.runtime.accepts(ticket) {
            return Ok(None);
        }
        Ok(m.modules
            .values()
            .find_map(|module| module.commands.get(&ticket.desired().binding.id).cloned()))
    }
    /// Retries explicitly; unresolved installs remain pending and retain exclusive ownership.
    pub fn retry(&self, owner: &str) -> Result<Vec<WireEffect>, String> {
        self.change(|m| {
            Self::owned(m, owner)?;
            if m.runtime
                .snapshot()
                .iter()
                .any(|s| matches!(s.status, Status::Installing | Status::Releasing))
            {
                return Ok(vec![]);
            }
            let mut effects = vec![];
            for slot in m.runtime.snapshot() {
                if matches!(slot.status, Status::ReleaseFailed(_)) {
                    effects.extend(
                        m.runtime
                            .retry(&slot.ticket)
                            .map_err(|_| "shortcuts_retry_failed")?,
                    );
                }
            }
            m.revision = m.revision.checked_add(1).ok_or("shortcuts_exhausted")?;
            for h in &mut m.health.modules {
                if m.modules.contains_key(&h.id) && h.error.is_none() {
                    h.phase = "installing".into();
                }
            }
            effects.extend(self.plan(m)?);
            Ok(effects)
        })
    }
    /// VM teardown revokes callbacks immediately. Host reset, not a timeout, permits takeover.
    pub fn abandon(&self, owner: &str) {
        let _ = self.change(|m| {
            if m.owner.as_deref() != Some(owner) {
                return Ok(vec![]);
            }
            m.owner = None;
            m.retired = true;
            let _ = m
                .runtime
                .dispose()
                .map_err(|_| "shortcuts_release_failed")?;
            Ok(vec![])
        });
    }
    fn project(m: &mut Machine) {
        let slots = m.runtime.snapshot();
        m.health.bindings = m
            .modules
            .values()
            .flat_map(|module| {
                module.bindings.iter().map(|b| {
                    let state = slots
                        .iter()
                        .find(|s| s.ticket.desired().binding.id == b.id)
                        .map(|s| &s.status);
                    BindingHealth {
                        id: b.id.as_str().into(),
                        module: module.descriptor.id.clone(),
                        keystroke: b.resource.stroke.as_str().into(),
                        state: match state {
                            Some(Status::Ready) => "registered",
                            Some(Status::Installing) => "installing",
                            Some(Status::Releasing) => "releasing",
                            Some(Status::Failed(_) | Status::ReleaseFailed(_)) => "failed",
                            None => "inactive",
                        }
                        .into(),
                    }
                })
            })
            .collect();
        for h in &mut m.health.modules {
            if h.error.is_none() && h.phase != "unsupported" && m.modules.contains_key(&h.id) {
                let bindings: Vec<_> = m
                    .health
                    .bindings
                    .iter()
                    .filter(|b| b.module == h.id)
                    .collect();
                h.phase = if bindings.iter().any(|b| b.state == "failed") {
                    "failed"
                } else if bindings
                    .iter()
                    .any(|b| b.state == "installing" || b.state == "releasing")
                {
                    "installing"
                } else {
                    "ready"
                }
                .into();
            }
        }
        m.health.global = if m.health.modules.is_empty() {
            "notRequested"
        } else if m.retired {
            "recovering"
        } else if m.health.modules.iter().any(|h| h.phase == "failed") {
            "failed"
        } else if m.health.modules.iter().any(|h| h.phase == "unsupported") {
            "unsupported"
        } else if m.health.modules.iter().all(|h| h.phase == "ready") {
            "ready"
        } else if m.health.modules.iter().any(|h| h.phase == "validating") {
            "validating"
        } else {
            "installing"
        }
        .into();
    }
}
