//! Appearance business state and deterministic theme snapshots owned by the application.
pub(crate) mod theme;

/// Export resolved bundled themes for startup rendering without a second JS resolver.
/// The same registry and token rules serve live settings and this generated artifact.
pub fn bundled_snapshots() -> serde_json::Value {
    let themes = theme::StyleThemeScan::load(None).resolved().into_iter().map(|value| {
        serde_json::json!({
            "id": value.id, "name": value.name, "names": value.names,
            "material": value.material.unwrap_or_default().as_str(),
            "cssVars": {"touch": value.css_vars.touch.0, "touchLight": value.css_vars.touch_light.0, "touchDark": value.css_vars.touch_dark.0, "light": value.css_vars.light.0, "dark": value.css_vars.dark.0}
        })
    }).collect::<Vec<_>>();
    serde_json::json!({"defaultThemeId": theme::DEFAULT_THEME_ID, "themes": themes})
}

#[cfg(test)]
mod tests {
    #[test]
    fn startup_themes_match_the_runtime_registry() {
        let expected: serde_json::Value = serde_json::from_str(include_str!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/ui/state/appearance/bundled.generated.json"
        )))
        .unwrap();
        assert_eq!(
            super::bundled_snapshots(),
            expected,
            "run cargo run --bin theme-snapshots -- ui/state/appearance/bundled.generated.json"
        );
    }
}

pub(crate) mod background;

pub mod model;

pub(crate) mod color;
