//! Persisted update-check preferences, separate from runtime update state.
use crate::state::defaults::default_enabled;
use serde::{Deserialize, Serialize};
/// `[updates]` is omitted while automatic checks retain the established enabled default.
#[derive(Deserialize, Serialize)]
#[serde(default)]
pub(crate) struct UpdatesSection {
    pub(crate) auto_check: bool,
}
impl Default for UpdatesSection {
    fn default() -> Self {
        Self {
            auto_check: default_enabled(),
        }
    }
}
impl UpdatesSection {
    pub(crate) fn is_default(&self) -> bool {
        self.auto_check == Self::default().auto_check
    }
}
