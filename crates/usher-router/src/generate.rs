//! Deterministic TypeScript contracts generated from the validated catalog.
use crate::{Router, ValueKind};
fn quoted(value: &str) -> String {
    serde_json::to_string(value).expect("strings serialize infallibly")
}
fn value_type(kind: &ValueKind) -> String {
    match kind {
        ValueKind::String => "string".into(),
        ValueKind::Integer => "`${bigint}`".into(),
        ValueKind::Boolean => "\"true\" | \"false\"".into(),
        ValueKind::Enum(values) => values
            .iter()
            .map(|v| quoted(v))
            .collect::<Vec<_>>()
            .join(" | "),
    }
}
impl Router {
    /// Emits only types: routes.json remains the single editable business catalog.
    /// Integer parameters are canonical decimal strings, validated at runtime as i64.
    pub fn typescript(&self) -> String {
        let mut output = String::from(
            "/** Generated route contracts. Edit the catalog, then regenerate this file. */\n",
        );
        let catalog = self.catalog();
        output.push_str(&format!(
            "export type RouteId = {};\n",
            catalog
                .routes
                .iter()
                .map(|r| quoted(&r.id))
                .collect::<Vec<_>>()
                .join(" | ")
        ));
        output.push_str(&format!(
            "export type PageKey = {};\n",
            catalog
                .pages
                .iter()
                .map(|p| quoted(p))
                .collect::<Vec<_>>()
                .join(" | ")
        ));
        output.push_str("export interface RouteParams {\n");
        for route in &catalog.routes {
            let params = route
                .params
                .iter()
                .map(|(key, p)| format!("{}: {}", quoted(key), value_type(&p.kind)))
                .collect::<Vec<_>>()
                .join("; ");
            let params = if params.is_empty() {
                "Record<string, never>".into()
            } else {
                format!("{{ {params} }}")
            };
            output.push_str(&format!("  {}: {params};\n", quoted(&route.id)));
        }
        output.push_str("}\nexport interface RouteQuery {\n");
        for route in &catalog.routes {
            output.push_str(&format!("  {}: {{", quoted(&route.id)));
            for (key, p) in &route.query {
                output.push_str(&format!(
                    " {}{}: readonly ({})[];",
                    quoted(key),
                    if p.required && p.defaults.is_empty() {
                        ""
                    } else {
                        "?"
                    },
                    value_type(&p.kind)
                ));
            }
            if route.unknown_query == crate::UnknownQuery::Preserve {
                output.push_str(" [key: string]: readonly string[] | undefined;");
            } else if route.query.is_empty() {
                output.push_str(" [key: string]: never;");
            }
            output.push_str(" };\n");
        }
        output.push_str("}\n");
        output
    }
}
