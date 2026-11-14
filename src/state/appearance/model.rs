//! Appearance preferences, theme validation and persisted appearance slice.
use super::color::{contrast_ratio, parse_color};
use super::{
    background::{self, BackgroundImage},
    theme::{self as style_themes, DEFAULT_THEME_ID},
};
use crate::state::defaults::{default_enabled, is_true};
use serde::{Deserialize, Serialize};
use std::str::FromStr;
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

    pub(crate) fn is_default(&self) -> bool {
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
pub(crate) struct Appearance {
    pub(crate) color_mode: ColorMode,
    #[serde(default = "default_enabled", skip_serializing_if = "is_true")]
    pub(crate) show_status_bar: bool,
    #[serde(default = "default_enabled", skip_serializing_if = "is_true")]
    pub(crate) sidebar_docked: bool,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub(crate) background_image: Option<BackgroundImage>,
    #[serde(
        default = "background::default_opacity",
        skip_serializing_if = "background::is_default_opacity"
    )]
    pub(crate) background_image_opacity: u32,
    #[serde(
        default = "background::default_content_transparency",
        skip_serializing_if = "background::is_default_content_transparency"
    )]
    pub(crate) content_background_transparency: u32,
    #[serde(default, skip_serializing_if = "ComponentFamily::is_default")]
    pub(crate) component_family: ComponentFamily,
    #[serde(default = "default_theme_id", skip_serializing_if = "is_default_theme")]
    pub(crate) theme_id: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub(crate) accent: Option<String>,
}

pub(crate) fn default_theme_id() -> String {
    DEFAULT_THEME_ID.to_owned()
}

pub(crate) fn is_default_theme(id: &str) -> bool {
    id == DEFAULT_THEME_ID
}

pub(crate) fn is_theme_id(id: &str) -> bool {
    id.len() <= 64
        && id.split('.').all(|part| {
            !part.is_empty()
                && part
                    .bytes()
                    .all(|byte| byte.is_ascii_lowercase() || byte.is_ascii_digit() || byte == b'-')
        })
}

pub(crate) fn validate_custom_id(id: &str) -> Result<(), &'static str> {
    if style_themes::is_style_id(id, false) {
        Ok(())
    } else {
        Err("invalid_theme_id")
    }
}

pub(crate) fn validate_accent(accent: Option<&str>) -> Result<(), &'static str> {
    match accent {
        Some(value) if parse_color(value).is_none() => Err("invalid_accent"),
        _ => Ok(()),
    }
}
