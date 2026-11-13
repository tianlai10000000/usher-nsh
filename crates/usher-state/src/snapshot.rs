//! Immutable snapshot envelopes and caller-owned store identities.

use std::sync::Arc;

/// An application-allocated identity. Never reuse it within a process lifetime,
/// including when rebuilding a store. It is not a persistence or window identity.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct StoreId(u128);

impl StoreId {
    /// Wraps an identity allocated by the application, without a global registry.
    pub const fn new(value: u128) -> Self {
        Self(value)
    }

    /// Returns the identity for transport encoding or equality checks.
    pub const fn value(self) -> u128 {
        self.0
    }
}

/// A cheap, immutable handle. Clone does not require `S: Clone`.
#[derive(Debug)]
pub struct Snapshot<S> {
    store_id: StoreId,
    sequence: u64,
    state: Arc<S>,
}

impl<S> Snapshot<S> {
    pub(crate) fn new(store_id: StoreId, sequence: u64, state: S) -> Self {
        Self {
            store_id,
            sequence,
            state: Arc::new(state),
        }
    }

    /// Identifies the store lifetime that produced this value.
    pub fn store_id(&self) -> StoreId {
        self.store_id
    }

    /// Monotonic publication sequence, starting at zero. Encode as decimal text
    /// when transporting to JavaScript; it is independent of a domain revision.
    pub fn sequence(&self) -> u64 {
        self.sequence
    }

    /// Borrows the immutable domain state, without exposing a lock guard.
    pub fn state(&self) -> &S {
        &self.state
    }
}

impl<S> Clone for Snapshot<S> {
    fn clone(&self) -> Self {
        Self {
            store_id: self.store_id,
            sequence: self.sequence,
            state: Arc::clone(&self.state),
        }
    }
}
