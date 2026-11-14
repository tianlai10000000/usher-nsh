//! Appearance shortcut intents; the existing settings transaction owns actual color-mode changes.
use super::super::{Command as ApplicationCommand, Error, catalog::ModuleConfig};
/// Toggles the application's effective light/dark appearance, never the operating system preference.
#[derive(Clone, Debug, PartialEq, Eq, serde::Serialize)]
#[serde(rename_all = "camelCase")]
pub enum Command {
    ToggleScheme,
}
pub(super) fn load() -> Result<ModuleConfig, Error> {
    serde_json::from_str(include_str!("../../../ui/config/shortcuts/appearance.json"))
        .map_err(|e| Error::InvalidConfig(e.to_string()))
}
pub(super) fn command(id: &str) -> Result<ApplicationCommand, Error> {
    match id {
        "toggleScheme" => Ok(ApplicationCommand::Appearance(Command::ToggleScheme)),
        _ => Err(Error::UnknownCommand(id.into())),
    }
}
