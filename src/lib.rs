//! Registers the application's shared Rust modules and native platform components.

use nsh_core::prelude::*;

mod diagnostic_bridge;
mod diagnostics;
mod router_bridge;
mod settings_bridge;
pub mod state;
mod state_bridge;
mod update_bridge;

#[cfg(target_os = "macos")]
#[path = "../platforms/macos/mod.rs"]
mod macos;

#[cfg(target_os = "macos")]
#[native_components(macos)]
fn macos_components(components: &nsh_platform_macos::Components) {
    macos::register(components);
}

#[native_module("usher/nsh")]
fn native(registry: &mut Registry) {
    registry
        .module::<Greeting>()
        .module::<settings_bridge::Settings>()
        .module::<settings_bridge::SettingsState>()
        .module::<router_bridge::WindowState>()
        .module::<diagnostic_bridge::DiagnosticState>()
        .module::<update_bridge::UpdateState>();
}

/// Greeting text shared with the TSX interface.
#[module]
pub struct Greeting;

#[module]
impl Greeting {
    /// Returns the message displayed when the app opens.
    pub fn message() -> String {
        "Hello, World!".to_owned()
    }
}

mod persistence;
