//! Uses NSH's parser and reserved-key rules without maintaining another key grammar.
use crate::shortcuts::Error;
use nsh_core::{keymap::Keystroke, platform::Os};
use usher_shortcuts::CanonicalStroke;

/// Parses a local binding for an explicit target platform and rejects reserved keys.
/// Global registration acceptance must additionally be confirmed by the native backend.
pub fn normalize_local(text: &str, os: Os) -> Result<CanonicalStroke, Error> {
    let stroke = Keystroke::parse(text, os).map_err(|e| Error::InvalidKeystroke(e.to_string()))?;
    if stroke.reserved(os).is_some() {
        return Err(Error::ReservedKeystroke(text.to_owned()));
    }
    CanonicalStroke::new(stroke.to_string()).map_err(Error::from)
}

/// Validates against both NSH input grammars so shared canonical keys detect local/global conflicts.
pub fn normalize_global(text: &str, os: Os) -> Result<CanonicalStroke, Error> {
    nsh_core::desktop::Shortcut::parse(text, os == Os::MacOs)
        .map_err(|e| Error::InvalidKeystroke(e.to_string()))?;
    normalize_local(text, os)
}
