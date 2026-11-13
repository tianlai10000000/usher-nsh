//! Maintains daily file buckets, shards and shared retention limits.
use chrono::NaiveDate;
use std::{
    fs,
    path::{Path, PathBuf},
};
pub(crate) fn day_directory(root: &Path, day: NaiveDate) -> PathBuf {
    root.join(day.format("%Y-%m-%d").to_string())
}

fn parse_day(name: &str) -> Option<NaiveDate> {
    NaiveDate::parse_from_str(name, "%Y-%m-%d").ok()
}

/// The shard a write of `incoming` bytes goes to, rolling to `usher.1.log`, ... as files fill.
#[cfg(test)]
pub(crate) fn shard_path(directory: &Path, incoming: u64) -> PathBuf {
    stream_shard_path(
        directory,
        crate::config::policy().log_file.as_str(),
        incoming,
    )
}

pub(crate) fn stream_shard_path(directory: &Path, filename: &str, incoming: u64) -> PathBuf {
    let mut index = 0u32;
    loop {
        let name = if index == 0 {
            filename.to_owned()
        } else {
            format!("{}.{index}.log", filename.trim_end_matches(".log"))
        };
        let path = directory.join(name);
        match fs::metadata(&path) {
            Ok(meta)
                if meta.len() > 0
                    && meta.len() + incoming > crate::config::policy().max_shard_bytes =>
            {
                index += 1
            }
            _ => return path,
        }
    }
}

/// Bucket directories sorted oldest first, ignoring anything that is not a date.
pub(crate) fn list_days(root: &Path) -> Vec<(NaiveDate, PathBuf)> {
    let mut days: Vec<_> = fs::read_dir(root)
        .into_iter()
        .flatten()
        .flatten()
        .filter(|entry| entry.file_type().is_ok_and(|kind| kind.is_dir()))
        .filter_map(|entry| {
            let day = parse_day(entry.file_name().to_str()?)?;
            Some((day, entry.path()))
        })
        .collect();
    days.sort();
    days
}

fn directory_size(path: &Path) -> u64 {
    fs::read_dir(path)
        .into_iter()
        .flatten()
        .flatten()
        .filter_map(|entry| {
            let meta = entry.metadata().ok()?;
            meta.is_file().then_some(meta.len())
        })
        .sum()
}

/// Removes days older than the retention window, then the oldest days until the quota holds.
/// The directory of `today` is never removed. Returns the days that were removed.
pub(crate) fn prune(
    root: &Path,
    today: NaiveDate,
    retention_days: i64,
    quota: u64,
) -> Vec<NaiveDate> {
    let mut removed = Vec::new();
    let mut days = list_days(root);
    days.retain(|(day, path)| {
        let expired = *day != today && (today - *day).num_days() >= retention_days;
        if expired && fs::remove_dir_all(path).is_ok() {
            removed.push(*day);
            return false;
        }
        true
    });
    let mut total: u64 = days.iter().map(|(_, path)| directory_size(path)).sum();
    for (day, path) in &days {
        if total <= quota {
            break;
        }
        if *day == today {
            continue;
        }
        let size = directory_size(path);
        if fs::remove_dir_all(path).is_ok() {
            total = total.saturating_sub(size);
            removed.push(*day);
        }
    }
    removed
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::test_support::date;
    use std::fs::File;
    #[test]
    fn full_shards_roll_to_numbered_files() {
        let dir = tempfile::tempdir().expect("tempdir");
        let first = dir.path().join(crate::config::policy().log_file.as_str());
        let file = File::create(&first).expect("create");
        file.set_len(crate::config::policy().max_shard_bytes - 10)
            .expect("grow");
        assert_eq!(shard_path(dir.path(), 5), first);
        assert_eq!(shard_path(dir.path(), 100), dir.path().join("usher.1.log"));
    }

    #[test]
    fn pruning_drops_expired_days_and_keeps_today() {
        let dir = tempfile::tempdir().expect("tempdir");
        let today = date(2026, 11, 10);
        for day in [date(2026, 10, 1), date(2026, 10, 28), date(2026, 11, 10)] {
            let directory = day_directory(dir.path(), day);
            fs::create_dir_all(&directory).expect("mkdir");
            fs::write(
                directory.join(crate::config::policy().log_file.as_str()),
                "x",
            )
            .expect("write");
        }
        fs::create_dir_all(dir.path().join("not-a-date")).expect("mkdir");
        let removed = prune(
            dir.path(),
            today,
            crate::config::policy().retention_days,
            crate::config::policy().disk_quota_bytes,
        );
        assert_eq!(removed, vec![date(2026, 10, 1)]);
        assert!(day_directory(dir.path(), date(2026, 10, 28)).exists());
        assert!(day_directory(dir.path(), today).exists());
        assert!(dir.path().join("not-a-date").exists());
    }

    #[test]
    fn pruning_enforces_the_quota_oldest_first_but_never_today() {
        let dir = tempfile::tempdir().expect("tempdir");
        let today = date(2026, 11, 10);
        for day in [date(2026, 11, 8), date(2026, 11, 9), today] {
            let directory = day_directory(dir.path(), day);
            fs::create_dir_all(&directory).expect("mkdir");
            fs::write(
                directory.join(crate::config::policy().log_file.as_str()),
                vec![b'x'; 100],
            )
            .expect("write");
        }
        let removed = prune(
            dir.path(),
            today,
            crate::config::policy().retention_days,
            150,
        );
        assert_eq!(removed, vec![date(2026, 11, 8), date(2026, 11, 9)]);
        assert!(day_directory(dir.path(), today).exists());
    }
}
