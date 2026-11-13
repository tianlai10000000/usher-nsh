//! Structured diagnostic logging: level filter, daily buckets, retention and export.
//!
//! Records are queued on a bounded channel and written by one background thread, so
//! callers never wait on disk I/O. Files live in `logs/YYYY-MM-DD/usher.log`, bucketed by
//! the local date the event happened. A full or unwritable disk never fails the caller.

use std::{
    collections::BTreeMap,
    fs::{self, File, OpenOptions},
    io::{self, Write},
    path::{Path, PathBuf},
    str::FromStr,
    sync::{
        Arc, LazyLock, Mutex,
        atomic::{AtomicU8, AtomicU64, Ordering},
        mpsc::{self, Receiver, RecvTimeoutError, SyncSender, TrySendError},
    },
    thread::{self, JoinHandle},
    time::Duration,
};

use chrono::{DateTime, Local, NaiveDate, SecondsFormat};
use flate2::{Compression, write::GzEncoder};
use serde_json::{Map, Value, json};

pub(crate) const LOG_FILE: &str = "usher.log";
/// One log file rolls into a numbered shard past this size.
pub(crate) const MAX_SHARD_BYTES: u64 = 8 * 1024 * 1024;
/// The whole log directory is pruned oldest-first past this size.
pub(crate) const DISK_QUOTA_BYTES: u64 = 256 * 1024 * 1024;
pub(crate) const RETENTION_DAYS: i64 = 14;
pub(crate) const EXPORT_FILE_LIMIT: u64 = 8 * 1024 * 1024;
pub(crate) const EXPORT_TOTAL_LIMIT: u64 = 64 * 1024 * 1024;
const QUEUE_CAPACITY: usize = 4096;
const FLUSH_INTERVAL: Duration = Duration::from_millis(500);
const MAX_MESSAGE_CHARS: usize = 2000;
const REDACTED: &str = "[redacted]";

/// Ordered from most to least severe; `Off` records nothing.
#[derive(Clone, Copy, Debug, Default, Eq, Ord, PartialEq, PartialOrd)]
pub(crate) enum Level {
    Off,
    Error,
    Warn,
    #[default]
    Info,
    Debug,
    Trace,
}

impl Level {
    pub(crate) fn as_str(self) -> &'static str {
        match self {
            Self::Off => "OFF",
            Self::Error => "ERROR",
            Self::Warn => "WARN",
            Self::Info => "INFO",
            Self::Debug => "DEBUG",
            Self::Trace => "TRACE",
        }
    }

    fn from_u8(value: u8) -> Self {
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
pub(crate) fn enabled(filter: Level, record: Level) -> bool {
    record != Level::Off && record <= filter
}

// --- Redaction -----------------------------------------------------------------------------

const SENSITIVE_KEYS: [&str; 12] = [
    "password",
    "passwd",
    "secret",
    "token",
    "authorization",
    "cookie",
    "apikey",
    "api_key",
    "credential",
    "body",
    "content",
    "email",
];

fn is_sensitive(key: &str) -> bool {
    let key = key.to_ascii_lowercase();
    SENSITIVE_KEYS.iter().any(|needle| key.contains(needle))
}

/// Replaces values of credential-like fields, recursively.
pub(crate) fn redact(fields: &mut Map<String, Value>) {
    for (key, value) in fields.iter_mut() {
        if is_sensitive(key) {
            *value = Value::String(REDACTED.to_owned());
        } else if let Value::Object(inner) = value {
            redact(inner);
        }
    }
}

/// Masks `Authorization: ...` style pairs and bearer tokens inside free text.
pub(crate) fn redact_text(text: &str) -> String {
    let mut out = String::with_capacity(text.len());
    let mut words = text.split(' ').peekable();
    let mut first = true;
    while let Some(word) = words.next() {
        if !first {
            out.push(' ');
        }
        first = false;
        let lower = word.to_ascii_lowercase();
        let labelled = lower
            .split(['=', ':'])
            .next()
            .is_some_and(|name| is_sensitive(name) && lower.contains(['=', ':']));
        if lower == "bearer" || lower == "basic" {
            out.push_str(word);
            if words.peek().is_some() {
                words.next();
                out.push(' ');
                out.push_str(REDACTED);
            }
        } else if labelled {
            let name = word.split(['=', ':']).next().unwrap_or_default();
            let separator = if word.contains('=') { '=' } else { ':' };
            out.push_str(name);
            out.push(separator);
            out.push_str(REDACTED);
        } else {
            out.push_str(word);
        }
    }
    out.chars().take(MAX_MESSAGE_CHARS).collect()
}

// --- Record formatting ---------------------------------------------------------------------

/// One JSON line: timestamp with offset, level, module, event and optional fields.
pub(crate) fn format_record(
    at: DateTime<Local>,
    level: Level,
    module: &str,
    event: &str,
    fields: Option<Map<String, Value>>,
) -> String {
    let mut record = json!({
        "ts": at.to_rfc3339_opts(SecondsFormat::Millis, false),
        "level": level.as_str(),
        "module": redact_text(module),
        "event": redact_text(event),
    });
    if let Some(mut fields) = fields.filter(|fields| !fields.is_empty()) {
        redact(&mut fields);
        record["fields"] = Value::Object(fields);
    }
    record.to_string()
}

// --- Daily buckets --------------------------------------------------------------------------

pub(crate) fn day_directory(root: &Path, day: NaiveDate) -> PathBuf {
    root.join(day.format("%Y-%m-%d").to_string())
}

fn parse_day(name: &str) -> Option<NaiveDate> {
    NaiveDate::parse_from_str(name, "%Y-%m-%d").ok()
}

/// The shard a write of `incoming` bytes goes to, rolling to `usher.1.log`, ... as files fill.
pub(crate) fn shard_path(directory: &Path, incoming: u64) -> PathBuf {
    let mut index = 0u32;
    loop {
        let name = if index == 0 {
            LOG_FILE.to_owned()
        } else {
            format!("usher.{index}.log")
        };
        let path = directory.join(name);
        match fs::metadata(&path) {
            Ok(meta) if meta.len() > 0 && meta.len() + incoming > MAX_SHARD_BYTES => index += 1,
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

// --- Background writer ----------------------------------------------------------------------

/// Coarse state the settings page can show.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(crate) enum Health {
    Ok,
    DirectoryUnavailable,
    WriteFailed,
}

impl Health {
    pub(crate) fn code(self) -> Option<&'static str> {
        match self {
            Self::Ok => None,
            Self::DirectoryUnavailable => Some("diagnostics_directory_unavailable"),
            Self::WriteFailed => Some("diagnostics_write_failed"),
        }
    }
}

enum Message {
    Line { day: NaiveDate, text: String },
    Flush(mpsc::Sender<()>),
    Stop,
}

struct Shared {
    level: AtomicU8,
    dropped: AtomicU64,
    health: Mutex<Health>,
    reporting: Mutex<Option<(u64, Arc<crate::diagnostic_state::StatusService>)>>,
}

impl Shared {
    fn publish(&self) {
        let reporting = self
            .reporting
            .lock()
            .unwrap_or_else(|p| p.into_inner())
            .clone();
        if let Some((source, service)) = reporting {
            service.update(source, || crate::diagnostic_state::Status {
                problem: self
                    .health
                    .lock()
                    .unwrap_or_else(|p| p.into_inner())
                    .code()
                    .map(str::to_owned),
                dropped: self.dropped.load(Ordering::Relaxed),
            });
        }
    }
}

/// A running pipeline. Dropping it drains the queue within a bounded time.
pub(crate) struct Logger {
    root: PathBuf,
    sender: SyncSender<Message>,
    shared: Arc<Shared>,
    worker: Option<JoinHandle<()>>,
}

impl Logger {
    pub(crate) fn start(root: PathBuf, level: Level) -> Self {
        Self::start_with_capacity(root, level, QUEUE_CAPACITY)
    }

    fn start_with_capacity(root: PathBuf, level: Level, capacity: usize) -> Self {
        let (sender, receiver) = mpsc::sync_channel(capacity);
        let shared = Arc::new(Shared {
            level: AtomicU8::new(level as u8),
            dropped: AtomicU64::new(0),
            health: Mutex::new(Health::Ok),
            reporting: Mutex::new(None),
        });
        let worker = {
            let root = root.clone();
            let shared = Arc::clone(&shared);
            thread::Builder::new()
                .name("usher-diagnostics".into())
                .spawn(move || run_worker(&root, &receiver, &shared))
                .ok()
        };
        Self {
            root,
            sender,
            shared,
            worker,
        }
    }

    pub(crate) fn root(&self) -> &Path {
        &self.root
    }

    /// Applies a new filter immediately; no restart is needed.
    pub(crate) fn set_level(&self, level: Level) {
        self.shared.level.store(level as u8, Ordering::Relaxed);
    }

    pub(crate) fn level(&self) -> Level {
        Level::from_u8(self.shared.level.load(Ordering::Relaxed))
    }

    pub(crate) fn health(&self) -> Health {
        *self
            .shared
            .health
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner())
    }

    /// Records lost because the queue was full.
    pub(crate) fn dropped(&self) -> u64 {
        self.shared.dropped.load(Ordering::Relaxed)
    }

    pub(crate) fn log(
        &self,
        level: Level,
        module: &str,
        event: &str,
        fields: Option<Map<String, Value>>,
    ) {
        self.log_at(Local::now(), level, module, event, fields);
    }

    pub(crate) fn log_at(
        &self,
        at: DateTime<Local>,
        level: Level,
        module: &str,
        event: &str,
        fields: Option<Map<String, Value>>,
    ) {
        if !enabled(self.level(), level) {
            return;
        }
        self.enqueue(at, level, module, event, fields);
    }

    /// Records a change of the filter itself. It ignores the filter, so a switch to a
    /// stricter level or to OFF still leaves a trace; nothing is written when both the old
    /// and the new level are OFF.
    pub(crate) fn record_level_change(&self, new: Level) {
        let old = self.level();
        if old == Level::Off && new == Level::Off {
            return;
        }
        let mut fields = Map::new();
        fields.insert("from".into(), Value::String(old.as_str().to_owned()));
        fields.insert("to".into(), Value::String(new.as_str().to_owned()));
        self.enqueue(
            Local::now(),
            Level::Info,
            "settings",
            "diagnostic_level_changed",
            Some(fields),
        );
    }

    fn enqueue(
        &self,
        at: DateTime<Local>,
        level: Level,
        module: &str,
        event: &str,
        fields: Option<Map<String, Value>>,
    ) {
        let message = Message::Line {
            day: at.date_naive(),
            text: format_record(at, level, module, event, fields),
        };
        match self.sender.try_send(message) {
            Ok(()) => {}
            Err(TrySendError::Full(message)) => {
                // Warnings and errors wait for room; chatter may be dropped and counted.
                if level <= Level::Warn {
                    let _ = self.sender.send(message);
                } else {
                    let _ = self.shared.dropped.fetch_update(
                        Ordering::Relaxed,
                        Ordering::Relaxed,
                        |value| Some(value.saturating_add(1)),
                    );
                    self.shared.publish();
                }
            }
            Err(TrySendError::Disconnected(_)) => {}
        }
    }

    /// Waits until everything queued so far is on disk, or `timeout` passes.
    pub(crate) fn flush(&self, timeout: Duration) -> bool {
        let (done, wait) = mpsc::channel();
        // Blocking send: a full queue must delay the flush, not fail it.
        if self.sender.send(Message::Flush(done)).is_err() {
            return false;
        }
        wait.recv_timeout(timeout).is_ok()
    }
}

impl Drop for Logger {
    fn drop(&mut self) {
        let _ = self.flush(Duration::from_secs(2));
        let _ = self.sender.send(Message::Stop);
        if let Some(worker) = self.worker.take() {
            let _ = worker.join();
        }
    }
}

struct Open {
    day: NaiveDate,
    file: File,
    written: u64,
}

fn run_worker(root: &Path, receiver: &Receiver<Message>, shared: &Shared) {
    let mut open: Option<Open> = None;
    let mut pruned_for: Option<NaiveDate> = None;
    let set_health = |health: Health| {
        *shared
            .health
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner()) = health;
        shared.publish();
    };
    let mut pending_flush = false;

    loop {
        let message = match receiver.recv_timeout(FLUSH_INTERVAL) {
            Ok(message) => message,
            Err(RecvTimeoutError::Timeout) => {
                if pending_flush {
                    if let Some(open) = open.as_mut() {
                        let _ = open.file.flush();
                    }
                    pending_flush = false;
                }
                continue;
            }
            Err(RecvTimeoutError::Disconnected) => break,
        };
        match message {
            Message::Stop => break,
            Message::Flush(done) => {
                if let Some(open) = open.as_mut() {
                    let _ = open.file.sync_data();
                }
                pending_flush = false;
                let _ = done.send(());
            }
            Message::Line { day, text } => {
                let line = format!("{text}\n");
                let incoming = line.len() as u64;
                // The file is reopened on a new local date or when this shard is full. Sizes
                // are counted in memory, so a write costs no filesystem query.
                let full = open.as_ref().is_some_and(|open| {
                    open.written > 0 && open.written + incoming > MAX_SHARD_BYTES
                });
                if open.as_ref().is_none_or(|open| open.day != day) || full {
                    let directory = day_directory(root, day);
                    let target = shard_path(&directory, incoming);
                    match open_append(&directory, &target) {
                        Ok(file) => {
                            let written = file.metadata().map_or(0, |meta| meta.len());
                            open = Some(Open { day, file, written });
                            set_health(Health::Ok);
                        }
                        Err(error) => {
                            open = None;
                            set_health(
                                if matches!(
                                    error.kind(),
                                    io::ErrorKind::NotFound | io::ErrorKind::PermissionDenied
                                ) {
                                    Health::DirectoryUnavailable
                                } else {
                                    Health::WriteFailed
                                },
                            );
                            continue;
                        }
                    }
                    // Retention follows the clock: it runs whenever a new day or shard starts,
                    // not only at launch, so a long-running process cleans up after itself.
                    if pruned_for != Some(day) {
                        pruned_for = Some(day);
                        prune(root, day, RETENTION_DAYS, DISK_QUOTA_BYTES);
                    }
                }
                if let Some(open) = open.as_mut() {
                    match open.file.write_all(line.as_bytes()) {
                        Ok(()) => {
                            open.written += incoming;
                            pending_flush = true;
                        }
                        Err(_) => set_health(Health::WriteFailed),
                    }
                }
            }
        }
    }
    if let Some(open) = open.as_mut() {
        let _ = open.file.flush();
    }
}

fn open_append(directory: &Path, path: &Path) -> io::Result<File> {
    fs::create_dir_all(directory)?;
    let mut options = OpenOptions::new();
    options.create(true).append(true);
    // Logs can hold paths and identifiers: keep them private to the user.
    #[cfg(unix)]
    std::os::unix::fs::OpenOptionsExt::mode(&mut options, 0o600);
    options.open(path)
}

// --- Export ---------------------------------------------------------------------------------

#[derive(Debug, Default, Eq, PartialEq)]
pub(crate) struct ExportReport {
    pub(crate) files: usize,
    pub(crate) skipped_links: usize,
    pub(crate) truncated_files: usize,
    pub(crate) omitted_files: usize,
    pub(crate) bytes: u64,
}

#[derive(Debug, Eq, PartialEq)]
pub(crate) enum ExportError {
    InvalidRange,
    NothingToExport,
    Cancelled,
    Io,
}

impl ExportError {
    pub(crate) fn code(&self) -> &'static str {
        match self {
            Self::InvalidRange => "diagnostics_export_invalid_range",
            Self::NothingToExport => "diagnostics_export_empty",
            Self::Cancelled => "diagnostics_export_cancelled",
            Self::Io => "diagnostics_export_failed",
        }
    }
}

pub(crate) struct ExportRequest<'a> {
    pub(crate) root: &'a Path,
    pub(crate) destination: &'a Path,
    pub(crate) from: NaiveDate,
    pub(crate) to: NaiveDate,
    pub(crate) app_version: &'a str,
    pub(crate) platform: &'a str,
    pub(crate) cancelled: &'a dyn Fn() -> bool,
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

    let mut remaining = EXPORT_TOTAL_LIMIT;
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
            let limit = EXPORT_FILE_LIMIT.min(remaining);
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

const KEPT_ARCHIVES: usize = 5;
const ARCHIVE_PREFIX: &str = "usher-diagnostics-";
static ARCHIVE_SEQUENCE: AtomicU64 = AtomicU64::new(0);

/// A unique, sortable name: time, process and a counter, so two exports in the same
/// millisecond never collide and name order is creation order.
pub(crate) fn archive_name() -> String {
    format!(
        "{ARCHIVE_PREFIX}{}-{}-{:06}.tar.gz",
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
            entry
                .file_name()
                .to_str()
                .is_some_and(|name| name.starts_with(ARCHIVE_PREFIX) && name.ends_with(".tar.gz"))
        })
        .map(|entry| entry.path())
        .collect();
    archives.sort();
    let mut remaining = archives.len();
    for archive in archives {
        if remaining <= KEPT_ARCHIVES {
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

// --- Crash records --------------------------------------------------------------------------

/// Chains a panic hook that writes one ERROR record, then runs the previous hook. It does
/// not replace the host's behaviour and does not request shutdown: whether the process
/// exits is the runtime's decision, not the logger's.
pub(crate) fn install_panic_record() {
    static INSTALLED: std::sync::Once = std::sync::Once::new();
    INSTALLED.call_once(|| {
        let previous = std::panic::take_hook();
        std::panic::set_hook(Box::new(move |info| {
            if let Some(logger) = current() {
                let place = info
                    .location()
                    .map(|place| format!("{}:{}", place.file(), place.line()))
                    .unwrap_or_default();
                let mut fields = Map::new();
                fields.insert("location".into(), Value::String(place));
                // The payload can carry user data, so only its presence is recorded.
                logger.log(Level::Error, "process", "panic", Some(fields));
                logger.flush(Duration::from_secs(1));
            }
            previous(info);
        }));
    });
}

// --- Process-wide pipeline -----------------------------------------------------------------

static LOGGER: LazyLock<Mutex<Option<Arc<Logger>>>> = LazyLock::new(|| Mutex::new(None));

/// Starts, or retargets, the process pipeline. A second call with the same directory only
/// changes the level, so several windows and hot reloads never create duplicate writers.
pub(crate) fn install(root: PathBuf, level: Level) -> Arc<Logger> {
    let mut slot = LOGGER
        .lock()
        .unwrap_or_else(|poisoned| poisoned.into_inner());
    if let Some(existing) = slot.as_ref().filter(|logger| logger.root() == root) {
        existing.set_level(level);
        return Arc::clone(existing);
    }
    let logger = Arc::new(Logger::start(root, level));
    if let Ok(status) = &*crate::diagnostic_state::STATUS
        && let Ok(source) = status.activate()
    {
        *logger
            .shared
            .reporting
            .lock()
            .unwrap_or_else(|p| p.into_inner()) = Some((source, Arc::clone(status)));
        logger.shared.publish();
    }
    *slot = Some(Arc::clone(&logger));
    drop(slot);
    install_panic_record();
    install_exit_flush();
    logger
}

pub(crate) fn current() -> Option<Arc<Logger>> {
    LOGGER
        .lock()
        .unwrap_or_else(|poisoned| poisoned.into_inner())
        .clone()
}

/// Flushes the process pipeline before exit or an update handoff.
pub(crate) fn flush_for_exit() {
    // Copy the handle out of the lock first, because the flush waits on the writer thread.
    // `try_lock` so an exit during `install` cannot deadlock.
    let logger = LOGGER
        .try_lock()
        .ok()
        .and_then(|slot| slot.as_ref().map(Arc::clone));
    if let Some(logger) = logger {
        logger.flush(Duration::from_secs(2));
    }
}

extern "C" fn flush_at_exit() {
    flush_for_exit();
}

/// Runs `flush_for_exit` when the process ends through `exit()`, which is how Quit,
/// `terminate:` and a normal return from `main` leave. Statics are not dropped then, so
/// without this the last queued lines would be lost.
fn install_exit_flush() {
    static INSTALLED: std::sync::Once = std::sync::Once::new();
    INSTALLED.call_once(|| {
        // SAFETY: `flush_at_exit` is a plain `extern "C" fn()` that lives for the program.
        unsafe {
            libc::atexit(flush_at_exit);
        }
    });
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::TimeZone;

    fn at(year: i32, month: u32, day: u32, hour: u32, minute: u32) -> DateTime<Local> {
        Local
            .with_ymd_and_hms(year, month, day, hour, minute, 0)
            .single()
            .expect("unambiguous local time")
    }

    fn date(year: i32, month: u32, day: u32) -> NaiveDate {
        NaiveDate::from_ymd_opt(year, month, day).expect("valid date")
    }

    /// Tests that install the process-wide logger take this, or one would swap the logger
    /// out from under another that is still using it.
    static GLOBAL_LOGGER: Mutex<()> = Mutex::new(());

    fn global_logger_guard() -> std::sync::MutexGuard<'static, ()> {
        GLOBAL_LOGGER
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner())
    }

    fn read_log(root: &Path, day: NaiveDate) -> String {
        fs::read_to_string(day_directory(root, day).join(LOG_FILE)).unwrap_or_default()
    }

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

    #[test]
    fn redaction_masks_credentials_in_fields_and_text() {
        let mut fields = Map::new();
        fields.insert("Authorization".into(), json!("Bearer abc"));
        fields.insert("requestBody".into(), json!("private"));
        fields.insert("nested".into(), json!({ "apiKey": "k", "ok": 1 }));
        fields.insert("count".into(), json!(3));
        redact(&mut fields);
        assert_eq!(fields["Authorization"], REDACTED);
        assert_eq!(fields["requestBody"], REDACTED);
        assert_eq!(fields["nested"]["apiKey"], REDACTED);
        assert_eq!(fields["nested"]["ok"], 1);
        assert_eq!(fields["count"], 3);

        assert_eq!(
            redact_text("sent Bearer abc123 ok"),
            "sent Bearer [redacted] ok"
        );
        assert_eq!(redact_text("token=xyz done"), "token=[redacted] done");
        assert_eq!(redact_text("plain text"), "plain text");
    }

    #[test]
    fn records_carry_timezone_level_module_and_event() {
        let line = format_record(
            at(2026, 11, 10, 9, 30),
            Level::Warn,
            "settings",
            "save_failed",
            None,
        );
        let value: Value = serde_json::from_str(&line).expect("valid json");
        assert_eq!(value["level"], "WARN");
        assert_eq!(value["module"], "settings");
        assert_eq!(value["event"], "save_failed");
        let stamp = value["ts"].as_str().expect("timestamp");
        assert!(DateTime::parse_from_rfc3339(stamp).is_ok(), "{stamp}");
        assert!(value.get("fields").is_none());
    }

    #[test]
    fn events_land_in_the_bucket_of_their_own_local_date() {
        let dir = tempfile::tempdir().expect("tempdir");
        let logger = Logger::start(dir.path().to_path_buf(), Level::Info);
        logger.log_at(
            at(2026, 11, 9, 23, 59),
            Level::Info,
            "app",
            "before_midnight",
            None,
        );
        logger.log_at(
            at(2026, 11, 10, 0, 1),
            Level::Info,
            "app",
            "after_midnight",
            None,
        );
        assert!(logger.flush(Duration::from_secs(5)));
        assert!(read_log(dir.path(), date(2026, 11, 9)).contains("before_midnight"));
        assert!(read_log(dir.path(), date(2026, 11, 10)).contains("after_midnight"));
        assert!(!read_log(dir.path(), date(2026, 11, 10)).contains("before_midnight"));
    }

    #[test]
    fn changing_the_level_takes_effect_without_a_restart() {
        let dir = tempfile::tempdir().expect("tempdir");
        let logger = Logger::start(dir.path().to_path_buf(), Level::Warn);
        let day = at(2026, 11, 10, 12, 0);
        logger.log_at(day, Level::Info, "app", "hidden", None);
        logger.set_level(Level::Debug);
        logger.log_at(day, Level::Debug, "app", "shown", None);
        logger.set_level(Level::Off);
        logger.log_at(day, Level::Error, "app", "silenced", None);
        assert!(logger.flush(Duration::from_secs(5)));
        let text = read_log(dir.path(), date(2026, 11, 10));
        assert!(text.contains("shown"));
        assert!(!text.contains("hidden"));
        assert!(!text.contains("silenced"));
    }

    #[test]
    fn a_level_change_is_recorded_even_when_the_new_filter_would_hide_it() {
        let dir = tempfile::tempdir().expect("tempdir");
        let logger = Logger::start(dir.path().to_path_buf(), Level::Debug);
        logger.record_level_change(Level::Error);
        logger.set_level(Level::Error);
        // Going from ERROR to OFF must also leave its own line, although INFO is filtered.
        logger.record_level_change(Level::Off);
        logger.set_level(Level::Off);
        // OFF to OFF writes nothing.
        logger.record_level_change(Level::Off);
        assert!(logger.flush(Duration::from_secs(5)));
        let today = Local::now().date_naive();
        let text = read_log(dir.path(), today);
        let lines: Vec<Value> = text
            .lines()
            .map(|line| serde_json::from_str(line).expect("json"))
            .collect();
        assert_eq!(lines.len(), 2, "{text}");
        assert_eq!(lines[0]["fields"]["from"], "DEBUG");
        assert_eq!(lines[0]["fields"]["to"], "ERROR");
        assert_eq!(lines[1]["fields"]["from"], "ERROR");
        assert_eq!(lines[1]["fields"]["to"], "OFF");
    }

    #[test]
    fn full_shards_roll_to_numbered_files() {
        let dir = tempfile::tempdir().expect("tempdir");
        let first = dir.path().join(LOG_FILE);
        let file = File::create(&first).expect("create");
        file.set_len(MAX_SHARD_BYTES - 10).expect("grow");
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
            fs::write(directory.join(LOG_FILE), "x").expect("write");
        }
        fs::create_dir_all(dir.path().join("not-a-date")).expect("mkdir");
        let removed = prune(dir.path(), today, RETENTION_DAYS, DISK_QUOTA_BYTES);
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
            fs::write(directory.join(LOG_FILE), vec![b'x'; 100]).expect("write");
        }
        let removed = prune(dir.path(), today, RETENTION_DAYS, 150);
        assert_eq!(removed, vec![date(2026, 11, 8), date(2026, 11, 9)]);
        assert!(day_directory(dir.path(), today).exists());
    }

    #[test]
    fn an_unwritable_directory_is_reported_and_never_panics() {
        let dir = tempfile::tempdir().expect("tempdir");
        // A file where the log root should be makes every write fail.
        let blocked = dir.path().join("logs");
        fs::write(&blocked, "in the way").expect("write");
        let logger = Logger::start(blocked, Level::Info);
        logger.log_at(at(2026, 11, 10, 12, 0), Level::Info, "app", "lost", None);
        assert!(logger.flush(Duration::from_secs(5)));
        assert_ne!(logger.health(), Health::Ok);
        assert!(logger.health().code().is_some());
    }

    #[test]
    fn a_full_queue_drops_chatter_but_counts_it() {
        let dir = tempfile::tempdir().expect("tempdir");
        let logger = Logger::start_with_capacity(dir.path().to_path_buf(), Level::Trace, 1);
        let day = at(2026, 11, 10, 12, 0);
        for index in 0..2000 {
            logger.log_at(day, Level::Trace, "bench", &format!("event_{index}"), None);
        }
        assert!(logger.flush(Duration::from_secs(10)));
        assert!(
            logger.dropped() > 0,
            "a one-slot queue cannot absorb 2000 events"
        );
    }

    fn make_logs(root: &Path) {
        for (day, body) in [
            (date(2026, 11, 3), "old\n"),
            (date(2026, 11, 8), "recent\n"),
            (date(2026, 11, 10), "today\n"),
        ] {
            let directory = day_directory(root, day);
            fs::create_dir_all(&directory).expect("mkdir");
            fs::write(directory.join(LOG_FILE), body).expect("write");
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

    #[cfg(unix)]
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
        assert_eq!(left.len(), KEPT_ARCHIVES);
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

    #[test]
    fn flushing_for_exit_puts_queued_lines_on_disk_and_never_hangs_without_a_logger() {
        let _serial = global_logger_guard();
        let dir = tempfile::tempdir().expect("tempdir");
        let logger = install(dir.path().to_path_buf(), Level::Info);
        logger.log_at(
            at(2026, 11, 10, 12, 0),
            Level::Info,
            "app",
            "last_words",
            None,
        );
        flush_for_exit();
        assert!(read_log(dir.path(), date(2026, 11, 10)).contains("last_words"));
    }

    #[test]
    fn installing_twice_for_one_directory_reuses_the_writer() {
        let _serial = global_logger_guard();
        let dir = tempfile::tempdir().expect("tempdir");
        let first = install(dir.path().to_path_buf(), Level::Info);
        let second = install(dir.path().to_path_buf(), Level::Debug);
        assert!(Arc::ptr_eq(&first, &second));
        assert_eq!(second.level(), Level::Debug);
    }
    #[test]
    fn worker_health_changes_reach_the_shared_status_store() {
        use crate::diagnostic_state::StatusService;
        use usher_test_support::task::ready;
        let directory = tempfile::tempdir().unwrap();
        let blocked = directory.path().join("blocked");
        fs::write(&blocked, b"not a directory").unwrap();
        let logger = Logger::start(blocked, Level::Info);
        let status = Arc::new(StatusService::new(usher_state::StoreId::new(u128::MAX)).unwrap());
        let source = status.activate().unwrap();
        *logger.shared.reporting.lock().unwrap() = Some((source, status.clone()));
        let mut subscription = status.subscribe().unwrap();
        logger.log(Level::Info, "test", "health_subscription", None);
        assert!(logger.flush(Duration::from_secs(2)));
        let snapshot = ready(subscription.next()).unwrap();
        assert_eq!(snapshot.state().problem.as_deref(), logger.health().code());
        assert!(snapshot.state().problem.is_some());
    }
}
