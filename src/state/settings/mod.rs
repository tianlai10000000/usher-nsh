//! Assembles directory-owned settings services and isolated engine sessions.

use std::{
    collections::HashMap,
    path::PathBuf,
    sync::{Arc, LazyLock, Mutex, OnceLock},
};

use crate::state::identity::next_store_id;
use usher_state::backends::WatchBackend;

pub use crate::state::appearance::background::BackgroundImage;
pub use crate::state::appearance::theme::{
    Material, StyleCssVars, StyleTheme, ThemeFileIssue, ThemeSource, Tokens,
};
pub use crate::state::settings::model::{
    ColorMode, Command, ComponentFamily, CustomTheme, DiagnosticLevel, DiagnosticOutput, Palette,
    Snapshot,
};
pub use crate::state::settings::service::{Error as SettingsError, SettingsService};

use crate::state::settings::model::SettingsStore;

pub(crate) const UNAVAILABLE: &str = "settings_store_unavailable";
pub(crate) static SETTINGS: LazyLock<SettingsRegistry> = LazyLock::new(SettingsRegistry::default);

type ServiceCell = Arc<OnceLock<Result<Arc<SettingsService>, SettingsError>>>;

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
    pub fn for_path(&self, path: PathBuf) -> Result<Arc<SettingsService>, SettingsError> {
        self.resolve(path.clone(), || {
            let id = next_store_id().map_err(SettingsError::Identity)?;
            SettingsService::new(SettingsStore::open(path), id, &WatchBackend).map(Arc::new)
        })
    }

    fn resolve(
        &self,
        path: PathBuf,
        initialize: impl FnOnce() -> Result<Arc<SettingsService>, SettingsError>,
    ) -> Result<Arc<SettingsService>, SettingsError> {
        if !path.is_absolute() {
            return Err(SettingsError::InvalidPath);
        }
        let entry = self
            .services
            .lock()
            .map_err(|_| SettingsError::Poisoned)?
            .entry(path.clone())
            .or_default()
            .clone();
        entry.get_or_init(initialize).clone()
    }
}

impl SettingsService {
    /// Creates an isolated, memory-only engine session. Dropping its last owner
    /// closes subscriptions. This service is never inserted into the directory registry.
    pub fn session() -> Result<Arc<Self>, SettingsError> {
        Self::new(
            SettingsStore::session_only(),
            next_store_id().map_err(SettingsError::Identity)?,
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
                        next_store_id().map_err(SettingsError::Identity)?,
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

pub(crate) mod model;
pub(crate) mod service;
