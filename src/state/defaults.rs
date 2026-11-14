//! Shared serde defaults for enabled preference flags in the existing file format.
pub(crate) fn default_enabled() -> bool {
    true
}

pub(crate) fn is_true(value: &bool) -> bool {
    *value
}
