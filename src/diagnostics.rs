//! Adapts the independent logging pipeline to application diagnostic state.
use std::{
    path::PathBuf,
    sync::{Arc, Mutex},
};
pub(crate) use usher_logging::{
    ExportRequest, Level, Logger, archive_name, current, export, prune_exports,
};

struct Observer {
    source: u64,
    service: Arc<crate::diagnostic_state::StatusService>,
}
impl usher_logging::StatusObserver for Observer {
    fn changed(&self, sample: &dyn Fn() -> usher_logging::HealthSnapshot) {
        self.service.update(self.source, || {
            let sample = sample();
            crate::diagnostic_state::Status {
                problem: sample.health.code().map(str::to_owned),
                dropped: sample.dropped,
            }
        });
    }
}

/// Serializes logger replacement and observer generation assignment across windows.
pub(crate) fn install(root: PathBuf, level: Level) -> Arc<Logger> {
    static INSTALL: Mutex<()> = Mutex::new(());
    let _guard = INSTALL.lock().unwrap_or_else(|p| p.into_inner());
    let logger = usher_logging::install(root, level);
    if let Ok(status) = &*crate::diagnostic_state::STATUS
        && let Ok(source) = status.activate()
    {
        logger.observe(Arc::new(Observer {
            source,
            service: status.clone(),
        }));
    }
    logger
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn worker_health_reaches_application_status() {
        let directory = tempfile::tempdir().unwrap();
        let path = directory.path().join("blocked");
        std::fs::write(&path, "file").unwrap();
        let logger = Logger::start(path, Level::Info);
        let service = Arc::new(
            crate::diagnostic_state::StatusService::new(usher_state::StoreId::new(u128::MAX))
                .unwrap(),
        );
        let source = service.activate().unwrap();
        logger.observe(Arc::new(Observer {
            source,
            service: service.clone(),
        }));
        logger.log(Level::Info, "test", "health", None);
        assert!(logger.flush(std::time::Duration::from_secs(2)));
        assert_eq!(
            service.snapshot().unwrap().state().problem.as_deref(),
            logger.health().code()
        );
        assert!(logger.health().code().is_some());
    }
}
