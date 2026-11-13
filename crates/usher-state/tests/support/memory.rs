//! Test-only reference backend with controlled faults and explicit wake registration.

use std::{
    future::poll_fn,
    sync::{
        Arc, Mutex, Weak,
        atomic::{AtomicBool, AtomicUsize, Ordering},
    },
    task::{Poll, Waker},
};
use usher_state::{
    BackendFactory, BackendParts, BackendReader, BackendRegistration, BackendSubscription,
    BackendWriter, Snapshot, SnapshotFuture, StoreError, SubscriptionEnd,
};

#[derive(Default)]
pub struct Control {
    pub fail_create: AtomicBool,
    pub fail_read: AtomicBool,
    pub fail_subscribe: AtomicBool,
    pub fail_publish: AtomicBool,
    pub fail_wait: AtomicBool,
    pub cursors: AtomicUsize,
}

#[derive(Default)]
pub struct MemoryFactory(pub Arc<Control>);

type Slot = Mutex<Option<Waker>>;
struct WaitRegistration(Arc<Slot>);

impl Drop for WaitRegistration {
    fn drop(&mut self) {
        let old = self.0.lock().unwrap().take();
        drop(old);
    }
}
struct Shared<S> {
    value: Snapshot<S>,
    closed: bool,
    slots: Vec<Weak<Slot>>,
}
struct Reader<S> {
    shared: Arc<Mutex<Shared<S>>>,
    control: Arc<Control>,
}
struct Writer<S> {
    shared: Arc<Mutex<Shared<S>>>,
    control: Arc<Control>,
}
struct Cursor<S> {
    shared: Arc<Mutex<Shared<S>>>,
    control: Arc<Control>,
    slot: Arc<Slot>,
    seen: u64,
}

impl<S: Send + Sync + 'static> BackendFactory<S> for MemoryFactory {
    fn create(&self, initial: Snapshot<S>) -> Result<BackendParts<S>, StoreError> {
        if self.0.fail_create.load(Ordering::SeqCst) {
            return Err(StoreError::BackendUnavailable);
        }
        let shared = Arc::new(Mutex::new(Shared {
            value: initial,
            closed: false,
            slots: vec![],
        }));
        Ok(BackendParts::new(
            Arc::new(Reader {
                shared: shared.clone(),
                control: self.0.clone(),
            }),
            Box::new(Writer {
                shared,
                control: self.0.clone(),
            }),
        ))
    }
}

impl<S: Send + Sync + 'static> BackendReader<S> for Reader<S> {
    fn snapshot(&self) -> Result<Snapshot<S>, StoreError> {
        if self.control.fail_read.load(Ordering::SeqCst) {
            return Err(StoreError::BackendUnavailable);
        }
        Ok(self.shared.lock().unwrap().value.clone())
    }

    fn subscribe(&self) -> Result<BackendRegistration<S>, StoreError> {
        if self.control.fail_subscribe.load(Ordering::SeqCst) {
            return Err(StoreError::BackendUnavailable);
        }
        let mut shared = self.shared.lock().unwrap();
        let initial = shared.value.clone();
        let slot = Arc::new(Mutex::new(None));
        shared.slots.retain(|slot| slot.strong_count() != 0);
        shared.slots.push(Arc::downgrade(&slot));
        self.control.cursors.fetch_add(1, Ordering::SeqCst);
        let cursor = Cursor {
            shared: self.shared.clone(),
            control: self.control.clone(),
            slot,
            seen: initial.sequence(),
        };
        Ok((initial, Box::new(cursor)))
    }
}

fn wake(slots: Vec<Arc<Slot>>) {
    for slot in slots {
        let waker = slot.lock().unwrap().take();
        if let Some(waker) = waker {
            waker.wake();
        }
    }
}

impl<S: Send + Sync + 'static> BackendWriter<S> for Writer<S> {
    fn publish(&mut self, snapshot: Snapshot<S>) -> Result<(), StoreError> {
        if self.control.fail_publish.load(Ordering::SeqCst) {
            return Err(StoreError::BackendUnavailable);
        }
        let (old, slots) = {
            let mut shared = self.shared.lock().unwrap();
            if shared.closed {
                return Err(StoreError::Closed);
            }
            let old = std::mem::replace(&mut shared.value, snapshot);
            shared.slots.retain(|slot| slot.strong_count() != 0);
            (old, shared.slots.iter().filter_map(Weak::upgrade).collect())
        };
        drop(old);
        wake(slots);
        Ok(())
    }

    fn close(&mut self) {
        let slots = {
            let mut shared = self.shared.lock().unwrap();
            shared.closed = true;
            shared.slots.iter().filter_map(Weak::upgrade).collect()
        };
        wake(slots);
    }
}

impl<S: Send + Sync + 'static> BackendSubscription<S> for Cursor<S> {
    fn next(&mut self) -> SnapshotFuture<'_, S> {
        Box::pin(async {
            let _registration = WaitRegistration(self.slot.clone());
            poll_fn(|cx| {
                if self.control.fail_wait.load(Ordering::SeqCst) {
                    return Poll::Ready(Err(SubscriptionEnd::BackendUnavailable));
                }
                let waker = cx.waker().clone();
                let shared = self.shared.lock().unwrap();
                if shared.value.sequence() != self.seen {
                    self.seen = shared.value.sequence();
                    return Poll::Ready(Ok(shared.value.clone()));
                }
                if shared.closed {
                    return Poll::Ready(Err(SubscriptionEnd::Closed));
                }
                let old = self.slot.lock().unwrap().replace(waker);
                drop(shared);
                drop(old);
                Poll::Pending
            })
            .await
        })
    }
}

impl<S> Drop for Cursor<S> {
    fn drop(&mut self) {
        self.control.cursors.fetch_sub(1, Ordering::SeqCst);
    }
}
