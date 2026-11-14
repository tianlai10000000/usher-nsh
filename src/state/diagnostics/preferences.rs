//! Persisted diagnostic output preferences and level conversion.
use serde::{Deserialize, Serialize};
use std::str::FromStr;
/// `[diagnostics]`: omitted from the file while it holds the default.
#[derive(Deserialize, Serialize)]
#[serde(default, rename_all = "camelCase")]
pub(crate) struct DiagnosticsSection {
    pub(crate) level: DiagnosticLevel,
    pub(crate) console: bool,
    pub(crate) file: bool,
    pub(crate) structured: bool,
}
impl Default for DiagnosticsSection {
    fn default() -> Self {
        let outputs = usher_logging::Outputs::default();
        Self {
            level: DiagnosticLevel::default(),
            console: outputs.console,
            file: outputs.file,
            structured: outputs.structured,
        }
    }
}
/// Supported application outputs; absent collectors have no writable preference.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum DiagnosticOutput {
    Console,
    File,
    Structured,
}
impl FromStr for DiagnosticOutput {
    type Err = ();
    fn from_str(value: &str) -> Result<Self, Self::Err> {
        match value {
            "console" => Ok(Self::Console),
            "file" => Ok(Self::File),
            "structured" => Ok(Self::Structured),
            _ => Err(()),
        }
    }
}
impl DiagnosticsSection {
    pub(crate) fn is_default(&self) -> bool {
        self.level == DiagnosticLevel::default()
            && self.console == Self::default().console
            && self.file == Self::default().file
            && self.structured == Self::default().structured
    }
}

/// Stored as the level's name so the file stays readable.
#[derive(Clone, Copy, Debug, Default, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "UPPERCASE")]
pub enum DiagnosticLevel {
    Off,
    Error,
    Warn,
    #[default]
    Info,
    Debug,
    Trace,
}

impl DiagnosticLevel {
    pub(crate) fn as_str(self) -> &'static str {
        self.to_level().as_str()
    }

    pub(crate) fn to_level(self) -> usher_logging::Level {
        use usher_logging::Level;
        match self {
            Self::Off => Level::Off,
            Self::Error => Level::Error,
            Self::Warn => Level::Warn,
            Self::Info => Level::Info,
            Self::Debug => Level::Debug,
            Self::Trace => Level::Trace,
        }
    }
}

impl FromStr for DiagnosticLevel {
    type Err = ();

    fn from_str(value: &str) -> Result<Self, Self::Err> {
        match value.to_ascii_uppercase().as_str() {
            "OFF" => Ok(Self::Off),
            "ERROR" => Ok(Self::Error),
            "WARN" => Ok(Self::Warn),
            "INFO" => Ok(Self::Info),
            "DEBUG" => Ok(Self::Debug),
            "TRACE" => Ok(Self::Trace),
            _ => Err(()),
        }
    }
}
