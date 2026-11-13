//! Owns the bounded queue, severity filter, output routing and ordered health notifications.
use crate::{
    Level, LogKind, Output, Outputs, Submit, config::policy, record::Record, runtime::LogWriter,
};
use chrono::{DateTime, Local};
use serde_json::{Map, Value};
use std::{
    collections::BTreeMap,
    path::PathBuf,
    sync::{
        Arc, Mutex,
        atomic::{AtomicU8, AtomicU64, Ordering},
        mpsc::{self, Receiver, RecvTimeoutError, SyncSender, TrySendError},
    },
    thread::{self, JoinHandle},
    time::Duration,
};

/// Stable, coarse failure classes for application health projections.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum Health {
    Ok,
    DirectoryUnavailable,
    WriteFailed,
}
impl Health {
    pub fn code(self) -> Option<&'static str> {
        match self {
            Self::Ok => None,
            Self::DirectoryUnavailable => Some("diagnostics_directory_unavailable"),
            Self::WriteFailed => Some("diagnostics_write_failed"),
        }
    }
}
/// Health sampled under the observer's own publication ordering.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct HealthSnapshot {
    pub health: Health,
    pub dropped: u64,
}
/// The callback must sample synchronously and must not re-enter logging.
pub trait StatusObserver: Send + Sync {
    fn changed(&self, sample: &dyn Fn() -> HealthSnapshot);
}

struct Shared {
    level: AtomicU8,
    outputs: Mutex<Outputs>,
    dropped: AtomicU64,
    health: Mutex<BTreeMap<Output, Health>>,
    reporting: Mutex<Option<Arc<dyn StatusObserver>>>,
}
impl Shared {
    fn health(&self) -> Health {
        self.health
            .lock()
            .unwrap_or_else(|p| p.into_inner())
            .values()
            .copied()
            .find(|h| *h != Health::Ok)
            .unwrap_or(Health::Ok)
    }
    fn publish(&self) {
        let observer = self
            .reporting
            .lock()
            .unwrap_or_else(|p| p.into_inner())
            .clone();
        if let Some(observer) = observer {
            observer.changed(&|| HealthSnapshot {
                health: self.health(),
                dropped: self.dropped.load(Ordering::Relaxed),
            });
        }
    }
    fn report(&self, output: Output, health: Health) {
        self.health
            .lock()
            .unwrap_or_else(|p| p.into_inner())
            .insert(output, health);
        self.publish();
    }
}
enum Message {
    Record(Record, Vec<Output>),
    Flush(mpsc::Sender<()>),
    Stop,
}

pub(crate) struct Pipeline {
    root: PathBuf,
    sender: SyncSender<Message>,
    shared: Arc<Shared>,
    worker: Option<JoinHandle<()>>,
}
impl Pipeline {
    pub(crate) fn start(root: PathBuf, level: Level) -> Self {
        Self::start_with_capacity(root, level, policy().queue_capacity)
    }
    pub(crate) fn start_with_capacity(root: PathBuf, level: Level, capacity: usize) -> Self {
        let writer = Box::new(crate::backends::file::FileWriter::new(root.clone()));
        Self::with_backend(root, level, capacity, writer)
    }
    pub(crate) fn with_backend(
        root: PathBuf,
        level: Level,
        capacity: usize,
        writer: Box<dyn LogWriter>,
    ) -> Self {
        let (sender, receiver) = mpsc::sync_channel(capacity);
        let shared = Arc::new(Shared {
            level: AtomicU8::new(level as u8),
            outputs: Mutex::new(Outputs::default()),
            dropped: AtomicU64::new(0),
            health: Mutex::new(BTreeMap::new()),
            reporting: Mutex::new(None),
        });
        let state = shared.clone();
        let worker = thread::Builder::new()
            .name("usher-diagnostics".into())
            .spawn(move || run_worker(writer, &receiver, &state))
            .ok();
        if worker.is_none() {
            shared.report(Output::File, Health::WriteFailed);
        }
        Self {
            root,
            sender,
            shared,
            worker,
        }
    }
    pub(crate) fn root(&self) -> &std::path::Path {
        &self.root
    }
    pub(crate) fn observe(&self, observer: Arc<dyn StatusObserver>) {
        *self
            .shared
            .reporting
            .lock()
            .unwrap_or_else(|p| p.into_inner()) = Some(observer);
        self.shared.publish();
    }
    pub(crate) fn level(&self) -> Level {
        Level::from_u8(self.shared.level.load(Ordering::Relaxed))
    }
    pub(crate) fn set_level(&self, level: Level) {
        self.shared.level.store(level as u8, Ordering::Relaxed);
    }
    pub(crate) fn outputs(&self) -> Outputs {
        *self
            .shared
            .outputs
            .lock()
            .unwrap_or_else(|p| p.into_inner())
    }
    pub(crate) fn set_outputs(&self, outputs: Outputs) {
        *self
            .shared
            .outputs
            .lock()
            .unwrap_or_else(|p| p.into_inner()) = outputs;
        self.shared
            .health
            .lock()
            .unwrap_or_else(|p| p.into_inner())
            .retain(|output, _| match output {
                Output::Console => outputs.console,
                Output::File => outputs.file,
                Output::Structured => outputs.structured,
            });
        self.shared.publish();
    }
    pub(crate) fn enabled(&self, kind: LogKind, level: Level) -> bool {
        crate::level::enabled(self.level(), level) && !self.outputs().destinations(kind).is_empty()
    }
    pub(crate) fn health(&self) -> Health {
        self.shared.health()
    }
    pub(crate) fn dropped(&self) -> u64 {
        self.shared.dropped.load(Ordering::Relaxed)
    }
    #[cfg(test)]
    pub(crate) fn log_at(
        &self,
        at: DateTime<Local>,
        level: Level,
        module: &str,
        event: &str,
        fields: Option<Map<String, Value>>,
    ) {
        self.submit(
            at,
            level,
            LogKind::Application,
            module,
            event,
            fields.unwrap_or_default(),
        );
    }
    pub(crate) fn submit(
        &self,
        at: DateTime<Local>,
        level: Level,
        kind: LogKind,
        module: &str,
        event: &str,
        fields: Map<String, Value>,
    ) -> Submit {
        if !self.enabled(kind, level) {
            return Submit::Filtered;
        }
        self.enqueue(Record::new(at, level, kind, module, event, fields))
    }
    pub(crate) fn record_level_change(&self, new: Level) {
        let old = self.level();
        if old == Level::Off && new == Level::Off {
            return;
        }
        let fields = Map::from_iter([
            ("from".into(), old.as_str().into()),
            ("to".into(), new.as_str().into()),
        ]);
        self.enqueue(Record::new(
            Local::now(),
            Level::Info,
            LogKind::Application,
            "settings",
            "diagnostic_level_changed",
            fields,
        ));
    }
    fn enqueue(&self, record: Record) -> Submit {
        let outputs = self.outputs().destinations(record.kind);
        if outputs.is_empty() {
            return Submit::Filtered;
        }
        let level = record.level;
        match self.sender.try_send(Message::Record(record, outputs)) {
            Ok(()) => Submit::Enqueued,
            Err(TrySendError::Full(message)) if level <= Level::Warn => self
                .sender
                .send(message)
                .map_or(Submit::Unavailable, |_| Submit::Enqueued),
            Err(TrySendError::Full(_)) => {
                let _ = self.shared.dropped.fetch_update(
                    Ordering::Relaxed,
                    Ordering::Relaxed,
                    |value| Some(value.saturating_add(1)),
                );
                self.shared.publish();
                Submit::Dropped
            }
            Err(TrySendError::Disconnected(_)) => Submit::Unavailable,
        }
    }
    /// Waits for a queue barrier. The send itself may block; this is not a total deadline.
    pub(crate) fn flush(&self, timeout: Duration) -> bool {
        let (done, wait) = mpsc::channel();
        if self.sender.send(Message::Flush(done)).is_err() {
            return false;
        }
        wait.recv_timeout(timeout).is_ok()
    }
}
impl Drop for Pipeline {
    fn drop(&mut self) {
        let _ = self.flush(Duration::from_millis(policy().exit_flush_ms));
        let _ = self.sender.send(Message::Stop);
        if let Some(worker) = self.worker.take() {
            let _ = worker.join();
        }
    }
}
fn run_worker(mut writer: Box<dyn LogWriter>, receiver: &Receiver<Message>, shared: &Shared) {
    loop {
        match receiver.recv_timeout(Duration::from_millis(policy().flush_interval_ms)) {
            Ok(Message::Record(record, destinations)) => {
                for output in destinations {
                    // Re-read switches between writes; never resurrect a destination absent at admission.
                    let enabled = shared
                        .outputs
                        .lock()
                        .unwrap_or_else(|p| p.into_inner())
                        .destinations(record.kind)
                        .contains(&output);
                    if enabled {
                        shared.report(
                            output,
                            writer.write(output, &record).err().unwrap_or(Health::Ok),
                        );
                    }
                }
            }
            Ok(Message::Flush(done)) => {
                let _ = writer.flush();
                let _ = done.send(());
            }
            Ok(Message::Stop) | Err(RecvTimeoutError::Disconnected) => {
                let _ = writer.flush();
                break;
            }
            Err(RecvTimeoutError::Timeout) => {
                let _ = writer.flush_pending();
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::test_support::{at, date, read_log};
    use Pipeline as Logger;
    use std::fs;
    #[test]
    fn events_land_in_the_bucket_of_their_own_local_date() {
        let dir = tempfile::tempdir().expect("tempdir");
        let logger = Logger::start(dir.path().to_path_buf(), Level::Info);
        logger.log_at(
            at(2026, 11, 9, 23, 59),
            Level::Info,
            "app",
            "before_midnight",
            None,
        );
        logger.log_at(
            at(2026, 11, 10, 0, 1),
            Level::Info,
            "app",
            "after_midnight",
            None,
        );
        assert!(logger.flush(Duration::from_secs(5)));
        assert!(read_log(dir.path(), date(2026, 11, 9)).contains("before_midnight"));
        assert!(read_log(dir.path(), date(2026, 11, 10)).contains("after_midnight"));
        assert!(!read_log(dir.path(), date(2026, 11, 10)).contains("before_midnight"));
    }

    #[test]
    fn changing_the_level_takes_effect_without_a_restart() {
        let dir = tempfile::tempdir().expect("tempdir");
        let logger = Logger::start(dir.path().to_path_buf(), Level::Warn);
        let day = at(2026, 11, 10, 12, 0);
        logger.log_at(day, Level::Info, "app", "hidden", None);
        logger.set_level(Level::Debug);
        logger.log_at(day, Level::Debug, "app", "shown", None);
        logger.set_level(Level::Off);
        logger.log_at(day, Level::Error, "app", "silenced", None);
        assert!(logger.flush(Duration::from_secs(5)));
        let text = read_log(dir.path(), date(2026, 11, 10));
        assert!(text.contains("shown"));
        assert!(!text.contains("hidden"));
        assert!(!text.contains("silenced"));
    }

    #[test]
    fn a_level_change_is_recorded_even_when_the_new_filter_would_hide_it() {
        let dir = tempfile::tempdir().expect("tempdir");
        let logger = Logger::start(dir.path().to_path_buf(), Level::Debug);
        logger.record_level_change(Level::Error);
        logger.set_level(Level::Error);
        // Going from ERROR to OFF must also leave its own line, although INFO is filtered.
        logger.record_level_change(Level::Off);
        logger.set_level(Level::Off);
        // OFF to OFF writes nothing.
        logger.record_level_change(Level::Off);
        assert!(logger.flush(Duration::from_secs(5)));
        let today = Local::now().date_naive();
        let text = read_log(dir.path(), today);
        let lines: Vec<Value> = text
            .lines()
            .map(|line| serde_json::from_str(line).expect("json"))
            .collect();
        assert_eq!(lines.len(), 2, "{text}");
        assert_eq!(lines[0]["fields"]["from"], "DEBUG");
        assert_eq!(lines[0]["fields"]["to"], "ERROR");
        assert_eq!(lines[1]["fields"]["from"], "ERROR");
        assert_eq!(lines[1]["fields"]["to"], "OFF");
    }

    #[test]
    fn an_unwritable_directory_is_reported_and_never_panics() {
        let dir = tempfile::tempdir().expect("tempdir");
        // A file where the log root should be makes every write fail.
        let blocked = dir.path().join("logs");
        fs::write(&blocked, "in the way").expect("write");
        let logger = Logger::start(blocked, Level::Info);
        logger.log_at(at(2026, 11, 10, 12, 0), Level::Info, "app", "lost", None);
        assert!(logger.flush(Duration::from_secs(5)));
        assert_ne!(logger.health(), Health::Ok);
        assert!(logger.health().code().is_some());
    }

    #[test]
    fn a_full_queue_drops_chatter_but_preserves_high_priority_records() {
        struct Gate {
            started: mpsc::Sender<()>,
            release: mpsc::Receiver<()>,
            events: Arc<Mutex<Vec<String>>>,
        }
        impl LogWriter for Gate {
            fn write(&mut self, _: Output, record: &Record) -> Result<(), Health> {
                if record.event == "hold" {
                    self.started.send(()).unwrap();
                    self.release.recv_timeout(Duration::from_secs(5)).unwrap();
                }
                self.events.lock().unwrap().push(record.event.clone());
                Ok(())
            }
            fn flush(&mut self) -> Result<(), Health> {
                Ok(())
            }
        }
        let (started, wait) = mpsc::channel();
        let (release, gate) = mpsc::channel();
        let events = Arc::new(Mutex::new(Vec::new()));
        let logger = Arc::new(Pipeline::with_backend(
            PathBuf::new(),
            Level::Trace,
            1,
            Box::new(Gate {
                started,
                release: gate,
                events: events.clone(),
            }),
        ));
        let submit = |level, event| {
            logger.submit(
                Local::now(),
                level,
                LogKind::Application,
                "test",
                event,
                Map::new(),
            )
        };
        assert_eq!(submit(Level::Info, "hold"), Submit::Enqueued);
        wait.recv_timeout(Duration::from_secs(5)).unwrap();
        assert_eq!(submit(Level::Info, "queued"), Submit::Enqueued);
        for level in [Level::Info, Level::Debug, Level::Trace] {
            assert_eq!(submit(level, "dropped"), Submit::Dropped);
        }
        assert_eq!(logger.dropped(), 3);
        let copy = logger.clone();
        let high = std::thread::spawn(move || {
            for level in [Level::Warn, Level::Error] {
                assert_eq!(
                    copy.submit(
                        Local::now(),
                        level,
                        LogKind::Application,
                        "test",
                        level.as_str(),
                        Map::new()
                    ),
                    Submit::Enqueued
                );
            }
        });
        release.send(()).unwrap();
        high.join().unwrap();
        assert!(logger.flush(Duration::from_secs(5)));
        assert_eq!(*events.lock().unwrap(), ["hold", "queued", "WARN", "ERROR"]);
    }
}
