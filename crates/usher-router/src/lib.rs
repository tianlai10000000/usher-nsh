//! Backend-independent route catalogs, URL resolution and immutable history transitions.
//! The caller owns state and commits successful transitions. No global state, I/O,
//! framework, asynchronous executor or application data lives in this crate.
mod catalog;
mod generate;
mod history;
mod location;
pub mod matcher;
mod resolve;
mod schema;

pub use catalog::Router;
pub use history::{Entry, History, NavigationCommand, NavigationResult, Outcome};
pub use location::{Location, Target};
pub use schema::{Catalog, Parameter, Route, UnknownQuery, ValueKind};

/// Stable failures shared by all matcher implementations and application adapters.
#[derive(Clone, Debug, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Error {
    InvalidCatalog,
    UnsupportedSchema,
    InvalidPattern,
    Conflict,
    UnknownRoute,
    InvalidTarget,
    InvalidParameter,
    InvalidQuery,
    InvalidEncoding,
    InvalidHistory,
    CapacityExhausted,
    BackendUnavailable,
    /// A matcher returned a result inconsistent with the validated catalog.
    InvalidMatch,
}
impl std::fmt::Display for Error {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{self:?}")
    }
}
impl std::error::Error for Error {}
