//! NSH window-lifetime transport for shortcut effects and versioned health subscriptions.
use super::{
    catalog::ModuleIndex,
    loader::ModuleLoader,
    service::{Health, RuntimePolicy, ShortcutService, WireEffect},
};
use crate::state_bridge::{Binding, Event};
use nsh_core::{
    element::{CallCx, Callback, Local, Stop, System},
    platform::Os,
    prelude::*,
};
use serde::Deserialize;
use std::{cell::RefCell, collections::BTreeMap, sync::Arc};

#[derive(Default)]
struct Services {
    windows: RefCell<BTreeMap<String, Arc<ShortcutService>>>,
    loader: RefCell<Option<Arc<ModuleLoader>>>,
}
impl Local for Services {
    fn close(&self) {
        for (_, service) in std::mem::take(&mut *self.windows.borrow_mut()) {
            service.close();
        }
        self.loader.borrow_mut().take();
    }
}
pub(crate) fn close_window(sys: &System, window: &str) {
    if let Some(service) = sys
        .locals
        .get::<Services>()
        .windows
        .borrow_mut()
        .remove(window)
    {
        service.close();
    }
}
fn service(sys: &System, scope: &str) -> Result<Arc<ShortcutService>, String> {
    sys.locals
        .get::<Services>()
        .windows
        .borrow()
        .values()
        .find(|s| {
            s.snapshot()
                .is_ok_and(|v| v.store_id().value().to_string() == scope)
        })
        .cloned()
        .ok_or("shortcuts_closed".into())
}
#[derive(JsObject)]
pub struct ShortcutEnvelope {
    pub store_id: String,
    pub sequence: String,
    pub value: String,
}
impl From<usher_state::Snapshot<Health>> for ShortcutEnvelope {
    fn from(value: usher_state::Snapshot<Health>) -> Self {
        Self {
            store_id: value.store_id().value().to_string(),
            sequence: value.sequence().to_string(),
            value: serde_json::to_string(value.state())
                .expect("health contains serializable values"),
        }
    }
}
#[derive(JsObject)]
pub struct ShortcutReply {
    pub snapshot: ShortcutEnvelope,
    pub effects: String,
    pub ticket: Option<String>,
}
#[derive(JsObject)]
pub struct ShortcutNotification {
    pub snapshot: Option<ShortcutEnvelope>,
    pub end: Option<String>,
}
#[derive(Deserialize)]
#[serde(tag = "kind", rename_all = "camelCase")]
enum Request {
    Initialize,
    Context {
        focused: bool,
        blocked: bool,
    },
    BeginPage {
        entry: String,
        page: String,
    },
    FinishPage {
        ticket: String,
    },
    LeavePage {
        ticket: String,
    },
    Acknowledge {
        ticket: String,
        success: bool,
        release: bool,
    },
    Retry,
}
/// Window-owned shortcut registration and health transport.
#[module]
pub struct ShortcutState;
#[module]
impl ShortcutState {
    /// Attaches an executor to an existing router opening; replacing it fences the old executor.
    pub fn attach(window: String, os: String, sys: System) -> Result<ShortcutEnvelope, String> {
        let local = sys.locals.get::<Services>();
        let loader = {
            let mut slot = local.loader.borrow_mut();
            if slot.is_none() {
                *slot = Some(Arc::new(ModuleLoader::new(
                    ModuleIndex::application().map_err(|_| "shortcuts_invalid_catalog")?,
                )));
            }
            slot.as_ref().ok_or("shortcuts_invalid_catalog")?.clone()
        };
        attach_with_loader(window, os, sys, loader)
    }

    /// Reads current versioned health without starting any module.
    pub fn read(scope: String, sys: System) -> Result<ShortcutEnvelope, String> {
        Ok(service(&sys, &scope)?.snapshot()?.into())
    }
    /// Applies a typed command and returns effects only after health publication.
    pub fn execute(scope: String, request: String, sys: System) -> Result<ShortcutReply, String> {
        let owner = service(&sys, &scope)?;
        let mut page_ticket = None;
        let effects: Vec<WireEffect> = match serde_json::from_str::<Request>(&request)
            .map_err(|_| "shortcuts_invalid_command")?
        {
            Request::Initialize => owner.initialize()?,
            Request::Context { focused, blocked } => owner.context(focused, blocked)?,
            Request::BeginPage { entry, page } => {
                let (ticket, effects) = owner.begin_page(&entry, &page)?;
                page_ticket = Some(ticket);
                effects
            }
            Request::FinishPage { ticket } => owner.finish_page(&ticket)?,
            Request::LeavePage { ticket } => owner.leave_page(&ticket)?,
            Request::Acknowledge {
                ticket,
                success,
                release,
            } => owner.acknowledge(&ticket, success, release)?,
            Request::Retry => owner.retry()?,
        };
        Ok(ShortcutReply {
            snapshot: owner.snapshot()?.into(),
            effects: serde_json::to_string(&effects).expect("effect DTO serializes"),
            ticket: page_ticket,
        })
    }
    /// Validates a native ticket and returns its current business command.
    pub fn trigger(scope: String, ticket: String, sys: System) -> Result<String, String> {
        serde_json::to_string(&service(&sys, &scope)?.trigger(&ticket)?)
            .map_err(|_| "shortcuts_invalid_command".into())
    }
    /// Releases only this executor identity, never a newer attachment to the same window.
    pub fn close(scope: String, sys: System) {
        let local = sys.locals.get::<Services>();
        let name = local
            .windows
            .borrow()
            .iter()
            .find(|(_, s)| {
                s.snapshot()
                    .is_ok_and(|v| v.store_id().value().to_string() == scope)
            })
            .map(|(k, _)| k.clone());
        if let Some(name) = name {
            close_window(&sys, &name);
        }
    }
    /// Subscribes atomically to the initial and subsequent health snapshots.
    pub fn subscribe(
        scope: String,
        callback: Callback<ShortcutNotification>,
        sys: System,
        cx: CallCx<'_>,
    ) -> Stop {
        let binding = Binding::default();
        let stop = binding.clone();
        cx.start(async move {
            binding
                .run(crate::state_bridge::forward(
                    async { service(&sys, &scope)?.subscribe() },
                    move |event| {
                        let notification = match event {
                            Event::Snapshot(value) => ShortcutNotification {
                                snapshot: Some(value.into()),
                                end: None,
                            },
                            Event::End(reason) => ShortcutNotification {
                                snapshot: None,
                                end: Some(
                                    match reason {
                                        usher_state::SubscriptionEnd::Closed => "closed",
                                        usher_state::SubscriptionEnd::Cancelled => "cancelled",
                                        _ => "error",
                                    }
                                    .into(),
                                ),
                            },
                        };
                        callback.call_async(notification)
                    },
                ))
                .await;
        });
        Stop::new(move || stop.close())
    }
}

/// Host injection boundary for alternative application catalogs; never exposed as a JS command.
/// The registered production transport and lifecycle remain shared with the injected host.
pub fn attach_with_loader(
    window: String,
    os: String,
    sys: System,
    loader: Arc<ModuleLoader>,
) -> Result<ShortcutEnvelope, String> {
    let owner = crate::router_bridge::service(&sys, &window)?;
    let lifetime = owner
        .snapshot()
        .map_err(String::from)?
        .store_id()
        .value()
        .to_string();
    let os = match os.as_str() {
        "macos" => Os::MacOs,
        "windows" => Os::Windows,
        "linux" => Os::Linux,
        "ios" => Os::Ios,
        "android" => Os::Android,
        "web" => Os::Web,
        _ => return Err("shortcuts_unsupported_platform".into()),
    };
    let local = sys.locals.get::<Services>();
    let policy: RuntimePolicy =
        serde_json::from_str(include_str!("../../ui/config/shortcuts/runtime.json"))
            .map_err(|_| "shortcuts_invalid_policy")?;
    let next = ShortcutService::new(owner, loader, os, policy.max_attempts)?;
    if let Some(old) = local.windows.borrow_mut().insert(lifetime, next.clone()) {
        old.close();
    }
    Ok(next.snapshot()?.into())
}
