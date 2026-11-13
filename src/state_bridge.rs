//! Engine-local subscription forwarding with bounded delivery and explicit cancellation.

use std::{
    cell::RefCell,
    future::{Future, poll_fn},
    rc::Rc,
    task::{Poll, Waker},
};
use usher_state::{Snapshot, Subscription, SubscriptionEnd};

/// One engine binding's lifetime; it never owns or closes the shared domain service.
#[derive(Clone, Default)]
pub(crate) struct Binding(Rc<RefCell<Lifetime>>);

#[derive(Default)]
struct Lifetime {
    closed: bool,
    waiter: Option<Waker>,
}

impl Binding {
    /// Wakes an outstanding initialization, subscription wait or callback acknowledgement.
    pub(crate) fn close(&self) {
        let waiter = {
            let mut lifetime = self.0.borrow_mut();
            lifetime.closed = true;
            lifetime.waiter.take()
        };
        if let Some(waiter) = waiter {
            waiter.wake();
        }
    }

    /// Dropping the task or closing its binding drops all in-flight work, including
    /// the subscription cursor. No callback or task is sent to a worker thread.
    pub(crate) async fn run(&self, work: impl Future<Output = ()>) {
        let _finish = Finish(self.clone());
        let mut work = std::pin::pin!(work);
        poll_fn(|cx| {
            {
                let mut lifetime = self.0.borrow_mut();
                if lifetime.closed {
                    return Poll::Ready(());
                }
                lifetime.waiter = Some(cx.waker().clone());
            }
            work.as_mut().poll(cx)
        })
        .await;
    }
}

struct Finish(Binding);
impl Drop for Finish {
    fn drop(&mut self) {
        self.0.close();
    }
}

pub(crate) enum Event<S> {
    Snapshot(Snapshot<S>),
    End(SubscriptionEnd),
}

/// Registration supplies its own atomic initial snapshot. Awaiting each delivery
/// leaves at most one callback outstanding; watch coalesces changes meanwhile.
/// A final snapshot is acknowledged before the terminal event is delivered.
pub(crate) async fn forward<S, E, F, A>(
    initialize: impl Future<Output = Result<Subscription<S>, E>>,
    mut deliver: F,
) where
    S: Send + Sync + 'static,
    F: FnMut(Event<S>) -> A,
    A: Future<Output = Result<(), String>>,
{
    let mut subscription = match initialize.await {
        Ok(subscription) => subscription,
        Err(_) => {
            let _ = deliver(Event::End(SubscriptionEnd::BackendUnavailable)).await;
            return;
        }
    };
    if deliver(Event::Snapshot(subscription.initial().clone()))
        .await
        .is_err()
    {
        return;
    }
    loop {
        match subscription.next().await {
            Ok(snapshot) => {
                if deliver(Event::Snapshot(snapshot)).await.is_err() {
                    return;
                }
            }
            Err(end) => {
                let _ = deliver(Event::End(end)).await;
                return;
            }
        }
    }
}

#[cfg(test)]
mod tests;
