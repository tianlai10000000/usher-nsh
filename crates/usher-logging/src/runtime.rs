//! Assembly-only lifecycle, backend and archive contracts, independent of the application host.
pub use crate::archive::{ExportError, ExportReport};
pub use crate::pipeline::{Health, HealthSnapshot, StatusObserver};
pub use crate::record::Record;
use crate::{Level, LogKind, Logger, Outputs, config::policy, pipeline::Pipeline};
use std::{
    path::{Path, PathBuf},
    sync::Arc,
    time::Duration,
};

/// A synchronous adapter owned by the logging worker. Never call back into logging here.
/// The common pipeline already filtered and redacted each record before this call.
pub trait LogWriter: Send + 'static {
    fn write(&mut self, output: crate::Output, record: &Record) -> Result<(), Health>;
    fn flush(&mut self) -> Result<(), Health>;
    /// Periodic buffer maintenance; adapters may avoid a full storage barrier here.
    fn flush_pending(&mut self) -> Result<(), Health> {
        self.flush()
    }
}

/// An export request uses ISO calendar dates and a host-supplied destination folder.
pub struct ExportRequest<'a> {
    pub destination_dir: &'a Path,
    pub from: &'a str,
    pub to: &'a str,
    pub app_version: &'a str,
    pub platform: &'a str,
    pub cancelled: &'a dyn Fn() -> bool,
}
#[derive(Debug)]
pub struct Exported {
    pub name: String,
    pub report: ExportReport,
}
/// Optional storage capability, supplied alongside an output adapter at assembly time.
pub trait ArchiveProvider: Send + Sync {
    fn export(&self, request: &ExportRequest<'_>) -> Result<Exported, ExportError>;
}
struct FileArchive {
    root: PathBuf,
}
impl ArchiveProvider for FileArchive {
    fn export(&self, request: &ExportRequest<'_>) -> Result<Exported, ExportError> {
        let parse = |value: &str| {
            chrono::NaiveDate::parse_from_str(value, "%Y-%m-%d")
                .map_err(|_| ExportError::InvalidRange)
        };
        let from = parse(request.from)?;
        let to = parse(request.to)?;
        if from > to {
            return Err(ExportError::InvalidRange);
        }
        std::fs::create_dir_all(request.destination_dir).map_err(|_| ExportError::Io)?;
        let name = crate::archive::archive_name();
        let destination = request.destination_dir.join(&name);
        let report = crate::archive::export(&crate::archive::ExportRequest {
            root: &self.root,
            destination: &destination,
            from,
            to,
            app_version: request.app_version,
            platform: request.platform,
            cancelled: request.cancelled,
        })?;
        crate::archive::prune_exports(request.destination_dir, &destination)
            .map_err(|_| ExportError::Io)?;
        Ok(Exported { name, report })
    }
}

/// An assembly handle. Views receive only its lightweight logger, never storage internals.
#[derive(Clone)]
pub struct Runtime {
    inner: Arc<Pipeline>,
    archive: Option<Arc<dyn ArchiveProvider>>,
}
impl Runtime {
    /// Creates an isolated pipeline without installing process hooks or global state.
    pub fn with_backend(
        level: Level,
        writer: Box<dyn LogWriter>,
        archive: Option<Arc<dyn ArchiveProvider>>,
    ) -> Self {
        Self {
            inner: Arc::new(Pipeline::with_backend(
                PathBuf::new(),
                level,
                policy().queue_capacity,
                writer,
            )),
            archive,
        }
    }
    /// Creates an isolated instance of the built-in file and console adapter.
    pub fn file(root: PathBuf, level: Level) -> Self {
        Self {
            inner: Arc::new(Pipeline::start(root.clone(), level)),
            archive: Some(Arc::new(FileArchive { root })),
        }
    }
    pub fn logger(&self) -> Logger {
        Logger {
            inner: self.inner.clone(),
            kind: LogKind::Application,
        }
    }
    pub fn outputs(&self) -> Outputs {
        self.inner.outputs()
    }
    pub fn set_outputs(&self, outputs: Outputs) {
        self.inner.set_outputs(outputs);
    }
    pub fn snapshot(&self) -> HealthSnapshot {
        HealthSnapshot {
            health: self.inner.health(),
            dropped: self.inner.dropped(),
        }
    }
    pub fn observe(&self, observer: Arc<dyn StatusObserver>) {
        self.inner.observe(observer);
    }
    /// Returns whether the queue barrier was acknowledged; it is not a durable-write guarantee.
    /// The timeout covers acknowledgement after admission, not a blocked queue send.
    pub fn flush(&self, timeout: Duration) -> bool {
        self.inner.flush(timeout)
    }
    /// Records the existing settings audit event, then applies the running filter.
    pub fn change_level(&self, level: Level) {
        if self.inner.level() != level {
            self.inner.record_level_change(level);
            self.inner
                .flush(Duration::from_millis(policy().change_flush_ms));
            self.inner.set_level(level);
        }
    }
    /// Exports the storage owned by this runtime; invoke on a blocking worker.
    pub fn export(&self, request: &ExportRequest<'_>) -> Result<Exported, ExportError> {
        let provider = self.archive.as_ref().ok_or(ExportError::Unsupported)?;
        self.flush(Duration::from_millis(policy().export_flush_ms));
        provider.export(request)
    }
}
/// Reuses the process writer for the same root across window mounts and HMR.
pub fn install(root: PathBuf, level: Level, outputs: Outputs) -> Runtime {
    let inner = crate::lifecycle::install(root.clone(), level);
    inner.set_outputs(outputs);
    Runtime {
        inner,
        archive: Some(Arc::new(FileArchive { root })),
    }
}
pub fn current() -> Option<Runtime> {
    crate::lifecycle::current().map(|inner| {
        let root = inner.root().to_owned();
        Runtime {
            inner,
            archive: Some(Arc::new(FileArchive { root })),
        }
    })
}
pub fn flush_for_exit() {
    crate::lifecycle::flush_for_exit();
}
