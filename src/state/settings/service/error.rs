//! Settings service failures retain domain, store and update causes until the transport boundary.
/// Application failures, distinct from legacy domain validation and transport encoding.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Error {
    Closed,
    Poisoned,
    InvalidPath,
    Identity(&'static str),
    RevisionExhausted,
    Store(usher_state::StoreError),
    Domain(&'static str),
    Update(crate::state::updates::Error),
    Synchronization(crate::state::updates::Error),
}
impl Error {
    /// Preserves the existing settings wire codes, including legacy domain rejections.
    pub fn code(&self) -> &'static str {
        match self {
            Self::Domain(code) | Self::Identity(code) => code,
            Self::Update(error) => error.code(),
            _ => crate::state::settings::UNAVAILABLE,
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
            Self::Store(error) => Some(error),
            Self::Update(error) | Self::Synchronization(error) => Some(error),
            _ => None,
        }
    }
}
impl From<&'static str> for Error {
    fn from(code: &'static str) -> Self {
        Self::Domain(code)
    }
}
impl From<crate::state::updates::Error> for Error {
    fn from(error: crate::state::updates::Error) -> Self {
        Self::Update(error)
    }
}
/// The native bridge emits stable codes, never debug-formatted domain data.
impl From<Error> for String {
    fn from(error: Error) -> Self {
        error.code().into()
    }
}
