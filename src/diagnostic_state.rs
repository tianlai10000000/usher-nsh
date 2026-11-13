//! Publishes the active diagnostic pipeline's live health independently of settings revisions.
use std::sync::{Arc, LazyLock, Mutex};
use usher_state::{
    Snapshot, StoreId, StoreReader, StoreWriter, Subscription, backends::WatchBackend, create_store,
};
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub(crate) struct Status {
    pub problem: Option<String>,
    pub dropped: u64,
}
struct Inner {
    writer: StoreWriter<Status>,
    source: u64,
}
pub(crate) struct StatusService {
    inner: Mutex<Inner>,
    reader: StoreReader<Status>,
}
pub(crate) static STATUS: LazyLock<Result<Arc<StatusService>, &'static str>> =
    LazyLock::new(|| StatusService::new(crate::app_state::next_store_id()?).map(Arc::new));
impl StatusService {
    pub(crate) fn new(id: StoreId) -> Result<Self, &'static str> {
        let (writer, reader) = create_store(id, Status::default(), &WatchBackend)
            .map_err(|_| "diagnostics_unavailable")?;
        Ok(Self {
            inner: Mutex::new(Inner { writer, source: 0 }),
            reader,
        })
    }
    pub(crate) fn activate(&self) -> Result<u64, &'static str> {
        let mut inner = self.inner.lock().map_err(|_| "diagnostics_unavailable")?;
        inner.source = inner
            .source
            .checked_add(1)
            .ok_or("diagnostics_unavailable")?;
        Ok(inner.source)
    }
    /// Sampling occurs while serializing publications, so a delayed producer cannot
    /// publish an older drop count after a newer one. Old logger generations are fenced.
    pub(crate) fn update(&self, source: u64, sample: impl FnOnce() -> Status) {
        if let Ok(mut inner) = self.inner.lock() {
            if source != inner.source {
                return;
            }
            let next = sample();
            if inner
                .writer
                .snapshot()
                .is_ok_and(|current| *current.state() != next)
                && inner.writer.publish(next).is_err()
            {
                inner.writer.close();
            }
        }
    }
    pub(crate) fn snapshot(&self) -> Result<Snapshot<Status>, String> {
        self.reader
            .snapshot()
            .map_err(|_| "diagnostics_unavailable".into())
    }
    pub(crate) fn subscribe(&self) -> Result<Subscription<Status>, String> {
        self.reader
            .subscribe()
            .map_err(|_| "diagnostics_unavailable".into())
    }
}
#[cfg(test)]
mod tests {
    use super::*;
    use usher_test_support::task::ready;
    #[test]
    fn live_health_and_drop_changes_publish_without_a_settings_command() {
        let service = StatusService::new(StoreId::new(u128::MAX)).unwrap();
        let source = service.activate().unwrap();
        let mut observer = service.subscribe().unwrap();
        service.update(source, || Status {
            problem: Some("diagnostics_write_failed".into()),
            dropped: 7,
        });
        let snapshot = ready(observer.next()).unwrap();
        assert_eq!(snapshot.state().dropped, 7);
        assert_eq!(snapshot.sequence(), 1);
        service.update(source, || snapshot.state().clone());
        assert_eq!(service.snapshot().unwrap().sequence(), 1);
    }
    #[test]
    fn replaced_logger_cannot_overwrite_the_new_pipeline_and_close_keeps_final_state() {
        let service = StatusService::new(StoreId::new(u128::MAX)).unwrap();
        let old = service.activate().unwrap();
        let new = service.activate().unwrap();
        service.update(new, || Status {
            problem: None,
            dropped: 2,
        });
        service.update(old, || panic!("obsolete producer must not even sample"));
        let mut observer = service.subscribe().unwrap();
        service.update(new, || Status {
            problem: None,
            dropped: 3,
        });
        drop(service);
        assert_eq!(ready(observer.next()).unwrap().state().dropped, 3);
        assert_eq!(
            ready(observer.next()).unwrap_err(),
            usher_state::SubscriptionEnd::Closed
        );
    }
}
