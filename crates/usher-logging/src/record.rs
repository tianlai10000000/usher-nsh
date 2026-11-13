//! Converts owned fields and sanitizes records before passing them to any backend.
use crate::redaction::{redact, redact_text};
use crate::{FieldValue, Fields, Level, LogKind};
use chrono::{DateTime, Local, SecondsFormat};
use serde_json::{Map, Value, json};
use std::time::SystemTime;

/// A sanitized record supplied to an output backend. It contains no backend-specific types.
#[derive(Clone, Debug)]
pub struct Record {
    pub timestamp: SystemTime,
    pub level: Level,
    pub kind: LogKind,
    pub module: String,
    pub event: String,
    pub fields: Fields,
}

impl Record {
    pub(crate) fn new(
        at: DateTime<Local>,
        level: Level,
        kind: LogKind,
        module: &str,
        event: &str,
        mut fields: Map<String, Value>,
    ) -> Self {
        redact(&mut fields);
        Self {
            timestamp: at.into(),
            level,
            kind,
            module: redact_text(module),
            event: redact_text(event),
            fields: from_map(fields),
        }
    }
    pub(crate) fn json(&self) -> String {
        let at: DateTime<Local> = self.timestamp.into();
        let mut record = json!({ "ts": at.to_rfc3339_opts(SecondsFormat::Millis, false), "level": self.level.as_str(), "module": self.module, "event": self.event });
        if self.kind != LogKind::Application {
            record["category"] = self.kind.as_str().into();
        }
        if !self.fields.0.is_empty() {
            record["fields"] = Value::Object(to_map(self.fields.clone()));
        }
        record.to_string()
    }
}

pub(crate) fn to_map(fields: Fields) -> Map<String, Value> {
    fields
        .0
        .into_iter()
        .map(|(key, value)| (key, to_json(value)))
        .collect()
}
fn to_json(value: FieldValue) -> Value {
    match value {
        FieldValue::Null => Value::Null,
        FieldValue::Bool(v) => v.into(),
        FieldValue::Integer(v) => v.into(),
        FieldValue::Unsigned(v) => v.into(),
        FieldValue::Float(v) => v.into(),
        FieldValue::Text(v) => v.into(),
        FieldValue::Array(v) => Value::Array(v.into_iter().map(to_json).collect()),
        FieldValue::Object(v) => Value::Object(to_map(v)),
    }
}
fn from_map(fields: Map<String, Value>) -> Fields {
    Fields(
        fields
            .into_iter()
            .map(|(key, value)| (key, from_json(value)))
            .collect(),
    )
}
fn from_json(value: Value) -> FieldValue {
    match value {
        Value::Null => FieldValue::Null,
        Value::Bool(v) => FieldValue::Bool(v),
        Value::String(v) => FieldValue::Text(v),
        Value::Array(v) => FieldValue::Array(v.into_iter().map(from_json).collect()),
        Value::Object(v) => FieldValue::Object(from_map(v)),
        Value::Number(v) => {
            if let Some(n) = v.as_u64() {
                FieldValue::Unsigned(n)
            } else if let Some(n) = v.as_i64() {
                FieldValue::Integer(n)
            } else {
                FieldValue::Float(v.as_f64().unwrap_or_default())
            }
        }
    }
}

#[cfg(test)]
pub(crate) fn format_record(
    at: DateTime<Local>,
    level: Level,
    module: &str,
    event: &str,
    fields: Option<Map<String, Value>>,
) -> String {
    Record::new(
        at,
        level,
        LogKind::Application,
        module,
        event,
        fields.unwrap_or_default(),
    )
    .json()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::test_support::at;
    #[test]
    fn records_carry_timezone_level_module_and_event() {
        let line = format_record(
            at(2026, 11, 10, 9, 30),
            Level::Warn,
            "settings",
            "save_failed",
            None,
        );
        let value: Value = serde_json::from_str(&line).expect("valid json");
        assert_eq!(value["level"], "WARN");
        assert_eq!(value["module"], "settings");
        assert_eq!(value["event"], "save_failed");
        let stamp = value["ts"].as_str().expect("timestamp");
        assert!(DateTime::parse_from_rfc3339(stamp).is_ok(), "{stamp}");
        assert!(value.get("fields").is_none());
    }
}
