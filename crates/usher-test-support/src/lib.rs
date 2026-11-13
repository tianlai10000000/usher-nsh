//! Shared Rust test helpers with no application, framework or runtime dependencies.
//!
//! Consume this crate through `[dev-dependencies]`. Helpers create per-test values;
//! they do not install an executor, global registry or shared mutable fixture.
//! Business-specific mocks, input scenarios and assertions stay with their owner.

pub mod fixture;
pub mod task;
