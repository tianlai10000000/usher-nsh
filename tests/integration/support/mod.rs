//! Isolated settings repositories and application-specific fixture inputs.

use std::{path::PathBuf, sync::Arc};
use usher_nsh::state::settings::{SettingsRegistry, SettingsService};

pub const LEGACY: &str = include_str!("../fixtures/settings-v1.toml");
pub const FUTURE: &str = include_str!("../fixtures/settings-future.toml");
pub const THEME: &str = include_str!("../fixtures/local-theme.json");

pub struct Repository {
    pub root: tempfile::TempDir,
    pub path: PathBuf,
    pub registry: SettingsRegistry,
}

impl Repository {
    pub fn new() -> Self {
        let root = tempfile::tempdir().unwrap();
        let path = root.path().join("profile/settings.toml");
        Self {
            root,
            path,
            registry: SettingsRegistry::default(),
        }
    }

    pub fn service(&self) -> Arc<SettingsService> {
        self.registry.for_path(self.path.clone()).unwrap()
    }

    pub fn seed(&self, text: &str) {
        std::fs::create_dir_all(self.path.parent().unwrap()).unwrap();
        std::fs::write(&self.path, text).unwrap();
    }
}
