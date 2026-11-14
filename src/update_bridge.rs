//! Exposes update commands, opaque execution claims and versioned state to NSH controllers.
mod logging;

use crate::{
    settings_bridge::with_settings_store,
    state::updates::{
        self, UpdateService,
        transport::{self, Configuration, Reply, Request},
    },
    state_bridge::{Binding, Event},
};
use nsh_core::{
    element::{CallCx, Callback, Stop, System},
    prelude::*,
};

#[derive(JsObject)]
pub struct Envelope {
    pub store_id: String,
    pub sequence: String,
    pub value: String,
}
impl From<usher_state::Snapshot<updates::State>> for Envelope {
    fn from(snapshot: usher_state::Snapshot<updates::State>) -> Self {
        Self {
            store_id: snapshot.store_id().value().to_string(),
            sequence: snapshot.sequence().to_string(),
            value: transport::state(snapshot.state()),
        }
    }
}
#[derive(JsObject)]
pub struct Notification {
    pub snapshot: Option<Envelope>,
    pub end: Option<String>,
}
impl From<Event<updates::State>> for Notification {
    fn from(event: Event<updates::State>) -> Self {
        match event {
            Event::Snapshot(snapshot) => Self {
                snapshot: Some(snapshot.into()),
                end: None,
            },
            Event::End(end) => Self {
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
        }
    }
}
async fn with_service<T: Send + 'static>(
    sys: System,
    operation: impl FnOnce(&UpdateService) -> Result<T, updates::Error> + Send + 'static,
) -> Result<T, String> {
    with_settings_store(sys, move |settings| {
        let service = settings.updates()?;
        let result = operation(&service).map_err(crate::state::settings::service::Error::Update);
        // Drain only committed records. Invalid callbacks must not produce detail logs.
        logging::drain(&service);
        result
    })
    .await
}

/// Application-owned updates; stopping a subscription never closes the service or its lease.
#[module]
pub struct UpdateState;
#[module]
impl UpdateState {
    /// Reads the shared state; a null domain means configuration is still pending.
    pub async fn snapshot(sys: System) -> Result<Envelope, String> {
        with_service(sys, |service| service.snapshot().map(Into::into)).await
    }
    /// Delivers the initial snapshot and subsequent changes; Stop releases only this binding.
    pub fn subscribe(callback: Callback<Notification>, sys: System, cx: CallCx<'_>) -> Stop {
        let binding = Binding::default();
        let stop = binding.clone();
        cx.start(async move {
            binding
                .run(crate::state_bridge::forward(
                    async move { with_service(sys, |service| service.subscribe()).await },
                    move |event| callback.call_async(event.into()),
                ))
                .await;
        });
        Stop::new(move || stop.close())
    }
    /// Configures once, using the persisted preference rather than trusting a second UI copy.
    pub async fn configure(value: String, sys: System) -> Result<Envelope, String> {
        let config: Configuration = transport::parse(&value).map_err(|e| e.code())?;
        with_settings_store(sys, move |settings| {
            settings.configure_updates(config)?;
            settings
                .updates()?
                .snapshot()
                .map(Into::into)
                .map_err(crate::state::settings::service::Error::Update)
        })
        .await
    }
    /// User intents cannot impersonate a native completion or select a settings directory.
    pub async fn request(value: String, sys: System) -> Result<Envelope, String> {
        let request: Request = transport::parse(&value).map_err(|e| e.code())?;
        with_settings_store(sys, move |settings| {
            let service = settings.updates()?;
            if service
                .snapshot()
                .map_err(crate::state::settings::service::Error::Update)?
                .state()
                .domain
                .is_none()
            {
                return Err(updates::Error::NotConfigured.into());
            }
            match request {
                Request::AutoCheck { enabled } => {
                    settings.apply(crate::state::settings::Command::SetUpdateAutoCheck(enabled))?;
                }
                Request::Install => {
                    let saved = settings.retry_save()?;
                    if !saved.saved {
                        return Err("updates_settings_unsaved".into());
                    }
                    service
                        .request(updates::Command::Install)
                        .map_err(crate::state::settings::service::Error::Update)?;
                }
                request => service
                    .request(
                        request
                            .command()
                            .map_err(crate::state::settings::service::Error::Update)?,
                    )
                    .map_err(crate::state::settings::service::Error::Update)?,
            }
            logging::drain(&service);
            service
                .snapshot()
                .map(Into::into)
                .map_err(crate::state::settings::service::Error::Update)
        })
        .await
    }
    /// Elects one executor and returns its opaque lease; fails before configuration.
    pub async fn acquire(sys: System) -> Result<String, String> {
        with_service(sys, |service| service.acquire().map(|lease| lease.token())).await
    }
    /// Returns one queued effect for the live lease, or null while idle or already executing.
    pub async fn claim(lease: String, sys: System) -> Result<Option<String>, String> {
        with_service(sys, move |service| {
            let lease = service.lease_token(&lease)?;
            let claim = service.claim(lease)?;
            if claim.as_ref().is_some_and(|c| {
                matches!(c.effect(), usher_update::state::Effect::StartInstall { .. })
            }) {
                crate::diagnostics::flush_for_exit();
            }
            Ok(claim.as_ref().map(transport::claim))
        })
        .await
    }
    /// Accepts decimal byte counts only for the currently claimed download.
    pub async fn progress(
        claim: String,
        downloaded: String,
        total: Option<String>,
        sys: System,
    ) -> Result<(), String> {
        let downloaded = transport::decimal(&downloaded).map_err(|e| e.code())?;
        let total = total
            .as_deref()
            .map(transport::decimal)
            .transpose()
            .map_err(|e| e.code())?;
        with_service(sys, move |service| {
            service.progress(&service.claim_token(&claim)?, downloaded, total)
        })
        .await
    }
    /// Accepts a typed completion once for the live claim; stale receipts are rejected.
    pub async fn complete(claim: String, value: String, sys: System) -> Result<(), String> {
        let (completion, detail) = transport::parse::<Reply>(&value)
            .and_then(Reply::completion)
            .map_err(|e| e.code())?;
        with_service(sys, move |service| {
            let claim = service.claim_token(&claim)?;
            let failure = match completion {
                updates::Completion::Failed(failure) => Some(failure),
                _ => None,
            };
            service.complete(&claim, completion)?;
            if let (Some(detail), Some(failure)) = (detail, failure) {
                logging::detail(&claim, failure, &detail);
            }
            Ok(())
        })
        .await
    }
    /// Explicitly flushes domain records to the logger; no free-text logging endpoint is exposed.
    pub async fn take_logs(sys: System) -> Result<(), String> {
        with_service(sys, |_| Ok(())).await
    }
}
