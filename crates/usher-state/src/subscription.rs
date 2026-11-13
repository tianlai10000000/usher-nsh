//! Per-consumer cancellation that wakes pending waits without backend-specific types.

use crate::{BackendSubscription, Snapshot, SubscriptionEnd};
use std::{
    future::poll_fn,
    sync::{Arc, Mutex},
    task::{Poll, Waker},
};

#[derive(Default)]
struct Cancellation {
    cancelled: bool,
    waker: Option<Waker>,
}

/// Cloneable cancellation authority for one subscription, including an in-flight
/// wait on another thread. Cancellation never closes the shared store.
#[derive(Clone, Default)]
pub struct CancellationHandle {
    state: Arc<Mutex<Cancellation>>,
}

impl CancellationHandle {
    /// Idempotently cancels and wakes the current waiter outside the lock.
    pub fn cancel(&self) {
        let waker = {
            // No user code runs while this private lock is held.
            let mut state = self.state.lock().unwrap_or_else(|e| e.into_inner());
            state.cancelled = true;
            state.waker.take()
        };
        if let Some(waker) = waker {
            waker.wake();
        }
    }
}

struct Registration(CancellationHandle);

impl Drop for Registration {
    fn drop(&mut self) {
        let old = self
            .0
            .state
            .lock()
            .unwrap_or_else(|e| e.into_inner())
            .waker
            .take();
        drop(old);
    }
}

/// Initial snapshot plus a latest-value cursor. No intermediate-event guarantee.
/// Dropping a wait leaves the subscription usable; dropping the subscription
/// cancels it and releases backend resources.
pub struct Subscription<S> {
    initial: Snapshot<S>,
    backend: Option<Box<dyn BackendSubscription<S>>>,
    cancellation: CancellationHandle,
    end: Option<SubscriptionEnd>,
}

impl<S: Send + Sync + 'static> Subscription<S> {
    pub(crate) fn new(initial: Snapshot<S>, backend: Box<dyn BackendSubscription<S>>) -> Self {
        Self {
            initial,
            backend: Some(backend),
            cancellation: CancellationHandle::default(),
            end: None,
        }
    }

    /// The value atomically captured and marked observed during registration.
    /// This is stable; use `next` for changes after registration.
    pub fn initial(&self) -> &Snapshot<S> {
        &self.initial
    }

    /// Obtains authority to interrupt an in-flight wait without borrowing it.
    pub fn cancellation_handle(&self) -> CancellationHandle {
        self.cancellation.clone()
    }

    /// Cancels and immediately releases the backend cursor. A detached handle
    /// instead releases it when the pending wait observes cancellation.
    pub fn cancel(&mut self) {
        self.cancellation.cancel();
        self.backend = None;
        if self.end.is_none() {
            self.end = Some(SubscriptionEnd::Cancelled);
        }
    }

    /// Waits for the latest unseen value, or returns a stable terminal outcome.
    /// Cancellation takes priority over unread values. Close delivers an unseen
    /// final value first. This future is Send and does not require a runtime.
    pub async fn next(&mut self) -> Result<Snapshot<S>, SubscriptionEnd> {
        if let Some(end) = self.end {
            return Err(end);
        }
        let registration = Registration(self.cancellation.clone());
        let result = {
            // A live subscription always owns its cursor; terminal paths set end.
            let mut next = self
                .backend
                .as_mut()
                .expect("live subscription owns a cursor")
                .next();
            poll_fn(|cx| {
                let waker = cx.waker().clone();
                let old = {
                    let mut state = registration
                        .0
                        .state
                        .lock()
                        .unwrap_or_else(|e| e.into_inner());
                    if state.cancelled {
                        return Poll::Ready(Err(SubscriptionEnd::Cancelled));
                    }
                    state.waker.replace(waker)
                };
                drop(old);
                next.as_mut().poll(cx)
            })
            .await
        };
        if let Err(end) = result {
            self.end = Some(end);
            self.backend = None;
        }
        result
    }
}

impl<S> Drop for Subscription<S> {
    fn drop(&mut self) {
        self.cancellation.cancel();
    }
}
