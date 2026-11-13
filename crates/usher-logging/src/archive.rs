//! Creates bounded diagnostic archives and publishes completed exports.
use crate::files::list_days;
use chrono::{Local, NaiveDate};
use flate2::{Compression, write::GzEncoder};
use serde_json::json;
use std::{
    collections::BTreeMap,
    fs::{self, File},
    io::{self, Write},
    path::{Path, PathBuf},
    sync::atomic::{AtomicU64, Ordering},
};
#[derive(Debug, Default, Eq, PartialEq)]
pub struct ExportReport {
    pub files: usize,
    pub skipped_links: usize,
    pub truncated_files: usize,
    pub omitted_files: usize,
    pub bytes: u64,
}

#[derive(Debug, Eq, PartialEq)]
pub enum ExportError {
    Unsupported,
    InvalidRange,
    NothingToExport,
    Cancelled,
    Io,
}

impl ExportError {
    pub fn code(&self) -> &'static str {
        match self {
            Self::Unsupported => "diagnostics_export_unavailable",
            Self::InvalidRange => "diagnostics_export_invalid_range",
            Self::NothingToExport => "diagnostics_export_empty",
            Self::Cancelled => "diagnostics_export_cancelled",
            Self::Io => "diagnostics_export_failed",
        }
    }
}

pub(crate) struct ExportRequest<'a> {
    pub root: &'a Path,
    pub destination: &'a Path,
    pub from: NaiveDate,
    pub to: NaiveDate,
    pub app_version: &'a str,
    pub platform: &'a str,
    pub cancelled: &'a dyn Fn() -> bool,
}

/// Writes a `.tar.gz` of the regular log files in `[from, to]`. The archive is built in a
/// temporary file beside `destination` and published by rename, so a failure or cancel
/// leaves nothing behind.
pub(crate) fn export(request: &ExportRequest<'_>) -> Result<ExportReport, ExportError> {
    if request.from > request.to {
        return Err(ExportError::InvalidRange);
    }
    let parent = request.destination.parent().ok_or(ExportError::Io)?;
    let temporary = tempfile::Builder::new()
        .prefix(".usher-diagnostics-")
        .suffix(".tmp")
        .tempfile_in(parent)
        .map_err(|_| ExportError::Io)?;

    let mut report = ExportReport::default();
    let result = build_archive(request, temporary.as_file(), &mut report);
    // On any error the temporary file is deleted when it drops.
    result?;
    if report.files == 0 {
        return Err(ExportError::NothingToExport);
    }
    if (request.cancelled)() {
        return Err(ExportError::Cancelled);
    }
    temporary
        .persist(request.destination)
        .map_err(|_| ExportError::Io)?;
    Ok(report)
}

fn build_archive(
    request: &ExportRequest<'_>,
    file: &File,
    report: &mut ExportReport,
) -> Result<(), ExportError> {
    let encoder = GzEncoder::new(file, Compression::default());
    let mut archive = tar::Builder::new(encoder);
    archive.mode(tar::HeaderMode::Deterministic);

    let summary = json!({
        "appVersion": request.app_version,
        "platform": request.platform,
        "from": request.from.to_string(),
        "to": request.to.to_string(),
    })
    .to_string();
    append_bytes(&mut archive, "summary.json", summary.as_bytes())?;

    let mut remaining = crate::config::policy().export_total_limit;
    // Newest first: when the budget runs out it is the oldest logs that are left out.
    for (day, directory) in list_days(request.root).into_iter().rev() {
        if day < request.from || day > request.to {
            continue;
        }
        let mut names: BTreeMap<String, PathBuf> = BTreeMap::new();
        for entry in fs::read_dir(&directory)
            .map_err(|_| ExportError::Io)?
            .flatten()
        {
            let Ok(kind) = entry.file_type() else {
                continue;
            };
            // Symbolic links could point outside the log directory.
            if kind.is_symlink() {
                report.skipped_links += 1;
                continue;
            }
            if !kind.is_file() {
                continue;
            }
            if let Some(name) = entry.file_name().to_str()
                && name.starts_with("usher")
                && name.ends_with(".log")
            {
                names.insert(name.to_owned(), entry.path());
            }
        }
        for (name, path) in names.into_iter().rev() {
            if (request.cancelled)() {
                return Err(ExportError::Cancelled);
            }
            if remaining == 0 {
                report.omitted_files += 1;
                continue;
            }
            let limit = crate::config::policy().export_file_limit.min(remaining);
            let data = read_tail(&path, limit).map_err(|_| ExportError::Io)?;
            if data.truncated {
                report.truncated_files += 1;
            }
            let entry = format!("{day}/{name}");
            append_bytes(&mut archive, &entry, &data.bytes)?;
            remaining = remaining.saturating_sub(data.bytes.len() as u64);
            report.files += 1;
            report.bytes += data.bytes.len() as u64;
        }
    }
    let encoder = archive.into_inner().map_err(|_| ExportError::Io)?;
    encoder.finish().map_err(|_| ExportError::Io)?;
    Ok(())
}

struct Tail {
    bytes: Vec<u8>,
    truncated: bool,
}

/// Keeps the newest `limit` bytes, dropping the partial first line.
fn read_tail(path: &Path, limit: u64) -> io::Result<Tail> {
    use io::{Read, Seek, SeekFrom};
    let mut file = File::open(path)?;
    let length = file.metadata()?.len();
    if length <= limit {
        let mut bytes = Vec::with_capacity(length as usize);
        file.read_to_end(&mut bytes)?;
        return Ok(Tail {
            bytes,
            truncated: false,
        });
    }
    file.seek(SeekFrom::Start(length - limit))?;
    let mut bytes = Vec::with_capacity(limit as usize);
    file.take(limit).read_to_end(&mut bytes)?;
    if let Some(newline) = bytes.iter().position(|byte| *byte == b'\n') {
        bytes.drain(..=newline);
    }
    Ok(Tail {
        bytes,
        truncated: true,
    })
}

fn append_bytes<W: Write>(
    archive: &mut tar::Builder<W>,
    name: &str,
    bytes: &[u8],
) -> Result<(), ExportError> {
    let mut header = tar::Header::new_gnu();
    header.set_size(bytes.len() as u64);
    header.set_mode(0o644);
    header.set_mtime(0);
    archive
        .append_data(&mut header, name, bytes)
        .map_err(|_| ExportError::Io)
}

// --- Export folder --------------------------------------------------------------------------

static ARCHIVE_SEQUENCE: AtomicU64 = AtomicU64::new(0);

/// A unique, sortable name: time, process and a counter, so two exports in the same
/// millisecond never collide and name order is creation order.
pub(crate) fn archive_name() -> String {
    format!(
        "{}{}-{}-{:06}.tar.gz",
        crate::config::policy().archive_prefix,
        Local::now().format("%Y%m%d-%H%M%S-%3f"),
        std::process::id(),
        ARCHIVE_SEQUENCE.fetch_add(1, Ordering::Relaxed),
    )
}

/// Keeps the newest few archives, and always the one just written. Unrelated files,
/// directories and links in the folder are never touched.
pub(crate) fn prune_exports(folder: &Path, current: &Path) -> io::Result<()> {
    let mut archives: Vec<PathBuf> = fs::read_dir(folder)?
        .flatten()
        .filter(|entry| entry.file_type().is_ok_and(|kind| kind.is_file()))
        .filter(|entry| {
            entry.file_name().to_str().is_some_and(|name| {
                name.starts_with(&crate::config::policy().archive_prefix)
                    && name.ends_with(".tar.gz")
            })
        })
        .map(|entry| entry.path())
        .collect();
    archives.sort();
    let mut remaining = archives.len();
    for archive in archives {
        if remaining <= crate::config::policy().kept_archives {
            break;
        }
        if archive == current {
            continue;
        }
        fs::remove_file(&archive)?;
        remaining -= 1;
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{files::day_directory, test_support::date};
    const ARCHIVE_PREFIX: &str = "usher-diagnostics-";
    fn make_logs(root: &Path) {
        for (day, body) in [
            (date(2026, 11, 3), "old\n"),
            (date(2026, 11, 8), "recent\n"),
            (date(2026, 11, 10), "today\n"),
        ] {
            let directory = day_directory(root, day);
            fs::create_dir_all(&directory).expect("mkdir");
            fs::write(
                directory.join(crate::config::policy().log_file.as_str()),
                body,
            )
            .expect("write");
        }
    }

    fn entry_names(archive: &Path) -> Vec<String> {
        let decoder = flate2::read::GzDecoder::new(File::open(archive).expect("open"));
        tar::Archive::new(decoder)
            .entries()
            .expect("entries")
            .map(|entry| {
                entry
                    .expect("entry")
                    .path()
                    .expect("path")
                    .to_string_lossy()
                    .into_owned()
            })
            .collect()
    }

    #[test]
    fn export_covers_only_the_chosen_days_and_adds_a_summary() {
        let dir = tempfile::tempdir().expect("tempdir");
        let logs = dir.path().join("logs");
        make_logs(&logs);
        let destination = dir.path().join("out.tar.gz");
        let report = export(&ExportRequest {
            root: &logs,
            destination: &destination,
            from: date(2026, 11, 4),
            to: date(2026, 11, 10),
            app_version: "1.2.3",
            platform: "macos",
            cancelled: &|| false,
        })
        .expect("export");
        assert_eq!(report.files, 2);
        let names = entry_names(&destination);
        assert!(names.contains(&"summary.json".to_owned()));
        assert!(names.contains(&"2026-11-08/usher.log".to_owned()));
        assert!(names.contains(&"2026-11-10/usher.log".to_owned()));
        assert!(!names.iter().any(|name| name.contains("2026-11-03")));
    }

    #[test]
    fn export_skips_symbolic_links() {
        let dir = tempfile::tempdir().expect("tempdir");
        let logs = dir.path().join("logs");
        make_logs(&logs);
        let secret = dir.path().join("secret.txt");
        fs::write(&secret, "do not export").expect("write");
        std::os::unix::fs::symlink(
            &secret,
            day_directory(&logs, date(2026, 11, 10)).join("usher.9.log"),
        )
        .expect("symlink");
        let destination = dir.path().join("out.tar.gz");
        let report = export(&ExportRequest {
            root: &logs,
            destination: &destination,
            from: date(2026, 11, 10),
            to: date(2026, 11, 10),
            app_version: "1",
            platform: "macos",
            cancelled: &|| false,
        })
        .expect("export");
        assert_eq!(report.skipped_links, 1);
        assert!(
            !entry_names(&destination)
                .iter()
                .any(|name| name.ends_with("usher.9.log"))
        );
    }

    #[test]
    fn oversized_files_keep_their_newest_lines_and_are_reported() {
        let dir = tempfile::tempdir().expect("tempdir");
        let path = dir.path().join("big.log");
        fs::write(&path, "aaaa\nbbbb\ncccc\n").expect("write");
        let tail = read_tail(&path, 8).expect("tail");
        assert!(tail.truncated);
        assert_eq!(String::from_utf8(tail.bytes).expect("utf8"), "cccc\n");
    }

    #[test]
    fn cancelling_or_failing_leaves_no_partial_archive() {
        let dir = tempfile::tempdir().expect("tempdir");
        let logs = dir.path().join("logs");
        make_logs(&logs);
        let destination = dir.path().join("out.tar.gz");
        let result = export(&ExportRequest {
            root: &logs,
            destination: &destination,
            from: date(2026, 11, 1),
            to: date(2026, 11, 10),
            app_version: "1",
            platform: "macos",
            cancelled: &|| true,
        });
        assert_eq!(result.err(), Some(ExportError::Cancelled));
        assert!(!destination.exists());
        let leftovers: Vec<_> = fs::read_dir(dir.path())
            .expect("read")
            .flatten()
            .filter(|entry| {
                entry
                    .file_name()
                    .to_string_lossy()
                    .starts_with(".usher-diagnostics-")
            })
            .collect();
        assert!(leftovers.is_empty());

        let empty = export(&ExportRequest {
            root: &logs,
            destination: &destination,
            from: date(2025, 1, 1),
            to: date(2025, 1, 2),
            app_version: "1",
            platform: "macos",
            cancelled: &|| false,
        });
        assert_eq!(empty.err(), Some(ExportError::NothingToExport));
        assert!(!destination.exists());

        let reversed = export(&ExportRequest {
            root: &logs,
            destination: &destination,
            from: date(2026, 11, 10),
            to: date(2026, 11, 1),
            app_version: "1",
            platform: "macos",
            cancelled: &|| false,
        });
        assert_eq!(reversed.err(), Some(ExportError::InvalidRange));
    }

    #[test]
    fn archive_names_are_unique_and_sort_in_creation_order() {
        let names: Vec<String> = (0..50).map(|_| archive_name()).collect();
        let mut sorted = names.clone();
        sorted.sort();
        sorted.dedup();
        assert_eq!(sorted, names, "names must be unique and already in order");
        assert!(names[0].starts_with(ARCHIVE_PREFIX) && names[0].ends_with(".tar.gz"));
    }

    #[test]
    fn exports_keep_only_the_newest_archives_and_the_one_just_written() {
        let dir = tempfile::tempdir().expect("tempdir");
        let mut written = Vec::new();
        for index in 0..8 {
            let path = dir.path().join(format!(
                "{ARCHIVE_PREFIX}20261110-0000{index:02}-1-000000.tar.gz"
            ));
            fs::write(&path, "x").expect("write");
            written.push(path);
        }
        // Unrelated files, folders and links must survive.
        fs::write(dir.path().join("notes.txt"), "keep").expect("write");
        fs::create_dir(dir.path().join(format!("{ARCHIVE_PREFIX}folder.tar.gz"))).expect("mkdir");

        prune_exports(dir.path(), &written[7]).expect("prune");
        let left: Vec<_> = written.iter().filter(|path| path.exists()).collect();
        assert_eq!(left.len(), crate::config::policy().kept_archives);
        assert!(written[7].exists());
        assert!(!written[0].exists() && !written[2].exists());
        assert!(dir.path().join("notes.txt").exists());
        assert!(
            dir.path()
                .join(format!("{ARCHIVE_PREFIX}folder.tar.gz"))
                .exists()
        );

        // The archive just written is protected even if its name sorts as the oldest.
        let oldest = dir
            .path()
            .join(format!("{ARCHIVE_PREFIX}19990101-000000-1-000000.tar.gz"));
        fs::write(&oldest, "x").expect("write");
        prune_exports(dir.path(), &oldest).expect("prune");
        assert!(oldest.exists());
    }
}
