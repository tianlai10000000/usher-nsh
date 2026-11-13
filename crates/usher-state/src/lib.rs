//! Framework-independent state transitions, snapshots and latest-value subscriptions.
//!
//! Inject a [`BackendFactory`] at construction. Domain services own the unique
//! [`StoreWriter`], compute transitions and execute effects only after a successful
//! commit. Consumers use [`StoreReader`] and [`Subscription`]. State must be immutable
//! after publication: interior mutability must not bypass the writer.
//!
//! This crate neither allocates application identities nor runs background tasks.
//! `backend-watch` enables the optional Tokio sync adapter without a runtime.

mod backend;
mod error;
mod snapshot;
mod store;
mod subscription;
mod transition;

#[cfg(feature = "backend-watch")]
pub mod backends;

pub use backend::{
    BackendFactory, BackendParts, BackendReader, BackendRegistration, BackendSubscription,
    BackendWriter, SnapshotFuture,
};
pub use error::{StoreError, SubscriptionEnd};
pub use snapshot::{Snapshot, StoreId};
pub use store::{Commit, StoreReader, StoreWriter, create_store};
pub use subscription::{CancellationHandle, Subscription};
pub use transition::Transition;
