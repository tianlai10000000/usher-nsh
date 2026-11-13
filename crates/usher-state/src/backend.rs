//! Object-safe extension points for storage and notification adapters.

use crate::{Snapshot, StoreError, SubscriptionEnd};
use std::{future::Future, pin::Pin, sync::Arc};

/// Runtime-independent, cancellation-safe wait for the next latest value.
pub type SnapshotFuture<'a, S> =
    Pin<Box<dyn Future<Output = Result<Snapshot<S>, SubscriptionEnd>> + Send + 'a>>;

/// Initial value and its cursor, captured together by a backend.
pub type BackendRegistration<S> = (Snapshot<S>, Box<dyn BackendSubscription<S>>);

/// Constructs independent channels for each supplied initial snapshot.
pub trait BackendFactory<S>: Send + Sync {
    /// Must preserve the supplied envelope and never share writable state between
    /// calls. A failure must leave no live background tasks or registrations.
    fn create(&self, initial: Snapshot<S>) -> Result<BackendParts<S>, StoreError>;
}

/// Backend read capability. Implementations must release locks before returning.
pub trait BackendReader<S>: Send + Sync {
    /// Returns a clone of the latest envelope, including after close.
    fn snapshot(&self) -> Result<Snapshot<S>, StoreError>;

    /// Atomically registers and captures the initial value, marked as observed.
    /// A concurrent publish must appear here or in `next`, never be lost.
    /// Subscribing after close still returns the final initial snapshot.
    fn subscribe(&self) -> Result<BackendRegistration<S>, StoreError>;
}

/// Unique backend write capability. The Store, not this adapter, owns sequencing.
pub trait BackendWriter<S>: Send {
    /// Atomically replaces the envelope and notifies outside backend locks.
    /// Even without receivers the value must be retained. On Err, no value or
    /// notification may change. Do not execute user callbacks under locks.
    fn publish(&mut self, snapshot: Snapshot<S>) -> Result<(), StoreError>;

    /// Idempotently terminates notifications, preserving the readable final value.
    /// Existing subscriptions deliver an unseen final value before ending.
    fn close(&mut self);
}

/// A backend cursor. Dropping it must release registrations and retained tasks.
pub trait BackendSubscription<S>: Send {
    /// Returns a newer envelope or a terminal outcome. May coalesce intermediate
    /// values. Dropping a pending future must not consume an unseen value.
    fn next(&mut self) -> SnapshotFuture<'_, S>;
}

/// Read/write capabilities produced by a backend, hidden from normal consumers.
pub struct BackendParts<S> {
    pub(crate) reader: Arc<dyn BackendReader<S>>,
    pub(crate) writer: Box<dyn BackendWriter<S>>,
}

impl<S> BackendParts<S> {
    /// Connects capabilities belonging to the same channel and initial envelope.
    pub fn new(reader: Arc<dyn BackendReader<S>>, writer: Box<dyn BackendWriter<S>>) -> Self {
        Self { reader, writer }
    }
}
