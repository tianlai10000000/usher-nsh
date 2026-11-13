//! Lossless, single-decoding internal locations and named navigation targets.
use crate::Error;
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case", deny_unknown_fields)]
pub enum Target {
    Path {
        location: String,
    },
    Named {
        id: String,
        #[serde(default)]
        params: BTreeMap<String, String>,
        #[serde(default)]
        query: BTreeMap<String, Vec<String>>,
    },
}
impl Target {
    pub fn path(location: impl Into<String>) -> Self {
        Self::Path {
            location: location.into(),
        }
    }
}
/// A fully validated location. None for id represents the configured NotFound page.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Location {
    pub id: Option<String>,
    pub page: String,
    pub path: String,
    pub params: BTreeMap<String, String>,
    pub query: BTreeMap<String, Vec<String>>,
    pub ancestors: Vec<String>,
}
impl Location {
    /// Canonical internal URL; query keys are sorted, repeated values retain order.
    pub fn url(&self) -> String {
        let query: Vec<_> = self
            .query
            .iter()
            .flat_map(|(key, values)| {
                values
                    .iter()
                    .map(move |value| format!("{}={}", encode(key), encode(value)))
            })
            .collect();
        if query.is_empty() {
            self.path.clone()
        } else {
            format!("{}?{}", self.path, query.join("&"))
        }
    }
}
pub(crate) fn encode(value: &str) -> String {
    let mut encoded = String::new();
    for byte in value.bytes() {
        if byte.is_ascii_alphanumeric() || matches!(byte, b'-' | b'.' | b'_' | b'~') {
            encoded.push(char::from(byte));
        } else {
            use std::fmt::Write;
            write!(encoded, "%{byte:02X}").expect("writing to String is infallible");
        }
    }
    encoded
}
pub(crate) fn decode(value: &str) -> Result<String, Error> {
    let mut bytes = Vec::new();
    let mut input = value.bytes();
    while let Some(byte) = input.next() {
        if byte == b'%' {
            let high = input
                .next()
                .and_then(|b| char::from(b).to_digit(16))
                .ok_or(Error::InvalidEncoding)?;
            let low = input
                .next()
                .and_then(|b| char::from(b).to_digit(16))
                .ok_or(Error::InvalidEncoding)?;
            bytes.push((high * 16 + low) as u8);
        } else {
            bytes.push(byte);
        }
    }
    let decoded = String::from_utf8(bytes).map_err(|_| Error::InvalidEncoding)?;
    if decoded.chars().any(char::is_control) {
        return Err(Error::InvalidEncoding);
    }
    Ok(decoded)
}
pub(crate) type Query = BTreeMap<String, Vec<String>>;
pub(crate) fn parse(value: &str) -> Result<(String, Query), Error> {
    if !value.starts_with('/') || value.starts_with("//") || value.contains(['#', '\\']) {
        return Err(Error::InvalidTarget);
    }
    let (path, query) = value.split_once('?').unwrap_or((value, ""));
    let mut normalized = Vec::new();
    for segment in path[1..].split('/') {
        let decoded = decode(segment)?;
        // Delimiters must be literal, so an encoded slash cannot alter matching.
        if decoded.contains(['/', '\\']) || matches!(decoded.as_str(), "." | "..") {
            return Err(Error::InvalidTarget);
        }
        normalized.push(encode(&decoded));
    }
    let path = format!("/{}", normalized.join("/"));
    let mut values: Query = BTreeMap::new();
    if !query.is_empty() {
        for pair in query.split('&') {
            let (key, value) = pair.split_once('=').unwrap_or((pair, ""));
            let key = decode(key)?;
            if key.is_empty() {
                return Err(Error::InvalidQuery);
            }
            values.entry(key).or_default().push(decode(value)?);
        }
    }
    Ok((path, values))
}
