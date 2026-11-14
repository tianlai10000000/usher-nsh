//! Shared color parsing and WCAG calculations for theme validation and derived tokens.
pub(crate) fn parse_color(value: &str) -> Option<u32> {
    let hex = value.strip_prefix('#')?;
    if hex.len() != 6 || !hex.bytes().all(|byte| byte.is_ascii_hexdigit()) {
        return None;
    }
    u32::from_str_radix(hex, 16).ok()
}

pub(crate) fn contrast_ratio(first: u32, second: u32) -> f64 {
    let first = relative_luminance(first);
    let second = relative_luminance(second);
    (first.max(second) + 0.05) / (first.min(second) + 0.05)
}

pub(crate) fn relative_luminance(rgb: u32) -> f64 {
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

/// Chooses readable foreground ink using the established accent contrast boundary.
pub(crate) fn foreground(rgb: u32) -> &'static str {
    if relative_luminance(rgb) >= 0.179 {
        "#000000"
    } else {
        "#ffffff"
    }
}
