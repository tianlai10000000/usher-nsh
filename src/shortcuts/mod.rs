//! Application-owned shortcut catalogs and business modules; native installation is separate.
pub mod adapters;
pub mod bridge;
pub mod catalog;
pub mod global;
pub mod global_bridge;
pub mod loader;
pub mod modules;
pub mod service;

/// Application command mapping, never exported by the generic shortcut crate.
#[derive(Clone, Debug, PartialEq, Eq, serde::Serialize)]
#[serde(tag = "module", content = "action", rename_all = "camelCase")]
pub enum Command {
    Navigation(modules::navigation::Command),
    Window(modules::window::Command),
    Appearance(modules::appearance::Command),
}

/// Stable application catalog errors; parser diagnostics stay at this boundary.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Error {
    InvalidConfig(String),
    UnknownModule(String),
    DuplicateModule(String),
    VersionMismatch(String),
    UnknownCommand(String),
    InvalidKeystroke(String),
    ReservedKeystroke(String),
    Core(usher_shortcuts::Error),
}
impl From<usher_shortcuts::Error> for Error {
    fn from(value: usher_shortcuts::Error) -> Self {
        Self::Core(value)
    }
}
