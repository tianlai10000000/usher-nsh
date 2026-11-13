//! Tests public logging behavior through injected backends, without application or NSH dependencies.
mod support;
use std::time::Duration;
use support::{Capture, FailingFile};
use usher_logging::{
    FieldValue, Fields, Level, LogKind, Output, Outputs, Submit,
    runtime::{Health, Runtime},
};
const KINDS: [LogKind; 4] = [
    LogKind::Application,
    LogKind::Tracing,
    LogKind::Terminal,
    LogKind::Memory,
];
const LEVELS: [Level; 6] = [
    Level::Off,
    Level::Error,
    Level::Warn,
    Level::Info,
    Level::Debug,
    Level::Trace,
];
fn all_outputs() -> Outputs {
    Outputs {
        console: true,
        file: true,
        structured: true,
        terminal: true,
        memory: true,
    }
}
fn barrier(runtime: &Runtime) {
    assert!(runtime.flush(Duration::from_secs(2)));
}
fn matrix(threshold: Level, accepted: &[Level]) {
    let capture = Capture::default();
    let runtime = Runtime::with_backend(threshold, Box::new(capture.clone()), None);
    runtime.set_outputs(all_outputs());
    for kind in KINDS {
        let logger = runtime.logger().for_kind(kind);
        for level in LEVELS {
            assert_eq!(
                logger.enabled(level),
                accepted.contains(&level),
                "{kind:?}/{threshold:?}/{level:?}"
            );
            let result = logger.log(
                level,
                "contract",
                level.as_str(),
                Fields::new().with("kind", kind.as_str()),
            );
            assert_eq!(
                result,
                if accepted.contains(&level) {
                    Submit::Enqueued
                } else {
                    Submit::Filtered
                }
            );
        }
    }
    barrier(&runtime);
    let records = capture.records();
    for kind in KINDS {
        let destinations: &[Output] = if kind == LogKind::Tracing {
            &[Output::Structured]
        } else {
            &[Output::Console, Output::File]
        };
        for output in [Output::Console, Output::File, Output::Structured] {
            let actual: Vec<_> = records
                .iter()
                .filter(|(target, record)| *target == output && record.kind == kind)
                .map(|(_, record)| record.level)
                .collect();
            assert_eq!(
                actual,
                if destinations.contains(&output) {
                    accepted
                } else {
                    &[]
                },
                "{kind:?}/{output:?}/{threshold:?}"
            );
        }
    }
}
#[test]
fn logging_levels_off_capture_nothing() {
    matrix(Level::Off, &[]);
}
#[test]
fn logging_levels_error() {
    matrix(Level::Error, &[Level::Error]);
}
#[test]
fn logging_levels_warn() {
    matrix(Level::Warn, &[Level::Error, Level::Warn]);
}
#[test]
fn logging_levels_info() {
    matrix(Level::Info, &[Level::Error, Level::Warn, Level::Info]);
}
#[test]
fn logging_levels_debug() {
    matrix(
        Level::Debug,
        &[Level::Error, Level::Warn, Level::Info, Level::Debug],
    );
}
#[test]
fn logging_levels_trace() {
    matrix(
        Level::Trace,
        &[
            Level::Error,
            Level::Warn,
            Level::Info,
            Level::Debug,
            Level::Trace,
        ],
    );
}
#[test]
fn logging_switches_are_independent_of_levels_and_types() {
    for mask in 0..32 {
        let outputs = Outputs {
            console: mask & 1 != 0,
            file: mask & 2 != 0,
            structured: mask & 4 != 0,
            terminal: mask & 8 != 0,
            memory: mask & 16 != 0,
        };
        let capture = Capture::default();
        let runtime = Runtime::with_backend(Level::Info, Box::new(capture.clone()), None);
        runtime.set_outputs(outputs);
        let mut expected = Vec::new();
        for kind in KINDS {
            runtime
                .logger()
                .for_kind(kind)
                .info("contract", "switches", Fields::new());
            if kind == LogKind::Tracing {
                if outputs.structured {
                    expected.push((Output::Structured, kind));
                }
            } else if kind == LogKind::Application
                || kind == LogKind::Terminal && outputs.terminal
                || kind == LogKind::Memory && outputs.memory
            {
                if outputs.console {
                    expected.push((Output::Console, kind));
                }
                if outputs.file {
                    expected.push((Output::File, kind));
                }
            }
        }
        barrier(&runtime);
        let actual: Vec<_> = capture
            .records()
            .iter()
            .map(|(out, record)| (*out, record.kind))
            .collect();
        assert_eq!(actual, expected, "switch mask {mask}");
    }
}

#[test]
fn logging_redaction_happens_before_any_backend_can_capture_fields() {
    let capture = Capture::default();
    let runtime = Runtime::with_backend(Level::Info, Box::new(capture.clone()), None);
    runtime.set_outputs(all_outputs());
    for kind in KINDS {
        runtime.logger().for_kind(kind).info(
            "contract",
            "redacted",
            Fields::new()
                .with("password", "hidden")
                .with("detail", "Authorization: Bearer secret-text failed")
                .with("body", "context")
                .with(
                    "items",
                    FieldValue::Array(vec![FieldValue::Object(
                        Fields::new().with("token", "nested-secret"),
                    )]),
                ),
        );
    }
    barrier(&runtime);
    let records = capture.records();
    assert_eq!(records.len(), 7);
    for (_, record) in records {
        assert_eq!(
            record.fields.get("password"),
            Some(&FieldValue::from("[redacted]"))
        );
        assert_eq!(
            record.fields.get("body"),
            Some(&FieldValue::from("context"))
        );
        let captured = format!("{:?}", record.fields);
        for secret in ["hidden", "secret-text", "nested-secret"] {
            assert!(!captured.contains(secret));
        }
    }
}
#[test]
fn logging_runtime_changes_keep_existing_handles_and_do_not_change_other_outputs() {
    let capture = Capture::default();
    let runtime = Runtime::with_backend(Level::Info, Box::new(capture.clone()), None);
    let logger = runtime.logger();
    logger.info("app", "before", Fields::new());
    barrier(&runtime);
    runtime.set_outputs(Outputs {
        file: false,
        console: true,
        ..Outputs::default()
    });
    logger.info("app", "console", Fields::new());
    barrier(&runtime);
    logger.set_level(Level::Off);
    assert_eq!(
        logger.error("app", "hidden", Fields::new()),
        Submit::Filtered
    );
    let records = capture.records();
    assert_eq!(
        records
            .iter()
            .map(|(out, r)| (*out, r.event.as_str()))
            .collect::<Vec<_>>(),
        vec![(Output::File, "before"), (Output::Console, "console")]
    );
}
#[test]
fn logging_runtime_disables_queued_destinations_without_backfilling_new_outputs() {
    let capture = Capture::default();
    let (started, wait) = std::sync::mpsc::channel();
    let (release, gate) = std::sync::mpsc::channel();
    let runtime = Runtime::with_backend(
        Level::Info,
        Box::new(support::GatedCapture {
            capture: capture.clone(),
            started,
            release: gate,
        }),
        None,
    );
    let logger = runtime.logger();
    assert_eq!(logger.info("app", "hold", Fields::new()), Submit::Enqueued);
    wait.recv_timeout(Duration::from_secs(5)).unwrap();
    assert_eq!(
        logger.info("app", "queued", Fields::new()),
        Submit::Enqueued
    );
    runtime.set_outputs(Outputs {
        console: true,
        file: false,
        ..Outputs::default()
    });
    release.send(()).unwrap();
    barrier(&runtime);
    logger.info("app", "future", Fields::new());
    barrier(&runtime);
    assert_eq!(
        capture
            .records()
            .iter()
            .map(|(out, r)| (*out, r.event.as_str()))
            .collect::<Vec<_>>(),
        [(Output::File, "hold"), (Output::Console, "future")]
    );
}
#[test]
fn logging_backend_failure_keeps_other_destinations_and_reports_health() {
    let capture = Capture::default();
    let runtime = Runtime::with_backend(
        Level::Info,
        Box::new(FailingFile {
            capture: capture.clone(),
        }),
        None,
    );
    runtime.set_outputs(all_outputs());
    runtime.logger().info("app", "failure", Fields::new());
    barrier(&runtime);
    assert_eq!(capture.records().len(), 1);
    assert_eq!(runtime.snapshot().health, Health::WriteFailed);
    assert_eq!(runtime.snapshot().dropped, 0);
}
#[test]
fn logging_public_fields_reject_non_finite_numbers() {
    let capture = Capture::default();
    let runtime = Runtime::with_backend(Level::Info, Box::new(capture.clone()), None);
    assert_eq!(
        runtime.logger().info(
            "app",
            "invalid",
            Fields::new().with("number", FieldValue::Float(f64::NAN))
        ),
        Submit::InvalidFields
    );
    barrier(&runtime);
    assert!(capture.records().is_empty());
}

#[test]
fn logging_backend_file_matches_capture_for_every_kind_and_level() {
    for threshold in LEVELS {
        let directory = tempfile::tempdir().unwrap();
        let file = Runtime::file(directory.path().to_owned(), threshold);
        let capture = Capture::default();
        let mock = Runtime::with_backend(threshold, Box::new(capture.clone()), None);
        let outputs = Outputs {
            console: false,
            ..all_outputs()
        };
        for runtime in [&file, &mock] {
            runtime.set_outputs(outputs);
            for kind in KINDS {
                for level in LEVELS {
                    runtime.logger().for_kind(kind).log(
                        level,
                        "contract",
                        kind.as_str(),
                        Fields::new()
                            .with("token", "secret-value")
                            .with("visible", "context"),
                    );
                }
            }
            barrier(runtime);
        }
        for (output, name) in [
            (Output::File, "usher.log"),
            (Output::Structured, "usher_tracing.log"),
        ] {
            let mut lines = Vec::new();
            for bucket in std::fs::read_dir(directory.path()).unwrap() {
                let path = bucket.unwrap().path().join(name);
                if path.is_file() {
                    lines.extend(
                        std::fs::read_to_string(path)
                            .unwrap()
                            .lines()
                            .map(|line| serde_json::from_str::<serde_json::Value>(line).unwrap()),
                    );
                }
            }
            let expected: Vec<_> = capture
                .records()
                .into_iter()
                .filter(|(target, _)| *target == output)
                .map(|(_, record)| (record.level.as_str().to_owned(), record.event))
                .collect();
            let actual: Vec<_> = lines
                .iter()
                .map(|line| {
                    assert_eq!(line["fields"]["token"], "[redacted]");
                    assert_eq!(line["fields"]["visible"], "context");
                    (
                        line["level"].as_str().unwrap().to_owned(),
                        line["event"].as_str().unwrap().to_owned(),
                    )
                })
                .collect();
            assert_eq!(actual, expected, "{threshold:?}/{output:?}");
        }
    }
}
