//! Window-scoped routing/session registration within an engine and versioned subscription transport.
use crate::{
    router_service::{
        Decision, RouterService, WindowSnapshot,
        session::{OperationKind, SessionCommand, Stage},
    },
    state_bridge::{Binding, Event},
};
use nsh_core::{
    element::{CallCx, Callback, Local, Stop, System},
    prelude::*,
};
use std::{
    cell::RefCell,
    collections::{BTreeMap, BTreeSet},
    sync::Arc,
};
use usher_router::{Catalog, NavigationCommand, Target};
#[derive(Default)]
struct Windows {
    services: RefCell<BTreeMap<String, Arc<RouterService>>>,
    close_listeners: RefCell<BTreeSet<String>>,
}
impl Local for Windows {
    fn close(&self) {
        self.close_listeners.borrow_mut().clear();
        for (_, service) in std::mem::take(&mut *self.services.borrow_mut()) {
            service.close();
        }
    }
}
fn service(sys: &System, scope: &str) -> Result<Arc<RouterService>, String> {
    let (store, catalog) = scope
        .split_once(':')
        .map_or((scope, None), |(store, catalog)| (store, Some(catalog)));
    sys.locals
        .get::<Windows>()
        .services
        .borrow()
        .values()
        .find(|service| {
            service.snapshot().is_ok_and(|value| {
                value.store_id().value().to_string() == store
                    && catalog
                        .is_none_or(|identity| value.state().history.catalog_identity() == identity)
            })
        })
        .cloned()
        .ok_or_else(|| "router_window_closed".into())
}
#[derive(JsObject)]
pub struct Envelope {
    pub store_id: String,
    pub sequence: String,
    pub value: String,
}
impl From<usher_state::Snapshot<WindowSnapshot>> for Envelope {
    fn from(value: usher_state::Snapshot<WindowSnapshot>) -> Self {
        Self {
            store_id: value.store_id().value().to_string(),
            sequence: value.sequence().to_string(),
            value: serde_json::to_string(value.state())
                .expect("window snapshots contain serializable data"),
        }
    }
}
#[derive(JsObject)]
pub struct Attachment {
    pub scope: String,
    pub snapshot: Envelope,
}
#[derive(JsObject)]
pub struct Notification {
    pub snapshot: Option<Envelope>,
    pub end: Option<String>,
}
#[derive(JsObject)]
pub struct Reply {
    pub receipt: String,
    pub snapshot: Envelope,
}
/// The desktop adapter registers the host window identity; commands use a Rust-issued store identity.
#[module]
pub struct WindowState;
#[module]
impl WindowState {
    /// Registers the host window captured by the adapter during rendering, not its focused window.
    pub fn register(catalog: String, window: String, sys: System) -> Result<Envelope, String> {
        if window.is_empty() {
            return Err("router_invalid_window".into());
        }
        let catalog: Catalog =
            serde_json::from_str(&catalog).map_err(|_| "router_invalid_catalog")?;
        let local = sys.locals.get::<Windows>();
        let mut windows = local.services.borrow_mut();
        let service = match windows.get(&window) {
            Some(existing) => {
                existing.register(&catalog)?;
                existing.clone()
            }
            None => {
                let service = RouterService::new(catalog)?;
                windows.insert(window, service.clone());
                service
            }
        };
        Ok(service.snapshot()?.into())
    }
    /// Attaches a catalog-versioned binding, migrating an existing window atomically.
    /// The bare StoreId remains the lifetime token for the window-owned close listener.
    pub fn attach(catalog: String, window: String, sys: System) -> Result<Attachment, String> {
        if window.is_empty() {
            return Err("router_invalid_window".into());
        }
        let catalog: Catalog =
            serde_json::from_str(&catalog).map_err(|_| "router_invalid_catalog")?;
        let local = sys.locals.get::<Windows>();
        let mut windows = local.services.borrow_mut();
        let service = match windows.get(&window) {
            Some(existing) => {
                existing.replace_catalog(catalog)?;
                existing.clone()
            }
            None => {
                let service = RouterService::new(catalog)?;
                windows.insert(window, service.clone());
                service
            }
        };
        let snapshot = service.snapshot()?;
        let scope = format!(
            "{}:{}",
            snapshot.store_id().value(),
            snapshot.state().history.catalog_identity()
        );
        Ok(Attachment {
            scope,
            snapshot: snapshot.into(),
        })
    }
    /// Claims the single window-owned close listener, independent of UI projection owners.
    /// A closed scope cannot claim; its delayed installation must be discarded.
    pub fn claim_close_listener(scope: String, sys: System) -> bool {
        if service(&sys, &scope).is_err() {
            return false;
        }
        sys.locals
            .get::<Windows>()
            .close_listeners
            .borrow_mut()
            .insert(scope)
    }
    /// Closes exactly one opening. Late cleanup cannot close a window reopened under the same name.
    pub fn close(scope: String, sys: System) {
        let local = sys.locals.get::<Windows>();
        local.close_listeners.borrow_mut().remove(&scope);
        let mut windows = local.services.borrow_mut();
        let name = windows
            .iter()
            .find(|(_, service)| {
                service
                    .snapshot()
                    .is_ok_and(|value| value.store_id().value().to_string() == scope)
            })
            .map(|(name, _)| name.clone());
        if let Some(name) = name
            && let Some(service) = windows.remove(&name)
        {
            service.close();
        }
    }
    /// Reads the latest versioned window data without creating a subscription.
    pub fn snapshot(scope: String, sys: System) -> Result<Envelope, String> {
        Ok(service(&sys, &scope)?.snapshot()?.into())
    }
    /// Applies one typed session action and returns its receipt and current snapshot.
    pub fn session(scope: String, command: String, sys: System) -> Result<Reply, String> {
        let command: SessionCommand =
            serde_json::from_str(&command).map_err(|_| "session_invalid_command")?;
        let service = service(&sys, &scope)?;
        let receipt = service.session(command)?;
        Ok(Reply {
            receipt,
            snapshot: service.snapshot()?.into(),
        })
    }
    /// Commits an unguarded navigation through the same authoritative router.
    pub fn navigate(scope: String, command: String, sys: System) -> Result<Reply, String> {
        let command: NavigationCommand =
            serde_json::from_str(&command).map_err(|_| "router_invalid_target")?;
        let service = service(&sys, &scope)?;
        let result = service.navigate(command)?;
        Ok(Reply {
            receipt: serde_json::to_string(&result).expect("enum serialization"),
            snapshot: service.snapshot()?.into(),
        })
    }
    /// Validates a target without mutating history.
    pub fn resolve(scope: String, target: String, sys: System) -> Result<String, String> {
        let target: Target = serde_json::from_str(&target).map_err(|_| "router_invalid_target")?;
        serde_json::to_string(&service(&sys, &scope)?.resolve(&target)?)
            .map_err(|_| "router_unavailable".into())
    }
    /// Starts a guarded navigation and returns a lossless ticket.
    pub fn begin(scope: String, command: String, sys: System) -> Result<String, String> {
        let command: NavigationCommand =
            serde_json::from_str(&command).map_err(|_| "router_invalid_target")?;
        Ok(service(&sys, &scope)?.begin(command)?)
    }
    /// Completes only the current ticket; null means a redirect still needs guards.
    pub fn complete(
        scope: String,
        ticket: String,
        decision: String,
        redirect: Option<String>,
        sys: System,
    ) -> Result<String, String> {
        let decision = match decision.as_str() {
            "allow" => Decision::Allow,
            "cancel" => Decision::Cancel,
            "fail" => Decision::Fail,
            "redirect" => Decision::Redirect(
                serde_json::from_str::<Target>(&redirect.ok_or("router_invalid_target")?)
                    .map_err(|_| "router_invalid_target")?,
            ),
            _ => return Err("router_invalid_guard".into()),
        };
        serde_json::to_string(&service(&sys, &scope)?.complete(&ticket, decision)?)
            .map_err(|_| "router_unavailable".into())
    }
    /// Returns an idempotent Stop tied to the reactive owner and VM lifetime.
    pub fn subscribe(
        scope: String,
        callback: Callback<Notification>,
        sys: System,
        cx: CallCx<'_>,
    ) -> Stop {
        let binding = Binding::default();
        let stop = binding.clone();
        cx.start(async move {
            binding
                .run(crate::state_bridge::forward(
                    async { service(&sys, &scope)?.subscribe().map_err(str::to_owned) },
                    move |event| {
                        let event = match event {
                            Event::Snapshot(snapshot) => Notification {
                                snapshot: Some(snapshot.into()),
                                end: None,
                            },
                            Event::End(end) => Notification {
                                snapshot: None,
                                end: Some(
                                    match end {
                                        usher_state::SubscriptionEnd::Closed => "closed",
                                        usher_state::SubscriptionEnd::Cancelled => "cancelled",
                                        _ => "error",
                                    }
                                    .into(),
                                ),
                            },
                        };
                        callback.call_async(event)
                    },
                ))
                .await;
        });
        Stop::new(move || stop.close())
    }
    /// Export ownership lives in Rust. Page unmounting does not lose its ticket/result.
    pub async fn export_diagnostics(
        scope: String,
        from: String,
        to: String,
        sys: System,
    ) -> Result<(), String> {
        let window = service(&sys, &scope)?;
        let ticket = window.session(SessionCommand::Begin {
            operation: OperationKind::Export,
            stage: Stage::Running,
        })?;
        let result = crate::settings_bridge::Settings::export_diagnostics(from, to, sys).await;
        let (stage, error, value, failed) = match result {
            Ok(name) => (Stage::Done, None, Some(name), false),
            Err(code) if code == "diagnostics_export_cancelled" => (Stage::Idle, None, None, false),
            Err(code) => (Stage::Error, Some(code), None, true),
        };
        window.session(SessionCommand::Finish {
            operation: OperationKind::Export,
            ticket,
            stage,
            error,
            value,
            failed,
        })?;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    const CATALOG: &str = include_str!("../ui/pages/routes.json");
    #[test]
    fn catalog_attachments_fence_old_bindings_but_keep_window_lifetime() {
        let sys = System::default();
        let old = WindowState::attach(CATALOG.into(), "main".into(), sys.clone()).unwrap();
        let mut catalog: Catalog = serde_json::from_str(CATALOG).unwrap();
        catalog.identity.push_str("-replacement");
        let next = WindowState::attach(
            serde_json::to_string(&catalog).unwrap(),
            "main".into(),
            sys.clone(),
        )
        .unwrap();
        assert_eq!(old.snapshot.store_id, next.snapshot.store_id);
        assert_ne!(old.scope, next.scope);
        assert!(
            WindowState::navigate(
                old.scope.clone(),
                r#"{"kind":"go","target":-1}"#.into(),
                sys.clone()
            )
            .is_err()
        );
        assert!(
            WindowState::session(old.scope, r#"{"kind":"increment"}"#.into(), sys.clone()).is_err()
        );
        assert!(WindowState::attach(CATALOG.into(), "main".into(), sys.clone()).is_err());
        assert!(WindowState::snapshot(next.scope.clone(), sys.clone()).is_ok());
        WindowState::close(old.snapshot.store_id, sys.clone());
        assert!(WindowState::snapshot(next.scope, sys).is_err());
    }
    #[test]
    fn windows_share_an_engine_but_not_history_and_stale_handles_cannot_close_reopened_windows() {
        let sys = System::default();
        let first = WindowState::register(CATALOG.into(), "main".into(), sys.clone()).unwrap();
        let second =
            WindowState::register(CATALOG.into(), "secondary".into(), sys.clone()).unwrap();
        assert_ne!(first.store_id, second.store_id);
        assert!(WindowState::claim_close_listener(
            second.store_id.clone(),
            sys.clone()
        ));
        assert!(!WindowState::claim_close_listener(
            second.store_id.clone(),
            sys.clone()
        ));
        WindowState::session(
            first.store_id.clone(),
            r#"{"kind":"increment"}"#.into(),
            sys.clone(),
        )
        .unwrap();
        assert_eq!(
            service(&sys, &first.store_id)
                .unwrap()
                .snapshot()
                .unwrap()
                .state()
                .session
                .clicks,
            1
        );
        assert_eq!(
            service(&sys, &second.store_id)
                .unwrap()
                .snapshot()
                .unwrap()
                .state()
                .session
                .clicks,
            0
        );
        let retained = service(&sys, &second.store_id).unwrap();
        WindowState::close(second.store_id.clone(), sys.clone());
        assert!(retained.snapshot().is_err());
        assert!(!WindowState::claim_close_listener(
            second.store_id.clone(),
            sys.clone()
        ));
        let reopened =
            WindowState::register(CATALOG.into(), "secondary".into(), sys.clone()).unwrap();
        assert_ne!(reopened.store_id, second.store_id);
        assert!(WindowState::claim_close_listener(
            reopened.store_id.clone(),
            sys.clone()
        ));
        WindowState::close(second.store_id.clone(), sys.clone());
        assert!(WindowState::snapshot(second.store_id, sys.clone()).is_err());
        assert!(WindowState::snapshot(reopened.store_id.clone(), sys.clone()).is_ok());
        assert_eq!(
            WindowState::register(CATALOG.into(), "main".into(), sys.clone())
                .unwrap()
                .store_id,
            first.store_id
        );
        sys.locals.close_all();
        assert!(WindowState::snapshot(reopened.store_id, sys).is_err());
    }
}
