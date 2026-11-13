//! Stable logging facade with structured values and replaceable output adapters.
//! Application code uses Logger; composition roots use the runtime namespace.
mod archive;
mod backends;
mod config;
mod event;
mod files;
mod level;
mod lifecycle;
mod pipeline;
mod record;
mod redaction;
pub mod runtime;
#[cfg(test)]
mod test_support;

pub use event::{FieldValue, Fields, LogKind, Output, Outputs, Submit};
pub use level::Level;
use std::sync::Arc;

/// Cloneable logging access without file paths, worker handles or backend types.
#[derive(Clone)]
pub struct Logger {
    pub(crate) inner: Arc<pipeline::Pipeline>,
    pub(crate) kind: LogKind,
}
impl Logger {
    /// Selects the diagnostic source while preserving the common filter and backend.
    pub fn for_kind(&self, kind: LogKind) -> Self {
        Self {
            inner: self.inner.clone(),
            kind,
        }
    }
    pub fn enabled(&self, level: Level) -> bool {
        self.inner.enabled(self.kind, level)
    }
    pub fn level(&self) -> Level {
        self.inner.level()
    }
    /// Changes only the running filter; persistent preferences belong to the application.
    pub fn set_level(&self, level: Level) {
        self.inner.set_level(level);
    }
    /// Queues a structured event. It never reports persistence as successful admission.
    pub fn log(&self, level: Level, module: &str, event: &str, fields: Fields) -> Submit {
        if !self.enabled(level) {
            return Submit::Filtered;
        }
        if !fields.valid() {
            return Submit::InvalidFields;
        }
        self.inner.submit(
            chrono::Local::now(),
            level,
            self.kind,
            module,
            event,
            record::to_map(fields),
        )
    }
    pub fn error(&self, module: &str, event: &str, fields: Fields) -> Submit {
        self.log(Level::Error, module, event, fields)
    }
    pub fn warn(&self, module: &str, event: &str, fields: Fields) -> Submit {
        self.log(Level::Warn, module, event, fields)
    }
    pub fn info(&self, module: &str, event: &str, fields: Fields) -> Submit {
        self.log(Level::Info, module, event, fields)
    }
    pub fn debug(&self, module: &str, event: &str, fields: Fields) -> Submit {
        self.log(Level::Debug, module, event, fields)
    }
    pub fn trace(&self, module: &str, event: &str, fields: Fields) -> Submit {
        self.log(Level::Trace, module, event, fields)
    }
}
/// Returns the process facade if initialized; does not create storage as a side effect.
pub fn current() -> Option<Logger> {
    runtime::current().map(|runtime| runtime.logger())
}
