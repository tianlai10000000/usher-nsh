//! Tokio watch adapter with retained values and no runtime or background tasks.

use crate::{
    BackendFactory, BackendParts, BackendReader, BackendRegistration, BackendSubscription,
    BackendWriter, Snapshot, SnapshotFuture, StoreError, SubscriptionEnd,
};
use std::sync::Arc;
use tokio::sync::watch;

/// Latest-value backend. Only Tokio's `sync` feature is used.
#[derive(Debug, Default)]
pub struct WatchBackend;

struct Reader<S> {
    receiver: watch::Receiver<Snapshot<S>>,
}
struct Writer<S> {
    sender: Option<watch::Sender<Snapshot<S>>>,
}
struct Cursor<S> {
    receiver: watch::Receiver<Snapshot<S>>,
}

impl<S: Send + Sync + 'static> BackendFactory<S> for WatchBackend {
    fn create(&self, initial: Snapshot<S>) -> Result<BackendParts<S>, StoreError> {
        let (sender, receiver) = watch::channel(initial);
        Ok(BackendParts::new(
            Arc::new(Reader { receiver }),
            Box::new(Writer {
                sender: Some(sender),
            }),
        ))
    }
}

impl<S: Send + Sync + 'static> BackendReader<S> for Reader<S> {
    fn snapshot(&self) -> Result<Snapshot<S>, StoreError> {
        Ok(self.receiver.borrow().clone())
    }

    fn subscribe(&self) -> Result<BackendRegistration<S>, StoreError> {
        let mut receiver = self.receiver.clone();
        let initial = receiver.borrow_and_update().clone();
        Ok((initial, Box::new(Cursor { receiver })))
    }
}

impl<S: Send + Sync + 'static> BackendWriter<S> for Writer<S> {
    fn publish(&mut self, snapshot: Snapshot<S>) -> Result<(), StoreError> {
        let sender = self.sender.as_ref().ok_or(StoreError::Closed)?;
        // send_replace retains the value without subscribers and releases the
        // channel lock before dropping the previous snapshot here.
        sender.send_replace(snapshot);
        Ok(())
    }

    fn close(&mut self) {
        self.sender = None;
    }
}

impl<S: Send + Sync + 'static> BackendSubscription<S> for Cursor<S> {
    fn next(&mut self) -> SnapshotFuture<'_, S> {
        Box::pin(async {
            self.receiver
                .changed()
                .await
                .map_err(|_| SubscriptionEnd::Closed)?;
            // Mark exactly the cloned value seen, including a publish that raced
            // with changed(). No watch borrow survives this expression.
            Ok(self.receiver.borrow_and_update().clone())
        })
    }
}
