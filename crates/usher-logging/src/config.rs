//! Loads the bundled compatibility profile as the single source of output policies.
use serde::Deserialize;
use std::sync::LazyLock;

#[derive(Deserialize)]
pub(crate) struct Policy {
    pub log_file: String,
    pub structured_file: String,
    pub archive_prefix: String,
    pub max_shard_bytes: u64,
    pub disk_quota_bytes: u64,
    pub retention_days: i64,
    pub export_file_limit: u64,
    pub export_total_limit: u64,
    pub kept_archives: usize,
    pub queue_capacity: usize,
    pub flush_interval_ms: u64,
    pub max_message_chars: usize,
    pub change_flush_ms: u64,
    pub exit_flush_ms: u64,
    pub export_flush_ms: u64,
    pub outputs: crate::event::Outputs,
}

pub(crate) fn policy() -> &'static Policy {
    static POLICY: LazyLock<Policy> = LazyLock::new(|| {
        // This immutable, bundled profile is checked by the crate's tests.
        serde_json::from_str(include_str!("../config/defaults.json"))
            .expect("valid bundled logging policy")
    });
    &POLICY
}
