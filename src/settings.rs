//! Local appearance, custom theme, and language preferences.
//!
//! The store owns the in-memory selection and its persistence status. Its caller
//! serializes access and runs disk operations away from the UI thread.

use std::{
    collections::{BTreeMap, HashSet},
    fs,
    io::{self, Write},
    path::{Path, PathBuf},
    str::FromStr,
};

use serde::{Deserialize, Serialize};
use tempfile::Builder;

use crate::background::{self, BackgroundAssets, BackgroundImage};
use crate::style_themes::{self, DEFAULT_THEME_ID, StyleTheme, StyleThemeScan, ThemeFileIssue};

const SCHEMA_VERSION: u32 = 1;
/// The combined style-and-family ID of early builds. It now means the glass style with AppKit controls.
const LEGACY_APPKIT_THEME_ID: &str = "apple-appkit";
const LEGACY_APPKIT_STYLE_ID: &str = "apple-glass";
const SYSTEM_LOCALE: &str = "system";
const MAX_CUSTOM_THEMES: usize = 64;
const DIRECTORY_UNAVAILABLE: &str = "settings_directory_unavailable";
const READ_FAILED: &str = "settings_read_failed";
const INVALID_RECOVERED: &str = "settings_invalid_recovered";
const BACKUP_FAILED: &str = "settings_backup_failed";
const UNSUPPORTED_VERSION: &str = "settings_unsupported_version";
const WRITE_FAILED: &str = "settings_write_failed";

/// The user's selection, independent of the operating system's current appearance.
#[derive(Clone, Copy, Debug, Default, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "lowercase")]
pub enum ColorMode {
    Light,
    Dark,
    #[default]
    System,
}

impl ColorMode {
    pub(crate) fn as_str(self) -> &'static str {
        match self {
            Self::Light => "light",
            Self::Dark => "dark",
            Self::System => "system",
        }
    }
}

impl FromStr for ColorMode {
    type Err = ();

    fn from_str(value: &str) -> Result<Self, Self::Err> {
        match value {
            "light" => Ok(Self::Light),
            "dark" => Ok(Self::Dark),
            "system" => Ok(Self::System),
            _ => Err(()),
        }
    }
}

pub enum Command {
    SetColorMode(ColorMode),
    SetShowStatusBar(bool),
    SetSidebarDocked(bool),
    ClearBackgroundImage,
    SetBackgroundImageOpacity(u32),
    SetContentBackgroundTransparency(u32),
    SetComponentFamily(ComponentFamily),
    SetTheme(String),
    SetAccent(Option<String>),
    SetLocale(String),
    SetDiagnosticLevel(DiagnosticLevel),
    SetUpdateAutoCheck(bool),
    SaveCustomTheme(Box<CustomTheme>),
    RemoveCustomTheme(String),
}

#[derive(Clone, Copy, Debug, Default, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "lowercase")]
pub enum ComponentFamily {
    #[default]
    Nsh,
    AppKit,
}

impl ComponentFamily {
    pub(crate) fn as_str(self) -> &'static str {
        match self {
            Self::Nsh => "nsh",
            Self::AppKit => "appkit",
        }
    }

    fn is_default(&self) -> bool {
        *self == Self::default()
    }
}

impl FromStr for ComponentFamily {
    type Err = ();

    fn from_str(value: &str) -> Result<Self, Self::Err> {
        match value {
            "nsh" => Ok(Self::Nsh),
            "appkit" => Ok(Self::AppKit),
            _ => Err(()),
        }
    }
}

/// A revision identifies a set of preferences, so retrying its write does not advance it.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Snapshot {
    pub color_mode: ColorMode,
    pub show_status_bar: bool,
    pub sidebar_docked: bool,
    pub background_image: Option<BackgroundImage>,
    pub background_image_opacity: u32,
    pub content_background_transparency: u32,
    pub component_family: ComponentFamily,
    pub theme_id: String,
    pub accent: Option<String>,
    pub locale_preference: String,
    pub diagnostic_level: DiagnosticLevel,
    pub update_auto_check: bool,
    pub custom_themes: Vec<CustomTheme>,
    pub style_themes: Vec<StyleTheme>,
    pub theme_file_issues: Vec<ThemeFileIssue>,
    pub revision: u32,
    pub saved: bool,
    pub problem: Option<&'static str>,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct Palette {
    pub background: String,
    pub surface: String,
    pub text: String,
    pub accent: String,
}

impl Palette {
    fn validate(&self) -> Result<(), &'static str> {
        const INVALID_COLOR: &str = "invalid_theme_color";
        let background = parse_color(&self.background).ok_or(INVALID_COLOR)?;
        let surface = parse_color(&self.surface).ok_or(INVALID_COLOR)?;
        let text = parse_color(&self.text).ok_or(INVALID_COLOR)?;
        parse_color(&self.accent).ok_or(INVALID_COLOR)?;

        if contrast_ratio(background, text) < 4.5 || contrast_ratio(surface, text) < 4.5 {
            return Err("invalid_theme_contrast");
        }
        Ok(())
    }
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct CustomTheme {
    pub id: String,
    pub name: String,
    pub light: Palette,
    pub dark: Palette,
}

impl CustomTheme {
    pub(crate) fn validate(&self) -> Result<(), &'static str> {
        validate_custom_id(&self.id)?;
        if !style_themes::valid_theme_name(&self.name) {
            return Err("invalid_theme_name");
        }
        self.light.validate()?;
        self.dark.validate()
    }
}

#[derive(Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
struct Document {
    schema_version: u32,
    appearance: Appearance,
    #[serde(default, skip_serializing_if = "General::is_default")]
    general: General,
    #[serde(default, skip_serializing_if = "DiagnosticsSection::is_default")]
    diagnostics: DiagnosticsSection,
    #[serde(default, skip_serializing_if = "UpdatesSection::is_default")]
    updates: UpdatesSection,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    custom_themes: Vec<CustomTheme>,
}

impl Default for Document {
    fn default() -> Self {
        Self {
            schema_version: SCHEMA_VERSION,
            appearance: Appearance {
                color_mode: ColorMode::default(),
                show_status_bar: default_enabled(),
                sidebar_docked: default_enabled(),
                background_image: None,
                background_image_opacity: background::default_opacity(),
                content_background_transparency: background::default_content_transparency(),
                component_family: ComponentFamily::default(),
                theme_id: default_theme_id(),
                accent: None,
            },
            general: General::default(),
            diagnostics: DiagnosticsSection::default(),
            updates: UpdatesSection::default(),
            custom_themes: Vec::new(),
        }
    }
}

impl Document {
    fn validate(&self) -> Result<(), &'static str> {
        // Preserve a valid ID for a theme missing from the current build.
        if !is_theme_id(&self.appearance.theme_id) {
            return Err("invalid_theme_id");
        }
        validate_accent(self.appearance.accent.as_deref())?;
        background::validate_opacity(self.appearance.background_image_opacity)?;
        background::validate_content_transparency(self.appearance.content_background_transparency)?;
        if let Some(image) = &self.appearance.background_image {
            image.validate()?;
        }
        validate_locale(&self.general.locale_preference)?;
        if self.custom_themes.len() > MAX_CUSTOM_THEMES {
            return Err("too_many_custom_themes");
        }
        let mut ids = HashSet::new();
        for theme in &self.custom_themes {
            theme.validate()?;
            if !ids.insert(theme.id.as_str()) {
                return Err("duplicate_theme_id");
            }
        }
        Ok(())
    }
}

#[derive(Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
struct Appearance {
    color_mode: ColorMode,
    #[serde(default = "default_enabled", skip_serializing_if = "is_true")]
    show_status_bar: bool,
    #[serde(default = "default_enabled", skip_serializing_if = "is_true")]
    sidebar_docked: bool,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    background_image: Option<BackgroundImage>,
    #[serde(
        default = "background::default_opacity",
        skip_serializing_if = "background::is_default_opacity"
    )]
    background_image_opacity: u32,
    #[serde(
        default = "background::default_content_transparency",
        skip_serializing_if = "background::is_default_content_transparency"
    )]
    content_background_transparency: u32,
    #[serde(default, skip_serializing_if = "ComponentFamily::is_default")]
    component_family: ComponentFamily,
    #[serde(default = "default_theme_id", skip_serializing_if = "is_default_theme")]
    theme_id: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    accent: Option<String>,
}

#[derive(Deserialize, Serialize)]
#[serde(default, rename_all = "camelCase")]
struct General {
    locale_preference: String,
}

impl Default for General {
    fn default() -> Self {
        Self {
            locale_preference: SYSTEM_LOCALE.to_owned(),
        }
    }
}

impl General {
    fn is_default(&self) -> bool {
        self.locale_preference == SYSTEM_LOCALE
    }
}

/// `[updates]` is omitted while automatic checks retain the established enabled default.
#[derive(Deserialize, Serialize)]
#[serde(default)]
struct UpdatesSection {
    auto_check: bool,
}
impl Default for UpdatesSection {
    fn default() -> Self {
        Self {
            auto_check: default_enabled(),
        }
    }
}
impl UpdatesSection {
    fn is_default(&self) -> bool {
        self.auto_check == Self::default().auto_check
    }
}

/// `[diagnostics]`: omitted from the file while it holds the default.
#[derive(Default, Deserialize, Serialize)]
#[serde(default, rename_all = "camelCase")]
struct DiagnosticsSection {
    level: DiagnosticLevel,
}

impl DiagnosticsSection {
    fn is_default(&self) -> bool {
        self.level == DiagnosticLevel::default()
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

    pub(crate) fn to_level(self) -> crate::diagnostics::Level {
        use crate::diagnostics::Level;
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

fn default_theme_id() -> String {
    DEFAULT_THEME_ID.to_owned()
}

fn is_default_theme(id: &str) -> bool {
    id == DEFAULT_THEME_ID
}

fn default_enabled() -> bool {
    true
}

fn is_true(value: &bool) -> bool {
    *value
}

fn is_theme_id(id: &str) -> bool {
    id.len() <= 64
        && id.split('.').all(|part| {
            !part.is_empty()
                && part
                    .bytes()
                    .all(|byte| byte.is_ascii_lowercase() || byte.is_ascii_digit() || byte == b'-')
        })
}

fn validate_custom_id(id: &str) -> Result<(), &'static str> {
    if style_themes::is_style_id(id, false) {
        Ok(())
    } else {
        Err("invalid_theme_id")
    }
}

fn validate_locale(locale: &str) -> Result<(), &'static str> {
    if matches!(locale, SYSTEM_LOCALE | "en" | "zh-Hans") {
        Ok(())
    } else {
        Err("invalid_locale")
    }
}

fn validate_accent(accent: Option<&str>) -> Result<(), &'static str> {
    match accent {
        Some(value) if parse_color(value).is_none() => Err("invalid_accent"),
        _ => Ok(()),
    }
}

fn parse_color(value: &str) -> Option<u32> {
    let hex = value.strip_prefix('#')?;
    if hex.len() != 6 || !hex.bytes().all(|byte| byte.is_ascii_hexdigit()) {
        return None;
    }
    u32::from_str_radix(hex, 16).ok()
}

fn contrast_ratio(first: u32, second: u32) -> f64 {
    let first = relative_luminance(first);
    let second = relative_luminance(second);
    (first.max(second) + 0.05) / (first.min(second) + 0.05)
}

fn relative_luminance(rgb: u32) -> f64 {
    // Convert encoded sRGB channels to linear light before applying the WCAG weights.
    let channel = |shift| {
        let value = f64::from((rgb >> shift) & 0xffu32) / 255.0;
        if value <= 0.04045 {
            value / 12.92
        } else {
            ((value + 0.055) / 1.055).powf(2.4)
        }
    };
    0.2126 * channel(16) + 0.7152 * channel(8) + 0.0722 * channel(0)
}

fn update<T: PartialEq>(current: &mut T, next: T) -> bool {
    if *current == next {
        return false;
    }
    *current = next;
    true
}

pub(crate) struct SettingsStore {
    path: Option<PathBuf>,
    theme_directory: Option<PathBuf>,
    document: Document,
    themes: StyleThemeScan,
    background_assets: BackgroundAssets,
    pending_themes: BTreeMap<String, Option<CustomTheme>>,
    pending_clone: Option<PendingStyleClone>,
    revision: u32,
    saved: bool,
    problem: Option<&'static str>,
}

struct PendingStyleClone {
    source_id: String,
    name: String,
    cloned_id: String,
}

impl SettingsStore {
    pub(crate) fn open(path: PathBuf) -> Self {
        let mut store = Self::new(Some(path.clone()));

        let document = match fs::read_to_string(&path) {
            Ok(text) => parse_document(&text),
            Err(error) if error.kind() == io::ErrorKind::NotFound => {
                store.reload_themes(false);
                return store;
            }
            Err(error) if error.kind() == io::ErrorKind::InvalidData => Err(DocumentError::Invalid),
            Err(_) => {
                store.block_writes(READ_FAILED);
                store.reload_themes(false);
                return store;
            }
        };

        match document {
            Ok(document) => store.document = document,
            Err(DocumentError::UnsupportedVersion) => store.block_writes(UNSUPPORTED_VERSION),
            Err(DocumentError::Invalid) => {
                if preserve_invalid(&path).is_ok() {
                    store.problem = Some(INVALID_RECOVERED);
                } else {
                    store.block_writes(BACKUP_FAILED);
                }
            }
        }

        store.reload_themes(true);
        let appearance = &mut store.document.appearance;
        if appearance.theme_id == LEGACY_APPKIT_THEME_ID {
            appearance.theme_id = LEGACY_APPKIT_STYLE_ID.to_owned();
            appearance.component_family = ComponentFamily::AppKit;
            store.saved = false;
        }
        if !store.pending_themes.is_empty() {
            store.saved = false;
        }
        store.persist();
        store
    }

    /// Keeps a preference for the current surface when the host provides no disk location.
    pub(crate) fn session_only() -> Self {
        let mut store = Self::new(None);
        store.block_writes(DIRECTORY_UNAVAILABLE);
        store
    }

    fn new(path: Option<PathBuf>) -> Self {
        let theme_directory = path.as_deref().map(|path| parent_dir(path).join("themes"));
        let background_assets = BackgroundAssets::new(path.as_deref().map(parent_dir));
        Self {
            path,
            theme_directory,
            document: Document::default(),
            themes: StyleThemeScan::load(None),
            background_assets,
            pending_themes: BTreeMap::new(),
            pending_clone: None,
            revision: 0,
            saved: true,
            problem: None,
        }
    }

    /// Themes as the interface reads them: inheritance resolved and the accent preference applied.
    fn resolved_themes(&self) -> Vec<StyleTheme> {
        let themes = self.themes.resolved_with_custom();
        match self.document.appearance.accent.as_deref() {
            Some(accent) => StyleThemeScan::with_accent(themes, accent),
            None => themes,
        }
    }

    pub(crate) fn snapshot(&self) -> Snapshot {
        Snapshot {
            color_mode: self.document.appearance.color_mode,
            show_status_bar: self.document.appearance.show_status_bar,
            sidebar_docked: self.document.appearance.sidebar_docked,
            background_image: self.document.appearance.background_image.clone(),
            background_image_opacity: self.document.appearance.background_image_opacity,
            content_background_transparency: self
                .document
                .appearance
                .content_background_transparency,
            component_family: self.document.appearance.component_family,
            theme_id: self.document.appearance.theme_id.clone(),
            accent: self.document.appearance.accent.clone(),
            locale_preference: self.document.general.locale_preference.clone(),
            diagnostic_level: self.document.diagnostics.level,
            update_auto_check: self.document.updates.auto_check,
            custom_themes: self.themes.custom_themes.clone(),
            style_themes: self.resolved_themes(),
            theme_file_issues: self.themes.issues.clone(),
            revision: self.revision,
            saved: self.saved,
            problem: self.problem,
        }
    }

    pub(crate) fn apply(&mut self, command: Command) -> Result<Snapshot, &'static str> {
        if self.change(command)? {
            self.revision += 1;
            self.saved = false;
        }
        self.persist();
        Ok(self.snapshot())
    }

    /// Imports a private image copy before committing its reference. The cached import also
    /// supports hosts without a data directory and retries after a settings write fails.
    pub(crate) fn import_background_image(
        &mut self,
        name: String,
        bytes: Vec<u8>,
    ) -> Result<Snapshot, &'static str> {
        let image = self
            .background_assets
            .import(name, bytes, self.path.is_some())?;
        if update(&mut self.document.appearance.background_image, Some(image)) {
            self.revision += 1;
            self.saved = false;
        }
        self.persist();
        Ok(self.snapshot())
    }

    /// Only the current selection is readable through the bridge; callers cannot name a path.
    pub(crate) fn load_background_image(&self, id: &str) -> Result<Vec<u8>, &'static str> {
        let image = self
            .document
            .appearance
            .background_image
            .as_ref()
            .filter(|image| image.id == id)
            .ok_or("background_image_unavailable")?;
        self.background_assets.load(image)
    }

    pub(crate) fn rescan_themes(&mut self) -> Snapshot {
        let previous = std::mem::take(&mut self.themes);
        self.reload_themes(false);
        if previous.custom_themes != self.themes.custom_themes
            || previous.style_themes != self.themes.style_themes
            || previous.issues != self.themes.issues
        {
            self.revision += 1;
        }
        self.snapshot()
    }

    pub(crate) fn clone_style_theme(
        &mut self,
        source_id: String,
        name: String,
    ) -> Result<Snapshot, &'static str> {
        let name = name.trim();
        if !style_themes::valid_theme_name(name) {
            return Err("style_theme_invalid_name");
        }
        let directory = self
            .theme_directory
            .as_deref()
            .filter(|_| self.path.is_some())
            .ok_or("style_theme_directory_unavailable")?;

        if let Some(pending) = &self.pending_clone
            && pending.source_id == source_id
            && pending.name == name
        {
            return self.apply(Command::SetTheme(pending.cloned_id.clone()));
        }
        let source = self
            .themes
            .style_themes
            .iter()
            .find(|theme| theme.id == source_id)
            .cloned()
            .ok_or("unknown_theme")?;
        let cloned = style_themes::clone_style(directory, source, name, |id| {
            self.themes.style_themes.iter().any(|theme| theme.id == id)
                || self.themes.custom_themes.iter().any(|theme| theme.id == id)
        })?;
        let cloned_id = cloned.id.clone();
        self.themes.style_themes.push(cloned);
        self.pending_clone = Some(PendingStyleClone {
            source_id,
            name: name.to_owned(),
            cloned_id: cloned_id.clone(),
        });
        self.apply(Command::SetTheme(cloned_id))
    }

    fn change(&mut self, command: Command) -> Result<bool, &'static str> {
        let appearance = &mut self.document.appearance;
        let changed = match command {
            Command::SetColorMode(mode) => update(&mut appearance.color_mode, mode),
            Command::SetShowStatusBar(visible) => update(&mut appearance.show_status_bar, visible),
            Command::SetSidebarDocked(docked) => update(&mut appearance.sidebar_docked, docked),
            Command::ClearBackgroundImage => update(&mut appearance.background_image, None),
            Command::SetBackgroundImageOpacity(value) => {
                background::validate_opacity(value)?;
                update(&mut appearance.background_image_opacity, value)
            }
            Command::SetContentBackgroundTransparency(value) => {
                background::validate_content_transparency(value)?;
                update(&mut appearance.content_background_transparency, value)
            }
            Command::SetComponentFamily(family) => update(&mut appearance.component_family, family),
            Command::SetTheme(id) => self.select_theme(id)?,
            Command::SetAccent(accent) => {
                validate_accent(accent.as_deref())?;
                update(&mut appearance.accent, accent)
            }
            Command::SetLocale(locale) => {
                validate_locale(&locale)?;
                update(&mut self.document.general.locale_preference, locale)
            }
            Command::SetUpdateAutoCheck(enabled) => {
                update(&mut self.document.updates.auto_check, enabled)
            }
            Command::SetDiagnosticLevel(level) => {
                update(&mut self.document.diagnostics.level, level)
            }
            Command::SaveCustomTheme(theme) => self.save_custom_theme(*theme)?,
            Command::RemoveCustomTheme(id) => self.remove_custom_theme(id)?,
        };
        Ok(changed)
    }

    fn select_theme(&mut self, id: String) -> Result<bool, &'static str> {
        if !is_theme_id(&id) {
            return Err("invalid_theme_id");
        }
        let appearance = &mut self.document.appearance;
        // Older callers treated the AppKit family as a combined theme ID.
        if id == LEGACY_APPKIT_THEME_ID {
            let family_changed = update(&mut appearance.component_family, ComponentFamily::AppKit);
            let style_changed = update(&mut appearance.theme_id, LEGACY_APPKIT_STYLE_ID.to_owned());
            return Ok(family_changed || style_changed);
        }
        if !self.themes.custom_themes.iter().any(|theme| theme.id == id)
            && !self.themes.style_themes.iter().any(|theme| theme.id == id)
        {
            return Err("unknown_theme");
        }
        Ok(update(&mut appearance.theme_id, id))
    }

    fn save_custom_theme(&mut self, theme: CustomTheme) -> Result<bool, &'static str> {
        theme.validate()?;
        let id = theme.id.clone();
        if self.themes.style_themes.iter().any(|style| style.id == id) {
            return Err("style_theme_read_only");
        }
        if self.themes.custom_themes.len() >= MAX_CUSTOM_THEMES
            && !self
                .themes
                .custom_themes
                .iter()
                .any(|existing| existing.id == id)
        {
            return Err("too_many_custom_themes");
        }
        let theme_changed = replace_custom_theme(&mut self.themes.custom_themes, theme.clone());
        if theme_changed && self.path.is_some() {
            self.pending_themes.insert(id.clone(), Some(theme));
        }
        let selection_changed = update(&mut self.document.appearance.theme_id, id);
        Ok(theme_changed || selection_changed)
    }

    fn remove_custom_theme(&mut self, id: String) -> Result<bool, &'static str> {
        validate_custom_id(&id)?;
        if self.themes.style_themes.iter().any(|style| style.id == id) {
            return Err("style_theme_read_only");
        }
        let previous_count = self.themes.custom_themes.len();
        self.themes.custom_themes.retain(|theme| theme.id != id);
        let removed = previous_count != self.themes.custom_themes.len();
        if removed && self.path.is_some() {
            self.pending_themes.insert(id.clone(), None);
        }
        if self.document.appearance.theme_id == id {
            self.document.appearance.theme_id = default_theme_id();
            Ok(true)
        } else {
            Ok(removed)
        }
    }

    fn reload_themes(&mut self, migrate: bool) {
        let mut themes = StyleThemeScan::load(self.theme_directory.as_deref());
        for legacy in &self.document.custom_themes {
            let existing = themes
                .custom_themes
                .iter()
                .find(|theme| theme.id == legacy.id);
            if existing.is_some_and(|theme| theme != legacy)
                || themes
                    .style_themes
                    .iter()
                    .any(|style| style.id == legacy.id)
            {
                themes.issues.push(ThemeFileIssue {
                    file_name: format!("{}.json", legacy.id),
                    code: "style_theme_legacy_conflict",
                });
                continue;
            }
            if existing.is_none() {
                themes.custom_themes.push(legacy.clone());
            }
            if migrate && self.path.is_some() {
                self.pending_themes
                    .insert(legacy.id.clone(), Some(legacy.clone()));
            }
        }
        for (id, edit) in &self.pending_themes {
            match edit {
                Some(theme) if !themes.style_themes.iter().any(|style| style.id == *id) => {
                    replace_custom_theme(&mut themes.custom_themes, theme.clone());
                }
                Some(_) => {}
                None => themes.custom_themes.retain(|theme| theme.id != *id),
            }
        }
        themes
            .custom_themes
            .sort_by(|left, right| left.id.cmp(&right.id));
        self.themes = themes;
    }

    fn persist(&mut self) {
        self.background_assets
            .retain_cached(self.document.appearance.background_image.as_ref());
        if self.saved {
            return;
        }
        let Some(path) = self.path.clone() else {
            return;
        };
        if self.write_theme_changes().is_ok() && self.write(&path).is_ok() {
            self.saved = true;
            self.problem = None;
            self.pending_clone = None;
            self.background_assets
                .prune(self.document.appearance.background_image.as_ref());
        } else {
            self.problem = Some(WRITE_FAILED);
        }
    }

    fn write_theme_changes(&mut self) -> Result<(), &'static str> {
        let Some(directory) = &self.theme_directory else {
            return Ok(());
        };
        while let Some((id, edit)) = self.pending_themes.first_key_value() {
            match edit {
                Some(theme) => style_themes::save_custom_theme(directory, theme)?,
                None => style_themes::remove_custom_theme(directory, id)?,
            }
            self.document
                .custom_themes
                .retain(|legacy| legacy.id != *id);
            self.pending_themes.pop_first();
        }
        Ok(())
    }

    fn block_writes(&mut self, problem: &'static str) {
        self.path = None;
        self.saved = false;
        self.problem = Some(problem);
    }

    fn write(&self, path: &Path) -> io::Result<()> {
        let text = toml::to_string(&self.document).map_err(io::Error::other)?;
        let directory = parent_dir(path);
        fs::create_dir_all(directory)?;

        // Write and sync the complete document before replacing the previous file.
        let mut temporary = Builder::new().prefix(".settings-").tempfile_in(directory)?;
        temporary.write_all(text.as_bytes())?;
        temporary.as_file().sync_all()?;
        temporary.persist(path).map_err(|error| error.error)?;
        Ok(())
    }
}

fn replace_custom_theme(themes: &mut Vec<CustomTheme>, theme: CustomTheme) -> bool {
    if let Some(current) = themes.iter_mut().find(|current| current.id == theme.id) {
        update(current, theme)
    } else {
        themes.push(theme);
        true
    }
}

enum DocumentError {
    Invalid,
    UnsupportedVersion,
}

fn parse_document(text: &str) -> Result<Document, DocumentError> {
    let table: toml::Table = toml::from_str(text).map_err(|_| DocumentError::Invalid)?;
    let version = table.get("schemaVersion").and_then(toml::Value::as_integer);

    match version {
        Some(version) if version == i64::from(SCHEMA_VERSION) => {
            let document: Document = table.try_into().map_err(|_| DocumentError::Invalid)?;
            document.validate().map_err(|_| DocumentError::Invalid)?;
            Ok(document)
        }
        Some(_) => Err(DocumentError::UnsupportedVersion),
        None => Err(DocumentError::Invalid),
    }
}

fn preserve_invalid(path: &Path) -> io::Result<()> {
    let (placeholder, backup_path) = Builder::new()
        .prefix("settings.toml.corrupt-")
        .tempfile_in(parent_dir(path))?
        .keep()
        .map_err(|error| error.error)?;
    drop(placeholder);

    fs::rename(path, &backup_path).inspect_err(|_| {
        let _ = fs::remove_file(&backup_path);
    })
}

fn parent_dir(path: &Path) -> &Path {
    path.parent()
        .filter(|parent| !parent.as_os_str().is_empty())
        .unwrap_or_else(|| Path::new("."))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn publication_adapter_rejects_revision_exhaustion_before_mutation_or_io() {
        let directory = tempfile::tempdir().unwrap();
        let path = directory.path().join("settings.toml");
        let mut store = SettingsStore::open(path.clone());
        store.revision = u32::MAX;
        let service = crate::app_state::SettingsService::new(
            store,
            usher_state::StoreId::new(u128::MAX),
            &usher_state::backends::WatchBackend,
        )
        .unwrap();
        let reader = service.reader();
        let initial = reader.snapshot().unwrap();
        assert_eq!(
            service.apply(Command::SetColorMode(ColorMode::Dark)),
            Err(crate::app_state::UNAVAILABLE)
        );
        assert_eq!(reader.snapshot().unwrap().state(), initial.state());
        assert!(!path.exists());
        assert_eq!(service.snapshot(), Err(crate::app_state::UNAVAILABLE));
    }

    #[test]
    fn defaults_to_system_and_restores_each_selection() {
        let directory = tempfile::tempdir().unwrap();
        let path = directory.path().join("settings.toml");
        let mut store = SettingsStore::open(path.clone());

        let initial = store.snapshot();
        assert_eq!(initial.color_mode, ColorMode::System);
        assert!(initial.saved);
        assert!(!path.exists());

        for (index, mode) in [ColorMode::Dark, ColorMode::Light, ColorMode::System]
            .into_iter()
            .enumerate()
        {
            let snapshot = store.apply(Command::SetColorMode(mode)).unwrap();
            assert_eq!(snapshot.color_mode, mode);
            assert_eq!(snapshot.revision, index as u32 + 1);
            assert!(snapshot.saved);
            assert_eq!(snapshot.problem, None);
            assert_eq!(store.apply(Command::SetColorMode(mode)), Ok(snapshot));
            assert_eq!(
                SettingsStore::open(path.clone()).snapshot().color_mode,
                mode
            );
            assert_eq!(mode.as_str().parse(), Ok(mode));
        }
        assert!("auto".parse::<ColorMode>().is_err());
        assert_eq!(
            fs::read_to_string(&path).unwrap(),
            "schemaVersion = 1\n\n[appearance]\ncolorMode = \"system\"\n"
        );
    }

    #[test]
    fn background_defaults_restore_old_settings_and_persist_image_and_opacity() {
        let directory = tempfile::tempdir().unwrap();
        let path = directory.path().join("settings.toml");
        let previous = "schemaVersion = 1\n\n[appearance]\ncolorMode = \"dark\"\n";
        fs::write(&path, previous).unwrap();
        let mut store = SettingsStore::open(path.clone());
        assert_eq!(store.snapshot().background_image, None);
        assert_eq!(
            store.snapshot().background_image_opacity,
            background::default_opacity()
        );
        assert_eq!(fs::read_to_string(&path).unwrap(), previous);
        let bytes = background_test_image();
        let selected = store
            .import_background_image("wallpaper.png".into(), bytes.clone())
            .unwrap();
        let id = selected.background_image.as_ref().unwrap().id.clone();
        assert!(selected.saved);
        store
            .apply(Command::SetBackgroundImageOpacity(
                background::CONFIG.max_opacity,
            ))
            .unwrap();
        let mut reopened = SettingsStore::open(path.clone());
        assert_eq!(
            reopened.snapshot().background_image,
            selected.background_image
        );
        assert_eq!(
            reopened.snapshot().background_image_opacity,
            background::CONFIG.max_opacity
        );
        assert_eq!(reopened.load_background_image(&id).unwrap(), bytes);
        let before = reopened.snapshot();
        assert!(
            reopened
                .apply(Command::SetBackgroundImageOpacity(
                    background::CONFIG.max_opacity + 1
                ))
                .is_err()
        );
        assert_eq!(reopened.snapshot(), before);
        reopened.apply(Command::ClearBackgroundImage).unwrap();
        assert!(
            SettingsStore::open(path)
                .snapshot()
                .background_image
                .is_none()
        );
        assert!(reopened.load_background_image(&id).is_err());
    }

    #[test]
    fn content_transparency_defaults_validate_and_persist_independently() {
        let directory = tempfile::tempdir().unwrap();
        let path = directory.path().join("settings.toml");
        let previous = "schemaVersion = 1\n\n[appearance]\ncolorMode = \"dark\"\n";
        fs::write(&path, previous).unwrap();
        let mut store = SettingsStore::open(path.clone());
        assert_eq!(
            store.snapshot().content_background_transparency,
            background::default_content_transparency()
        );
        assert_eq!(fs::read_to_string(&path).unwrap(), previous);
        let range = &background::CONFIG.content_transparency;
        for value in [range.max, (range.min + range.max) / 2, range.min] {
            let saved = store
                .apply(Command::SetContentBackgroundTransparency(value))
                .unwrap();
            assert!(saved.saved);
            store = SettingsStore::open(path.clone());
            assert_eq!(store.snapshot().content_background_transparency, value);
            assert_eq!(store.snapshot().background_image, None);
            assert_eq!(
                store.snapshot().background_image_opacity,
                background::default_opacity()
            );
        }
        let before = store.snapshot();
        assert_eq!(
            store.apply(Command::SetContentBackgroundTransparency(range.max + 1)),
            Err("content_background_transparency_invalid")
        );
        assert_eq!(store.snapshot(), before);
        assert_eq!(
            SettingsStore::open(path)
                .snapshot()
                .content_background_transparency,
            range.min
        );
    }

    #[test]
    fn failed_background_metadata_writes_keep_saved_assets_and_retry_without_duplicates() {
        let directory = tempfile::tempdir().unwrap();
        let path = directory.path().join("settings.toml");
        let backup = directory.path().join("saved.toml");
        let mut store = SettingsStore::open(path.clone());
        let bytes = background_test_image();
        let first = store
            .import_background_image("first.png".into(), bytes.clone())
            .unwrap();
        let first_image = first.background_image.unwrap();
        fs::rename(&path, &backup).unwrap();
        fs::create_dir(&path).unwrap();
        let pending = store
            .import_background_image("second.png".into(), bytes.clone())
            .unwrap();
        assert!(!pending.saved);
        let saved_store = SettingsStore::open(backup.clone());
        assert_eq!(
            saved_store.load_background_image(&first_image.id).unwrap(),
            bytes
        );
        let retried = store
            .import_background_image("second.png".into(), bytes.clone())
            .unwrap();
        assert_eq!(retried.revision, pending.revision);
        assert_eq!(retried.background_image, pending.background_image);
        fs::remove_dir(&path).unwrap();
        fs::rename(&backup, &path).unwrap();
        let committed = store
            .import_background_image("second.png".into(), bytes.clone())
            .unwrap();
        assert!(committed.saved);
        assert_eq!(committed.revision, pending.revision);
        let selected_id = committed.background_image.unwrap().id;
        assert_eq!(
            SettingsStore::open(path.clone())
                .load_background_image(&selected_id)
                .unwrap(),
            bytes
        );
        assert!(saved_store.load_background_image(&first_image.id).is_err());

        fs::rename(&path, &backup).unwrap();
        fs::create_dir(&path).unwrap();
        let cleared = store.apply(Command::ClearBackgroundImage).unwrap();
        assert!(!cleared.saved);
        assert_eq!(
            SettingsStore::open(backup.clone())
                .load_background_image(&selected_id)
                .unwrap(),
            bytes
        );
        fs::remove_dir(&path).unwrap();
        fs::rename(&backup, &path).unwrap();
        assert!(store.apply(Command::ClearBackgroundImage).unwrap().saved);
        assert!(
            SettingsStore::open(path)
                .load_background_image(&selected_id)
                .is_err()
        );
    }

    fn background_test_image() -> Vec<u8> {
        let fixture: serde_json::Value =
            serde_json::from_str(include_str!("../tests/fixtures/background.json")).unwrap();
        nsh_core::image::decode_data_uri(fixture["dataUrl"].as_str().unwrap())
            .unwrap()
            .1
    }

    #[test]
    fn status_bar_defaults_visible_in_existing_files_and_persists_changes() {
        let directory = tempfile::tempdir().unwrap();
        let path = directory.path().join("settings.toml");
        let previous = "schemaVersion = 1\n\n[appearance]\ncolorMode = \"dark\"\n";
        fs::write(&path, previous).unwrap();
        let mut store = SettingsStore::open(path.clone());

        let initial = store.snapshot();
        assert!(initial.show_status_bar);
        assert_eq!(
            store.apply(Command::SetShowStatusBar(true)).unwrap(),
            initial
        );
        assert_eq!(fs::read_to_string(&path).unwrap(), previous);

        let hidden = store.apply(Command::SetShowStatusBar(false)).unwrap();
        assert!(!hidden.show_status_bar);
        assert!(hidden.saved);
        assert_eq!(hidden.revision, initial.revision + 1);
        assert_eq!(
            store.apply(Command::SetShowStatusBar(false)).unwrap(),
            hidden
        );

        let mut reopened = SettingsStore::open(path.clone());
        let persisted = reopened.snapshot();
        assert!(!persisted.show_status_bar);
        assert_eq!(persisted.color_mode, ColorMode::Dark);
        let visible = reopened.apply(Command::SetShowStatusBar(true)).unwrap();
        assert!(visible.show_status_bar);
        assert!(visible.saved);
        assert_eq!(fs::read_to_string(path).unwrap(), previous);
    }

    #[test]
    fn sidebar_defaults_docked_and_restores_an_undocked_preference() {
        let directory = tempfile::tempdir().unwrap();
        let path = directory.path().join("settings.toml");
        let mut store = SettingsStore::open(path.clone());
        assert!(store.snapshot().sidebar_docked);
        let undocked = store.apply(Command::SetSidebarDocked(false)).unwrap();
        assert!(!undocked.sidebar_docked);
        assert!(undocked.saved);
        assert_eq!(
            store.apply(Command::SetSidebarDocked(false)).unwrap(),
            undocked
        );
        assert!(!SettingsStore::open(path).snapshot().sidebar_docked);
    }

    #[test]
    fn update_preferences_round_trip_and_default_omission_preserve_legacy_documents() {
        let directory = tempfile::tempdir().unwrap();
        let path = directory.path().join("settings.toml");
        let mut store = SettingsStore::open(path.clone());
        assert!(store.snapshot().update_auto_check);
        let disabled = store.apply(Command::SetUpdateAutoCheck(false)).unwrap();
        assert!(disabled.saved);
        let text = fs::read_to_string(&path).unwrap();
        assert!(text.contains("[updates]") && text.contains("auto_check = false"));
        let mut reopened = SettingsStore::open(path.clone());
        assert!(!reopened.snapshot().update_auto_check);
        assert!(
            reopened
                .apply(Command::SetUpdateAutoCheck(true))
                .unwrap()
                .saved
        );
        assert!(!fs::read_to_string(&path).unwrap().contains("[updates]"));
        assert!(SettingsStore::open(path).snapshot().update_auto_check);
    }

    #[test]
    fn diagnostic_level_defaults_to_info_and_survives_a_restart() {
        let directory = tempfile::tempdir().unwrap();
        let path = directory.path().join("settings.toml");
        let mut store = SettingsStore::open(path.clone());
        assert_eq!(store.snapshot().diagnostic_level, DiagnosticLevel::Info);

        let debug = store
            .apply(Command::SetDiagnosticLevel(DiagnosticLevel::Debug))
            .unwrap();
        assert_eq!(debug.diagnostic_level, DiagnosticLevel::Debug);
        assert!(debug.saved);
        let text = fs::read_to_string(&path).unwrap();
        assert!(
            text.contains("[diagnostics]") && text.contains("DEBUG"),
            "{text}"
        );
        assert_eq!(
            SettingsStore::open(path.clone())
                .snapshot()
                .diagnostic_level,
            DiagnosticLevel::Debug
        );

        // The default is left out of the file, so older builds can still read it.
        store
            .apply(Command::SetDiagnosticLevel(DiagnosticLevel::Info))
            .unwrap();
        assert!(!fs::read_to_string(&path).unwrap().contains("[diagnostics]"));
        assert_eq!("warn".parse::<DiagnosticLevel>(), Ok(DiagnosticLevel::Warn));
        assert!("loud".parse::<DiagnosticLevel>().is_err());
    }

    #[test]
    fn preserves_corrupt_settings_before_saving() {
        let directory = tempfile::tempdir().unwrap();
        let path = directory.path().join("settings.toml");
        let corrupt_bytes = b"schemaVersion = 1\n[appearance]\ncolorMode = \"dark\xFF\"";
        fs::write(&path, corrupt_bytes).unwrap();

        let mut store = SettingsStore::open(path.clone());
        assert_eq!(store.snapshot().problem, Some(INVALID_RECOVERED));
        assert!(
            store
                .apply(Command::SetColorMode(ColorMode::Dark))
                .unwrap()
                .saved
        );
        let backup = fs::read_dir(directory.path())
            .unwrap()
            .map(|entry| entry.unwrap().path())
            .find(|entry| entry != &path)
            .unwrap();
        assert_eq!(fs::read(backup).unwrap(), corrupt_bytes);
        assert_eq!(
            SettingsStore::open(path).snapshot().color_mode,
            ColorMode::Dark
        );
    }

    #[test]
    fn protects_newer_schemas_even_when_their_fields_are_unknown() {
        let directory = tempfile::tempdir().unwrap();
        let path = directory.path().join("settings.toml");
        let future = "schemaVersion = 2\n[appearance]\ncolorMode = \"new-mode\"\n";
        fs::write(&path, future).unwrap();

        let mut store = SettingsStore::open(path.clone());
        let changed = store
            .apply(Command::SetColorMode(ColorMode::Light))
            .unwrap();
        assert_eq!(changed.color_mode, ColorMode::Light);
        assert!(!changed.saved);
        assert_eq!(changed.problem, Some(UNSUPPORTED_VERSION));
        assert_eq!(fs::read_to_string(path).unwrap(), future);
    }

    #[test]
    fn preserves_unreadable_paths() {
        let directory = tempfile::tempdir().unwrap();
        let mut store = SettingsStore::open(directory.path().to_path_buf());
        let changed = store.apply(Command::SetColorMode(ColorMode::Dark)).unwrap();
        assert!(!changed.saved);
        assert_eq!(changed.problem, Some(READ_FAILED));
        assert!(directory.path().is_dir());
    }

    #[test]
    fn failed_writes_keep_selection_and_retry_without_advancing_revision() {
        let directory = tempfile::tempdir().unwrap();
        let parent = directory.path().join("settings");
        let path = parent.join("settings.toml");
        let mut store = SettingsStore::open(path.clone());
        fs::write(&parent, "block the settings directory").unwrap();

        let pending = store.apply(Command::SetColorMode(ColorMode::Dark)).unwrap();
        assert_eq!(pending.color_mode, ColorMode::Dark);
        assert!(!pending.saved);
        assert_eq!(pending.problem, Some(WRITE_FAILED));

        fs::remove_file(parent).unwrap();
        let saved = store.apply(Command::SetColorMode(ColorMode::Dark)).unwrap();
        assert!(saved.saved);
        assert_eq!(saved.revision, pending.revision);
        assert_eq!(saved.problem, None);
        assert_eq!(
            SettingsStore::open(path).snapshot().color_mode,
            ColorMode::Dark
        );
    }

    #[test]
    fn migrates_a_combined_appkit_theme_without_coupling_later_selections() {
        let directory = tempfile::tempdir().unwrap();
        let path = directory.path().join("settings.toml");
        fs::write(
            &path,
            "schemaVersion = 1\n\n[appearance]\ncolorMode = \"dark\"\nthemeId = \"apple-appkit\"\n\n[general]\nlocalePreference = \"zh-Hans\"\n",
        )
        .unwrap();
        let mut store = SettingsStore::open(path.clone());
        let migrated = store.snapshot();
        assert!(migrated.saved);
        assert_eq!(migrated.theme_id, LEGACY_APPKIT_STYLE_ID);
        assert_eq!(migrated.component_family, ComponentFamily::AppKit);
        assert_eq!(migrated.color_mode, ColorMode::Dark);
        assert_eq!(migrated.locale_preference, "zh-Hans");
        assert!(
            fs::read_to_string(&path)
                .unwrap()
                .contains("componentFamily = \"appkit\"")
        );

        let preset = store
            .apply(Command::SetTheme("builtin.primer".into()))
            .unwrap();
        assert_eq!(preset.component_family, ComponentFamily::AppKit);
        let standard = store
            .apply(Command::SetComponentFamily(ComponentFamily::Nsh))
            .unwrap();
        assert_eq!(standard.theme_id, "builtin.primer");
        assert_eq!(standard.color_mode, ColorMode::Dark);
        assert_eq!(standard.locale_preference, "zh-Hans");
        let reopened = SettingsStore::open(path).snapshot();
        assert_eq!(reopened.theme_id, "builtin.primer");
        assert_eq!(reopened.component_family, ComponentFamily::Nsh);
    }

    #[test]
    fn extends_batch_one_preferences_and_round_trips_custom_themes() {
        let directory = tempfile::tempdir().unwrap();
        let path = directory.path().join("settings.toml");
        fs::write(
            &path,
            "schemaVersion = 1\n\n[appearance]\ncolorMode = \"dark\"\n",
        )
        .unwrap();
        let mut store = SettingsStore::open(path.clone());
        let initial = store.snapshot();
        assert_eq!(initial.color_mode, ColorMode::Dark);
        assert_eq!(initial.theme_id, "default");
        assert_eq!(initial.accent, None);
        assert_eq!(initial.locale_preference, "system");
        assert!(initial.custom_themes.is_empty());

        store
            .apply(Command::SetTheme("apple-glass".into()))
            .unwrap();
        store
            .apply(Command::SetAccent(Some("#aabbcc".into())))
            .unwrap();
        store.apply(Command::SetLocale("zh-Hans".into())).unwrap();
        let mut theme = ocean_theme();
        let snapshot = store
            .apply(Command::SaveCustomTheme(Box::new(theme.clone())))
            .unwrap();
        assert!(snapshot.saved);
        assert_eq!(snapshot.theme_id, theme.id);
        assert_eq!(snapshot.custom_themes, [theme.clone()]);

        theme.name = "Ocean Blue".into();
        let edited = store
            .apply(Command::SaveCustomTheme(Box::new(theme.clone())))
            .unwrap();
        assert_eq!(edited.custom_themes, [theme.clone()]);
        let mut reopened = SettingsStore::open(path.clone());
        let persisted = reopened.snapshot();
        assert_eq!(persisted.color_mode, ColorMode::Dark);
        assert_eq!(persisted.accent.as_deref(), Some("#aabbcc"));
        assert_eq!(persisted.locale_preference, "zh-Hans");
        assert_eq!(persisted.theme_id, theme.id);
        assert_eq!(persisted.custom_themes, [theme.clone()]);
        let disk_style =
            fs::read_to_string(directory.path().join("themes/custom-ocean.json")).unwrap();
        assert!(disk_style.contains("\"editorManaged\": true"));
        assert!(!fs::read_to_string(&path).unwrap().contains("customThemes"));

        let removed = reopened
            .apply(Command::RemoveCustomTheme(theme.id))
            .unwrap();
        assert_eq!(removed.theme_id, "default");
        assert!(removed.custom_themes.is_empty());
        assert_eq!(SettingsStore::open(path).snapshot().theme_id, "default");
    }

    #[test]
    fn migrates_legacy_embedded_palettes_and_preserves_conflicting_files() {
        let directory = tempfile::tempdir().unwrap();
        let path = directory.path().join("settings.toml");
        let theme = ocean_theme();
        let mut legacy = Document::default();
        legacy.appearance.theme_id.clone_from(&theme.id);
        legacy.custom_themes.push(theme.clone());
        let old_settings = toml::to_string(&legacy).unwrap();
        fs::write(&path, &old_settings).unwrap();
        let theme_directory = directory.path().join("themes");
        fs::create_dir(&theme_directory).unwrap();
        let file = theme_directory.join("custom-ocean.json");
        let external = r##"{"schemaVersion":1,"id":"custom-ocean","name":"External ocean","extends":"default","cssVars":{"light":{"sidebar":"#EFEDE9B3"}}}"##;
        fs::write(&file, external).unwrap();

        let mut conflicted = SettingsStore::open(path.clone());
        assert_eq!(
            conflicted.snapshot().theme_file_issues[0].code,
            "style_theme_legacy_conflict"
        );
        assert_eq!(fs::read_to_string(&file).unwrap(), external);
        assert_eq!(fs::read_to_string(&path).unwrap(), old_settings);
        assert_eq!(
            conflicted.apply(Command::SaveCustomTheme(Box::new(theme.clone()))),
            Err("style_theme_read_only")
        );

        fs::remove_file(&file).unwrap();
        let migrated = SettingsStore::open(path.clone()).snapshot();
        assert!(migrated.saved);
        assert_eq!(migrated.theme_id, theme.id);
        assert_eq!(migrated.custom_themes, std::slice::from_ref(&theme));
        assert!(migrated.theme_file_issues.is_empty());
        assert!(file.is_file());
        assert!(!fs::read_to_string(&path).unwrap().contains("customThemes"));
        assert_eq!(SettingsStore::open(path).snapshot().custom_themes, [theme]);
    }

    #[test]
    fn rescans_and_selects_a_local_style_without_rewriting_the_file() {
        let directory = tempfile::tempdir().unwrap();
        let path = directory.path().join("settings.toml");
        let mut store = SettingsStore::open(path.clone());
        let style_dir = directory.path().join("themes/warm-frost");
        fs::create_dir_all(&style_dir).unwrap();
        let style_path = style_dir.join("theme.json");
        let style = r##"{"schemaVersion":1,"id":"user.warm-frost","name":"暖灰磨砂","extends":"default","cssVars":{"light":{"sidebar":"#EFEDE9B3"},"dark":{"sidebar":"#242322B3"}}}"##;
        fs::write(&style_path, style).unwrap();
        let rescanned = store.rescan_themes();
        assert!(
            rescanned
                .style_themes
                .iter()
                .any(|theme| theme.id == "user.warm-frost")
        );
        assert_eq!(rescanned.revision, 1);
        assert!(rescanned.saved);
        assert_eq!(store.rescan_themes().revision, rescanned.revision);
        assert_eq!(
            store
                .apply(Command::SetTheme("user.warm-frost".into()))
                .unwrap()
                .theme_id,
            "user.warm-frost"
        );
        assert_eq!(fs::read_to_string(style_path).unwrap(), style);
        assert_eq!(
            SettingsStore::open(path).snapshot().theme_id,
            "user.warm-frost"
        );
    }

    #[test]
    fn cloning_a_style_preserves_all_tokens_and_retries_without_a_second_file() {
        let directory = tempfile::tempdir().unwrap();
        let path = directory.path().join("settings.toml");
        let theme_directory = directory.path().join("themes");
        fs::create_dir(&theme_directory).unwrap();
        let source = r##"{"schemaVersion":1,"id":"user.reference","name":"Reference","extends":"apple-glass","cssVars":{"light":{"sidebar":"#EFEDE9B3","window-tint":"#11223340","radius":20},"dark":{"panel":"#242322B3"}}}"##;
        let source_path = theme_directory.join("reference.json");
        fs::write(&source_path, source).unwrap();
        let mut store = SettingsStore::open(path.clone());
        let original = store
            .snapshot()
            .style_themes
            .into_iter()
            .find(|theme| theme.id == "user.reference")
            .unwrap();

        // The theme file can be written even if the settings replacement fails.
        fs::create_dir(&path).unwrap();
        let pending = store
            .clone_style_theme("user.reference".into(), "Local copy".into())
            .unwrap();
        assert!(!pending.saved);
        assert_eq!(pending.theme_id, "user.reference-copy");
        assert_eq!(fs::read_dir(&theme_directory).unwrap().count(), 2);
        fs::remove_dir(&path).unwrap();
        let saved = store
            .clone_style_theme("user.reference".into(), "Local copy".into())
            .unwrap();
        assert!(saved.saved);
        assert_eq!(saved.theme_id, pending.theme_id);
        assert_eq!(saved.revision, pending.revision);
        assert_eq!(fs::read_dir(&theme_directory).unwrap().count(), 2);

        let reopened = SettingsStore::open(path).snapshot();
        assert_eq!(reopened.theme_id, saved.theme_id);
        let copied = reopened
            .style_themes
            .into_iter()
            .find(|theme| theme.id == saved.theme_id)
            .unwrap();
        assert_eq!(copied.name, "Local copy");
        assert_eq!(copied.extends, original.extends);
        assert_eq!(copied.css_vars, original.css_vars);
        assert_eq!(fs::read_to_string(source_path).unwrap(), source);
    }

    #[test]
    fn cloning_built_in_styles_skips_existing_paths_and_requires_a_directory() {
        let mut session = SettingsStore::session_only();
        assert_eq!(
            session.clone_style_theme("default".into(), "Copy".into()),
            Err("style_theme_directory_unavailable")
        );

        let directory = tempfile::tempdir().unwrap();
        let path = directory.path().join("settings.toml");
        let theme_directory = directory.path().join("themes");
        fs::create_dir(&theme_directory).unwrap();
        let existing = theme_directory.join("user.apple-glass-copy.json");
        fs::write(&existing, "keep this file").unwrap();
        let mut store = SettingsStore::open(path);
        let cloned = store
            .clone_style_theme("apple-glass".into(), "Glass copy".into())
            .unwrap();
        assert!(cloned.saved);
        assert_eq!(cloned.theme_id, "user.apple-glass-copy-2");
        let copied = cloned
            .style_themes
            .iter()
            .find(|theme| theme.id == cloned.theme_id)
            .unwrap();
        let glass = cloned
            .style_themes
            .iter()
            .find(|theme| theme.id == "apple-glass")
            .unwrap();
        assert_eq!(copied.material, glass.material);
        assert_eq!(copied.css_vars, glass.css_vars);
        assert_eq!(fs::read_to_string(existing).unwrap(), "keep this file");
    }

    #[test]
    fn rejects_invalid_commands_before_changing_state() {
        let directory = tempfile::tempdir().unwrap();
        let path = directory.path().join("settings.toml");
        let mut store = SettingsStore::open(path.clone());
        let initial = store.snapshot();

        let mut low_contrast = ocean_theme();
        low_contrast.dark.text = low_contrast.dark.background.clone();
        let mut invalid_color = ocean_theme();
        invalid_color.light.accent = "#+12345".into();
        let mut empty_name = ocean_theme();
        empty_name.name = "   ".into();
        for (command, code) in [
            (Command::SetAccent(Some("red".into())), "invalid_accent"),
            (Command::SetLocale("en-GB".into()), "invalid_locale"),
            (Command::SetTheme("custom-missing".into()), "unknown_theme"),
            (
                Command::RemoveCustomTheme("apple-glass".into()),
                "invalid_theme_id",
            ),
            (
                Command::SaveCustomTheme(Box::new(low_contrast)),
                "invalid_theme_contrast",
            ),
            (
                Command::SaveCustomTheme(Box::new(invalid_color)),
                "invalid_theme_color",
            ),
            (
                Command::SaveCustomTheme(Box::new(empty_name)),
                "invalid_theme_name",
            ),
        ] {
            assert_eq!(store.apply(command), Err(code));
            assert_eq!(store.snapshot(), initial);
        }
        assert!(!path.exists());
    }

    #[test]
    fn an_unavailable_theme_stays_selected_when_other_settings_change() {
        let directory = tempfile::tempdir().unwrap();
        let path = directory.path().join("settings.toml");
        fs::write(
            &path,
            "schemaVersion = 1\n\n[appearance]\ncolorMode = \"system\"\nthemeId = \"missing-theme\"\n",
        )
        .unwrap();
        let mut store = SettingsStore::open(path.clone());

        assert_eq!(store.snapshot().theme_id, "missing-theme");
        let changed = store.apply(Command::SetLocale("en".into())).unwrap();
        assert!(changed.saved);
        assert_eq!(
            SettingsStore::open(path).snapshot().theme_id,
            "missing-theme"
        );
    }

    #[test]
    fn retrying_a_removed_theme_persists_without_another_revision() {
        let directory = tempfile::tempdir().unwrap();
        let active = directory.path().join("active");
        let backup = directory.path().join("backup");
        let path = active.join("settings.toml");
        let mut store = SettingsStore::open(path.clone());
        let theme = ocean_theme();
        assert!(
            store
                .apply(Command::SaveCustomTheme(Box::new(theme.clone())))
                .unwrap()
                .saved
        );
        fs::rename(&active, &backup).unwrap();
        fs::write(&active, "block the settings directory").unwrap();

        let removed = store
            .apply(Command::RemoveCustomTheme(theme.id.clone()))
            .unwrap();
        assert_eq!(removed.theme_id, "default");
        assert!(removed.custom_themes.is_empty());
        assert!(!removed.saved);
        fs::remove_file(&active).unwrap();
        fs::rename(backup, active).unwrap();

        let retried = store.apply(Command::RemoveCustomTheme(theme.id)).unwrap();
        assert!(retried.saved);
        assert_eq!(retried.revision, removed.revision);
        assert!(
            SettingsStore::open(path)
                .snapshot()
                .custom_themes
                .is_empty()
        );
    }

    fn ocean_theme() -> CustomTheme {
        CustomTheme {
            id: "custom-ocean".into(),
            name: "Ocean".into(),
            light: Palette {
                background: "#FFFFFF".into(),
                surface: "#F5F5F5".into(),
                text: "#111111".into(),
                accent: "#006688".into(),
            },
            dark: Palette {
                background: "#000000".into(),
                surface: "#111111".into(),
                text: "#EEEEEE".into(),
                accent: "#99CCFF".into(),
            },
        }
    }
}
