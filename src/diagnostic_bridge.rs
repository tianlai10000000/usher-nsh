//! Read-only live diagnostic status transport, reusing the generic subscription lifecycle.
use crate::{
    diagnostic_state::{STATUS, Status},
    state_bridge::{Binding, Event},
};
use nsh_core::{
    element::{CallCx, Callback, Stop},
    prelude::*,
};
#[derive(JsObject)]
pub struct DiagnosticValue {
    pub problem: Option<String>,
    pub dropped: String,
}
#[derive(JsObject)]
pub struct DiagnosticEnvelope {
    pub store_id: String,
    pub sequence: String,
    pub value: DiagnosticValue,
}
impl From<usher_state::Snapshot<Status>> for DiagnosticEnvelope {
    fn from(snapshot: usher_state::Snapshot<Status>) -> Self {
        Self {
            store_id: snapshot.store_id().value().to_string(),
            sequence: snapshot.sequence().to_string(),
            value: DiagnosticValue {
                problem: snapshot.state().problem.clone(),
                dropped: snapshot.state().dropped.to_string(),
            },
        }
    }
}
#[derive(JsObject)]
pub struct DiagnosticEvent {
    pub snapshot: Option<DiagnosticEnvelope>,
    pub end: Option<String>,
}
/// Read-only process diagnostic status, separate from persisted preferences.
#[module]
pub struct DiagnosticState;
#[module]
impl DiagnosticState {
    /// Reads live health independently of preference publications.
    pub fn snapshot() -> Result<DiagnosticEnvelope, String> {
        Ok(STATUS
            .as_ref()
            .map_err(|e| e.to_string())?
            .snapshot()?
            .into())
    }
    /// Observes the active pipeline; Stop releases only this engine binding.
    pub fn subscribe(callback: Callback<DiagnosticEvent>, cx: CallCx<'_>) -> Stop {
        let binding = Binding::default();
        let stop = binding.clone();
        cx.start(async move {
            binding
                .run(crate::state_bridge::forward(
                    async { STATUS.as_ref().map_err(|e| e.to_string())?.subscribe() },
                    move |event| {
                        callback.call_async(match event {
                            Event::Snapshot(snapshot) => DiagnosticEvent {
                                snapshot: Some(snapshot.into()),
                                end: None,
                            },
                            Event::End(end) => DiagnosticEvent {
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
                        })
                    },
                ))
                .await;
        });
        Stop::new(move || stop.close())
    }
}
