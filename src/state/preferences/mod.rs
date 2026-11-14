//! Language preference schema and validation.
use serde::{Deserialize, Serialize};
const SYSTEM_LOCALE: &str = "system";
#[derive(Deserialize, Serialize)]
#[serde(default, rename_all = "camelCase")]
pub(crate) struct General {
    pub(crate) locale_preference: String,
}

impl Default for General {
    fn default() -> Self {
        Self {
            locale_preference: SYSTEM_LOCALE.to_owned(),
        }
    }
}

impl General {
    pub(crate) fn is_default(&self) -> bool {
        self.locale_preference == SYSTEM_LOCALE
    }
}

pub(crate) fn validate_locale(locale: &str) -> Result<(), &'static str> {
    if matches!(locale, SYSTEM_LOCALE | "en" | "zh-Hans") {
        Ok(())
    } else {
        Err("invalid_locale")
    }
}
