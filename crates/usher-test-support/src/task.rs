//! Deterministic single-poll assertions and observable wakeups without an executor.

use std::{
    future::Future,
    pin::Pin,
    sync::{
        Arc,
        atomic::{AtomicUsize, Ordering},
    },
    task::{Context, Poll, Wake, Waker},
};

/// A per-test wake observer. Share its Arc between a poller and assertions.
/// Observing a wake does not poll or schedule the future.
#[derive(Default)]
pub struct WakeCounter(AtomicUsize);

impl WakeCounter {
    /// Returns the number of wake calls observed so far, including wake_by_ref.
    pub fn count(&self) -> usize {
        self.0.load(Ordering::SeqCst)
    }
}

impl Wake for WakeCounter {
    fn wake(self: Arc<Self>) {
        self.0.fetch_add(1, Ordering::SeqCst);
    }

    fn wake_by_ref(self: &Arc<Self>) {
        self.0.fetch_add(1, Ordering::SeqCst);
    }
}

/// Polls exactly once with the supplied wake observer, preserving Ready/Pending.
/// The caller owns pinning and decides when to poll again; no runtime is started.
/// Any waker retained by the future belongs to that future, not this helper.
pub fn poll<F: Future + ?Sized>(future: Pin<&mut F>, wakes: &Arc<WakeCounter>) -> Poll<F::Output> {
    let waker = Waker::from(wakes.clone());
    future.poll(&mut Context::from_waker(&waker))
}

/// Asserts that a future completes on its first poll and returns its output.
///
/// # Panics
/// Panics on Pending, even if the future wakes itself. This is an assertion,
/// not block_on: it never retries, sleeps or waits for background work.
#[track_caller]
pub fn ready<F: Future>(future: F) -> F::Output {
    let mut future = std::pin::pin!(future);
    match poll(future.as_mut(), &Arc::default()) {
        Poll::Ready(value) => value,
        Poll::Pending => panic!("expected a deterministically ready future"),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::{future::poll_fn, marker::PhantomPinned};

    #[test]
    fn wake_observer_is_shared_across_threads_and_wake_methods() {
        let wakes = Arc::new(WakeCounter::default());
        let unrelated = WakeCounter::default();
        let before = wakes.count();
        let waker = Waker::from(wakes.clone());
        waker.wake_by_ref();
        assert_eq!(wakes.count(), before + 1);
        let before = wakes.count();
        std::thread::spawn(move || waker.wake()).join().unwrap();
        assert_eq!(wakes.count(), before + 1);
        assert_eq!(unrelated.count(), usize::default());
    }

    #[test]
    fn poll_preserves_pending_without_retrying_and_releases_its_waker() {
        let wakes = Arc::new(WakeCounter::default());
        let ownership = Arc::strong_count(&wakes);
        let mut polled = false;
        let mut future = std::pin::pin!(poll_fn(|cx| {
            assert!(!polled, "one helper call must never poll twice");
            polled = true;
            cx.waker().wake_by_ref();
            Poll::<()>::Pending
        }));
        let before = wakes.count();
        assert!(poll(future.as_mut(), &wakes).is_pending());
        assert_eq!(wakes.count(), before + 1);
        assert_eq!(Arc::strong_count(&wakes), ownership);
    }

    #[test]
    fn ready_accepts_non_unpin_futures_and_preserves_outputs() {
        struct PinnedReady(PhantomPinned);
        impl Future for PinnedReady {
            type Output = Arc<()>;
            fn poll(self: Pin<&mut Self>, _: &mut Context<'_>) -> Poll<Self::Output> {
                Poll::Ready(Arc::new(()))
            }
        }
        let value = ready(PinnedReady(PhantomPinned));
        let returned = ready(std::future::ready(value.clone()));
        assert!(Arc::ptr_eq(&value, &returned));
        assert_eq!(ready(std::future::ready(Err::<(), _>(()))), Err(()));
    }

    #[test]
    #[should_panic(expected = "expected a deterministically ready future")]
    fn ready_rejects_self_waking_pending_futures() {
        ready(poll_fn(|cx| {
            cx.waker().wake_by_ref();
            Poll::<()>::Pending
        }));
    }
}
