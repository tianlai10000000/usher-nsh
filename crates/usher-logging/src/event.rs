//! Backend-independent event values and output selection, with no application dependencies.
use std::collections::BTreeMap;

/// A diagnostic source, independent of severity and the selected output destination.
#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub enum LogKind {
    #[default]
    Application,
    Tracing,
    Terminal,
    Memory,
}

impl LogKind {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Application => "application",
            Self::Tracing => "tracing",
            Self::Terminal => "terminal_verbose",
            Self::Memory => "memory",
        }
    }
}

/// A destination identifier; file paths and backend implementations remain private.
#[derive(Clone, Copy, Debug, Eq, Ord, PartialEq, PartialOrd)]
pub enum Output {
    Console,
    File,
    Structured,
}

/// Runtime switches. Producers own terminal and memory sampling, not the logger.
#[derive(Clone, Copy, Debug, Eq, PartialEq, serde::Deserialize)]
pub struct Outputs {
    pub console: bool,
    pub file: bool,
    pub structured: bool,
    pub terminal: bool,
    pub memory: bool,
}

impl Default for Outputs {
    fn default() -> Self {
        crate::config::policy().outputs
    }
}

impl Outputs {
    pub(crate) fn destinations(self, kind: LogKind) -> Vec<Output> {
        let ordinary = match kind {
            LogKind::Application => true,
            LogKind::Tracing => {
                return if self.structured {
                    vec![Output::Structured]
                } else {
                    vec![]
                };
            }
            LogKind::Terminal => self.terminal,
            LogKind::Memory => self.memory,
        };
        let mut outputs = Vec::new();
        if ordinary && self.console {
            outputs.push(Output::Console);
        }
        if ordinary && self.file {
            outputs.push(Output::File);
        }
        outputs
    }
}

/// Structured values owned by this crate, rather than a JSON or backend library.
#[derive(Clone, Debug, PartialEq)]
pub enum FieldValue {
    Null,
    Bool(bool),
    Integer(i64),
    Unsigned(u64),
    Float(f64),
    Text(String),
    Array(Vec<FieldValue>),
    Object(Fields),
}

impl From<&str> for FieldValue {
    fn from(value: &str) -> Self {
        Self::Text(value.into())
    }
}
impl From<String> for FieldValue {
    fn from(value: String) -> Self {
        Self::Text(value)
    }
}
impl From<bool> for FieldValue {
    fn from(value: bool) -> Self {
        Self::Bool(value)
    }
}
impl From<u64> for FieldValue {
    fn from(value: u64) -> Self {
        Self::Unsigned(value)
    }
}
impl From<i64> for FieldValue {
    fn from(value: i64) -> Self {
        Self::Integer(value)
    }
}
impl<T: Into<FieldValue>> From<Option<T>> for FieldValue {
    fn from(value: Option<T>) -> Self {
        value.map(Into::into).unwrap_or(Self::Null)
    }
}

/// Named fields. Duplicate keys replace their previous value, like a map.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct Fields(pub(crate) BTreeMap<String, FieldValue>);

impl Fields {
    pub fn new() -> Self {
        Self::default()
    }
    pub fn with(mut self, name: impl Into<String>, value: impl Into<FieldValue>) -> Self {
        self.0.insert(name.into(), value.into());
        self
    }
    pub fn get(&self, name: &str) -> Option<&FieldValue> {
        self.0.get(name)
    }
    pub fn iter(&self) -> impl Iterator<Item = (&str, &FieldValue)> {
        self.0.iter().map(|(name, value)| (name.as_str(), value))
    }
    pub(crate) fn valid(&self) -> bool {
        self.0.values().all(FieldValue::valid)
    }
}

impl FieldValue {
    fn valid(&self) -> bool {
        match self {
            Self::Float(value) => value.is_finite(),
            Self::Array(values) => values.iter().all(Self::valid),
            Self::Object(fields) => fields.valid(),
            _ => true,
        }
    }
}

/// Submission outcome. Enqueued does not promise persistence or delivery to every output.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum Submit {
    Filtered,
    Enqueued,
    Dropped,
    Unavailable,
    InvalidFields,
}
