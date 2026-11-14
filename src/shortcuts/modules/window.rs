//! Window business intents shared by global shortcut declarations and the desktop dispatcher.
/// A process-level window intent; it never captures a page or a focused Router.
#[derive(Clone, Debug, PartialEq, Eq, serde::Serialize)]
#[serde(rename_all = "camelCase")]
pub enum Command {
    FocusMain,
    OpenSettings,
}

/// Loads the product's process-level window declarations.
pub(super) fn load() -> Result<super::super::catalog::ModuleConfig, super::super::Error> {
    serde_json::from_str(include_str!("../../../ui/config/shortcuts/window.json"))
        .map_err(|e| super::super::Error::InvalidConfig(e.to_string()))
}
pub(super) fn command(id: &str) -> Result<super::super::Command, super::super::Error> {
    match id {
        "openSettings" => Ok(super::super::Command::Window(Command::OpenSettings)),
        _ => Err(super::super::Error::UnknownCommand(id.into())),
    }
}
