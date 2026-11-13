//! Assembles directory-owned settings services and isolated engine sessions.

use std::{
    collections::HashMap,
    path::PathBuf,
    sync::{Arc, LazyLock, Mutex, OnceLock},
};

use usher_state::{StoreId, backends::WatchBackend};

pub use crate::background::BackgroundImage;
pub use crate::settings::{
    ColorMode, Command, ComponentFamily, CustomTheme, DiagnosticLevel, Palette, Snapshot,
};
pub use crate::settings_service::SettingsService;
pub use crate::style_themes::{
    Material, StyleCssVars, StyleTheme, ThemeFileIssue, ThemeSource, Tokens,
};

use crate::settings::SettingsStore;

pub(crate) const UNAVAILABLE: &str = "settings_store_unavailable";
pub(crate) static SETTINGS: LazyLock<SettingsRegistry> = LazyLock::new(SettingsRegistry::default);

// IDs belong to the process, not a registry, path or persistent document. Checked
// allocation prevents reuse even when independent registries or sessions are dropped.
static NEXT_STORE_ID: Mutex<u128> = Mutex::new(0);

pub(crate) fn next_store_id() -> Result<StoreId, &'static str> {
    let mut next = NEXT_STORE_ID.lock().map_err(|_| UNAVAILABLE)?;
    *next = next.checked_add(1).ok_or(UNAVAILABLE)?;
    Ok(StoreId::new(*next))
}

type ServiceCell = Arc<OnceLock<Result<Arc<SettingsService>, &'static str>>>;

/// Owns one settings service per host-resolved absolute settings file path.
/// Paths are trusted application inputs, never supplied by JavaScript. Callers
/// must resolve aliases consistently. Retain one registry for the application
/// lifetime; creating two registries for the same file does not coordinate writes.
#[derive(Default)]
pub struct SettingsRegistry {
    services: Mutex<HashMap<PathBuf, ServiceCell>>,
}

impl SettingsRegistry {
    /// Finds or initializes a service on a blocking worker. Registry locking only
    /// covers entry lookup; initialization and persistence use per-entry ownership.
    pub fn for_path(&self, path: PathBuf) -> Result<Arc<SettingsService>, &'static str> {
        self.resolve(path.clone(), || {
            let id = next_store_id()?;
            SettingsService::new(SettingsStore::open(path), id, &WatchBackend).map(Arc::new)
        })
    }

    fn resolve(
        &self,
        path: PathBuf,
        initialize: impl FnOnce() -> Result<Arc<SettingsService>, &'static str>,
    ) -> Result<Arc<SettingsService>, &'static str> {
        if !path.is_absolute() {
            return Err(UNAVAILABLE);
        }
        let entry = self
            .services
            .lock()
            .map_err(|_| UNAVAILABLE)?
            .entry(path.clone())
            .or_default()
            .clone();
        entry.get_or_init(initialize).clone()
    }
}

impl SettingsService {
    /// Creates an isolated, memory-only engine session. Dropping its last owner
    /// closes subscriptions. This service is never inserted into the directory registry.
    pub fn session() -> Result<Arc<Self>, &'static str> {
        Self::new(
            SettingsStore::session_only(),
            next_store_id()?,
            &WatchBackend,
        )
        .map(Arc::new)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::{sync::mpsc, thread};

    #[test]
    fn slow_initialization_does_not_hold_the_directory_registry_lock() {
        let directory = tempfile::tempdir().unwrap();
        let registry = SettingsRegistry::default();
        let slow_path = directory.path().join("slow/settings.toml");
        let fast_path = directory.path().join("fast/settings.toml");
        let (entered_tx, entered_rx) = mpsc::channel();
        let (release_tx, release_rx) = mpsc::channel();
        thread::scope(|scope| {
            let registry = &registry;
            let worker = scope.spawn(move || {
                registry.resolve(slow_path.clone(), || {
                    entered_tx.send(()).unwrap();
                    release_rx.recv().unwrap();
                    SettingsService::new(
                        SettingsStore::open(slow_path),
                        next_store_id()?,
                        &WatchBackend,
                    )
                    .map(Arc::new)
                })
            });
            entered_rx.recv().unwrap();
            let fast = registry.for_path(fast_path).unwrap();
            fast.apply(Command::SetLocale("zh-Hans".into())).unwrap();
            release_tx.send(()).unwrap();
            let slow = worker.join().unwrap().unwrap();
            assert_eq!(slow.snapshot().unwrap().locale_preference, "system");
        });
    }
}
