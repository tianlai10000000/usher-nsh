//! Process singleton bridge with VM-local lease cleanup and read-only cross-window subscriptions.
use super::{
    bridge::{ShortcutEnvelope, ShortcutNotification, ShortcutReply},
    catalog::ModuleIndex,
    global::GlobalService,
    loader::ModuleLoader,
    service::RuntimePolicy,
};
use crate::state_bridge::{Binding, Event};
use nsh_core::{
    element::{CallCx, Callback, Local, Stop, System},
    platform::Os,
    prelude::*,
};
use serde::Deserialize;
use std::{
    cell::RefCell,
    sync::{Arc, Mutex},
};

static PROCESS: Mutex<Option<Arc<GlobalService>>> = Mutex::new(None);
#[derive(Default)]
struct Lease(RefCell<Option<(Arc<GlobalService>, String)>>);
impl Local for Lease {
    fn close(&self) {
        if let Some((service, owner)) = self.0.borrow_mut().take() {
            service.abandon(&owner);
        }
    }
}
#[derive(JsObject)]
pub struct GlobalShortcutAttachment {
    pub snapshot: ShortcutEnvelope,
    pub executor: bool,
}
fn service() -> Result<Arc<GlobalService>, String> {
    PROCESS
        .lock()
        .map_err(|_| "shortcuts_closed")?
        .clone()
        .ok_or("shortcuts_not_started".into())
}
fn owner(sys: &System) -> Result<String, String> {
    sys.locals
        .get::<Lease>()
        .0
        .borrow()
        .as_ref()
        .map(|(_, id)| id.clone())
        .ok_or("shortcuts_stale_executor".into())
}
/// Attaches a host-provided catalog once per process. This injection boundary is Rust-only.
pub fn attach_with_loader(
    sys: System,
    loader: Arc<ModuleLoader>,
) -> Result<GlobalShortcutAttachment, String> {
    let service = {
        let mut process = PROCESS.lock().map_err(|_| "shortcuts_closed")?;
        if process.is_none() {
            let policy: RuntimePolicy =
                serde_json::from_str(include_str!("../../ui/config/shortcuts/runtime.json"))
                    .map_err(|_| "shortcuts_invalid_policy")?;
            *process = Some(GlobalService::new(loader, sys.os, policy.max_attempts)?);
        }
        process.as_ref().ok_or("shortcuts_not_started")?.clone()
    };
    let local = sys.locals.get::<Lease>();
    if local.0.borrow().is_none() {
        *local.0.borrow_mut() = Some((
            service.clone(),
            crate::state::identity::next_store_id()?.value().to_string(),
        ));
    }
    // NSH 0.5 macOS Surface::stop clears pending commands and releases global registrations
    // before a replacement VM starts. Other hosts retain unresolved ownership until verified.
    let executor = service.claim(&owner(&sys)?, sys.os == Os::MacOs)?;
    Ok(GlobalShortcutAttachment {
        snapshot: service.snapshot()?.into(),
        executor,
    })
}
#[derive(Deserialize)]
#[serde(tag = "kind", rename_all = "camelCase")]
enum Request {
    Initialize,
    Retry,
    Acknowledge {
        ticket: String,
        success: bool,
        release: bool,
    },
}
/// Starts global initialization independently of page and window shortcut registration.
#[module]
pub struct GlobalShortcutState;
#[module]
impl GlobalShortcutState {
    /// Idempotently attaches this VM to the process singleton and claims execution when available.
    pub fn attach(sys: System) -> Result<GlobalShortcutAttachment, String> {
        attach_with_loader(
            sys,
            Arc::new(ModuleLoader::new(
                ModuleIndex::application().map_err(|_| "shortcuts_invalid_catalog")?,
            )),
        )
    }
    /// Reads shared process health; never runs a module factory.
    pub fn read() -> Result<ShortcutEnvelope, String> {
        Ok(service()?.snapshot()?.into())
    }
    /// Reclaims execution after a previous VM's confirmed teardown; does not steal live ownership.
    pub fn claim(sys: System) -> Result<bool, String> {
        service()?.claim(&owner(&sys)?, sys.os == Os::MacOs)
    }
    /// Runs a lease-checked lifecycle command and returns native effects after publication.
    pub fn execute(request: String, sys: System) -> Result<ShortcutReply, String> {
        let service = service()?;
        let owner = owner(&sys)?;
        let effects = match serde_json::from_str::<Request>(&request)
            .map_err(|_| "shortcuts_invalid_command")?
        {
            Request::Initialize => service.initialize(&owner)?,
            Request::Retry => service.retry(&owner)?,
            Request::Acknowledge {
                ticket,
                success,
                release,
            } => service.acknowledge(&owner, &ticket, success, release)?,
        };
        Ok(ShortcutReply {
            snapshot: service.snapshot()?.into(),
            effects: serde_json::to_string(&effects).map_err(|_| "shortcuts_invalid_effect")?,
            ticket: None,
        })
    }
    /// Returns a command only for a confirmed registration owned by this VM.
    pub fn trigger(ticket: String, sys: System) -> Result<String, String> {
        serde_json::to_string(&service()?.trigger(&owner(&sys)?, &ticket)?)
            .map_err(|_| "shortcuts_invalid_command".into())
    }
    /// Subscriptions borrow process state and never release registrations when a view unmounts.
    pub fn subscribe(callback: Callback<ShortcutNotification>, cx: CallCx<'_>) -> Stop {
        let binding = Binding::default();
        let stop = binding.clone();
        cx.start(async move {
            binding
                .run(crate::state_bridge::forward(
                    async { service()?.subscribe() },
                    move |event| {
                        callback.call_async(match event {
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
                        })
                    },
                ))
                .await;
        });
        Stop::new(move || stop.close())
    }
}
