//! Writes private JSONL files and optional console output from the single pipeline worker.
use crate::{
    Output,
    config::policy,
    files::{day_directory, prune, stream_shard_path},
    runtime::{Health, LogWriter, Record},
};
use chrono::{DateTime, Local, NaiveDate};
use std::{
    collections::BTreeMap,
    fs::{self, File, OpenOptions},
    io::{self, Write},
    path::PathBuf,
};
struct Open {
    day: NaiveDate,
    file: File,
    written: u64,
}
pub(crate) struct FileWriter {
    root: PathBuf,
    streams: BTreeMap<Output, Open>,
    pruned_for: Option<NaiveDate>,
}
impl FileWriter {
    pub(crate) fn new(root: PathBuf) -> Self {
        Self {
            root,
            streams: BTreeMap::new(),
            pruned_for: None,
        }
    }
}
fn health(error: io::Error) -> Health {
    if matches!(
        error.kind(),
        io::ErrorKind::NotFound | io::ErrorKind::PermissionDenied
    ) {
        Health::DirectoryUnavailable
    } else {
        Health::WriteFailed
    }
}
impl LogWriter for FileWriter {
    fn write(&mut self, output: Output, record: &Record) -> Result<(), Health> {
        let text = record.json();
        if output == Output::Console {
            return writeln!(io::stdout().lock(), "{text}").map_err(health);
        }
        let at: DateTime<Local> = record.timestamp.into();
        let day = at.date_naive();
        let line = format!("{text}\n");
        let incoming = line.len() as u64;
        let filename = if output == Output::Structured {
            &policy().structured_file
        } else {
            &policy().log_file
        };
        let reopen = self.streams.get(&output).is_none_or(|open| {
            open.day != day
                || (open.written > 0 && open.written + incoming > policy().max_shard_bytes)
        });
        if reopen {
            self.streams.remove(&output);
            let directory = day_directory(&self.root, day);
            let target = stream_shard_path(&directory, filename, incoming);
            fs::create_dir_all(&directory).map_err(health)?;
            let mut options = OpenOptions::new();
            options.create(true).append(true);
            #[cfg(unix)]
            std::os::unix::fs::OpenOptionsExt::mode(&mut options, 0o600);
            let file = options.open(target).map_err(health)?;
            let written = file.metadata().map_or(0, |meta| meta.len());
            self.streams.insert(output, Open { day, file, written });
            if self.pruned_for != Some(day) {
                self.pruned_for = Some(day);
                prune(
                    &self.root,
                    day,
                    policy().retention_days,
                    policy().disk_quota_bytes,
                );
            }
        }
        if let Some(open) = self.streams.get_mut(&output) {
            open.file.write_all(line.as_bytes()).map_err(health)?;
            open.written += incoming;
        }
        Ok(())
    }
    fn flush_pending(&mut self) -> Result<(), Health> {
        for open in self.streams.values_mut() {
            open.file.flush().map_err(health)?;
        }
        Ok(())
    }
    fn flush(&mut self) -> Result<(), Health> {
        let mut failure = None;
        for open in self.streams.values_mut() {
            if let Err(error) = open.file.sync_data() {
                failure = Some(health(error));
            }
        }
        failure.map_or(Ok(()), Err)
    }
}
