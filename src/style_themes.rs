//! File-backed appearance themes.
//!
//! Every theme, bundled or local, is one JSON document in the same schema. A theme lists design
//! tokens for the light and dark schemes and names the theme it `extends`; the registry resolves
//! the chain so a consumer receives a complete token table. No theme is special-cased in code
//! except the root, the default theme, which completes every chain. A style file never chooses
//! a native component family.

use std::{
    collections::{BTreeMap, HashSet},
    fs::{self, File},
    io::{self, Read, Write},
    path::{Path, PathBuf},
};

use serde::{Deserialize, Deserializer, Serialize, Serializer, de::Error as _};
use tempfile::Builder;

use crate::settings::{CustomTheme, Palette};

/// The root of every inheritance chain and the fallback selection.
pub(crate) const DEFAULT_THEME_ID: &str = "default";

const SCHEMA_VERSION: u32 = 1;
const MAX_FILE_BYTES: u64 = 128 * 1024;
const MAX_TOKENS_PER_SCHEME: usize = 192;
const MAX_TOKEN_VALUE_LEN: usize = 200;
const MAX_EXTENDS_DEPTH: usize = 8;
const MAX_LOCALIZED_NAMES: usize = 8;
/// Position for themes that do not state one: after every bundled theme.
const DEFAULT_ORDER: u32 = 1000;

// `BUNDLED_THEMES` lists every file under `ui/theme/`; see `build.rs`.
include!(concat!(env!("OUT_DIR"), "/bundled_themes.rs"));

/// How the window material shows through a theme's surfaces.
#[derive(Clone, Copy, Debug, Default, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "lowercase")]
pub enum Material {
    #[default]
    Solid,
    Glass,
}

impl Material {
    pub(crate) fn as_str(self) -> &'static str {
        match self {
            Self::Solid => "solid",
            Self::Glass => "glass",
        }
    }
}

/// Design tokens of one color scheme, keyed by their CSS variable name.
///
/// Values are strings so one table can hold colors, lengths and shadow lists. A JSON number is
/// accepted for lengths and written back as a number.
#[derive(Clone, Debug, Default, Eq, PartialEq)]
pub struct Tokens(pub BTreeMap<String, String>);

impl<'de> Deserialize<'de> for Tokens {
    fn deserialize<D: Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        let raw = BTreeMap::<String, serde_json::Value>::deserialize(deserializer)?;
        raw.into_iter()
            .map(|(name, value)| match value {
                serde_json::Value::String(text) => Ok((name, text)),
                // Metrics are written as plain numbers (`28`, `0.97`) and kept as their text.
                serde_json::Value::Number(number) => Ok((name, number.to_string())),
                _ => Err(D::Error::custom("token values are strings or numbers")),
            })
            .collect::<Result<_, _>>()
            .map(Self)
    }
}

impl Serialize for Tokens {
    fn serialize<S: Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        use serde::ser::SerializeMap;
        let mut map = serializer.serialize_map(Some(self.0.len()))?;
        for (name, value) in &self.0 {
            // Numeric families round-trip as numbers so a copied theme reads like the original.
            match family_of(name) {
                Family::Free => map.serialize_entry(name, value)?,
                _ => match value.parse::<serde_json::Number>() {
                    Ok(number) => map.serialize_entry(name, &number)?,
                    Err(_) => map.serialize_entry(name, value)?,
                },
            }
        }
        map.end()
    }
}

impl Tokens {
    fn is_empty(&self) -> bool {
        self.0.is_empty()
    }

    fn validate(&self) -> Result<(), &'static str> {
        if self.0.len() > MAX_TOKENS_PER_SCHEME {
            return Err("style_theme_too_many_tokens");
        }
        self.0
            .iter()
            .try_for_each(|(name, value)| validate_token(name, value))
    }

    fn from_palette(palette: &Palette) -> Self {
        Self(BTreeMap::from([
            ("background".to_owned(), palette.background.clone()),
            ("card".to_owned(), palette.surface.clone()),
            ("foreground".to_owned(), palette.text.clone()),
            ("primary".to_owned(), palette.accent.clone()),
        ]))
    }

    /// Returns the palette when the table holds exactly the palette editor's four tokens.
    fn to_palette(&self) -> Option<Palette> {
        if self.0.len() != 4 {
            return None;
        }
        Some(Palette {
            background: self.0.get("background")?.clone(),
            surface: self.0.get("card")?.clone(),
            text: self.0.get("foreground")?.clone(),
            accent: self.0.get("primary")?.clone(),
        })
    }
}

#[derive(Clone, Debug, Default, Deserialize, Eq, PartialEq, Serialize)]
#[serde(default, deny_unknown_fields)]
pub struct StyleCssVars {
    /// Tokens that do not depend on the color scheme, such as sizes and motion. A scheme's own
    /// table wins over this one.
    #[serde(skip_serializing_if = "Tokens::is_empty")]
    pub shared: Tokens,
    /// Metrics for touch screens, in the same names as `shared`. The registry keeps them as a
    /// separate table: whether a device is a touch device is known to the interface, not here.
    #[serde(skip_serializing_if = "Tokens::is_empty")]
    pub touch: Tokens,
    pub light: Tokens,
    pub dark: Tokens,
}

/// A theme as written in its file. The internal theme scanner merges it with its bases.
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct StyleTheme {
    pub id: String,
    pub name: String,
    /// Display names by UI locale, used instead of `name` when one matches.
    #[serde(default, skip_serializing_if = "BTreeMap::is_empty")]
    pub names: BTreeMap<String, String>,
    /// The theme this one inherits from. Omitted means the default theme.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub extends: Option<String>,
    /// Position in the theme list. Equal positions keep their discovery order.
    #[serde(default = "default_order")]
    pub order: u32,
    /// Where a vendored theme comes from, kept so a later sync can be traced and credited.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub source: Option<ThemeSource>,
    /// Inherited from the base theme when omitted.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub material: Option<Material>,
    #[serde(default)]
    pub css_vars: StyleCssVars,
}

/// Provenance of a theme derived from an external design system.
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct ThemeSource {
    pub name: String,
    pub url: String,
    pub license: String,
    pub revision: String,
}

fn default_order() -> u32 {
    DEFAULT_ORDER
}

impl StyleTheme {
    fn validate(&self, bundled: bool) -> Result<(), &'static str> {
        if !is_style_id(&self.id, bundled) {
            return Err("style_theme_invalid_id");
        }
        if !valid_theme_name(&self.name) {
            return Err("style_theme_invalid_name");
        }
        if self.names.len() > MAX_LOCALIZED_NAMES
            || self
                .names
                .iter()
                .any(|(locale, name)| !valid_locale_tag(locale) || !valid_theme_name(name))
        {
            return Err("style_theme_invalid_name");
        }
        match self.extends.as_deref() {
            // The root completes every chain, so it cannot depend on another theme.
            Some(_) if self.id == DEFAULT_THEME_ID => return Err("style_theme_invalid_base"),
            Some(base) if base == self.id || !is_theme_ref(base) => {
                return Err("style_theme_invalid_base");
            }
            _ => {}
        }
        self.css_vars.shared.validate()?;
        self.css_vars.touch.validate()?;
        self.css_vars.light.validate()?;
        self.css_vars.dark.validate()
    }

    /// The ID this theme inherits from, or `None` for the root.
    fn base_id(&self) -> Option<&str> {
        match self.extends.as_deref() {
            Some(base) => Some(base),
            None if self.id == DEFAULT_THEME_ID => None,
            None => Some(DEFAULT_THEME_ID),
        }
    }
}

#[derive(Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
struct ThemeFile {
    schema_version: u32,
    #[serde(default, skip_serializing_if = "std::ops::Not::not")]
    editor_managed: bool,
    #[serde(flatten)]
    style: StyleTheme,
}

impl ThemeFile {
    fn parse(text: &str, bundled: bool) -> Result<Self, &'static str> {
        let value: serde_json::Value =
            serde_json::from_str(text).map_err(|_| "style_theme_invalid_json")?;
        match value
            .get("schemaVersion")
            .and_then(serde_json::Value::as_u64)
        {
            Some(version) if version == u64::from(SCHEMA_VERSION) => {}
            Some(_) => return Err("style_theme_unsupported_version"),
            None => return Err("style_theme_invalid_format"),
        }
        let file: Self = serde_json::from_value(value).map_err(|_| "style_theme_invalid_format")?;
        file.style.validate(bundled)?;
        Ok(file)
    }

    fn from_custom(theme: &CustomTheme) -> Self {
        Self {
            schema_version: SCHEMA_VERSION,
            editor_managed: true,
            style: StyleTheme {
                id: theme.id.clone(),
                name: theme.name.clone(),
                names: BTreeMap::new(),
                extends: None,
                order: DEFAULT_ORDER,
                source: None,
                material: None,
                css_vars: StyleCssVars {
                    shared: Tokens::default(),
                    touch: Tokens::default(),
                    light: Tokens::from_palette(&theme.light),
                    dark: Tokens::from_palette(&theme.dark),
                },
            },
        }
    }

    fn custom_theme(&self) -> Option<CustomTheme> {
        if !self.editor_managed
            || self.style.base_id() != Some(DEFAULT_THEME_ID)
            || self.style.material.is_some()
            || !self.style.names.is_empty()
        {
            return None;
        }
        let theme = CustomTheme {
            id: self.style.id.clone(),
            name: self.style.name.clone(),
            light: self.style.css_vars.light.to_palette()?,
            dark: self.style.css_vars.dark.to_palette()?,
        };
        theme.validate().ok()?;
        Some(theme)
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ThemeFileIssue {
    pub file_name: String,
    pub code: &'static str,
}

#[derive(Default)]
pub(crate) struct StyleThemeScan {
    pub(crate) custom_themes: Vec<CustomTheme>,
    /// Themes as authored, ordered by their `order` field. Use [`Self::resolved`] to read tokens.
    pub(crate) style_themes: Vec<StyleTheme>,
    pub(crate) issues: Vec<ThemeFileIssue>,
}

impl StyleThemeScan {
    pub(crate) fn load(directory: Option<&Path>) -> Self {
        let mut scan = Self::default();
        let mut sources: Vec<String> = Vec::new();
        for text in BUNDLED_THEMES {
            match ThemeFile::parse(text, true) {
                Ok(file) => {
                    sources.push(file.style.id.clone());
                    scan.style_themes.push(file.style);
                }
                // A broken bundled theme is a build defect: `bundled_themes_are_valid` fails first.
                Err(code) => scan.issues.push(ThemeFileIssue {
                    file_name: "bundled".into(),
                    code,
                }),
            }
        }
        if let Some(directory) = directory {
            scan.read_directory(directory, &mut sources);
        }
        scan.prune_unresolvable(&mut sources);
        // Stable sort: themes with the same `order` keep bundled-then-file discovery order.
        scan.style_themes.sort_by_key(|theme| theme.order);
        scan
    }

    fn read_directory(&mut self, directory: &Path, sources: &mut Vec<String>) {
        match fs::symlink_metadata(directory) {
            Ok(metadata) if metadata.is_dir() => {}
            Err(error) if error.kind() == io::ErrorKind::NotFound => return,
            _ => return self.directory_unavailable(),
        }
        let Ok(entries) = fs::read_dir(directory) else {
            return self.directory_unavailable();
        };
        let mut entries = entries.filter_map(Result::ok).collect::<Vec<_>>();
        entries.sort_by_key(fs::DirEntry::file_name);
        let mut seen: HashSet<_> = self
            .style_themes
            .iter()
            .map(|theme| theme.id.clone())
            .collect();
        for entry in entries {
            let Ok(kind) = entry.file_type() else {
                continue;
            };
            let path = entry.path();
            let path = if kind.is_dir() {
                let path = path.join("theme.json");
                if let Err(error) = fs::symlink_metadata(&path)
                    && error.kind() == io::ErrorKind::NotFound
                {
                    continue;
                }
                path
            } else if path
                .extension()
                .is_some_and(|ext| ext.eq_ignore_ascii_case("json"))
            {
                path
            } else {
                continue;
            };
            let file_name = path
                .strip_prefix(directory)
                .unwrap_or(&path)
                .to_string_lossy()
                .into_owned();
            match read_file(&path) {
                Ok(file) if seen.insert(file.style.id.clone()) => {
                    if let Some(custom) = file
                        .custom_theme()
                        .filter(|theme| path == editor_path(directory, &theme.id))
                    {
                        self.custom_themes.push(custom);
                    } else {
                        sources.push(file_name);
                        self.style_themes.push(file.style);
                    }
                }
                Ok(_) => self.issues.push(ThemeFileIssue {
                    file_name,
                    code: "style_theme_duplicate_id",
                }),
                Err(code) => self.issues.push(ThemeFileIssue { file_name, code }),
            }
        }
    }

    fn directory_unavailable(&mut self) {
        self.issues.push(ThemeFileIssue {
            file_name: "themes".into(),
            code: "style_theme_directory_unavailable",
        });
    }

    /// Drops themes whose `extends` chain is missing, cyclic, too deep or holds a bad reference.
    /// Removing one theme can orphan another that extends it, so this repeats until stable.
    fn prune_unresolvable(&mut self, sources: &mut Vec<String>) {
        loop {
            let broken = self
                .style_themes
                .iter()
                .position(|theme| self.resolve(theme).is_err());
            let Some(index) = broken else { return };
            let code = self.resolve(&self.style_themes[index]).unwrap_err();
            self.style_themes.remove(index);
            self.issues.push(ThemeFileIssue {
                file_name: sources.remove(index),
                code,
            });
        }
    }

    /// Merges a theme with the chain it extends, then substitutes `var(--name)` references.
    fn resolve(&self, theme: &StyleTheme) -> Result<StyleTheme, &'static str> {
        let mut chain = vec![theme];
        while let Some(base) = chain.last().and_then(|theme| theme.base_id()) {
            if chain.len() > MAX_EXTENDS_DEPTH || chain.iter().any(|seen| seen.id == base) {
                return Err("style_theme_invalid_base");
            }
            let base = self
                .style_themes
                .iter()
                .find(|candidate| candidate.id == base)
                .ok_or("style_theme_invalid_base")?;
            chain.push(base);
        }
        if chain.last().is_none_or(|root| root.id != DEFAULT_THEME_ID) {
            return Err("style_theme_invalid_base");
        }
        let mut resolved = theme.clone();
        resolved.material = chain
            .iter()
            .find_map(|link| link.material)
            .or(Some(Material::default()));
        for scheme in [Scheme::Light, Scheme::Dark] {
            let mut tokens = BTreeMap::new();
            // Root first, so each theme overrides the one it extends. Within a theme the
            // scheme's table wins over the shared one.
            for link in chain.iter().rev() {
                tokens.extend(link.css_vars.shared.0.clone());
                tokens.extend(scheme.tokens(&link.css_vars).0.clone());
            }
            substitute_references(&mut tokens)?;
            // A theme that states its own `focus-ring` keeps it; every other theme gets the
            // lightest mix of its text and page colors that still reads at 3:1.
            if !tokens.contains_key("focus-ring")
                && let Some(ring) = derive_focus_ring(&tokens)
            {
                tokens.insert("focus-ring".into(), ring);
            }
            *scheme.tokens_mut(&mut resolved.css_vars) = Tokens(tokens);
        }
        resolved.css_vars.shared = Tokens::default();
        // The touch table inherits along the same chain. A theme that states a few touch sizes
        // keeps the base theme's others, so a touch device never falls back to pointer sizes
        // that the base theme states differently.
        let mut touch = BTreeMap::new();
        for link in chain.iter().rev() {
            touch.extend(link.css_vars.touch.0.clone());
        }
        resolved.css_vars.touch = Tokens(touch);
        Ok(resolved)
    }

    /// Every theme with inherited tokens merged in, in display order.
    pub(crate) fn resolved(&self) -> Vec<StyleTheme> {
        self.style_themes
            .iter()
            .filter_map(|theme| self.resolve(theme).ok())
            .collect()
    }

    /// The user's accent applied to every table. The accent replaces `primary`, and `ring` and
    /// `primary-foreground` follow it: the foreground is whichever of black and white reads
    /// better on the accent, so a custom accent never leaves unreadable text on a button.
    pub(crate) fn with_accent(mut themes: Vec<StyleTheme>, accent: &str) -> Vec<StyleTheme> {
        let Some(rgb) = parse_hex_rgb(accent) else {
            return themes;
        };
        let foreground = if relative_luminance_of(rgb) >= 0.179 {
            "#000000"
        } else {
            "#ffffff"
        };
        for theme in &mut themes {
            for scheme in [Scheme::Light, Scheme::Dark] {
                let tokens = &mut scheme.tokens_mut(&mut theme.css_vars).0;
                tokens.insert("primary".into(), accent.to_owned());
                tokens.insert("primary-foreground".into(), foreground.to_owned());
                tokens.insert("accent-fill".into(), accent.to_owned());
            }
        }
        themes
    }

    /// Every theme and every palette-editor theme as the interface reads them: complete tables,
    /// bundled and file themes in display order, then editor themes by ID.
    pub(crate) fn resolved_with_custom(&self) -> Vec<StyleTheme> {
        let mut themes = self.resolved();
        themes.extend(
            self.custom_themes
                .iter()
                .filter_map(|theme| self.resolve_custom(theme)),
        );
        themes
    }

    /// The tokens an editor-managed theme produces: the default theme with the four palette
    /// colors replaced and the remaining surfaces derived from them.
    pub(crate) fn resolve_custom(&self, theme: &CustomTheme) -> Option<StyleTheme> {
        let root = self
            .style_themes
            .iter()
            .find(|candidate| candidate.id == DEFAULT_THEME_ID)?;
        let mut resolved = self.resolve(root).ok()?;
        resolved.id = theme.id.clone();
        resolved.name = theme.name.clone();
        resolved.names.clear();
        resolved.extends = None;
        resolved.order = DEFAULT_ORDER;
        for scheme in [Scheme::Light, Scheme::Dark] {
            let tokens = &mut scheme.tokens_mut(&mut resolved.css_vars).0;
            tokens.extend(derive_palette_tokens(scheme.palette(theme)));
            // The root's text-on-accent color was chosen for the root's accent. Without it the
            // interface computes a readable one from the accent in use.
            tokens.remove("primary-foreground");
            // The root's focus color was derived from the root's surfaces, not this palette's.
            tokens.remove("focus-ring");
            if let Some(ring) = derive_focus_ring(tokens) {
                tokens.insert("focus-ring".into(), ring);
            }
        }
        Some(resolved)
    }
}

#[derive(Clone, Copy)]
enum Scheme {
    Light,
    Dark,
}

impl Scheme {
    fn tokens(self, vars: &StyleCssVars) -> &Tokens {
        match self {
            Self::Light => &vars.light,
            Self::Dark => &vars.dark,
        }
    }

    fn tokens_mut(self, vars: &mut StyleCssVars) -> &mut Tokens {
        match self {
            Self::Light => &mut vars.light,
            Self::Dark => &mut vars.dark,
        }
    }

    fn palette(self, theme: &CustomTheme) -> &Palette {
        match self {
            Self::Light => &theme.light,
            Self::Dark => &theme.dark,
        }
    }
}

fn parse_hex_rgb(value: &str) -> Option<u32> {
    let hex = value.strip_prefix('#')?;
    if hex.len() != 6 || !hex.bytes().all(|byte| byte.is_ascii_hexdigit()) {
        return None;
    }
    u32::from_str_radix(hex, 16).ok()
}

/// WCAG relative luminance of an `0xRRGGBB` color.
fn relative_luminance_of(rgb: u32) -> f64 {
    let channel = |shift: u32| {
        let value = f64::from((rgb >> shift) & 0xff) / 255.0;
        if value <= 0.04045 {
            value / 12.92
        } else {
            ((value + 0.055) / 1.055).powf(2.4)
        }
    };
    0.2126 * channel(16) + 0.7152 * channel(8) + 0.0722 * channel(0)
}

/// The surfaces a focused control can sit on. The ring has to read on every one of them.
const FOCUS_SURFACES: [&str; 5] = ["background", "card", "panel", "sidebar", "menu-surface"];

/// WCAG 2.1 SC 1.4.11 asks 3:1 for what shows keyboard focus. The margin absorbs the rounding
/// of channels to whole numbers.
const FOCUS_CONTRAST: f64 = 3.1;

/// `color` (`#rrggbb` or `#rrggbbaa`) as the screen shows it over the opaque `background`.
fn composite_over(color: &str, background: u32) -> Option<u32> {
    let hex = color.strip_prefix('#')?;
    if !matches!(hex.len(), 6 | 8) || !hex.bytes().all(|byte| byte.is_ascii_hexdigit()) {
        return None;
    }
    let channel = |at: usize| u8::from_str_radix(&hex[at..at + 2], 16).ok().map(f64::from);
    let alpha = if hex.len() == 8 {
        channel(6)? / 255.0
    } else {
        1.0
    };
    let mut shown = 0u32;
    for (index, at) in [0, 2, 4].into_iter().enumerate() {
        let behind = f64::from((background >> (16 - 8 * index)) & 0xff);
        let mixed = (alpha * channel(at)? + (1.0 - alpha) * behind).round() as u32;
        shown = shown << 8 | mixed;
    }
    Some(shown)
}

fn contrast_between(first: u32, second: u32) -> f64 {
    let (first, second) = (relative_luminance_of(first), relative_luminance_of(second));
    (first.max(second) + 0.05) / (first.min(second) + 0.05)
}

/// The focus ring for one scheme: the mix of text and page color closest to the page that still
/// reaches [`FOCUS_CONTRAST`] on every surface a control sits on. It stays in the theme's own
/// hue, so a warm theme gets a warm ring, and it only gets as strong as it has to be. `None`
/// when the table lacks the colors to judge, in which case the theme draws no ring of its own.
fn derive_focus_ring(tokens: &BTreeMap<String, String>) -> Option<String> {
    let page = parse_hex_rgb(tokens.get("background")?)?;
    let text = composite_over(tokens.get("foreground")?, page)?;
    let surfaces: Vec<u32> = FOCUS_SURFACES
        .iter()
        .filter_map(|name| tokens.get(*name))
        .filter_map(|value| composite_over(value, page))
        .collect();
    let mix = |percent: u32| -> u32 {
        (0..3).fold(0, |shown, index| {
            let shift = 16 - 8 * index;
            let (from, to) = ((page >> shift) & 0xff, (text >> shift) & 0xff);
            let mixed = (from * (100 - percent) + to * percent + 50) / 100;
            shown << 8 | mixed
        })
    };
    let reads = |ring: u32| {
        surfaces
            .iter()
            .all(|surface| contrast_between(ring, *surface) >= FOCUS_CONTRAST)
    };
    // Not every theme can reach it with its own text color; the text color itself is the
    // strongest ring the theme can offer, so it is the answer then.
    let percent = (0..=100)
        .find(|percent| reads(mix(*percent)))
        .unwrap_or(100);
    Some(format!("#{:06x}", mix(percent)))
}

/// Surfaces derived from a palette editor theme's four colors. The alpha suffixes keep the
/// sidebar and panel translucent over the window material and the borders subtle.
fn derive_palette_tokens(palette: &Palette) -> BTreeMap<String, String> {
    BTreeMap::from([
        ("background".to_owned(), palette.background.clone()),
        ("card".to_owned(), palette.surface.clone()),
        ("foreground".to_owned(), palette.text.clone()),
        ("primary".to_owned(), palette.accent.clone()),
        ("muted-foreground".to_owned(), palette.text.clone()),
        ("border".to_owned(), format!("{}3d", palette.text)),
        ("sidebar".to_owned(), format!("{}b3", palette.surface)),
        ("panel".to_owned(), format!("{}ed", palette.background)),
        ("shell-border".to_owned(), format!("{}26", palette.text)),
        (
            "window-tint".to_owned(),
            format!("{}66", palette.background),
        ),
    ])
}

/// Replaces every `var(--name)` with the named token's value. A reference can point at another
/// reference, so values are rewritten until none are left or the depth limit is hit.
fn substitute_references(tokens: &mut BTreeMap<String, String>) -> Result<(), &'static str> {
    for _ in 0..MAX_EXTENDS_DEPTH {
        let snapshot = tokens.clone();
        let mut changed = false;
        for value in tokens.values_mut() {
            while let Some(start) = value.find("var(--") {
                let name_start = start + "var(--".len();
                let end = value[name_start..]
                    .find(')')
                    .ok_or("style_theme_invalid_reference")?
                    + name_start;
                let replacement = snapshot
                    .get(&value[name_start..end])
                    .ok_or("style_theme_invalid_reference")?;
                value.replace_range(start..=end, replacement);
                changed = true;
            }
        }
        if !changed {
            return Ok(());
        }
    }
    Err("style_theme_invalid_reference")
}

pub(crate) fn ensure_directory(directory: &Path) -> io::Result<()> {
    match fs::symlink_metadata(directory) {
        Ok(metadata) if metadata.is_dir() => Ok(()),
        Ok(_) => Err(io::Error::other(
            "The theme directory is not a regular directory",
        )),
        Err(error) if error.kind() == io::ErrorKind::NotFound => {
            fs::create_dir_all(directory)?;
            if fs::symlink_metadata(directory)?.is_dir() {
                Ok(())
            } else {
                Err(io::Error::other(
                    "The theme directory is not a regular directory",
                ))
            }
        }
        Err(error) => Err(error),
    }
}

pub(crate) fn save_custom_theme(directory: &Path, theme: &CustomTheme) -> Result<(), &'static str> {
    ensure_directory(directory).map_err(|_| "style_theme_write_failed")?;
    let path = editor_path(directory, &theme.id);
    let existing = check_editor_file(&path, &theme.id)?;
    let temporary = write_temp_file(directory, &ThemeFile::from_custom(theme))?;
    if existing {
        temporary
            .persist(path)
            .map_err(|_| "style_theme_write_failed")?;
    } else {
        temporary
            .persist_noclobber(path)
            .map_err(|_| "style_theme_write_failed")?;
    }
    Ok(())
}

pub(crate) fn clone_style(
    directory: &Path,
    style: StyleTheme,
    name: &str,
    id_is_used: impl Fn(&str) -> bool,
) -> Result<StyleTheme, &'static str> {
    ensure_directory(directory).map_err(|_| "style_theme_write_failed")?;
    let stem = style.id.rsplit('.').next().unwrap_or("style");
    // IDs were already validated as ASCII and the suffix leaves room for a counter.
    let stem = stem[..stem.len().min(40)].to_owned();
    let mut file = ThemeFile {
        schema_version: SCHEMA_VERSION,
        editor_managed: false,
        style,
    };
    file.style.name = name.to_owned();
    // The copy is a user theme: it has its own name and sits after the bundled themes.
    file.style.names.clear();
    file.style.order = DEFAULT_ORDER;
    for attempt in 1..=1024 {
        let id = if attempt == 1 {
            format!("user.{stem}-copy")
        } else {
            format!("user.{stem}-copy-{attempt}")
        };
        if id_is_used(&id) {
            continue;
        }
        let path = editor_path(directory, &id);
        match fs::symlink_metadata(&path) {
            Ok(_) => continue,
            Err(error) if error.kind() == io::ErrorKind::NotFound => {}
            Err(_) => return Err("style_theme_write_failed"),
        }
        file.style.id = id;
        let temporary = write_temp_file(directory, &file)?;
        match temporary.persist_noclobber(path) {
            Ok(_) => return Ok(file.style),
            Err(error) if error.error.kind() == io::ErrorKind::AlreadyExists => {}
            Err(_) => return Err("style_theme_write_failed"),
        }
    }
    Err("style_theme_id_exhausted")
}

fn write_temp_file(
    directory: &Path,
    theme: &ThemeFile,
) -> Result<tempfile::NamedTempFile, &'static str> {
    let text = serde_json::to_string_pretty(theme).map_err(|_| "style_theme_write_failed")?;
    let mut temporary = Builder::new()
        .prefix(".theme-")
        .tempfile_in(directory)
        .map_err(|_| "style_theme_write_failed")?;
    temporary
        .write_all(text.as_bytes())
        .and_then(|_| temporary.write_all(b"\n"))
        .and_then(|_| temporary.as_file().sync_all())
        .map_err(|_| "style_theme_write_failed")?;
    Ok(temporary)
}

pub(crate) fn remove_custom_theme(directory: &Path, id: &str) -> Result<(), &'static str> {
    match fs::symlink_metadata(directory) {
        Ok(metadata) if metadata.is_dir() => {}
        Err(error) if error.kind() == io::ErrorKind::NotFound => return Ok(()),
        _ => return Err("style_theme_write_failed"),
    }
    let path = editor_path(directory, id);
    // A disappeared theme has already been removed. A replaced file still
    // belongs to its author, so an old editor action cannot delete it.
    if check_editor_file(&path, id)? {
        fs::remove_file(path).map_err(|_| "style_theme_write_failed")?;
    }
    Ok(())
}

fn check_editor_file(path: &Path, id: &str) -> Result<bool, &'static str> {
    match fs::symlink_metadata(path) {
        Ok(_) => {
            let file = read_file(path).map_err(|_| "style_theme_read_only")?;
            if file.style.id == id && file.custom_theme().is_some() {
                Ok(true)
            } else {
                Err("style_theme_read_only")
            }
        }
        Err(error) if error.kind() == io::ErrorKind::NotFound => Ok(false),
        Err(_) => Err("style_theme_write_failed"),
    }
}

fn read_file(path: &Path) -> Result<ThemeFile, &'static str> {
    let metadata = fs::symlink_metadata(path).map_err(|_| "style_theme_read_failed")?;
    if !metadata.is_file() {
        return Err("style_theme_not_regular_file");
    }
    if metadata.len() > MAX_FILE_BYTES {
        return Err("style_theme_file_too_large");
    }
    let mut bytes = Vec::new();
    File::open(path)
        .and_then(|file| file.take(MAX_FILE_BYTES + 1).read_to_end(&mut bytes))
        .map_err(|_| "style_theme_read_failed")?;
    if bytes.len() as u64 > MAX_FILE_BYTES {
        return Err("style_theme_file_too_large");
    }
    let text = std::str::from_utf8(&bytes).map_err(|_| "style_theme_invalid_json")?;
    ThemeFile::parse(text, false)
}

pub(crate) fn editor_path(directory: &Path, id: &str) -> PathBuf {
    directory.join(format!("{id}.json"))
}

/// Bundled themes use a plain ID (`apple-glass`) or `builtin.<name>`. Files in the themes folder
/// must use `user.` or `custom-`, so a local file cannot take the place of a bundled theme.
pub(crate) fn is_style_id(id: &str, bundled: bool) -> bool {
    if id.len() > 64 {
        return false;
    }
    if bundled {
        return valid_id_parts(id.strip_prefix("builtin.").unwrap_or(id));
    }
    id.strip_prefix("user.")
        .or_else(|| id.strip_prefix("custom-"))
        .is_some_and(valid_id_parts)
}

fn is_theme_ref(id: &str) -> bool {
    is_style_id(id, true) || is_style_id(id, false)
}

fn valid_locale_tag(tag: &str) -> bool {
    !tag.is_empty()
        && tag.len() <= 16
        && tag
            .bytes()
            .all(|byte| byte.is_ascii_alphanumeric() || byte == b'-')
}

/// What a token's value may be, decided by the token's name family, so a new token of an
/// existing family needs no code. Names outside every family are checked by the value's shape.
#[derive(Clone, Copy)]
enum Family {
    /// Whole points from 0 to the limit.
    Length(u64),
    /// Whole points from minus the limit to the limit: an offset that may point inward.
    Offset(i64),
    /// A decimal number from 0 to the limit, such as a scale factor or an opacity.
    Ratio(f64),
    /// Anything the shape check accepts: colors, CSS functions and shadow lists.
    Free,
}

/// Name prefixes that hold numbers, with the largest value each may take. Metrics live under
/// `size-`, `space-`, `font-`, `layout-`, `motion-` and `feedback-` so they cannot collide with shadcn's
/// color tokens (`card`, `card-foreground`, `field`, `hover`). The limits keep a theme file from
/// producing an unusable interface (a 400-point button) while leaving every real design
/// system's values well inside them.
const NUMERIC_FAMILIES: &[(&str, Family)] = &[
    ("radius", Family::Length(32)),
    ("size-", Family::Length(64)),
    ("space-", Family::Length(48)),
    ("font-", Family::Length(32)),
    ("focus-width", Family::Length(8)),
    ("focus-offset", Family::Offset(8)),
    ("layout-", Family::Length(1200)),
    ("motion-", Family::Length(1000)),
    ("feedback-", Family::Ratio(1.0)),
];

fn family_of(name: &str) -> Family {
    NUMERIC_FAMILIES
        .iter()
        .find(|(prefix, _)| {
            // A prefix ending in `-` names a family (`control-height`); one without is also a
            // token of its own (`radius`). `card` and `field` are colors and match neither.
            if prefix.ends_with('-') {
                name.starts_with(prefix)
            } else {
                name == *prefix || name.starts_with(&format!("{prefix}-"))
            }
        })
        .map_or(Family::Free, |(_, family)| *family)
}

/// Checks one token. Numeric families are checked by name, everything else by the shape of the
/// value: a hex color, a CSS function, or a space or comma separated list.
fn validate_token(name: &str, value: &str) -> Result<(), &'static str> {
    let name_ok = name.len() <= 48
        && name.as_bytes().first().is_some_and(u8::is_ascii_lowercase)
        && name
            .bytes()
            .all(|byte| byte.is_ascii_lowercase() || byte.is_ascii_digit() || byte == b'-');
    if !name_ok || value.is_empty() || value.len() > MAX_TOKEN_VALUE_LEN {
        return Err("style_theme_invalid_token");
    }
    match family_of(name) {
        Family::Length(limit) => {
            return match value.parse::<u64>() {
                Ok(points) if points <= limit => Ok(()),
                _ => Err(if name.starts_with("radius") {
                    "style_theme_invalid_radius"
                } else {
                    "style_theme_invalid_metric"
                }),
            };
        }
        Family::Offset(limit) => {
            return match value.parse::<i64>() {
                Ok(points) if points.abs() <= limit => Ok(()),
                _ => Err("style_theme_invalid_metric"),
            };
        }
        Family::Ratio(limit) => {
            return match value.parse::<f64>() {
                Ok(ratio) if ratio.is_finite() && (0.0..=limit).contains(&ratio) => Ok(()),
                _ => Err("style_theme_invalid_metric"),
            };
        }
        Family::Free => {}
    }
    if value.starts_with('#') {
        return if is_hex_color(value) {
            Ok(())
        } else {
            Err("style_theme_invalid_color")
        };
    }
    // CSS's two "nothing" keywords are valid values: a finish with no gradient or shadow, and a
    // transparent fill. Any other bare word is a named color, which resolved tokens never use.
    if matches!(value, "none" | "transparent") {
        return Ok(());
    }
    if !value.contains(['(', ' ']) {
        return Err("style_theme_invalid_color");
    }
    let mut depth = 0i32;
    for byte in value.bytes() {
        match byte {
            b'(' => depth += 1,
            b')' => depth -= 1,
            b if b.is_ascii_alphanumeric() => {}
            b' ' | b'#' | b'%' | b'.' | b',' | b'-' | b'+' | b'/' | b'_' => {}
            _ => return Err("style_theme_invalid_token"),
        }
        if depth < 0 {
            return Err("style_theme_invalid_token");
        }
    }
    if depth == 0 {
        Ok(())
    } else {
        Err("style_theme_invalid_token")
    }
}

fn valid_id_parts(suffix: &str) -> bool {
    suffix.split('.').all(|part| {
        part.as_bytes()
            .first()
            .is_some_and(u8::is_ascii_alphanumeric)
            && part
                .bytes()
                .all(|byte| byte.is_ascii_lowercase() || byte.is_ascii_digit() || byte == b'-')
    })
}

pub(crate) fn valid_theme_name(name: &str) -> bool {
    !name.trim().is_empty() && name.chars().count() <= 80 && !name.chars().any(char::is_control)
}

fn is_hex_color(value: &str) -> bool {
    value.strip_prefix('#').is_some_and(|hex| {
        matches!(hex.len(), 6 | 8) && hex.bytes().all(|byte| byte.is_ascii_hexdigit())
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn tokens(theme: &StyleTheme, scheme: Scheme) -> &BTreeMap<String, String> {
        &scheme.tokens(&theme.css_vars).0
    }

    fn resolved(scan: &StyleThemeScan, id: &str) -> StyleTheme {
        scan.resolved()
            .into_iter()
            .find(|theme| theme.id == id)
            .unwrap_or_else(|| panic!("theme {id} is not resolvable"))
    }

    #[test]
    fn bundled_themes_are_valid_and_resolve_completely() {
        // A defect in a shipped file must fail the build, not hide a theme at run time.
        for text in BUNDLED_THEMES {
            ThemeFile::parse(text, true).expect("bundled theme file is valid");
        }
        let scan = StyleThemeScan::load(None);
        assert!(scan.issues.is_empty(), "{:?}", scan.issues);
        let root = resolved(&scan, DEFAULT_THEME_ID);
        let required: Vec<_> = tokens(&root, Scheme::Light).keys().cloned().collect();
        for theme in scan.resolved() {
            for scheme in [Scheme::Light, Scheme::Dark] {
                let table = tokens(&theme, scheme);
                for name in &required {
                    assert!(table.contains_key(name), "{} lacks {name}", theme.id);
                }
                assert!(
                    table.values().all(|value| !value.contains("var(")),
                    "{} keeps an unresolved reference",
                    theme.id
                );
            }
        }
    }

    /// WCAG 2.1 contrast of `foreground` over `background`. `foreground` may carry an alpha
    /// channel, which is composited over `background` first, as the screen does.
    fn contrast(foreground: &str, background: &str) -> f64 {
        let rgba = |hex: &str| {
            let hex = hex.strip_prefix('#').expect("a hex color");
            let channel = |at: usize| f64::from(u8::from_str_radix(&hex[at..at + 2], 16).unwrap());
            let alpha = if hex.len() == 8 {
                channel(6) / 255.0
            } else {
                1.0
            };
            ([channel(0), channel(2), channel(4)], alpha)
        };
        let (back, _) = rgba(background);
        let (front, alpha) = rgba(foreground);
        let shown: [f64; 3] = std::array::from_fn(|i| alpha * front[i] + (1.0 - alpha) * back[i]);
        let rgb = |c: [f64; 3]| {
            relative_luminance_of(
                (c[0].round() as u32) << 16 | (c[1].round() as u32) << 8 | c[2].round() as u32,
            )
        };
        let (a, b) = (rgb(shown), rgb(back));
        (a.max(b) + 0.05) / (a.min(b) + 0.05)
    }

    /// WCAG 2.1 SC 1.4.11: the part of a control that shows keyboard focus needs 3:1 against
    /// every surface a control sits on. Controls here sit on the page and on cards.
    #[test]
    fn every_bundled_focus_ring_reaches_three_to_one_on_every_surface() {
        let scan = StyleThemeScan::load(None);
        let mut failures = Vec::new();
        for theme in scan.resolved() {
            for scheme in [Scheme::Light, Scheme::Dark] {
                let table = tokens(&theme, scheme);
                let ring = &table["focus-ring"];
                for surface in ["background", "card", "panel"] {
                    let Some(color) = table.get(surface).filter(|c| c.starts_with('#')) else {
                        continue;
                    };
                    // A translucent surface is judged over the page behind it.
                    let ratio = contrast(ring, &table["background"])
                        .min(contrast(ring, &composite(color, &table["background"])));
                    if ratio < 3.0 {
                        failures.push(format!(
                            "{} {} focus-ring {ring} on {surface} {color}: {ratio:.2}:1",
                            theme.id,
                            if matches!(scheme, Scheme::Light) {
                                "light"
                            } else {
                                "dark"
                            },
                        ));
                    }
                }
            }
        }
        assert!(failures.is_empty(), "\n{}", failures.join("\n"));
    }

    /// `color` composited over an opaque `background`, as an opaque `#rrggbb`.
    fn composite(color: &str, background: &str) -> String {
        let hex = color.strip_prefix('#').unwrap();
        if hex.len() == 6 {
            return color.to_owned();
        }
        let back = background.strip_prefix('#').unwrap();
        let at = |s: &str, i: usize| f64::from(u8::from_str_radix(&s[i..i + 2], 16).unwrap());
        let alpha = at(hex, 6) / 255.0;
        let mix = |i: usize| (alpha * at(hex, i) + (1.0 - alpha) * at(back, i)).round() as u8;
        format!("#{:02x}{:02x}{:02x}", mix(0), mix(2), mix(4))
    }

    #[test]
    fn themes_are_listed_by_order_and_the_default_comes_first() {
        let scan = StyleThemeScan::load(None);
        let ids: Vec<_> = scan
            .style_themes
            .iter()
            .map(|theme| theme.id.as_str())
            .collect();
        assert_eq!(ids.first(), Some(&DEFAULT_THEME_ID));
        let orders: Vec<_> = scan.style_themes.iter().map(|theme| theme.order).collect();
        assert!(orders.windows(2).all(|pair| pair[0] <= pair[1]));
    }

    #[test]
    fn a_theme_inherits_missing_tokens_and_the_material_from_its_base() {
        let scan = StyleThemeScan::load(None);
        let primer = resolved(&scan, "builtin.primer");
        let root = resolved(&scan, DEFAULT_THEME_ID);
        assert_eq!(tokens(&primer, Scheme::Light)["card"], "#FFFFFF");
        assert_eq!(tokens(&primer, Scheme::Light)["primary"], "#0969DA");
        assert_eq!(
            tokens(&primer, Scheme::Light)["item-shadow"],
            tokens(&root, Scheme::Light)["item-shadow"]
        );
        assert_eq!(primer.material, Some(Material::Solid));
        assert_eq!(
            resolved(&scan, "apple-glass").material,
            Some(Material::Glass)
        );
    }

    #[test]
    fn the_gallery_theme_keeps_its_tokens_and_credits_its_source() {
        let scan = StyleThemeScan::load(None);
        let gallery = resolved(&scan, "builtin.nsh-gallery");
        let source = gallery
            .source
            .as_ref()
            .expect("a vendored theme names its source");
        assert_eq!(source.license, "Apache-2.0");
        assert_eq!(source.revision.len(), 40);
        let light = tokens(&gallery, Scheme::Light);
        // Upstream values pass through unchanged, including CSS functions NSH resolves itself.
        assert_eq!(light["primary"], "#c6ef6a");
        assert!(light["scrollbar-thumb"].starts_with("color-mix("));
        // Pixel lengths become points, and mapped roles resolve their references.
        assert_eq!(light["radius"], "10");
        assert_eq!(light["shell-border"], light["line"]);
        // Tokens upstream does not define come from the root theme; ones it defines replace them.
        assert_eq!(light["scrim"], "rgba(0,0,0,0.45)");
        // Gallery's own sizes: 28 point buttons, 32 point fields, a 3 point focus halo.
        assert_eq!(light["size-button"], "28");
        assert_eq!(light["size-field"], "32");
        assert_eq!(light["focus-width"], "3");
        assert_eq!(light["feedback-press-scale"], "0.97");
        assert_eq!(
            light["scrollbar-thumb-hover"],
            tokens(&gallery, Scheme::Light)["scrollbar-thumb-hover"]
        );
        assert_eq!(light["item-hover"], light["hover"]);
        // A role the map defines wins over the root and is resolved against the upstream table.
        assert_eq!(light["item-selected"], light["sidebar-accent"]);
        // Dark inherits what upstream only states for light, as in CSS.
        assert_eq!(
            tokens(&gallery, Scheme::Dark)["sidebar-primary"],
            tokens(&gallery, Scheme::Dark)["primary"]
        );
    }

    #[test]
    fn references_are_substituted_through_a_chain() {
        let directory = tempfile::tempdir().unwrap();
        let theme = r##"{"schemaVersion":1,"id":"user.refs","name":"Refs","cssVars":{
            "light":{"card":"#FFFFFF","raised":"var(--card)","chip":"var(--raised)",
                     "hover":"color-mix(in srgb, var(--foreground) 5%, transparent)"}}}"##;
        fs::write(directory.path().join("refs.json"), theme).unwrap();
        let scan = StyleThemeScan::load(Some(directory.path()));
        let light = resolved(&scan, "user.refs");
        let light = tokens(&light, Scheme::Light);
        assert_eq!(light["chip"], "#FFFFFF");
        assert!(light["hover"].contains(&light["foreground"]));
    }

    #[test]
    fn broken_references_and_bases_skip_only_the_affected_theme() {
        let directory = tempfile::tempdir().unwrap();
        let write = |name: &str, text: &str| fs::write(directory.path().join(name), text).unwrap();
        write(
            "dangling.json",
            r#"{"schemaVersion":1,"id":"user.dangling","name":"D","cssVars":{"light":{"card":"var(--missing)"}}}"#,
        );
        write(
            "missing-base.json",
            r#"{"schemaVersion":1,"id":"user.orphan","name":"O","extends":"user.nowhere"}"#,
        );
        write(
            "self.json",
            r#"{"schemaVersion":1,"id":"user.self","name":"S","extends":"user.self"}"#,
        );
        write(
            "cycle-a.json",
            r#"{"schemaVersion":1,"id":"user.a","name":"A","extends":"user.b"}"#,
        );
        write(
            "cycle-b.json",
            r#"{"schemaVersion":1,"id":"user.b","name":"B","extends":"user.a"}"#,
        );
        write(
            "child.json",
            r#"{"schemaVersion":1,"id":"user.child","name":"C","extends":"user.orphan"}"#,
        );
        write(
            "fine.json",
            r#"{"schemaVersion":1,"id":"user.fine","name":"F","extends":"builtin.primer"}"#,
        );
        let scan = StyleThemeScan::load(Some(directory.path()));
        let ids: Vec<_> = scan.resolved().into_iter().map(|theme| theme.id).collect();
        assert!(ids.contains(&"user.fine".to_owned()));
        for skipped in [
            "user.dangling",
            "user.orphan",
            "user.self",
            "user.a",
            "user.b",
            "user.child",
        ] {
            assert!(
                !ids.contains(&skipped.to_owned()),
                "{skipped} should be skipped"
            );
        }
        let mut codes: Vec<_> = scan
            .issues
            .iter()
            .map(|issue| (issue.file_name.as_str(), issue.code))
            .collect();
        codes.sort();
        assert_eq!(
            codes,
            [
                ("child.json", "style_theme_invalid_base"),
                ("cycle-a.json", "style_theme_invalid_base"),
                ("cycle-b.json", "style_theme_invalid_base"),
                ("dangling.json", "style_theme_invalid_reference"),
                ("missing-base.json", "style_theme_invalid_base"),
                ("self.json", "style_theme_invalid_base"),
            ]
        );
    }

    #[test]
    fn invalid_files_and_duplicates_do_not_hide_other_styles() {
        let directory = tempfile::tempdir().unwrap();
        let valid = r##"{"schemaVersion":1,"id":"user.example","name":"Example","extends":"apple-glass","cssVars":{"light":{"sidebar":"#EFEDE9B3","radius":16}}}"##;
        for (name, contents) in [
            ("a.json", valid.to_owned()),
            ("b.json", valid.to_owned()),
            ("bad.json", "not JSON".to_owned()),
            ("bad-color.json", valid.replace("#EFEDE9B3", "red")),
            ("bad-id.json", valid.replace("user.example", "user.abc/def")),
            ("bad-radius.json", valid.replace("16", "33")),
            ("bad-token.json", valid.replace("\"radius\"", "\"Radius!\"")),
            (
                "bundled-id.json",
                valid.replace("user.example", "builtin.fake"),
            ),
            (
                "future.json",
                valid.replace("\"schemaVersion\":1", "\"schemaVersion\":2"),
            ),
        ] {
            fs::write(directory.path().join(name), contents).unwrap();
        }
        let scan = StyleThemeScan::load(Some(directory.path()));
        let style = resolved(&scan, "user.example");
        assert_eq!(style.material, Some(Material::Glass));
        assert_eq!(tokens(&style, Scheme::Light)["sidebar"], "#EFEDE9B3");
        assert_eq!(tokens(&style, Scheme::Light)["radius"], "16");
        let glass = resolved(&scan, "apple-glass");
        assert_eq!(tokens(&style, Scheme::Dark), tokens(&glass, Scheme::Dark));
        let mut issues: Vec<_> = scan
            .issues
            .iter()
            .map(|issue| (issue.file_name.as_str(), issue.code))
            .collect();
        issues.sort();
        assert_eq!(
            issues,
            [
                ("b.json", "style_theme_duplicate_id"),
                ("bad-color.json", "style_theme_invalid_color"),
                ("bad-id.json", "style_theme_invalid_id"),
                ("bad-radius.json", "style_theme_invalid_radius"),
                ("bad-token.json", "style_theme_invalid_token"),
                ("bad.json", "style_theme_invalid_json"),
                ("bundled-id.json", "style_theme_invalid_id"),
                ("future.json", "style_theme_unsupported_version"),
            ]
        );
    }

    #[test]
    fn token_values_are_checked_by_shape() {
        for (name, value, ok) in [
            ("brand", "#c6ef6a", true),
            ("brand", "#c6ef6a80", true),
            ("brand", "#c6ef6", false),
            (
                "hover",
                "color-mix(in srgb, var(--foreground) 5%, transparent)",
                true,
            ),
            ("scrim", "rgba(0,0,0,0.45)", true),
            ("shadow", "0 1px 2px #0000001a, inset 0 1px 0 #ffffff", true),
            ("radius", "10", true),
            ("radius-card", "33", false),
            ("radius", "10px", false),
            ("brand", "red", false),
            ("finish-raised-image", "none", true),
            ("finish-raised-fill", "transparent", true),
            (
                "finish-raised-image",
                "linear-gradient(180deg, #ffffff 0%, #e9ebf0 100%)",
                true,
            ),
            (
                "finish-raised-shadow",
                "inset 0 1px 0 #ffffff, 0 1px 0 rgba(10,15,25,0.05)",
                true,
            ),
            ("finish-raised-shadow", "0 1px 0 #fff; drop", false),
            ("brand", "url(x)", true),
            ("brand", "rgb(0,0,0", false),
            ("brand", "a;b(", false),
            ("Brand", "#ffffff", false),
            ("brand", "", false),
        ] {
            let result = validate_token(name, value);
            // `url(` passes the shape check: references to files are not interpreted by the app.
            assert_eq!(result.is_ok(), ok, "{name}: {value}");
        }
    }

    #[test]
    fn metrics_are_validated_by_family_and_range() {
        for (name, value, ok) in [
            ("size-button", "28", true),
            ("size-button", "65", false),
            ("size-button", "-1", false),
            ("size-button", "28.5", false),
            ("space-card", "24", true),
            ("font-body", "33", false),
            ("focus-offset", "-2", true),
            ("focus-offset", "9", false),
            ("feedback-press-scale", "0.97", true),
            ("feedback-press-scale", "1.5", false),
            ("feedback-press-scale", "fast", false),
            ("motion-press", "150", true),
            ("motion-press", "1001", false),
            ("layout-menu-width", "240", true),
            ("layout-menu-width", "1201", false),
            // Colors share a prefix with no family: `card` and `field` stay color tokens.
            ("card", "#ffffff", true),
            ("card-foreground", "#101010", true),
            ("field", "#ffffff", true),
            ("hover", "color-mix(in srgb, #000000 5%, transparent)", true),
        ] {
            assert_eq!(validate_token(name, value).is_ok(), ok, "{name}: {value}");
        }
    }

    #[test]
    fn shared_tokens_reach_both_schemes_and_a_scheme_value_wins() {
        let directory = tempfile::tempdir().unwrap();
        let theme = r##"{"schemaVersion":1,"id":"user.metrics","name":"Metrics","cssVars":{
            "shared":{"size-button":40,"focus-offset":-2,"feedback-press-scale":0.95},
            "dark":{"size-button":44}}}"##;
        fs::write(directory.path().join("metrics.json"), theme).unwrap();
        let scan = StyleThemeScan::load(Some(directory.path()));
        let style = resolved(&scan, "user.metrics");
        assert_eq!(tokens(&style, Scheme::Light)["size-button"], "40");
        assert_eq!(tokens(&style, Scheme::Dark)["size-button"], "44");
        assert_eq!(tokens(&style, Scheme::Light)["focus-offset"], "-2");
        assert_eq!(
            tokens(&style, Scheme::Light)["feedback-press-scale"],
            "0.95"
        );
        // A theme that states some metrics keeps the root's others.
        let root = resolved(&scan, DEFAULT_THEME_ID);
        assert_eq!(
            tokens(&style, Scheme::Light)["size-field"],
            tokens(&root, Scheme::Light)["size-field"]
        );
        // The shared section is merged away: consumers read one table per scheme.
        assert!(style.css_vars.shared.is_empty());
    }

    #[test]
    fn bundled_design_systems_differ_in_size_and_not_only_in_color() {
        let scan = StyleThemeScan::load(None);
        let height = |id: &str| tokens(&resolved(&scan, id), Scheme::Light)["size-button"].clone();
        assert_eq!(height(DEFAULT_THEME_ID), "28");
        assert_eq!(height("builtin.primer"), "32");
        assert_eq!(height("builtin.slate"), "36");
        // A copy of a theme keeps its metrics: the file round-trips numbers as numbers.
        let slate = scan
            .style_themes
            .iter()
            .find(|theme| theme.id == "builtin.slate")
            .unwrap();
        let text = serde_json::to_string(slate).unwrap();
        assert!(text.contains(r#""size-button":36"#), "{text}");
    }

    #[test]
    fn the_accent_preference_recolors_the_theme_and_keeps_its_text_readable() {
        let scan = StyleThemeScan::load(None);
        let themes = StyleThemeScan::with_accent(scan.resolved(), "#ffff00");
        let first = &themes[0];
        assert_eq!(tokens(first, Scheme::Light)["primary"], "#ffff00");
        // Yellow is light, so black reads better on it than white.
        assert_eq!(
            tokens(first, Scheme::Light)["primary-foreground"],
            "#000000"
        );
        let themes = StyleThemeScan::with_accent(scan.resolved(), "#102040");
        assert_eq!(
            tokens(&themes[0], Scheme::Dark)["primary-foreground"],
            "#ffffff"
        );
        // An invalid accent is ignored rather than corrupting the tables.
        let unchanged = StyleThemeScan::with_accent(scan.resolved(), "red");
        assert_eq!(unchanged, scan.resolved());
    }

    #[test]
    fn editor_themes_resolve_with_derived_surfaces() {
        let scan = StyleThemeScan::load(None);
        let custom = CustomTheme {
            id: "custom-ocean".into(),
            name: "Ocean".into(),
            light: Palette {
                background: "#FFFFFF".into(),
                surface: "#F0F0F0".into(),
                text: "#101010".into(),
                accent: "#0066CC".into(),
            },
            dark: Palette {
                background: "#101010".into(),
                surface: "#202020".into(),
                text: "#F0F0F0".into(),
                accent: "#66AAFF".into(),
            },
        };
        let theme = scan.resolve_custom(&custom).unwrap();
        let light = tokens(&theme, Scheme::Light);
        assert_eq!(light["primary"], "#0066CC");
        assert_eq!(light["sidebar"], "#F0F0F0b3");
        assert!(!light.contains_key("primary-foreground"));
        assert!(light.contains_key("item-selected"));
    }

    #[cfg(unix)]
    #[test]
    fn theme_scan_does_not_follow_symbolic_links() {
        use std::os::unix::fs::symlink;

        let directory = tempfile::tempdir().unwrap();
        let outside = tempfile::tempdir().unwrap();
        let file = outside.path().join("theme.json");
        fs::write(
            &file,
            r#"{"schemaVersion":1,"id":"user.link","name":"Link","cssVars":{}}"#,
        )
        .unwrap();
        symlink(file, directory.path().join("link.json")).unwrap();
        let scan = StyleThemeScan::load(Some(directory.path()));
        assert_eq!(scan.style_themes.len(), BUNDLED_THEMES.len());
        assert_eq!(scan.issues[0].code, "style_theme_not_regular_file");
    }
}
