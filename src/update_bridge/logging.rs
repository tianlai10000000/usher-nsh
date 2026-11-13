//! Writes committed update events and classifies native details for diagnostics only.
use crate::update_service::{Claim, UpdateService, transport};
use usher_logging::{Fields, Level, LogKind, current};
use usher_update::state;

pub(super) fn drain(service: &UpdateService) {
    let Ok(records) = service.take_logs() else {
        return;
    };
    let Some(logger) = current() else { return };
    for record in records {
        write_record(&logger, record);
    }
}
fn write_record(logger: &usher_logging::Logger, record: state::Record) {
    let level = match record.level {
        state::Level::Info => Level::Info,
        state::Level::Warn => Level::Warn,
        state::Level::Error => Level::Error,
    };
    let fields = Fields::new()
        .with("ticket", record.ticket.map(|v| v.to_string()))
        .with("version", record.version.map(|v| v.to_string()))
        .with("kind", record.kind.map(transport::kind))
        .with("failure", record.failure.map(state::Failure::as_str))
        .with("failures", record.failures.map(|v| v.to_string()));
    logger.for_kind(LogKind::Tracing).log(
        level,
        "update",
        "state_transition",
        fields.clone().with("transition", record.event),
    );
    logger.log(level, "update", record.event, fields);
}

pub(super) fn detail(claim: &Claim, failure: state::Failure, detail: &str) {
    if let Some(logger) = current() {
        let fields = Fields::new()
            .with("claim", claim.token())
            .with("detail", detail)
            .with("reason", reason(detail));
        logger.log(
            match failure {
                state::Failure::Network | state::Failure::Busy => Level::Warn,
                state::Failure::Cancelled => Level::Info,
                _ => Level::Error,
            },
            "update",
            "native_failure",
            fields,
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
    fn update_records_keep_their_severity_and_fields_in_both_outputs() {
        use std::sync::{Arc, Mutex};
        use usher_logging::{
            FieldValue, Output, Outputs,
            runtime::{Health, LogWriter, Record, Runtime},
        };
        #[derive(Clone, Default)]
        struct Capture(Arc<Mutex<Vec<(Output, Record)>>>);
        impl LogWriter for Capture {
            fn write(&mut self, output: Output, record: &Record) -> Result<(), Health> {
                self.0.lock().unwrap().push((output, record.clone()));
                Ok(())
            }
            fn flush(&mut self) -> Result<(), Health> {
                Ok(())
            }
        }
        let capture = Capture::default();
        let runtime = Runtime::with_backend(Level::Trace, Box::new(capture.clone()), None);
        runtime.set_outputs(Outputs {
            structured: true,
            ..Outputs::default()
        });
        for (domain, expected) in [
            (state::Level::Info, Level::Info),
            (state::Level::Warn, Level::Warn),
            (state::Level::Error, Level::Error),
        ] {
            runtime.logger().set_level(expected);
            write_record(
                &runtime.logger(),
                state::Record {
                    level: domain,
                    event: "check_failed",
                    ticket: Some(u64::MAX),
                    version: None,
                    kind: None,
                    failure: Some(state::Failure::Network),
                    failures: Some(2),
                },
            );
            assert!(runtime.flush(std::time::Duration::from_secs(2)));
            let records = std::mem::take(&mut *capture.0.lock().unwrap());
            assert_eq!(records.len(), 2);
            for (output, record) in records {
                assert_eq!(record.level, expected);
                assert_eq!(
                    record.fields.get("ticket"),
                    Some(&FieldValue::from(u64::MAX.to_string()))
                );
                assert_eq!(record.fields.get("version"), Some(&FieldValue::Null));
                assert_eq!(
                    record.fields.get("failure"),
                    Some(&FieldValue::from("network"))
                );
                if output == Output::Structured {
                    assert_eq!(record.kind, LogKind::Tracing);
                    assert_eq!(
                        record.fields.get("transition"),
                        Some(&FieldValue::from("check_failed"))
                    );
                } else {
                    assert_eq!(output, Output::File);
                    assert_eq!(record.event, "check_failed");
                }
            }
        }
    }
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
