//! Update policy and the update state machine. Pure logic: no NSH, no UI, no network client
//! and no logger, so it builds and tests on its own in seconds. Signing, certificates and
//! manifest verification are NSH's (docs/update.md, D1); this crate holds only the rules about
//! what to do with what NSH reports.
pub mod state;

/// The version type the state machine uses, re-exported so a caller builds versions with the same
/// `semver` the crate was compiled against instead of depending on it separately.
pub use semver::Version;
