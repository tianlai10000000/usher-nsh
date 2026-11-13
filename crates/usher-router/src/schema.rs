//! Serializable route catalog and portable parameter constraints.
use crate::Error;
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;

/// One catalog is installed atomically. Identity belongs to the UI package;
/// consumers must compare the full catalog when an identity is reused.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct Catalog {
    pub schema_version: u32,
    pub identity: String,
    pub initial: String,
    pub not_found_page: String,
    pub pages: Vec<String>,
    pub routes: Vec<Route>,
    pub redirect_limit: usize,
}
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct Route {
    pub id: String,
    pub path: String,
    pub page: String,
    pub parent: Option<String>,
    pub title_key: String,
    pub menu: Option<String>,
    #[serde(default)]
    pub params: BTreeMap<String, Parameter>,
    #[serde(default)]
    pub query: BTreeMap<String, Parameter>,
    pub unknown_query: UnknownQuery,
}
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct Parameter {
    pub kind: ValueKind,
    #[serde(default)]
    pub required: bool,
    #[serde(default)]
    pub multiple: bool,
    #[serde(default)]
    pub defaults: Vec<String>,
}
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "type", content = "values", rename_all = "snake_case")]
pub enum ValueKind {
    String,
    Integer,
    Boolean,
    Enum(Vec<String>),
}
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum UnknownQuery {
    Reject,
    Preserve,
}
impl Parameter {
    pub(crate) fn validate(&self, value: &str) -> Result<(), Error> {
        let valid = match &self.kind {
            ValueKind::String => true,
            ValueKind::Integer => value
                .parse::<i64>()
                .is_ok_and(|number| number.to_string() == value),
            ValueKind::Boolean => matches!(value, "true" | "false"),
            ValueKind::Enum(values) => values.iter().any(|allowed| allowed == value),
        };
        if valid {
            Ok(())
        } else {
            Err(Error::InvalidParameter)
        }
    }
    pub(crate) fn valid_schema(&self) -> bool {
        if let ValueKind::Enum(values) = &self.kind {
            let unique: std::collections::BTreeSet<_> = values.iter().collect();
            if values.is_empty() || unique.len() != values.len() {
                return false;
            }
        }
        if !self.multiple && self.defaults.len() > 1 {
            return false;
        }
        self.defaults
            .iter()
            .all(|value| self.validate(value).is_ok())
    }
}
