//! Fault injection and deterministic lock-isolation checks for the settings adapter.

use super::*;
use crate::{app_state::SettingsRegistry, settings::ColorMode};
use std::{
    fs,
    sync::{Arc, mpsc},
    thread,
};
use usher_state::{
    BackendParts, BackendReader, BackendRegistration, BackendWriter, Snapshot as Envelope,
    StoreError, SubscriptionEnd, backends::WatchBackend,
};
use usher_test_support::task::{poll, ready};

// Deliberately failing boundary, local to this adapter's fault-handling contract.
struct FailingFactory(Arc<AtomicBool>);
struct FrozenReader(Envelope<Snapshot>);
struct FailedWriter(Arc<AtomicBool>);

impl BackendFactory<Snapshot> for FailingFactory {
    fn create(&self, initial: Envelope<Snapshot>) -> Result<BackendParts<Snapshot>, StoreError> {
        Ok(BackendParts::new(
            Arc::new(FrozenReader(initial)),
            Box::new(FailedWriter(self.0.clone())),
        ))
    }
}
impl BackendReader<Snapshot> for FrozenReader {
    fn snapshot(&self) -> Result<Envelope<Snapshot>, StoreError> {
        Ok(self.0.clone())
    }
    fn subscribe(&self) -> Result<BackendRegistration<Snapshot>, StoreError> {
        Err(StoreError::BackendUnavailable)
    }
}
impl BackendWriter<Snapshot> for FailedWriter {
    fn publish(&mut self, _: Envelope<Snapshot>) -> Result<(), StoreError> {
        Err(StoreError::BackendUnavailable)
    }
    fn close(&mut self) {
        self.0.store(true, Ordering::Release);
    }
}

#[test]
fn publication_fault_closes_service_without_rolling_back_a_successful_write() {
    let directory = tempfile::tempdir().unwrap();
    let path = directory.path().join("settings.toml");
    let closed = Arc::new(AtomicBool::new(false));
    let service = SettingsService::new(
        SettingsStore::open(path.clone()),
        StoreId::new(u128::MAX),
        &FailingFactory(closed.clone()),
    )
    .unwrap();
    let reader = service.reader();
    let before = reader.snapshot().unwrap();
    assert_eq!(
        service.apply(Command::SetColorMode(ColorMode::Dark)),
        Err(UNAVAILABLE)
    );
    assert!(closed.load(Ordering::Acquire));
    assert_eq!(reader.snapshot().unwrap().state(), before.state());
    assert_eq!(service.snapshot(), Err(UNAVAILABLE));
    let saved = fs::read(&path).unwrap();
    assert_eq!(
        service.apply(Command::SetColorMode(ColorMode::Light)),
        Err(UNAVAILABLE)
    );
    assert_eq!(fs::read(&path).unwrap(), saved);
    let recovered = SettingsRegistry::default().for_path(path).unwrap();
    assert_eq!(recovered.snapshot().unwrap().color_mode, ColorMode::Dark);
    assert_ne!(
        recovered.reader().snapshot().unwrap().store_id(),
        before.store_id()
    );
}

#[test]
fn unwinding_a_mutation_closes_waiters_and_rejects_the_poisoned_service() {
    let service = SettingsService::session().unwrap();
    let mut sub = service.reader().subscribe().unwrap();
    let initial = sub.initial().state().clone();
    let result = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
        service.mutate(|store| {
            store.apply(Command::SetColorMode(ColorMode::Dark)).unwrap();
            panic!("injected fault between mutation and publication");
        })
    }));
    assert!(result.is_err());
    assert_eq!(ready(sub.next()).unwrap_err(), SubscriptionEnd::Closed);
    assert_eq!(service.reader().snapshot().unwrap().state(), &initial);
    assert_eq!(
        service.apply(Command::SetColorMode(ColorMode::Light)),
        Err(UNAVAILABLE)
    );
    service.close();
}

#[test]
fn a_pending_transaction_does_not_block_readers_or_another_directory() {
    let directory = tempfile::tempdir().unwrap();
    let registry = SettingsRegistry::default();
    let slow = registry
        .for_path(directory.path().join("slow/settings.toml"))
        .unwrap();
    let reader = slow.reader();
    let initial = reader.snapshot().unwrap();
    let (entered_tx, entered_rx) = mpsc::channel();
    let (release_tx, release_rx) = mpsc::channel();
    thread::scope(|scope| {
        let worker = scope.spawn(move || {
            slow.mutate(|store| {
                entered_tx.send(()).unwrap();
                release_rx.recv().unwrap();
                store.apply(Command::SetColorMode(ColorMode::Dark))
            })
        });
        entered_rx.recv().unwrap();
        assert_eq!(reader.snapshot().unwrap().state(), initial.state());
        let mut sub = reader.subscribe().unwrap();
        assert!(poll(std::pin::pin!(sub.next()), &Arc::default()).is_pending());
        let fast = registry
            .for_path(directory.path().join("fast/settings.toml"))
            .unwrap();
        fast.apply(Command::SetLocale("zh-Hans".into())).unwrap();
        release_tx.send(()).unwrap();
        let changed = worker.join().unwrap().unwrap();
        assert_eq!(ready(sub.next()).unwrap().state(), &changed);
    });
}

#[test]
fn backend_creation_failure_does_not_create_a_live_service() {
    struct Unavailable;
    impl BackendFactory<Snapshot> for Unavailable {
        fn create(&self, _: Envelope<Snapshot>) -> Result<BackendParts<Snapshot>, StoreError> {
            Err(StoreError::BackendUnavailable)
        }
    }
    assert!(matches!(
        SettingsService::new(
            SettingsStore::session_only(),
            StoreId::new(u128::MAX),
            &Unavailable
        ),
        Err(UNAVAILABLE)
    ));
    // Confirm the same domain can be assembled with the configured production backend.
    assert!(
        SettingsService::new(
            SettingsStore::session_only(),
            StoreId::new(u128::MAX),
            &WatchBackend
        )
        .is_ok()
    );
}
