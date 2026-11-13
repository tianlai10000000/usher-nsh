//! Writes committed update events and classifies native details for diagnostics only.
use crate::update_service::{Claim, UpdateService, transport};
use serde_json::json;
use usher_logging::{Level, current};
use usher_update::state;

pub(super) fn drain(service: &UpdateService) {
    let Ok(records) = service.take_logs() else {
        return;
    };
    let Some(logger) = current() else { return };
    for record in records {
        let fields = json!({
            "ticket": record.ticket.map(|v| v.to_string()),
            "version": record.version.map(|v| v.to_string()),
            "kind": record.kind.map(transport::kind),
            "failure": record.failure.map(state::Failure::as_str),
            "failures": record.failures.map(|v| v.to_string()),
        });
        logger.log(
            match record.level {
                state::Level::Info => Level::Info,
                state::Level::Warn => Level::Warn,
                state::Level::Error => Level::Error,
            },
            "update",
            record.event,
            fields.as_object().cloned(),
        );
    }
}
pub(super) fn detail(claim: &Claim, failure: state::Failure, detail: &str) {
    if let Some(logger) = current() {
        let fields = json!({"claim": claim.token(), "detail": detail, "reason": reason(detail)});
        logger.log(
            match failure {
                state::Failure::Network | state::Failure::Busy => Level::Warn,
                state::Failure::Cancelled => Level::Info,
                _ => Level::Error,
            },
            "update",
            "native_failure",
            fields.as_object().cloned(),
        );
    }
}
fn reason(detail: &str) -> &'static str {
    let message = detail.to_ascii_lowercase();
    for (fragment, reason) in [
        ("already in progress", "already_in_progress"),
        ("signature", "signature"),
        ("not newer", "build_not_newer"),
        ("runtime", "runtime_too_old"),
        ("hash", "hash_mismatch"),
        ("size", "size_mismatch"),
    ] {
        if message.contains(fragment) {
            return reason;
        }
    }
    "unclassified"
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn unknown_messages_remain_diagnostic_details() {
        assert_eq!(
            reason("an update operation is already in progress"),
            "already_in_progress"
        );
        assert_eq!(reason("the signature does not verify"), "signature");
        assert_eq!(reason("new upstream wording"), "unclassified");
    }
}
