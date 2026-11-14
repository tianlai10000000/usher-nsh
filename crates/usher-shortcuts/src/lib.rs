//! Pure shortcut catalogs, registration plans and receipt-driven resource lifecycles.
//! Callers own parsing, business dispatch, persistence, clocks and platform effects.
mod lifecycle;
mod model;
mod registry;

pub use lifecycle::*;
pub use model::*;
pub use registry::*;
