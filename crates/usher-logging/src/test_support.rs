//! Shared local fixtures for logging rule tests; never compiled into production.
use crate::files::day_directory;
use chrono::{DateTime, Local, NaiveDate, TimeZone};
use std::{fs, path::Path};
pub(crate) fn at(year: i32, month: u32, day: u32, hour: u32, minute: u32) -> DateTime<Local> {
    Local
        .with_ymd_and_hms(year, month, day, hour, minute, 0)
        .single()
        .expect("unambiguous local time")
}
pub(crate) fn date(year: i32, month: u32, day: u32) -> NaiveDate {
    NaiveDate::from_ymd_opt(year, month, day).expect("valid date")
}
pub(crate) fn read_log(root: &Path, day: NaiveDate) -> String {
    fs::read_to_string(day_directory(root, day).join(crate::config::policy().log_file.as_str()))
        .unwrap_or_default()
}
