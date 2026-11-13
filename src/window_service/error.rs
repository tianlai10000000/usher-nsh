//! Retains window service failure causes while exposing stable bridge error codes.
use usher_state::StoreError;

/// Window failures retain their cause; code() is the transport-facing classification.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Error {
    Closed,
    Poisoned,
    Identity(&'static str),
    Catalog(usher_router::Error),
    Target(usher_router::Error),
    Store(StoreError),
    Session(crate::window_session::Error),
    CatalogMismatch,
    CapacityExhausted,
    Pending,
}
impl Error {
    pub fn code(&self) -> &'static str {
        match self {
            Self::Closed => "router_closed",
            Self::Poisoned => "router_unavailable",
            Self::Identity(code) => code,
            Self::Store(StoreError::Closed) => "router_closed",
            Self::Store(StoreError::SequenceExhausted) | Self::CapacityExhausted => {
                "router_capacity_exhausted"
            }
            Self::Store(StoreError::BackendUnavailable) => "router_unavailable",
            Self::Catalog(
                usher_router::Error::BackendUnavailable | usher_router::Error::InvalidMatch,
            )
            | Self::Target(
                usher_router::Error::BackendUnavailable | usher_router::Error::InvalidMatch,
            ) => "router_unavailable",
            Self::Catalog(_) => "router_invalid_catalog",
            Self::Target(usher_router::Error::CapacityExhausted) => "router_capacity_exhausted",
            Self::Target(usher_router::Error::InvalidHistory) => "router_invalid_history",
            Self::Target(_) => "router_invalid_target",
            Self::Session(error) => error.code(),
            Self::CatalogMismatch => "router_catalog_mismatch",
            Self::Pending => "router_pending",
        }
    }
}
impl std::fmt::Display for Error {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(self.code())
    }
}
impl std::error::Error for Error {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            Self::Catalog(error) | Self::Target(error) => Some(error),
            Self::Store(error) => Some(error),
            Self::Session(error) => Some(error),
            _ => None,
        }
    }
}
/// Bridges serialize only the stable code; Rust callers retain the typed error.
impl From<Error> for String {
    fn from(error: Error) -> Self {
        error.code().into()
    }
}
