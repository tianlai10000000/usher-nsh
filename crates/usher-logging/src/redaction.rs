//! Masks credential fields and home paths before records reach any output.
use serde_json::{Map, Value};
const REDACTED: &str = "[redacted]";
const SENSITIVE_KEYS: [&str; 9] = [
    "password",
    "passwd",
    "secret",
    "token",
    "authorization",
    "apikey",
    "api_key",
    "credential",
    "bearer",
];

fn is_sensitive(key: &str) -> bool {
    let key = key.to_ascii_lowercase();
    SENSITIVE_KEYS.iter().any(|needle| key.contains(needle))
}

/// Replaces values of credential-like fields, recursively.
pub(crate) fn redact(fields: &mut Map<String, Value>) {
    for (key, value) in fields.iter_mut() {
        if is_sensitive(key) {
            *value = Value::String(REDACTED.to_owned());
        } else {
            redact_value(value);
        }
    }
}

fn redact_value(value: &mut Value) {
    match value {
        Value::Object(fields) => redact(fields),
        Value::Array(values) => values.iter_mut().for_each(redact_value),
        Value::String(text) => *text = redact_text(text),
        _ => {}
    }
}

/// Masks `Authorization: ...` style pairs and bearer tokens inside free text.
pub(crate) fn redact_text(text: &str) -> String {
    let mut out = String::with_capacity(text.len());
    let text = hide_home(text);
    let mut words = text.split_whitespace().peekable();
    let mut first = true;
    while let Some(word) = words.next() {
        if !first {
            out.push(' ');
        }
        first = false;
        let lower = word.to_ascii_lowercase();
        let labelled = lower
            .split(['=', ':'])
            .next()
            .is_some_and(|name| is_sensitive(name) && lower.contains(['=', ':']));
        if lower == "bearer" || lower == "basic" {
            out.push_str(word);
            if words.peek().is_some() {
                words.next();
                out.push(' ');
                out.push_str(REDACTED);
            }
        } else if labelled {
            let name = word.split(['=', ':']).next().unwrap_or_default();
            let separator = if word.contains('=') { '=' } else { ':' };
            out.push_str(name);
            out.push(separator);
            out.push_str(REDACTED);
            if word.ends_with(['=', ':'])
                && let Some(value) = words.next()
                && matches!(value.to_ascii_lowercase().as_str(), "bearer" | "basic")
            {
                words.next();
            }
        } else {
            out.push_str(word);
        }
    }
    out.chars()
        .take(crate::config::policy().max_message_chars)
        .collect()
}

/// Replaces the current user's home directory only at a directory boundary.
pub(crate) fn hide_home(text: &str) -> String {
    let Some(home) = std::env::var_os("HOME") else {
        return text.to_owned();
    };
    let home = home.to_string_lossy();
    if home.is_empty() {
        return text.to_owned();
    }
    replace_home(text, &home)
}

pub(crate) fn replace_home(text: &str, home: &str) -> String {
    let mut result = String::new();
    let mut cursor = 0;
    for (index, _) in text.match_indices(home) {
        let end = index + home.len();
        let boundary = text[end..]
            .chars()
            .next()
            .is_none_or(|c| c == '/' || c.is_whitespace() || matches!(c, '\"' | '\'' | ')' | ':'));
        result.push_str(&text[cursor..index]);
        result.push_str(if boundary { "~" } else { home });
        cursor = end;
    }
    result.push_str(&text[cursor..]);
    result
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;
    #[test]
    fn redaction_masks_credentials_in_fields_and_text() {
        let mut fields = Map::new();
        fields.insert("Authorization".into(), json!("Bearer abc"));
        fields.insert("requestBody".into(), json!("private"));
        fields.insert("nested".into(), json!({ "apiKey": "k", "ok": 1 }));
        fields.insert("count".into(), json!(3));
        redact(&mut fields);
        assert_eq!(fields["Authorization"], REDACTED);
        assert_eq!(fields["requestBody"], "private");
        assert_eq!(fields["nested"]["apiKey"], REDACTED);
        assert_eq!(fields["nested"]["ok"], 1);
        assert_eq!(fields["count"], 3);

        assert_eq!(
            redact_text("sent Bearer abc123 ok"),
            "sent Bearer [redacted] ok"
        );
        assert_eq!(redact_text("token=xyz done"), "token=[redacted] done");
        assert_eq!(redact_text("plain text"), "plain text");
    }

    #[test]
    fn nested_details_keep_context_but_remove_credentials() {
        let mut fields = serde_json::from_value::<Map<String, Value>>(json!({
            "detail": "Authorization: Bearer private-token\nfailed",
            "items": [{ "password": "hidden", "message": "bearer hidden" }],
            "body": "context", "email": "context", "cookie": "context"
        }))
        .unwrap();
        redact(&mut fields);
        let text = serde_json::to_string(&fields).unwrap();
        assert!(!text.contains("private-token") && !text.contains("hidden"));
        assert_eq!(fields["body"], "context");
        assert_eq!(
            replace_home("/Users/test/file /Users/tester", "/Users/test"),
            "~/file /Users/tester"
        );
    }
}
