//! Assembles logging and adapts its health observer to the application state service.
use std::{
    path::PathBuf,
    sync::{Arc, Mutex},
};
pub(crate) use usher_logging::runtime::{current, flush_for_exit};
use usher_logging::{
    Fields, Level, LogKind, Outputs,
    runtime::{HealthSnapshot, Runtime, StatusObserver},
};
struct Observer {
    source: u64,
    service: Arc<crate::state::diagnostics::StatusService>,
}
impl StatusObserver for Observer {
    fn changed(&self, sample: &dyn Fn() -> HealthSnapshot) {
        self.service.update(self.source, || {
            let sample = sample();
            crate::state::diagnostics::Status {
                problem: sample.health.code().map(str::to_owned),
                dropped: sample.dropped,
            }
        });
    }
}
/// Serializes pipeline installation and application observer generation assignment.
pub(crate) fn install(root: PathBuf, level: Level, outputs: Outputs) -> Runtime {
    static INSTALL: Mutex<()> = Mutex::new(());
    let _guard = INSTALL.lock().unwrap_or_else(|p| p.into_inner());
    let runtime = usher_logging::runtime::install(root, level, outputs);
    if let Ok(status) = &*crate::state::diagnostics::STATUS
        && let Ok(source) = status.activate()
    {
        runtime.observe(Arc::new(Observer {
            source,
            service: status.clone(),
        }));
    }
    runtime
}
pub(crate) fn started(runtime: &Runtime) {
    runtime.logger().info(
        "app",
        "diagnostics_started",
        Fields::new()
            .with("version", env!("CARGO_PKG_VERSION"))
            .with("level", runtime.logger().level().as_str()),
    );
    configured(runtime);
}
pub(crate) fn configured(runtime: &Runtime) {
    let outputs = runtime.outputs();
    runtime.logger().for_kind(LogKind::Tracing).info(
        "diagnostics",
        "output_configuration",
        Fields::new()
            .with("console", outputs.console)
            .with("file", outputs.file)
            .with("structured", outputs.structured)
            .with("level", runtime.logger().level().as_str()),
    );
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn worker_health_reaches_application_status() {
        let directory = tempfile::tempdir().unwrap();
        let path = directory.path().join("blocked");
        std::fs::write(&path, "file").unwrap();
        let runtime = Runtime::file(path, Level::Info);
        let service = Arc::new(
            crate::state::diagnostics::StatusService::new(usher_state::StoreId::new(u128::MAX))
                .unwrap(),
        );
        let source = service.activate().unwrap();
        runtime.observe(Arc::new(Observer {
            source,
            service: service.clone(),
        }));
        runtime.logger().info("test", "health", Fields::new());
        assert!(runtime.flush(std::time::Duration::from_secs(2)));
        assert_eq!(
            service.snapshot().unwrap().state().problem.as_deref(),
            runtime.snapshot().health.code()
        );
        assert!(runtime.snapshot().health.code().is_some());
    }
}
