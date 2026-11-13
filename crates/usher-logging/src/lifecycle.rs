//! Owns the process pipeline and chains the host panic and normal-exit hooks once.
use crate::{Fields, Level, Logger, config::policy, pipeline::Pipeline};
use std::{
    path::PathBuf,
    sync::{Arc, LazyLock, Mutex, Once},
    time::Duration,
};
static LOGGER: LazyLock<Mutex<Option<Arc<Pipeline>>>> = LazyLock::new(|| Mutex::new(None));
pub(crate) fn install(root: PathBuf, level: Level) -> Arc<Pipeline> {
    let mut slot = LOGGER.lock().unwrap_or_else(|p| p.into_inner());
    if let Some(existing) = slot.as_ref().filter(|logger| logger.root() == root) {
        existing.set_level(level);
        return existing.clone();
    }
    let logger = Arc::new(Pipeline::start(root, level));
    let previous = slot.replace(logger.clone());
    drop(slot);
    // Draining the previous writer must not hold the registry lock.
    drop(previous);
    install_hooks();
    logger
}
pub(crate) fn current() -> Option<Arc<Pipeline>> {
    LOGGER.lock().unwrap_or_else(|p| p.into_inner()).clone()
}
pub(crate) fn flush_for_exit() {
    let logger = LOGGER.try_lock().ok().and_then(|slot| slot.clone());
    if let Some(logger) = logger {
        logger.flush(Duration::from_millis(policy().exit_flush_ms));
    }
}
extern "C" fn flush_at_exit() {
    flush_for_exit();
}
fn install_hooks() {
    static INSTALLED: Once = Once::new();
    INSTALLED.call_once(|| {
        let previous = std::panic::take_hook();
        std::panic::set_hook(Box::new(move |info| {
            if let Some(inner) = current() {
                let location = info
                    .location()
                    .map(|p| format!("{}:{}", p.file(), p.line()))
                    .unwrap_or_default();
                let logger = Logger {
                    inner: inner.clone(),
                    kind: crate::LogKind::Application,
                };
                logger.error("process", "panic", Fields::new().with("location", location));
                inner.flush(Duration::from_millis(policy().change_flush_ms));
            }
            previous(info);
        }));
        // SAFETY: the callback has C ABI and remains valid for the lifetime of this process.
        unsafe {
            libc::atexit(flush_at_exit);
        }
    });
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::test_support::{at, date, read_log};
    static GLOBAL_LOGGER: Mutex<()> = Mutex::new(());
    fn global_logger_guard() -> std::sync::MutexGuard<'static, ()> {
        GLOBAL_LOGGER
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner())
    }

    #[test]
    fn flushing_for_exit_puts_queued_lines_on_disk_and_never_hangs_without_a_logger() {
        let _serial = global_logger_guard();
        let dir = tempfile::tempdir().expect("tempdir");
        let logger = install(dir.path().to_path_buf(), Level::Info);
        logger.log_at(
            at(2026, 11, 10, 12, 0),
            Level::Info,
            "app",
            "last_words",
            None,
        );
        flush_for_exit();
        assert!(read_log(dir.path(), date(2026, 11, 10)).contains("last_words"));
    }

    #[test]
    fn installing_twice_for_one_directory_reuses_the_writer() {
        let _serial = global_logger_guard();
        let dir = tempfile::tempdir().expect("tempdir");
        let first = install(dir.path().to_path_buf(), Level::Info);
        let second = install(dir.path().to_path_buf(), Level::Debug);
        assert!(Arc::ptr_eq(&first, &second));
        assert_eq!(second.level(), Level::Debug);
    }
}
