//! Exposes the application-owned settings store to NSH surfaces.

use std::{cell::OnceCell, path::PathBuf, sync::Arc};

use nsh_core::{
    element::{CallCx, Callback, Local, Stop, System},
    prelude::*,
};

use crate::state::settings::{
    ColorMode, Command, ComponentFamily, CustomTheme, DiagnosticLevel, DiagnosticOutput, Palette,
    Snapshot,
};
use std::collections::BTreeMap;

use crate::state::appearance::theme::{StyleCssVars, StyleTheme, ThemeFileIssue};

use crate::state::settings::{SETTINGS, SettingsError, SettingsService};

#[derive(Default)]
struct SessionSettings(OnceCell<Result<Arc<SettingsService>, SettingsError>>);

impl Local for SessionSettings {
    fn close(&self) {
        if let Some(Ok(service)) = self.0.get() {
            service.close();
        }
    }
}

/// The current appearance preference and its persistence status.
#[derive(JsObject)]
pub struct SettingsSnapshot {
    /// The user's preference: light, dark, or system.
    pub color_mode: String,
    /// Whether the shell displays its bottom status bar.
    pub show_status_bar: bool,
    /// Whether the sidebar docks into the shell on wide desktop windows.
    pub sidebar_docked: bool,
    /// The application-owned image reference, absent when the overlay is disabled.
    pub background_image: Option<BackgroundImageValue>,
    /// Image opacity as an integer percentage, independent of the window tint.
    pub background_image_opacity: u32,
    /// Transparency of the menu and content fills; the configured maximum removes their fill.
    pub content_background_transparency: u32,
    /// The selected component family, independent of the style palette.
    pub component_family: String,
    /// The stable ID of the selected theme.
    pub theme_id: String,
    /// An optional accent override that follows the user across themes.
    pub accent: Option<String>,
    /// The selected UI language, or system to follow the host.
    pub locale_preference: String,
    /// The diagnostic log level: OFF, ERROR, WARN, INFO, DEBUG or TRACE.
    pub diagnostic_level: String,
    pub diagnostic_console: bool,
    pub diagnostic_file: bool,
    pub diagnostic_structured: bool,
    /// Persisted automatic update preference; execution is owned by the update controller.
    pub update_auto_check: bool,
    /// A stable code when the log files cannot be written, else nothing.
    pub diagnostic_problem: Option<String>,
    /// Records dropped because the log queue was full since the app started.
    pub diagnostic_dropped: f64,
    /// The user's saved themes and their light and dark palettes.
    pub custom_themes: Vec<ThemeValue>,
    /// Bundled presets and valid file themes outside the in-app palette editor.
    pub style_themes: Vec<StyleThemeValue>,
    /// Readable file names and stable error codes for skipped theme files.
    pub theme_file_issues: Vec<ThemeFileIssueValue>,
    /// The in-memory revision within this application process.
    pub revision: u32,
    /// Whether the current preference is safely stored or uses a clean default.
    pub saved: bool,
    /// A stable status code when recovery or another save attempt is needed.
    pub problem: Option<String>,
}

/// Metadata only; image bytes use a separate binary bridge call.
#[derive(JsObject)]
pub struct BackgroundImageValue {
    pub id: String,
    pub name: String,
}

/// The semantic colors edited by the appearance page.
#[derive(JsObject)]
pub struct PaletteValue {
    /// The application canvas color.
    pub background: String,
    /// The color for panels and other raised surfaces.
    pub surface: String,
    /// The foreground color on the canvas and panels.
    pub text: String,
    /// The default accent for controls in this palette.
    pub accent: String,
}

/// A theme authored by the user, independent of the active color mode.
#[derive(JsObject)]
pub struct ThemeValue {
    /// A stable identifier independent of the display name.
    pub id: String,
    /// The name shown in the theme list.
    pub name: String,
    /// The palette applied when the app resolves to light mode.
    pub light: PaletteValue,
    /// The palette applied when the app resolves to dark mode.
    pub dark: PaletteValue,
}

/// Resolved design tokens of the two color schemes. Every theme carries the complete table, so
/// the interface reads tokens by name and never needs per-theme knowledge.
#[derive(JsObject)]
pub struct StyleCssVarsValue {
    /// Legacy authored overrides. Current clients render the resolved touch scheme tables.
    pub touch: BTreeMap<String, String>,
    pub touch_light: BTreeMap<String, String>,
    pub touch_dark: BTreeMap<String, String>,
    pub light: BTreeMap<String, String>,
    pub dark: BTreeMap<String, String>,
}

/// A style that can be selected independently of the component family.
#[derive(JsObject)]
pub struct StyleThemeValue {
    pub id: String,
    /// The theme's own name.
    pub name: String,
    /// Names by UI locale. A locale without an entry uses `name`.
    pub names: BTreeMap<String, String>,
    /// Either `solid` or `glass`.
    pub material: String,
    pub css_vars: StyleCssVarsValue,
}

/// A file-system or validation issue that can be localized by the UI.
#[derive(JsObject)]
pub struct ThemeFileIssueValue {
    pub file_name: String,
    pub code: String,
}

impl From<StyleCssVars> for StyleCssVarsValue {
    fn from(vars: StyleCssVars) -> Self {
        Self {
            touch: vars.touch.0,
            touch_light: vars.touch_light.0,
            touch_dark: vars.touch_dark.0,
            light: vars.light.0,
            dark: vars.dark.0,
        }
    }
}

impl From<StyleTheme> for StyleThemeValue {
    fn from(theme: StyleTheme) -> Self {
        Self {
            id: theme.id,
            name: theme.name,
            names: theme.names,
            material: theme.material.unwrap_or_default().as_str().to_owned(),
            css_vars: theme.css_vars.into(),
        }
    }
}

impl From<ThemeFileIssue> for ThemeFileIssueValue {
    fn from(issue: ThemeFileIssue) -> Self {
        Self {
            file_name: issue.file_name,
            code: issue.code.to_owned(),
        }
    }
}

impl From<Palette> for PaletteValue {
    fn from(value: Palette) -> Self {
        let Palette {
            background,
            surface,
            text,
            accent,
        } = value;
        Self {
            background,
            surface,
            text,
            accent,
        }
    }
}

impl From<PaletteValue> for Palette {
    fn from(value: PaletteValue) -> Self {
        let PaletteValue {
            background,
            surface,
            text,
            accent,
        } = value;
        Self {
            background,
            surface,
            text,
            accent,
        }
    }
}

impl From<CustomTheme> for ThemeValue {
    fn from(value: CustomTheme) -> Self {
        Self {
            id: value.id,
            name: value.name,
            light: value.light.into(),
            dark: value.dark.into(),
        }
    }
}

impl From<ThemeValue> for CustomTheme {
    fn from(value: ThemeValue) -> Self {
        Self {
            id: value.id,
            name: value.name,
            light: value.light.into(),
            dark: value.dark.into(),
        }
    }
}

impl From<Snapshot> for SettingsSnapshot {
    fn from(snapshot: Snapshot) -> Self {
        Self {
            color_mode: snapshot.color_mode.as_str().to_owned(),
            show_status_bar: snapshot.show_status_bar,
            sidebar_docked: snapshot.sidebar_docked,
            background_image: snapshot.background_image.map(|image| BackgroundImageValue {
                id: image.id,
                name: image.name,
            }),
            background_image_opacity: snapshot.background_image_opacity,
            content_background_transparency: snapshot.content_background_transparency,
            component_family: snapshot.component_family.as_str().to_owned(),
            theme_id: snapshot.theme_id,
            accent: snapshot.accent,
            locale_preference: snapshot.locale_preference,
            diagnostic_level: snapshot.diagnostic_level.as_str().to_owned(),
            diagnostic_console: snapshot.diagnostic_console,
            diagnostic_file: snapshot.diagnostic_file,
            diagnostic_structured: snapshot.diagnostic_structured,
            update_auto_check: snapshot.update_auto_check,
            // The log writer is process-wide, so its health is read where the snapshot is
            // handed out, not stored in the settings file.
            diagnostic_problem: crate::diagnostics::current()
                .and_then(|logger| logger.snapshot().health.code())
                .map(str::to_owned),
            diagnostic_dropped: crate::diagnostics::current()
                .map_or(0.0, |logger| logger.snapshot().dropped as f64),
            custom_themes: snapshot.custom_themes.into_iter().map(Into::into).collect(),
            style_themes: snapshot.style_themes.into_iter().map(Into::into).collect(),
            theme_file_issues: snapshot
                .theme_file_issues
                .into_iter()
                .map(Into::into)
                .collect(),
            revision: snapshot.revision,
            saved: snapshot.saved,
            problem: snapshot.problem.map(str::to_owned),
        }
    }
}

/// Reads and changes the settings shared by the application's pages.
#[module]
pub struct Settings;

#[module]
impl Settings {
    /// Reports whether the macOS host registered the AppKit theme controls.
    pub fn app_kit_theme_available() -> bool {
        #[cfg(usher_macos_appkit)]
        {
            crate::macos::theme_available()
        }
        #[cfg(not(usher_macos_appkit))]
        {
            false
        }
    }

    /// Reports whether the macOS host registered its native window material.
    pub fn window_material_available() -> bool {
        #[cfg(target_os = "macos")]
        {
            crate::macos::window_material_available()
        }
        #[cfg(not(target_os = "macos"))]
        {
            false
        }
    }

    /// Loads the settings on first use and returns the latest snapshot.
    pub async fn snapshot(sys: System) -> Result<SettingsSnapshot, String> {
        with_store(sys, |store| store.snapshot()).await
    }

    /// Sets the appearance preference and reports whether it was saved.
    pub async fn set_color_mode(mode: String, sys: System) -> Result<SettingsSnapshot, String> {
        let mode: ColorMode = mode.parse().map_err(|()| "invalid_color_mode".to_owned())?;
        dispatch(sys, Command::SetColorMode(mode)).await
    }

    /// Shows or hides the application's bottom status bar.
    pub async fn set_show_status_bar(
        visible: bool,
        sys: System,
    ) -> Result<SettingsSnapshot, String> {
        dispatch(sys, Command::SetShowStatusBar(visible)).await
    }

    /// Saves the desktop sidebar's docked preference.
    pub async fn set_sidebar_docked(docked: bool, sys: System) -> Result<SettingsSnapshot, String> {
        dispatch(sys, Command::SetSidebarDocked(docked)).await
    }

    /// Copies image bytes into application storage and selects them. No caller-supplied path
    /// is read or written; failure leaves the previous saved image available.
    pub async fn import_background_image(
        name: String,
        bytes: Vec<u8>,
        sys: System,
    ) -> Result<SettingsSnapshot, String> {
        with_store(sys, move |store| store.import_background_image(name, bytes)).await
    }

    /// Removes the image overlay while retaining its opacity preference.
    pub async fn clear_background_image(sys: System) -> Result<SettingsSnapshot, String> {
        dispatch(sys, Command::ClearBackgroundImage).await
    }

    /// Sets an integer opacity percentage in the configured range.
    pub async fn set_background_image_opacity(
        value: f64,
        sys: System,
    ) -> Result<SettingsSnapshot, String> {
        if !value.is_finite()
            || value.fract() != 0.0
            || value < f64::from(crate::state::appearance::background::CONFIG.min_opacity)
            || value > f64::from(crate::state::appearance::background::CONFIG.max_opacity)
        {
            return Err("background_opacity_invalid".to_owned());
        }
        dispatch(sys, Command::SetBackgroundImageOpacity(value as u32)).await
    }

    /// Sets an integer transparency percentage without fading content or controls.
    pub async fn set_content_background_transparency(
        value: f64,
        sys: System,
    ) -> Result<SettingsSnapshot, String> {
        let range = &crate::state::appearance::background::CONFIG.content_transparency;
        if !value.is_finite()
            || value.fract() != 0.0
            || value < f64::from(range.min)
            || value > f64::from(range.max)
        {
            return Err("content_background_transparency_invalid".to_owned());
        }
        dispatch(sys, Command::SetContentBackgroundTransparency(value as u32)).await
    }

    /// Loads only the currently selected image as a Uint8Array, off the UI thread on disk hosts.
    pub async fn load_background_image(id: String, sys: System) -> Result<Vec<u8>, String> {
        with_settings_store(sys, move |store| store.load_background_image(&id)).await
    }

    /// Selects the component family without modifying the style palette.
    pub async fn set_component_family(
        family: String,
        sys: System,
    ) -> Result<SettingsSnapshot, String> {
        let family: ComponentFamily = family
            .parse()
            .map_err(|()| "invalid_component_family".to_owned())?;
        dispatch(sys, Command::SetComponentFamily(family)).await
    }

    /// Selects a built-in theme or a saved custom theme.
    pub async fn set_theme(theme_id: String, sys: System) -> Result<SettingsSnapshot, String> {
        dispatch(sys, Command::SetTheme(theme_id)).await
    }

    /// Sets an accent override, or restores the current theme's accent.
    pub async fn set_accent(
        accent: Option<String>,
        sys: System,
    ) -> Result<SettingsSnapshot, String> {
        dispatch(sys, Command::SetAccent(accent)).await
    }

    /// Changes the preferred UI language.
    pub async fn set_locale(locale: String, sys: System) -> Result<SettingsSnapshot, String> {
        dispatch(sys, Command::SetLocale(locale)).await
    }

    /// Sets the diagnostic log level. It applies to the running logger at once.
    pub async fn set_diagnostic_level(
        level: String,
        sys: System,
    ) -> Result<SettingsSnapshot, String> {
        let level: DiagnosticLevel = level
            .parse()
            .map_err(|()| "invalid_diagnostic_level".to_owned())?;
        dispatch(sys, Command::SetDiagnosticLevel(level)).await
    }

    /// Changes one supported output without accepting arbitrary logger configuration or paths.
    pub async fn set_diagnostic_output(
        output: String,
        enabled: bool,
        sys: System,
    ) -> Result<SettingsSnapshot, String> {
        let output: DiagnosticOutput = output
            .parse()
            .map_err(|()| "invalid_diagnostic_output".to_owned())?;
        dispatch(sys, Command::SetDiagnosticOutput(output, enabled)).await
    }

    /// Reuses the process pipeline and restores preferences under the settings transaction.
    pub async fn start_diagnostics(sys: System) -> Result<SettingsSnapshot, String> {
        let logs = logs_directory(sys.data_dir.clone());
        with_store(sys, move |store| match logs {
            Some(root) => store.start_diagnostics(root),
            None => store.snapshot(),
        })
        .await
    }

    /// Records one event from the interface. The level, module and event are
    /// checked here; free-form fields are redacted before they are written.
    pub async fn log_event(
        level: String,
        module: String,
        event: String,
        sys: System,
    ) -> Result<(), String> {
        let _ = sys;
        let level: usher_logging::Level = level
            .parse()
            .map_err(|()| "invalid_diagnostic_level".to_owned())?;
        if level == usher_logging::Level::Off || !is_log_name(&module) || !is_log_name(&event) {
            return Err("invalid_log_event".to_owned());
        }
        if let Some(logger) = usher_logging::current() {
            logger.log(
                level,
                &format!("ui.{module}"),
                &event,
                usher_logging::Fields::new(),
            );
        }
        Ok(())
    }

    /// Exports the logs of an inclusive date range (`YYYY-MM-DD`) as a `.tar.gz`
    /// into the application's exports folder, then reveals it. Returns the file name.
    pub async fn export_diagnostics(
        from: String,
        to: String,
        sys: System,
    ) -> Result<String, String> {
        let logs =
            logs_directory(sys.data_dir.clone()).ok_or("diagnostics_directory_unavailable")?;
        let exports = logs
            .parent()
            .map(|parent| parent.join("exports"))
            .ok_or("diagnostics_directory_unavailable")?;
        parse_range(&from, &to)?;
        let destination_dir = exports.clone();
        let name = blocking(move || -> Result<String, &'static str> {
            let runtime =
                crate::diagnostics::current().ok_or("diagnostics_directory_unavailable")?;
            runtime
                .export(&usher_logging::runtime::ExportRequest {
                    destination_dir: &destination_dir,
                    from: &from,
                    to: &to,
                    app_version: env!("CARGO_PKG_VERSION"),
                    platform: std::env::consts::OS,
                    cancelled: &|| false,
                })
                .map(|exported| exported.name)
                .map_err(|error| error.code())
        })
        .await
        .map_err(|_| "diagnostics_export_failed")??;
        #[cfg(any(target_os = "macos", target_os = "linux", target_os = "windows"))]
        {
            let _ = open_path(exports).await;
        }
        Ok(name)
    }

    /// Validates, saves, and selects a custom theme in one store transaction.
    pub async fn save_custom_theme(
        theme: ThemeValue,
        sys: System,
    ) -> Result<SettingsSnapshot, String> {
        dispatch(sys, Command::SaveCustomTheme(Box::new(theme.into()))).await
    }

    /// Removes a custom theme and restores the default if it was selected.
    pub async fn remove_custom_theme(
        theme_id: String,
        sys: System,
    ) -> Result<SettingsSnapshot, String> {
        dispatch(sys, Command::RemoveCustomTheme(theme_id)).await
    }

    /// Refreshes the style catalog from the application's local theme directory.
    pub async fn rescan_themes(sys: System) -> Result<SettingsSnapshot, String> {
        with_store(sys, |store| store.rescan_themes()).await
    }

    /// Copies all tokens from a style into a new file and selects it.
    pub async fn clone_style_theme(
        id: String,
        name: String,
        sys: System,
    ) -> Result<SettingsSnapshot, String> {
        with_store(sys, move |store| store.clone_style_theme(id, name)).await
    }

    /// Opens the application's local theme directory in the desktop file manager.
    pub async fn open_theme_directory(sys: System) -> Result<(), String> {
        #[cfg(any(target_os = "macos", target_os = "linux", target_os = "windows"))]
        {
            let directory = settings_directory(sys.data_dir)
                .ok_or("style_theme_directory_unavailable")?
                .join("themes");
            let directory_to_create = directory.clone();
            blocking(move || {
                crate::state::appearance::theme::ensure_directory(&directory_to_create)
            })
            .await
            .map_err(|_| "style_theme_directory_unavailable")?
            .map_err(|_| "style_theme_directory_unavailable")?;
            open_path(directory)
                .await
                .map_err(|_| "style_theme_directory_open_failed".to_owned())
        }
        #[cfg(not(any(target_os = "macos", target_os = "linux", target_os = "windows")))]
        {
            let _ = sys;
            Err("style_theme_directory_unavailable".to_owned())
        }
    }
}

async fn dispatch(sys: System, command: Command) -> Result<SettingsSnapshot, String> {
    with_store(sys, move |store| store.apply(command)).await
}

async fn with_store(
    sys: System,
    operation: impl FnOnce(&Arc<SettingsService>) -> Result<Snapshot, SettingsError> + Send + 'static,
) -> Result<SettingsSnapshot, String> {
    with_settings_store(sys, operation).await.map(Into::into)
}

pub(crate) async fn with_settings_store<T: Send + 'static>(
    sys: System,
    operation: impl FnOnce(&Arc<SettingsService>) -> Result<T, SettingsError> + Send + 'static,
) -> Result<T, String> {
    // NSH supplies the application data path, including the sandbox on mobile.
    // The JavaScript caller cannot select a file or a directory to write to.
    let Some(directory) = settings_directory(sys.data_dir) else {
        // Headless surfaces may have no data path. Keep their state isolated
        // within the VM so they can still exercise the normal settings flow.
        let session = sys.locals.get::<SessionSettings>();
        let service = session
            .0
            .get_or_init(SettingsService::session)
            .as_ref()
            .map_err(|code| (*code).to_owned())?;
        return operation(service).map_err(String::from);
    };
    let path = directory.join("settings.toml");

    // Directory lookup releases the registry lock before initialization or I/O.
    // Each service serializes only its own commands, persistence and publication.
    blocking(move || {
        let service = SETTINGS.for_path(path)?;
        operation(&service)
    })
    .await
    .map_err(|_| "settings_store_unavailable")?
    .map_err(String::from)
}

/// Event and module names are short identifiers, so the interface cannot smuggle
/// free text (and with it user content) into the log.
fn is_log_name(name: &str) -> bool {
    !name.is_empty()
        && name.len() <= 64
        && name
            .bytes()
            .all(|byte| byte.is_ascii_alphanumeric() || matches!(byte, b'_' | b'.' | b'-'))
}

fn parse_range(
    from: &str,
    to: &str,
) -> Result<(chrono::NaiveDate, chrono::NaiveDate), &'static str> {
    let parse = |value: &str| chrono::NaiveDate::parse_from_str(value, "%Y-%m-%d");
    match (parse(from), parse(to)) {
        (Ok(from), Ok(to)) if from <= to => Ok((from, to)),
        _ => Err("diagnostics_export_invalid_range"),
    }
}

/// Logs sit beside the settings file, in their own directory, never inside `themes/`.
fn logs_directory(path: Option<PathBuf>) -> Option<PathBuf> {
    Some(settings_directory(path)?.join("logs"))
}

fn settings_directory(path: Option<PathBuf>) -> Option<PathBuf> {
    let mut directory = path.filter(|path| path.is_absolute())?;
    if cfg!(debug_assertions) {
        directory.push("development");
    }
    Some(directory)
}

/// Complete settings state with lossless process identity and publication ordering.
#[derive(JsObject)]
pub struct SettingsEnvelope {
    pub store_id: String,
    pub sequence: String,
    pub value: SettingsSnapshot,
}

impl From<usher_state::Snapshot<Snapshot>> for SettingsEnvelope {
    fn from(snapshot: usher_state::Snapshot<Snapshot>) -> Self {
        Self {
            store_id: snapshot.store_id().value().to_string(),
            sequence: snapshot.sequence().to_string(),
            value: snapshot.state().clone().into(),
        }
    }
}

/// Exactly one of snapshot and end is present. End is closed, cancelled or error.
#[derive(JsObject)]
pub struct SettingsStateEvent {
    pub snapshot: Option<SettingsEnvelope>,
    pub end: Option<String>,
}

impl From<crate::state_bridge::Event<Snapshot>> for SettingsStateEvent {
    fn from(event: crate::state_bridge::Event<Snapshot>) -> Self {
        use crate::state_bridge::Event;
        use usher_state::SubscriptionEnd;
        match event {
            Event::Snapshot(snapshot) => Self {
                snapshot: Some(snapshot.into()),
                end: None,
            },
            Event::End(end) => Self {
                snapshot: None,
                end: Some(
                    match end {
                        SubscriptionEnd::Closed => "closed",
                        SubscriptionEnd::Cancelled => "cancelled",
                        SubscriptionEnd::BackendUnavailable => "error",
                    }
                    .into(),
                ),
            },
        }
    }
}

/// Versioned settings reads and engine-owned subscriptions; legacy commands stay compatible.
#[module]
pub struct SettingsState;

#[module]
impl SettingsState {
    /// Reads authoritative state for explicit refreshes and command reconciliation.
    pub async fn snapshot(sys: System) -> Result<SettingsEnvelope, String> {
        with_settings_store(sys, |service| service.state_snapshot())
            .await
            .map(Into::into)
    }

    /// Registers atomically and delivers the initial state followed by latest changes.
    /// The returned stop function is idempotent and also runs on owner/HMR/VM cleanup.
    pub fn subscribe(listener: Callback<SettingsStateEvent>, sys: System, cx: CallCx<'_>) -> Stop {
        let binding = crate::state_bridge::Binding::default();
        let stop = binding.clone();
        cx.start(async move {
            binding
                .run(crate::state_bridge::forward(
                    async move { with_settings_store(sys, |service| service.subscribe()).await },
                    move |event| listener.call_async(event.into()),
                ))
                .await;
        });
        Stop::new(move || stop.close())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use usher_test_support::task::ready;

    #[test]
    fn vm_local_cleanup_closes_session_even_while_other_handles_are_retained() {
        let local = SessionSettings::default();
        let service = local
            .0
            .get_or_init(SettingsService::session)
            .as_ref()
            .unwrap()
            .clone();
        let mut subscription = service.subscribe().unwrap();
        local.close();
        local.close();
        assert_eq!(
            ready(subscription.next()).unwrap_err(),
            usher_state::SubscriptionEnd::Closed
        );
        assert!(service.subscribe().is_err());
        assert!(service.state_snapshot().is_err());
        assert!(
            service
                .apply(Command::SetColorMode(ColorMode::Dark))
                .is_err()
        );
    }
}
