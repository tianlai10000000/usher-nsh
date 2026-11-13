//! Stable severity values and filtering, independent of output backends.
use std::str::FromStr;
/// Ordered from most to least severe; `Off` records nothing.
#[derive(Clone, Copy, Debug, Default, Eq, Ord, PartialEq, PartialOrd)]
pub enum Level {
    Off,
    Error,
    Warn,
    #[default]
    Info,
    Debug,
    Trace,
}

impl Level {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Off => "OFF",
            Self::Error => "ERROR",
            Self::Warn => "WARN",
            Self::Info => "INFO",
            Self::Debug => "DEBUG",
            Self::Trace => "TRACE",
        }
    }

    pub(crate) fn from_u8(value: u8) -> Self {
        match value {
            0 => Self::Off,
            1 => Self::Error,
            2 => Self::Warn,
            3 => Self::Info,
            4 => Self::Debug,
            _ => Self::Trace,
        }
    }
}

impl FromStr for Level {
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

/// Whether a record at `record` passes a filter set to `filter`.
pub fn enabled(filter: Level, record: Level) -> bool {
    record != Level::Off && record <= filter
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn filter_keeps_the_chosen_level_and_more_severe_ones() {
        assert!(enabled(Level::Info, Level::Error));
        assert!(enabled(Level::Info, Level::Info));
        assert!(!enabled(Level::Info, Level::Debug));
        assert!(!enabled(Level::Off, Level::Error));
        assert!(enabled(Level::Trace, Level::Trace));
        assert!("warn".parse::<Level>().is_ok());
        assert!("loud".parse::<Level>().is_err());
    }
}
