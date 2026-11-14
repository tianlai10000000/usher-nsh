//! Verifies settings persistence and the production logging facade together, using isolated data.
use std::{
    fs,
    sync::{Arc, Mutex},
    time::Duration,
};
use usher_logging::{
    Fields, Level, LogKind, Output,
    runtime::{Health, LogWriter, Record, Runtime},
};
use usher_nsh::state::settings::{Command, DiagnosticLevel, DiagnosticOutput, SettingsRegistry};
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
#[test]
fn diagnostics_settings_restore_outputs_and_keep_default_documents_compatible() {
    let root = tempfile::tempdir().unwrap();
    let path = root.path().join("settings.toml");
    let registry = SettingsRegistry::default();
    let service = registry.for_path(path.clone()).unwrap();
    let before = service.snapshot().unwrap();
    assert!(!before.diagnostic_console && before.diagnostic_file && !before.diagnostic_structured);
    for (output, enabled) in [
        (DiagnosticOutput::Console, true),
        (DiagnosticOutput::File, false),
        (DiagnosticOutput::Structured, true),
    ] {
        service
            .apply(Command::SetDiagnosticOutput(output, enabled))
            .unwrap();
    }
    let reopened = SettingsRegistry::default().for_path(path.clone()).unwrap();
    let snapshot = reopened.snapshot().unwrap();
    assert!(
        snapshot.diagnostic_console && !snapshot.diagnostic_file && snapshot.diagnostic_structured
    );
    for (output, enabled) in [
        (DiagnosticOutput::Console, false),
        (DiagnosticOutput::File, true),
        (DiagnosticOutput::Structured, false),
    ] {
        reopened
            .apply(Command::SetDiagnosticOutput(output, enabled))
            .unwrap();
    }
    assert!(!fs::read_to_string(path).unwrap().contains("[diagnostics]"));
    assert!("terminal".parse::<DiagnosticOutput>().is_err());
    assert!("memory".parse::<DiagnosticOutput>().is_err());
}
#[test]
fn diagnostics_settings_drive_a_bound_runtime_and_survive_failed_persistence() {
    let root = tempfile::tempdir().unwrap();
    let folder = root.path().join("profile");
    let path = folder.join("settings.toml");
    let service = SettingsRegistry::default().for_path(path).unwrap();
    let capture = Capture::default();
    let runtime = Runtime::with_backend(Level::Trace, Box::new(capture.clone()), None);
    service.attach_diagnostics(runtime.clone()).unwrap();
    assert_eq!(runtime.logger().level(), Level::Info);
    fs::write(&folder, "block saving").unwrap();
    let snapshot = service
        .apply(Command::SetDiagnosticOutput(
            DiagnosticOutput::Structured,
            true,
        ))
        .unwrap();
    assert!(!snapshot.saved);
    assert!(runtime.outputs().structured);
    runtime
        .logger()
        .for_kind(LogKind::Tracing)
        .info("contract", "unsaved_output", Fields::new());
    assert!(runtime.flush(Duration::from_secs(2)));
    assert!(
        capture
            .0
            .lock()
            .unwrap()
            .iter()
            .any(|(out, record)| *out == Output::Structured && record.event == "unsaved_output")
    );
    fs::remove_file(&folder).unwrap();
    assert!(
        service
            .apply(Command::SetDiagnosticOutput(
                DiagnosticOutput::Structured,
                true
            ))
            .unwrap()
            .saved
    );
    service
        .apply(Command::SetDiagnosticLevel(DiagnosticLevel::Off))
        .unwrap();
    assert_eq!(runtime.logger().level(), Level::Off);
    assert!(
        !runtime
            .logger()
            .for_kind(LogKind::Tracing)
            .enabled(Level::Error)
    );
}
#[test]
fn diagnostics_real_files_are_separate_and_export_preserves_both_streams() {
    let root = tempfile::tempdir().unwrap();
    let logs = root.path().join("logs");
    let exports = root.path().join("exports");
    let runtime = Runtime::file(logs.clone(), Level::Info);
    runtime.set_outputs(usher_logging::Outputs {
        structured: true,
        ..Default::default()
    });
    runtime.logger().info(
        "integration",
        "ordinary",
        Fields::new().with("secret", "hidden"),
    );
    runtime
        .logger()
        .for_kind(LogKind::Tracing)
        .info("integration", "structured", Fields::new());
    assert!(runtime.flush(Duration::from_secs(2)));
    let day = chrono::Local::now().date_naive().to_string();
    let directory = logs.join(&day);
    let application = fs::read_to_string(directory.join("usher.log")).unwrap();
    let structured = fs::read_to_string(directory.join("usher_tracing.log")).unwrap();
    assert!(
        application.contains("ordinary")
            && !application.contains("hidden")
            && !application.contains("structured")
    );
    assert!(structured.contains("structured") && structured.contains("tracing"));
    runtime.set_outputs(usher_logging::Outputs {
        file: false,
        structured: false,
        ..Default::default()
    });
    let exported = runtime
        .export(&usher_logging::runtime::ExportRequest {
            destination_dir: &exports,
            from: &day,
            to: &day,
            app_version: "test",
            platform: "test",
            cancelled: &|| false,
        })
        .unwrap();
    assert_eq!(exported.report.files, 2);
    assert!(exports.join(exported.name).exists());
}
