//! Window-owned shortcut orchestration and authoritative health publication, independent of layout.
use super::{
    Command,
    catalog::{LoadedModule, ModuleKind},
    loader::ModuleLoader,
    modules::navigation,
};
use crate::state::window::WindowService;
use nsh_core::platform::Os;
use serde::{Deserialize, Serialize};
use std::{
    collections::{BTreeMap, BTreeSet},
    sync::{Arc, Mutex},
};
use usher_shortcuts::{
    Catalog, EditingPolicy, Effect, Failure, OwnerToken, Receipt, Runtime, ScopeRelations, Status,
    Ticket,
};
use usher_state::{
    Snapshot, StoreReader, StoreWriter, Subscription, backends::WatchBackend, create_store,
};

#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ModuleHealth {
    pub id: String,
    pub kind: ModuleKind,
    pub page: Option<String>,
    pub phase: String,
    pub error: Option<String>,
}
#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct BindingHealth {
    pub id: String,
    pub module: String,
    pub keystroke: String,
    pub state: String,
}
#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Health {
    pub startup: String,
    pub page: String,
    pub global: String,
    pub entry: Option<String>,
    pub modules: Vec<ModuleHealth>,
    pub bindings: Vec<BindingHealth>,
}
#[derive(Clone, Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct WireEffect {
    pub kind: String,
    pub ticket: String,
    pub binding: String,
    pub keystroke: String,
    pub editing: String,
    pub repeat: bool,
    pub pass_through: bool,
}
#[derive(Clone)]
struct Machine {
    runtime: Runtime,
    revision: u64,
    counter: u64,
    owner: OwnerToken,
    modules: BTreeMap<String, LoadedModule>,
    health: Health,
    tickets: BTreeMap<String, Ticket>,
    page_ticket: Option<String>,
    focused: bool,
    blocked: bool,
    closed: bool,
}
struct Inner {
    machine: Machine,
    writer: StoreWriter<Health>,
}
/// Application service with a single published health source. Native work is returned as effects.
pub struct ShortcutService {
    inner: Mutex<Inner>,
    reader: StoreReader<Health>,
    loader: Arc<ModuleLoader>,
    window: Arc<WindowService>,
    os: Os,
}
impl ShortcutService {
    /// Creates an idle service; callers must initialize it and execute every returned native effect.
    pub fn new(
        window: Arc<WindowService>,
        loader: Arc<ModuleLoader>,
        os: Os,
        attempts: usize,
    ) -> Result<Arc<Self>, String> {
        let id = crate::state::identity::next_store_id()?;
        let health = Health {
            startup: "validating".into(),
            page: "notRequested".into(),
            global: "notRequested".into(),
            entry: None,
            modules: loader
                .index()
                .descriptors()
                .filter(|d| d.kind != ModuleKind::Global)
                .map(|d| ModuleHealth {
                    id: d.id.clone(),
                    kind: d.kind.clone(),
                    page: d.page.clone(),
                    phase: "notLoaded".into(),
                    error: None,
                })
                .collect(),
            bindings: vec![],
        };
        let (writer, reader) = create_store(id, health.clone(), &WatchBackend)
            .map_err(|_| "shortcuts_publish_failed")?;
        let machine = Machine {
            runtime: Runtime::new(attempts).map_err(|_| "shortcuts_invalid_policy")?,
            revision: 0,
            counter: 0,
            owner: OwnerToken::new(id.value().to_string())
                .map_err(|_| "shortcuts_invalid_owner")?,
            modules: BTreeMap::new(),
            health,
            tickets: BTreeMap::new(),
            page_ticket: None,
            focused: false,
            blocked: false,
            closed: false,
        };
        Ok(Arc::new(Self {
            inner: Mutex::new(Inner { machine, writer }),
            reader,
            loader,
            window,
            os,
        }))
    }
    /// Returns the current authoritative health without loading declarations.
    pub fn snapshot(&self) -> Result<Snapshot<Health>, String> {
        self.reader
            .snapshot()
            .map_err(|_| "shortcuts_closed".into())
    }
    /// Atomically subscribes to current health and future ordered versions.
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
        if inner.machine.closed {
            return Err("shortcuts_closed".into());
        }
        let mut candidate = inner.machine.clone();
        let effects = run(&mut candidate)?;
        self.project(&mut candidate);
        let mut wire = Vec::new();
        for effect in effects {
            let (kind, ticket) = match effect {
                Effect::Install(t) => ("install", t),
                Effect::Release(t) => ("release", t),
            };
            let token = if let Some((id, _)) = candidate.tickets.iter().find(|(_, t)| **t == ticket)
            {
                id.clone()
            } else {
                candidate.counter = candidate
                    .counter
                    .checked_add(1)
                    .ok_or("shortcuts_exhausted")?;
                let id = candidate.counter.to_string();
                candidate.tickets.insert(id.clone(), ticket.clone());
                id
            };
            let binding = &ticket.desired().binding;
            wire.push(WireEffect {
                kind: kind.into(),
                ticket: token,
                binding: binding.id.as_str().into(),
                keystroke: binding.resource.stroke.as_str().into(),
                editing: match binding.policy.editing {
                    EditingPolicy::Auto => "auto",
                    EditingPolicy::Always => "always",
                    EditingPolicy::NotEditing => "notEditing",
                }
                .into(),
                repeat: binding.policy.repeat,
                pass_through: binding.policy.pass_through,
            });
        }
        let live: Vec<_> = candidate
            .runtime
            .snapshot()
            .into_iter()
            .map(|s| s.ticket)
            .collect();
        candidate.tickets.retain(|_, t| live.contains(t));
        if *inner
            .writer
            .snapshot()
            .map_err(|_| "shortcuts_publish_failed")?
            .state()
            != candidate.health
        {
            inner
                .writer
                .publish(candidate.health.clone())
                .map_err(|_| "shortcuts_publish_failed")?;
        }
        inner.machine = candidate;
        Ok(wire)
    }
    fn available(&self, command: &Command) -> bool {
        self.window.snapshot().is_ok_and(|s| match command {
            Command::Navigation(navigation::Command::Back) => s.state().history.can_go_back(),
            Command::Navigation(navigation::Command::Forward) => s.state().history.can_go_forward(),
            Command::Window(_) | Command::Appearance(_) => false,
        })
    }
    fn plan(&self, m: &mut Machine) -> Result<Vec<Effect>, String> {
        let window = self
            .window
            .snapshot()
            .map_err(|_| "shortcuts_window_closed")?;
        let current = window.state().history.current();
        let page_current = m.health.entry.as_deref() == Some(current.key.as_str());
        let bindings: Vec<_> = m
            .modules
            .values()
            .filter(|module| {
                module.descriptor.kind == ModuleKind::Application
                    || (page_current
                        && module.descriptor.page.as_deref()
                            == Some(current.location.page.as_str()))
            })
            .flat_map(|module| module.bindings.clone())
            .collect();
        let scopes = BTreeSet::from_iter(bindings.iter().map(|b| b.scope.clone()));
        let catalog = Catalog::new(
            m.revision,
            bindings,
            ScopeRelations::new(scopes.clone(), vec![]).map_err(|_| "shortcuts_invalid_scope")?,
        )
        .map_err(|_| "shortcuts_conflict")?;
        let enabled = BTreeSet::from_iter(
            m.modules
                .values()
                .flat_map(|module| module.commands.iter())
                .filter(|(_, c)| m.focused && !m.blocked && self.available(c))
                .map(|(id, _)| id.clone()),
        );
        // Only current-module identifiers are eligible for this plan.
        let enabled = enabled
            .into_iter()
            .filter(|id| catalog.bindings().any(|b| b.id == *id))
            .collect();
        let plan = catalog
            .plan(&scopes, &enabled, m.owner.clone())
            .map_err(|_| "shortcuts_conflict")?;
        m.runtime
            .reconcile(m.revision, plan)
            .map_err(|_| "shortcuts_registration_failed".into())
    }
    fn project(&self, m: &mut Machine) {
        let states = m.runtime.snapshot();
        m.health.bindings = m
            .modules
            .values()
            .flat_map(|module| {
                module.bindings.iter().map(|b| {
                    let state = states
                        .iter()
                        .find(|s| s.ticket.desired().binding.id == b.id);
                    BindingHealth {
                        id: b.id.as_str().into(),
                        module: module.descriptor.id.clone(),
                        keystroke: b.resource.stroke.as_str().into(),
                        state: match state.map(|s| &s.status) {
                            Some(Status::Ready) => "installed",
                            Some(Status::Installing) => "installing",
                            Some(Status::Failed(_) | Status::ReleaseFailed(_)) => "failed",
                            Some(Status::Releasing) => "releasing",
                            None => "inactive",
                        }
                        .into(),
                    }
                })
            })
            .collect();
        for module in &mut m.health.modules {
            if matches!(module.phase.as_str(), "installing" | "ready") {
                let items: Vec<_> = m
                    .health
                    .bindings
                    .iter()
                    .filter(|b| b.module == module.id)
                    .collect();
                module.phase = if items.iter().any(|b| b.state == "failed") {
                    "failed"
                } else if items
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
        let app: Vec<_> = m
            .health
            .modules
            .iter()
            .filter(|s| s.kind == ModuleKind::Application)
            .collect();
        m.health.startup = aggregate(&app);
        if m.health.entry.is_some() {
            let page: Vec<_> = m
                .health
                .modules
                .iter()
                .filter(|s| {
                    s.kind == ModuleKind::Page && s.phase != "inactive" && s.phase != "notLoaded"
                })
                .collect();
            m.health.page = aggregate(&page);
        }
    }
    /// Loads window modules only; repeated calls reuse cached declarations.
    pub fn initialize(&self) -> Result<Vec<WireEffect>, String> {
        self.change(|m| {
            for descriptor in self
                .loader
                .index()
                .descriptors()
                .filter(|d| d.kind == ModuleKind::Application)
            {
                match self.loader.load(&descriptor.id, self.os) {
                    Ok(module) => {
                        m.modules.insert(descriptor.id.clone(), module);
                        if let Err(code) = self.validate_modules(m) {
                            m.modules.remove(&descriptor.id);
                            set_phase(m, &descriptor.id, "failed", Some(code));
                        } else {
                            set_phase(m, &descriptor.id, "installing", None);
                        }
                    }
                    Err(_) => set_phase(
                        m,
                        &descriptor.id,
                        "failed",
                        Some("shortcuts_module_failed".into()),
                    ),
                }
            }
            self.plan(m)
        })
    }
    /// Reconciles input ownership, overlays and current Router availability.
    pub fn context(&self, focused: bool, blocked: bool) -> Result<Vec<WireEffect>, String> {
        self.change(|m| {
            m.focused = focused;
            m.blocked = blocked;
            self.plan(m)
        })
    }
    /// Invalidates the old page immediately. Loading remains a separate, cancellable request.
    pub fn begin_page(&self, entry: &str, page: &str) -> Result<(String, Vec<WireEffect>), String> {
        let window = self
            .window
            .snapshot()
            .map_err(|_| "shortcuts_window_closed")?;
        if window.state().history.current().key != entry
            || window.state().history.current().location.page != page
        {
            return Err("shortcuts_stale_page".into());
        }
        let mut token = String::new();
        let effects = self.change(|m| {
            m.counter = m.counter.checked_add(1).ok_or("shortcuts_exhausted")?;
            token = m.counter.to_string();
            m.page_ticket = Some(token.clone());
            m.health.entry = Some(entry.into());
            m.modules
                .retain(|_, module| module.descriptor.kind != ModuleKind::Page);
            for item in m
                .health
                .modules
                .iter_mut()
                .filter(|h| h.kind == ModuleKind::Page)
            {
                item.phase = if item.page.as_deref() == Some(page) {
                    "loading"
                } else if item.phase == "notLoaded" {
                    "notLoaded"
                } else {
                    "inactive"
                }
                .into();
                item.error = None;
            }
            self.plan(m)
        })?;
        Ok((token, effects))
    }
    pub fn finish_page(&self, token: &str) -> Result<Vec<WireEffect>, String> {
        self.change(|m| {
            if m.page_ticket.as_deref() != Some(token) {
                return Ok(vec![]);
            }
            let window = self
                .window
                .snapshot()
                .map_err(|_| "shortcuts_window_closed")?;
            let current = window.state().history.current();
            if m.health.entry.as_deref() != Some(current.key.as_str()) {
                return Ok(vec![]);
            }
            let ids: Vec<_> = m
                .health
                .modules
                .iter()
                .filter(|h| {
                    h.kind == ModuleKind::Page
                        && h.page.as_deref() == Some(current.location.page.as_str())
                })
                .map(|h| h.id.clone())
                .collect();
            for id in ids {
                match self.loader.load(&id, self.os) {
                    Ok(module) => {
                        m.modules.insert(id.clone(), module);
                        if let Err(code) = self.validate_modules(m) {
                            m.modules.remove(&id);
                            set_phase(m, &id, "failed", Some(code));
                        } else {
                            set_phase(m, &id, "installing", None);
                        }
                    }
                    Err(_) => set_phase(m, &id, "failed", Some("shortcuts_module_failed".into())),
                }
            }
            self.plan(m)
        })
    }
    fn validate_modules(&self, m: &Machine) -> Result<(), String> {
        let bindings: Vec<_> = m
            .modules
            .values()
            .flat_map(|module| module.bindings.clone())
            .collect();
        let mut bindings = bindings;
        for descriptor in self
            .loader
            .index()
            .descriptors()
            .filter(|d| d.kind == ModuleKind::Global)
        {
            if let Ok(module) = self.loader.load(&descriptor.id, self.os) {
                bindings.extend(module.bindings);
            }
        }
        let scopes = BTreeSet::from_iter(bindings.iter().map(|b| b.scope.clone()));
        Catalog::new(
            m.revision,
            bindings,
            ScopeRelations::new(scopes, vec![]).map_err(|_| "shortcuts_invalid_scope")?,
        )
        .map(|_| ())
        .map_err(|_| "shortcuts_conflict".into())
    }
    /// Retires only the matching page lease, leaving newer leases untouched.
    pub fn leave_page(&self, token: &str) -> Result<Vec<WireEffect>, String> {
        self.change(|m| {
            if m.page_ticket.as_deref() != Some(token) {
                return Ok(vec![]);
            }
            m.page_ticket = None;
            m.health.entry = None;
            m.health.page = "notRequested".into();
            m.modules
                .retain(|_, module| module.descriptor.kind != ModuleKind::Page);
            for h in m
                .health
                .modules
                .iter_mut()
                .filter(|h| h.kind == ModuleKind::Page && h.phase != "notLoaded")
            {
                h.phase = "inactive".into();
            }
            self.plan(m)
        })
    }
    pub fn acknowledge(
        &self,
        token: &str,
        success: bool,
        release: bool,
    ) -> Result<Vec<WireEffect>, String> {
        self.change(|m| {
            let Some(ticket) = m.tickets.get(token) else {
                return Ok(vec![]);
            };
            let receipt = match (release, success) {
                (true, true) => Receipt::Released,
                (true, false) => Receipt::ReleaseFailed(Failure::Unknown),
                (false, true) => Receipt::Installed,
                (false, false) => Receipt::InstallFailed(Failure::Unknown),
            };
            m.runtime
                .acknowledge(ticket, receipt)
                .map_err(|_| "shortcuts_invalid_receipt".into())
        })
    }
    /// Rechecks authoritative router availability and entry ownership before dispatch.
    pub fn trigger(&self, token: &str) -> Result<Option<Command>, String> {
        let inner = self.inner.lock().map_err(|_| "shortcuts_closed")?;
        let m = &inner.machine;
        if m.closed || !m.focused || m.blocked {
            return Ok(None);
        }
        let Some(ticket) = m.tickets.get(token) else {
            return Ok(None);
        };
        if !m.runtime.accepts(ticket) {
            return Ok(None);
        }
        let window = self
            .window
            .snapshot()
            .map_err(|_| "shortcuts_window_closed")?;
        for module in m.modules.values() {
            if module.descriptor.kind == ModuleKind::Page
                && m.health.entry.as_deref() != Some(window.state().history.current().key.as_str())
            {
                continue;
            }
            if let Some(command) = module.commands.get(&ticket.desired().binding.id) {
                return Ok(self.available(command).then(|| command.clone()));
            }
        }
        Ok(None)
    }
    /// Retries failed releases before replacing registration generations.
    pub fn retry(&self) -> Result<Vec<WireEffect>, String> {
        self.change(|m| {
            let mut effects = Vec::new();
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
                if h.phase == "failed" && m.modules.contains_key(&h.id) {
                    h.phase = "installing".into();
                    h.error = None;
                }
            }
            effects.extend(self.plan(m)?);
            Ok(effects)
        })
    }
    /// Fences all callbacks and closes health publication; the executor releases native handles.
    pub fn close(&self) {
        if let Ok(mut inner) = self.inner.lock() {
            inner.machine.closed = true;
            let _ = inner.machine.runtime.dispose();
            inner.writer.close();
        }
    }
}
fn aggregate(items: &[&ModuleHealth]) -> String {
    if items.is_empty() {
        "notRequested"
    } else if items.iter().any(|s| s.phase == "failed") {
        "failed"
    } else if items.iter().any(|s| s.phase == "loading") {
        "loading"
    } else if items.iter().any(|s| s.phase == "notLoaded") {
        "validating"
    } else if items.iter().any(|s| s.phase == "installing") {
        "installing"
    } else {
        "ready"
    }
    .into()
}
fn set_phase(m: &mut Machine, id: &str, phase: &str, error: Option<String>) {
    if let Some(h) = m.health.modules.iter_mut().find(|h| h.id == id) {
        h.phase = phase.into();
        h.error = error;
    }
}
#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct RuntimePolicy {
    pub max_attempts: usize,
}
