//! Injects a global window declaration and an independent native registration oracle for acceptance.
use nsh_core::{element::System, prelude::*};
use std::sync::Arc;
#[cfg(target_os = "macos")]
pub use usher_nsh::__nsh_native_components_macos;
use usher_nsh::shortcuts::{
    self,
    catalog::{ModuleConfig, ModuleIndex},
    global_bridge::{GlobalShortcutAttachment, attach_with_loader},
    loader::ModuleLoader,
    modules::ModuleFactory,
};
#[cfg(target_os = "macos")]
mod oracle;
pub fn __nsh_native_module() -> Registry {
    let mut registry = usher_nsh::__nsh_native_module();
    registry.module::<GlobalFixture>();
    registry
}
fn config() -> serde_json::Value {
    serde_json::from_str(include_str!("../../config.json")).expect("fixture config")
}
fn load() -> Result<ModuleConfig, shortcuts::Error> {
    serde_json::from_value(config()["payload"].clone())
        .map_err(|e| shortcuts::Error::InvalidConfig(e.to_string()))
}
fn command(id: &str) -> Result<shortcuts::Command, shortcuts::Error> {
    match id {
        "focusMain" => Ok(shortcuts::Command::Window(
            shortcuts::modules::window::Command::FocusMain,
        )),
        "openSettings" => Ok(shortcuts::Command::Window(
            shortcuts::modules::window::Command::OpenSettings,
        )),
        "toggleScheme" => Ok(shortcuts::Command::Appearance(
            shortcuts::modules::appearance::Command::ToggleScheme,
        )),
        _ => Err(shortcuts::Error::UnknownCommand(id.into())),
    }
}
/// Test-only catalog and OS allocation observations.
#[module]
pub struct GlobalFixture;
#[module]
impl GlobalFixture {
    /// Uses exactly the production process coordinator and bridge with an isolated catalog.
    pub fn attach(sys: System) -> Result<GlobalShortcutAttachment, String> {
        let production = ModuleIndex::application().map_err(|_| "fixture_index")?;
        let mut descriptors: Vec<_> = production
            .descriptors()
            .filter(|d| d.kind != shortcuts::catalog::ModuleKind::Global)
            .map(|d| serde_json::to_value(d).expect("descriptor"))
            .collect();
        descriptors.push(config()["module"].clone());
        let mut factories: Vec<_> = shortcuts::modules::factories()
            .into_iter()
            .filter(|factory| factory.id == "navigation")
            .collect();
        factories.push(ModuleFactory {
            id: "fixture-window",
            load,
            command,
        });
        let index = ModuleIndex::parse(
            &serde_json::json!({"schemaVersion":1,"modules":descriptors}).to_string(),
            factories,
        )
        .map_err(|_| "fixture_catalog")?;
        attach_with_loader(sys, Arc::new(ModuleLoader::new(index)))
    }
    /// Independently attempts to reserve the configured test key; never accesses production handles.
    pub fn reserve() -> bool {
        #[cfg(target_os = "macos")]
        {
            oracle::reserve(false)
        }
        #[cfg(not(target_os = "macos"))]
        {
            false
        }
    }
    /// Checks OS exclusivity, releasing any oracle-owned handle before returning.
    pub fn available() -> bool {
        #[cfg(target_os = "macos")]
        {
            oracle::reserve(true)
        }
        #[cfg(not(target_os = "macos"))]
        {
            false
        }
    }
    /// Hides this isolated host and asks the permissioned helper to send a configured system key.
    pub fn background_press(key: String) -> Result<String, String> {
        #[cfg(target_os = "macos")]
        {
            oracle::background_press(&key)
        }
        #[cfg(not(target_os = "macos"))]
        {
            let _ = key;
            Err("requires_macos".into())
        }
    }
    /// Reports actual AppKit activation, independent of the shortcut's health projection.
    pub fn active() -> bool {
        #[cfg(target_os = "macos")]
        {
            oracle::active()
        }
        #[cfg(not(target_os = "macos"))]
        {
            false
        }
    }
    /// Releases only the oracle's blocker, not the service registration.
    pub fn release() {
        #[cfg(target_os = "macos")]
        oracle::release();
    }
}
