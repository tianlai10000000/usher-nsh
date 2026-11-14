//! Explicit business module factories, evaluated only when their descriptor is requested.
pub mod appearance;
pub mod navigation;
pub mod window;
use super::{Command, Error, catalog::ModuleConfig};

/// Module code entry points; construction does not load declarations or create handlers.
#[derive(Clone, Copy)]
pub struct ModuleFactory {
    pub id: &'static str,
    pub load: fn() -> Result<ModuleConfig, Error>,
    pub command: fn(&str) -> Result<Command, Error>,
}
/// Returns the known factories without executing them.
pub fn factories() -> Vec<ModuleFactory> {
    vec![
        ModuleFactory {
            id: "window",
            load: window::load,
            command: window::command,
        },
        ModuleFactory {
            id: "appearance",
            load: appearance::load,
            command: appearance::command,
        },
        ModuleFactory {
            id: navigation::ID,
            load: navigation::load,
            command: navigation::command,
        },
    ]
}
