//! Navigation shortcut declarations and typed mappings to existing router actions.
use super::super::{Command as ApplicationCommand, Error, catalog::ModuleConfig};

/// Stable module identity shared with the checked module index.
pub const ID: &str = "navigation";
/// Navigation intent; execution and history availability remain owned by the router.
#[derive(Clone, Debug, PartialEq, Eq, serde::Serialize)]
#[serde(rename_all = "camelCase")]
pub enum Command {
    Back,
    Forward,
}

pub(super) fn load() -> Result<ModuleConfig, Error> {
    serde_json::from_str(include_str!("../../../ui/config/shortcuts/navigation.json"))
        .map_err(|e| Error::InvalidConfig(e.to_string()))
}
pub(super) fn command(id: &str) -> Result<ApplicationCommand, Error> {
    match id {
        "back" => Ok(ApplicationCommand::Navigation(Command::Back)),
        "forward" => Ok(ApplicationCommand::Navigation(Command::Forward)),
        _ => Err(Error::UnknownCommand(id.to_owned())),
    }
}
