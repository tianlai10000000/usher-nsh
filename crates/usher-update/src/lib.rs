//! Update policy, release signing and the update state machine. Pure logic: no NSH, no UI,
//! no network client, so it builds and tests on its own in seconds.
pub mod signature;
pub mod state;
