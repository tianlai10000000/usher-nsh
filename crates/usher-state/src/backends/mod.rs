//! Optional production adapters; normal consumers only use the crate-level API.

mod watch;
pub use watch::WatchBackend;
