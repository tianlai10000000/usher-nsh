//! Stable errors shared by consumers and backend implementations.

use std::fmt;

/// A rejected store operation. A failed publish must leave the previous value intact.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum StoreError {
    /// The unique writer has closed.
    Closed,
    /// The next sequence cannot be represented; no wraparound is permitted.
    SequenceExhausted,
    /// The backend could not create, read, subscribe or atomically publish.
    BackendUnavailable,
}

impl fmt::Display for StoreError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(match self {
            Self::Closed => "store_closed",
            Self::SequenceExhausted => "store_sequence_exhausted",
            Self::BackendUnavailable => "store_backend_unavailable",
        })
    }
}

impl std::error::Error for StoreError {}

/// Terminal subscription outcomes; subsequent waits return the same outcome.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SubscriptionEnd {
    /// All unseen state has been observed and the writer is closed.
    Closed,
    /// The consumer cancelled this subscription, possibly discarding unseen state.
    Cancelled,
    /// The subscription failed at its backend boundary.
    BackendUnavailable,
}

impl fmt::Display for SubscriptionEnd {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(match self {
            Self::Closed => "subscription_closed",
            Self::Cancelled => "subscription_cancelled",
            Self::BackendUnavailable => "subscription_backend_unavailable",
        })
    }
}

impl std::error::Error for SubscriptionEnd {}
