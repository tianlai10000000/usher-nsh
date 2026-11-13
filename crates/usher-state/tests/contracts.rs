//! Identical consumer contracts executed against injected reference and watch backends.

mod support;

use std::{
    sync::{Arc, Barrier, atomic::Ordering},
    task::Poll,
};
use support::{Machine, fixtures};
use usher_state::{
    BackendFactory, Snapshot, StateMachine, StoreError, StoreId, SubscriptionEnd, Transition,
    create_store,
};
use usher_test_support::task::{WakeCounter, poll, ready};

fn assert_snapshot(snapshot: &Snapshot<i64>, value: i64, sequence: u64) {
    assert_eq!(*snapshot.state(), value);
    assert_eq!(snapshot.sequence(), sequence);
    assert_eq!(snapshot.store_id(), StoreId::new(fixtures().store_ids[0]));
}

fn domain_scenarios(factory: &dyn BackendFactory<i64>) {
    let data = fixtures();
    for scenario in data.scenarios {
        let (mut writer, reader) =
            create_store(StoreId::new(data.store_ids[0]), data.initial, factory).unwrap();
        let mut sub = reader.subscribe().unwrap();
        let wakes = Arc::new(WakeCounter::default());
        for action in scenario.actions {
            let before = writer.snapshot().unwrap();
            let decision = Machine.transition(before.state(), action.command);
            assert_eq!(decision.is_err(), action.rejected, "{}", scenario.case_id);
            if let Ok(transition) = decision {
                let commit = writer.apply(transition).unwrap();
                assert_eq!(commit.effects, action.effects);
                assert_snapshot(&commit.snapshot, action.state, action.sequence);
            }
            assert_snapshot(&reader.snapshot().unwrap(), action.state, action.sequence);
            if action.sequence != before.sequence() {
                assert_snapshot(&ready(sub.next()).unwrap(), action.state, action.sequence);
            } else {
                assert!(poll(std::pin::pin!(sub.next()), &wakes).is_pending());
            }
        }
    }
}

fn latest_without_consumers(factory: &dyn BackendFactory<i64>) {
    let data = fixtures();
    let (mut writer, reader) =
        create_store(StoreId::new(data.store_ids[0]), data.initial, factory).unwrap();
    for value in &data.published {
        writer.publish(*value).unwrap();
    }
    let mut sub = reader.subscribe().unwrap();
    assert_snapshot(
        sub.initial(),
        *data.published.last().unwrap(),
        data.published.len() as u64,
    );
    assert!(poll(std::pin::pin!(sub.next()), &Arc::default()).is_pending());
    drop(sub);
    writer.publish(data.initial).unwrap();
    assert_eq!(*reader.subscribe().unwrap().initial().state(), data.initial);
}

fn slow_and_fast_consumers(factory: &dyn BackendFactory<i64>) {
    let data = fixtures();
    let (mut writer, reader) =
        create_store(StoreId::new(data.store_ids[0]), data.initial, factory).unwrap();
    let mut slow = reader.subscribe().unwrap();
    let mut fast = reader.subscribe().unwrap();
    let initial = slow.initial().clone();
    for (index, value) in data.published.iter().enumerate() {
        writer.publish(*value).unwrap();
        assert_snapshot(&ready(fast.next()).unwrap(), *value, index as u64 + 1);
    }
    assert_snapshot(
        &ready(slow.next()).unwrap(),
        *data.published.last().unwrap(),
        data.published.len() as u64,
    );
    assert_snapshot(&initial, data.initial, 0);
}

fn pending_publish_wakes(factory: &dyn BackendFactory<i64>) {
    let data = fixtures();
    let (mut writer, reader) =
        create_store(StoreId::new(data.store_ids[0]), data.initial, factory).unwrap();
    let mut sub = reader.subscribe().unwrap();
    let wakes = Arc::new(WakeCounter::default());
    let mut next = Box::pin(sub.next());
    assert!(poll(next.as_mut(), &wakes).is_pending());
    writer.publish(data.published[0]).unwrap();
    assert!(wakes.count() > 0);
    let Poll::Ready(Ok(snapshot)) = poll(next.as_mut(), &wakes) else {
        panic!("publish must wake a ready waiter")
    };
    assert_snapshot(&snapshot, data.published[0], 1);
}

fn final_value_then_close(factory: &dyn BackendFactory<i64>) {
    let data = fixtures();
    let (mut writer, reader) =
        create_store(StoreId::new(data.store_ids[0]), data.initial, factory).unwrap();
    let mut sub = reader.subscribe().unwrap();
    writer.publish(data.published[0]).unwrap();
    writer.close();
    writer.close();
    assert_eq!(
        writer.publish(data.initial).unwrap_err(),
        StoreError::Closed
    );
    assert_eq!(
        writer
            .apply(Transition::Unchanged { effects: vec![()] })
            .unwrap_err(),
        StoreError::Closed
    );
    assert_snapshot(&ready(sub.next()).unwrap(), data.published[0], 1);
    assert_eq!(ready(sub.next()).unwrap_err(), SubscriptionEnd::Closed);
    sub.cancel();
    assert_eq!(ready(sub.next()).unwrap_err(), SubscriptionEnd::Closed);
    let mut late = reader.subscribe().unwrap();
    assert_snapshot(late.initial(), data.published[0], 1);
    assert_eq!(ready(late.next()).unwrap_err(), SubscriptionEnd::Closed);
    assert_snapshot(&reader.snapshot().unwrap(), data.published[0], 1);
}

fn drop_writer_wakes(factory: &dyn BackendFactory<i64>) {
    let data = fixtures();
    let (writer, reader) =
        create_store(StoreId::new(data.store_ids[0]), data.initial, factory).unwrap();
    let mut sub = reader.subscribe().unwrap();
    let wakes = Arc::new(WakeCounter::default());
    let mut next = Box::pin(sub.next());
    assert!(poll(next.as_mut(), &wakes).is_pending());
    drop(writer);
    assert!(wakes.count() > 0);
    assert!(matches!(
        poll(next.as_mut(), &wakes),
        Poll::Ready(Err(SubscriptionEnd::Closed))
    ));
}

fn cancellation_is_local(factory: &dyn BackendFactory<i64>) {
    let data = fixtures();
    let (mut writer, reader) =
        create_store(StoreId::new(data.store_ids[0]), data.initial, factory).unwrap();
    let mut cancelled = reader.subscribe().unwrap();
    let mut live = reader.subscribe().unwrap();
    let handle = cancelled.cancellation_handle();
    let wakes = Arc::new(WakeCounter::default());
    {
        let mut next = Box::pin(cancelled.next());
        assert!(poll(next.as_mut(), &wakes).is_pending());
        std::thread::spawn(move || {
            handle.cancel();
            handle.cancel();
        })
        .join()
        .unwrap();
        assert!(wakes.count() > 0);
        assert!(matches!(
            poll(next.as_mut(), &wakes),
            Poll::Ready(Err(SubscriptionEnd::Cancelled))
        ));
    }
    writer.publish(data.published[0]).unwrap();
    assert_eq!(
        ready(cancelled.next()).unwrap_err(),
        SubscriptionEnd::Cancelled
    );
    assert_snapshot(&ready(live.next()).unwrap(), data.published[0], 1);
    live.cancel();
    live.cancel();
    assert_eq!(ready(live.next()).unwrap_err(), SubscriptionEnd::Cancelled);
}

fn dropped_wait_does_not_consume(factory: &dyn BackendFactory<i64>) {
    let data = fixtures();
    let (mut writer, reader) =
        create_store(StoreId::new(data.store_ids[0]), data.initial, factory).unwrap();
    let mut sub = reader.subscribe().unwrap();
    assert!(poll(std::pin::pin!(sub.next()), &Arc::default()).is_pending());
    writer.publish(data.published[0]).unwrap();
    assert_snapshot(&ready(sub.next()).unwrap(), data.published[0], 1);
}

fn dropped_wait_releases_task(factory: &dyn BackendFactory<i64>) {
    let data = fixtures();
    let (_writer, reader) =
        create_store(StoreId::new(data.store_ids[0]), data.initial, factory).unwrap();
    let mut sub = reader.subscribe().unwrap();
    let wakes = Arc::new(WakeCounter::default());
    let weak = Arc::downgrade(&wakes);
    assert!(poll(std::pin::pin!(sub.next()), &wakes).is_pending());
    drop(wakes);
    assert!(
        weak.upgrade().is_none(),
        "dropping a wait releases its task even while subscribed"
    );
}

fn cancellation_races_publish(factory: &dyn BackendFactory<i64>) {
    let data = fixtures();
    let (mut writer, reader) =
        create_store(StoreId::new(data.store_ids[0]), data.initial, factory).unwrap();
    let mut sub = reader.subscribe().unwrap();
    let cancel = sub.cancellation_handle();
    let gate = Arc::new(Barrier::new(2));
    let publish_gate = gate.clone();
    let wakes = Arc::new(WakeCounter::default());
    let mut next = Box::pin(sub.next());
    assert!(poll(next.as_mut(), &wakes).is_pending());
    let value = data.published[0];
    let producer = std::thread::spawn(move || {
        publish_gate.wait();
        writer.publish(value).unwrap();
        writer
    });
    let canceller = std::thread::spawn(move || {
        gate.wait();
        cancel.cancel();
    });
    let _writer = producer.join().unwrap();
    canceller.join().unwrap();
    assert!(matches!(
        poll(next.as_mut(), &wakes),
        Poll::Ready(Err(SubscriptionEnd::Cancelled))
    ));
    assert_snapshot(&reader.snapshot().unwrap(), value, 1);
}

fn reentrant_wake_can_read(factory: &dyn BackendFactory<i64>) {
    struct ReadOnWake(usher_state::StoreReader<i64>);
    impl std::task::Wake for ReadOnWake {
        fn wake(self: Arc<Self>) {
            self.0.snapshot().unwrap();
        }
    }
    let data = fixtures();
    let (mut writer, reader) =
        create_store(StoreId::new(data.store_ids[0]), data.initial, factory).unwrap();
    let mut sub = reader.subscribe().unwrap();
    let waker = std::task::Waker::from(Arc::new(ReadOnWake(reader)));
    let mut cx = std::task::Context::from_waker(&waker);
    let mut next = Box::pin(sub.next());
    assert!(std::future::Future::poll(next.as_mut(), &mut cx).is_pending());
    // A backend holding its value lock while waking would deadlock this call.
    writer.publish(data.published[0]).unwrap();
    assert!(std::future::Future::poll(next.as_mut(), &mut cx).is_ready());
}

fn serialized_command_sources(factory: &dyn BackendFactory<i64>) {
    let data = fixtures();
    let (writer, reader) =
        create_store(StoreId::new(data.store_ids[0]), data.initial, factory).unwrap();
    let writer = Arc::new(std::sync::Mutex::new(writer));
    let gate = Arc::new(Barrier::new(data.published.len()));
    let mut sub = reader.subscribe().unwrap();
    let threads: Vec<_> = data
        .published
        .into_iter()
        .map(|value| {
            let writer = writer.clone();
            let gate = gate.clone();
            std::thread::spawn(move || {
                gate.wait();
                writer.lock().unwrap().publish(value).unwrap()
            })
        })
        .collect();
    let mut committed: Vec<_> = threads
        .into_iter()
        .map(|thread| thread.join().unwrap())
        .collect();
    committed.sort_by_key(Snapshot::sequence);
    for (index, snapshot) in committed.iter().enumerate() {
        assert_eq!(snapshot.sequence(), index as u64 + 1);
    }
    let final_snapshot = committed.last().unwrap();
    assert_snapshot(
        &ready(sub.next()).unwrap(),
        *final_snapshot.state(),
        final_snapshot.sequence(),
    );
}

fn registration_races_publish(factory: &dyn BackendFactory<i64>) {
    let data = fixtures();
    // Repeat for every explicit fixture value, without timing assumptions.
    for value in data.published {
        let (mut writer, reader) =
            create_store(StoreId::new(data.store_ids[0]), data.initial, factory).unwrap();
        let barrier = Arc::new(Barrier::new(2));
        let producer_gate = barrier.clone();
        let producer = std::thread::spawn(move || {
            producer_gate.wait();
            writer.publish(value).unwrap();
            writer
        });
        barrier.wait();
        let mut sub = reader.subscribe().unwrap();
        let _writer = producer.join().unwrap();
        let observed = if sub.initial().sequence() == 0 {
            ready(sub.next()).unwrap()
        } else {
            sub.initial().clone()
        };
        assert_snapshot(&observed, value, 1);
    }
}

fn stores_are_isolated(factory: &dyn BackendFactory<i64>) {
    let data = fixtures();
    let (mut first, first_reader) =
        create_store(StoreId::new(data.store_ids[0]), data.initial, factory).unwrap();
    let (_second, second_reader) =
        create_store(StoreId::new(data.store_ids[1]), data.initial, factory).unwrap();
    first.publish(data.published[0]).unwrap();
    assert_snapshot(&first_reader.snapshot().unwrap(), data.published[0], 1);
    assert_eq!(*second_reader.snapshot().unwrap().state(), data.initial);
    assert_eq!(second_reader.snapshot().unwrap().sequence(), 0);
    assert_ne!(
        first_reader.snapshot().unwrap().store_id(),
        second_reader.snapshot().unwrap().store_id()
    );
}

struct NonCloneState;

fn immutable_handles_do_not_require_clone(factory: &dyn BackendFactory<NonCloneState>) {
    fn is_send<T: Send>(_: &T) {}
    fn is_sync<T: Sync>(_: &T) {}
    let (writer, reader) = create_store(
        StoreId::new(fixtures().store_ids[0]),
        NonCloneState,
        factory,
    )
    .unwrap();
    let snapshot = reader.clone().snapshot().unwrap().clone();
    let mut sub = reader.subscribe().unwrap();
    is_send(&writer);
    is_sync(&reader);
    is_sync(&snapshot);
    is_send(&sub.next());
}

macro_rules! contracts {
    ($factory:expr) => {
        #[test]
        fn domain_scenarios() {
            super::domain_scenarios(&$factory);
        }
        #[test]
        fn latest_without_consumers() {
            super::latest_without_consumers(&$factory);
        }
        #[test]
        fn slow_and_fast_consumers() {
            super::slow_and_fast_consumers(&$factory);
        }
        #[test]
        fn pending_publish_wakes() {
            super::pending_publish_wakes(&$factory);
        }
        #[test]
        fn final_value_then_close() {
            super::final_value_then_close(&$factory);
        }
        #[test]
        fn drop_writer_wakes() {
            super::drop_writer_wakes(&$factory);
        }
        #[test]
        fn cancellation_is_local() {
            super::cancellation_is_local(&$factory);
        }
        #[test]
        fn dropped_wait_does_not_consume() {
            super::dropped_wait_does_not_consume(&$factory);
        }
        #[test]
        fn registration_races_publish() {
            super::registration_races_publish(&$factory);
        }
        #[test]
        fn stores_are_isolated() {
            super::stores_are_isolated(&$factory);
        }
        #[test]
        fn immutable_handles_do_not_require_clone() {
            super::immutable_handles_do_not_require_clone(&$factory);
        }
        #[test]
        fn dropped_wait_releases_task() {
            super::dropped_wait_releases_task(&$factory);
        }
        #[test]
        fn cancellation_races_publish() {
            super::cancellation_races_publish(&$factory);
        }
        #[test]
        fn reentrant_wake_can_read() {
            super::reentrant_wake_can_read(&$factory);
        }
        #[test]
        fn serialized_command_sources() {
            super::serialized_command_sources(&$factory);
        }
    };
}

mod reference {
    contracts!(super::support::memory::MemoryFactory::default());
}
#[cfg(feature = "backend-watch")]
mod watch {
    contracts!(usher_state::backends::WatchBackend);
}

#[test]
fn backend_failures_preserve_state_sequence_and_effects() {
    let data = fixtures();
    let factory = support::memory::MemoryFactory::default();
    factory.0.fail_create.store(true, Ordering::SeqCst);
    assert!(matches!(
        create_store(StoreId::new(data.store_ids[0]), data.initial, &factory),
        Err(StoreError::BackendUnavailable)
    ));
    factory.0.fail_create.store(false, Ordering::SeqCst);
    let (mut writer, reader) =
        create_store(StoreId::new(data.store_ids[0]), data.initial, &factory).unwrap();
    let mut sub = reader.subscribe().unwrap();
    factory.0.fail_publish.store(true, Ordering::SeqCst);
    let result = writer.apply(Transition::Changed {
        state: data.published[0],
        effects: vec!["must_not_escape"],
    });
    assert_eq!(result.unwrap_err(), StoreError::BackendUnavailable);
    assert_snapshot(&reader.snapshot().unwrap(), data.initial, 0);
    assert!(poll(std::pin::pin!(sub.next()), &Arc::default()).is_pending());
    factory.0.fail_publish.store(false, Ordering::SeqCst);
    assert_snapshot(
        &writer.publish(data.published[0]).unwrap(),
        data.published[0],
        1,
    );
    assert_snapshot(&ready(sub.next()).unwrap(), data.published[0], 1);
    factory.0.fail_read.store(true, Ordering::SeqCst);
    assert_eq!(
        reader.snapshot().unwrap_err(),
        StoreError::BackendUnavailable
    );
    factory.0.fail_subscribe.store(true, Ordering::SeqCst);
    assert!(matches!(
        reader.subscribe(),
        Err(StoreError::BackendUnavailable)
    ));
    factory.0.fail_wait.store(true, Ordering::SeqCst);
    assert_eq!(
        ready(sub.next()).unwrap_err(),
        SubscriptionEnd::BackendUnavailable
    );
    factory.0.fail_wait.store(false, Ordering::SeqCst);
    assert_eq!(
        ready(sub.next()).unwrap_err(),
        SubscriptionEnd::BackendUnavailable
    );
    assert_eq!(factory.0.cursors.load(Ordering::SeqCst), 0);
}

#[test]
fn cancellation_and_drop_release_registrations() {
    let data = fixtures();
    let factory = support::memory::MemoryFactory::default();
    let (_writer, reader) =
        create_store(StoreId::new(data.store_ids[0]), data.initial, &factory).unwrap();
    let mut sub = reader.subscribe().unwrap();
    let wakes = Arc::new(WakeCounter::default());
    let weak_wakes = Arc::downgrade(&wakes);
    assert!(poll(std::pin::pin!(sub.next()), &wakes).is_pending());
    sub.cancel();
    drop(wakes);
    assert_eq!(factory.0.cursors.load(Ordering::SeqCst), 0);
    assert!(weak_wakes.upgrade().is_none());
    let sub = reader.subscribe().unwrap();
    assert_eq!(factory.0.cursors.load(Ordering::SeqCst), 1);
    drop(sub);
    assert_eq!(factory.0.cursors.load(Ordering::SeqCst), 0);
}
