//! Deterministic bridge lifecycle, backpressure and terminal-event contract tests.

use super::*;
use std::{cell::Cell, sync::Arc};
use usher_state::{StoreId, backends::WatchBackend, create_store};
use usher_test_support::task::{WakeCounter, poll, ready};

#[derive(Debug, PartialEq)]
enum Seen {
    Value(u64, u8),
    End(SubscriptionEnd),
}
fn record(events: &RefCell<Vec<Seen>>, event: Event<u8>) {
    events.borrow_mut().push(match event {
        Event::Snapshot(value) => Seen::Value(value.sequence(), *value.state()),
        Event::End(end) => Seen::End(end),
    });
}
fn store() -> (usher_state::StoreWriter<u8>, usher_state::StoreReader<u8>) {
    create_store(StoreId::new(u128::MAX), u8::default(), &WatchBackend).unwrap()
}

#[test]
fn initial_registration_and_slow_callback_coalesce_without_losing_final_or_close() {
    let (mut writer, reader) = store();
    let subscription = reader.subscribe().unwrap();
    writer.publish(1).unwrap();
    let events = RefCell::new(Vec::new());
    let acknowledged = Cell::new(false);
    let binding = Binding::default();
    let work = binding.run(forward(
        std::future::ready(Ok::<_, ()>(subscription)),
        |event| {
            record(&events, event);
            poll_fn(|_| {
                if acknowledged.get() {
                    Poll::Ready(Ok(()))
                } else {
                    Poll::Pending
                }
            })
        },
    ));
    let mut work = std::pin::pin!(work);
    let wakes = Arc::new(WakeCounter::default());
    assert!(poll(work.as_mut(), &wakes).is_pending());
    writer.publish(2).unwrap();
    writer.publish(3).unwrap();
    drop(writer);
    assert!(poll(work.as_mut(), &wakes).is_pending());
    assert_eq!(*events.borrow(), vec![Seen::Value(0, 0)]);
    acknowledged.set(true);
    assert!(poll(work.as_mut(), &wakes).is_ready());
    assert_eq!(
        *events.borrow(),
        vec![
            Seen::Value(0, 0),
            Seen::Value(3, 3),
            Seen::End(SubscriptionEnd::Closed)
        ]
    );
}

#[test]
fn stop_wakes_pending_wait_and_releases_its_registration_without_closing_store() {
    let (mut writer, reader) = store();
    let events = RefCell::new(Vec::new());
    let binding = Binding::default();
    let work = binding.run(forward(std::future::ready(reader.subscribe()), |event| {
        record(&events, event);
        std::future::ready(Ok(()))
    }));
    let mut work = std::pin::pin!(work);
    let wakes = Arc::new(WakeCounter::default());
    let references = Arc::strong_count(&wakes);
    assert!(poll(work.as_mut(), &wakes).is_pending());
    binding.close();
    binding.close();
    assert!(wakes.count() > 0);
    assert!(poll(work.as_mut(), &wakes).is_ready());
    assert_eq!(Arc::strong_count(&wakes), references);
    writer.publish(1).unwrap();
    assert_eq!(*reader.snapshot().unwrap().state(), 1);
    assert_eq!(*events.borrow(), vec![Seen::Value(0, 0)]);
}

#[test]
fn stop_interrupts_an_unacknowledged_callback_and_drops_its_captures() {
    let (_writer, reader) = store();
    let binding = Binding::default();
    let retained = Rc::new(());
    let weak = Rc::downgrade(&retained);
    let events = RefCell::new(Vec::new());
    let work = binding.run(forward(
        std::future::ready(reader.subscribe()),
        move |event| {
            record(&events, event);
            let retained = retained.clone();
            async move {
                let _retained = retained;
                std::future::pending::<Result<(), String>>().await
            }
        },
    ));
    let mut work = std::pin::pin!(work);
    let wakes = Arc::default();
    assert!(poll(work.as_mut(), &wakes).is_pending());
    assert!(weak.upgrade().is_some());
    binding.close();
    assert!(poll(work.as_mut(), &wakes).is_ready());
    assert!(weak.upgrade().is_none());
}

#[test]
fn disposal_before_or_during_registration_never_delivers_a_late_initial_value() {
    let binding = Binding::default();
    binding.close();
    let polled = Cell::new(false);
    ready(binding.run(poll_fn(|_| {
        polled.set(true);
        Poll::Ready(())
    })));
    assert!(!polled.get());

    let binding = Binding::default();
    let retained = Rc::new(());
    let weak = Rc::downgrade(&retained);
    let initialize = async move {
        let _retained = retained;
        std::future::pending::<Result<Subscription<u8>, ()>>().await
    };
    let mut work = Box::pin(binding.run(forward(initialize, |_| async {
        panic!("cancelled registration must not deliver")
    })));
    let wakes = Arc::default();
    assert!(poll(work.as_mut(), &wakes).is_pending());
    binding.close();
    assert!(poll(work.as_mut(), &wakes).is_ready());
    assert!(weak.upgrade().is_none());
}

#[test]
fn dropping_engine_task_releases_waiter_and_callback_without_a_final_poll() {
    let (_writer, reader) = store();
    let binding = Binding::default();
    let retained = Rc::new(());
    let weak = Rc::downgrade(&retained);
    let mut work = Box::pin(binding.run(forward(
        std::future::ready(reader.subscribe()),
        move |_| {
            let _retained = &retained;
            std::future::ready(Ok(()))
        },
    )));
    let wakes = Arc::default();
    assert!(poll(work.as_mut(), &wakes).is_pending());
    drop(work);
    assert!(weak.upgrade().is_none());
    assert_eq!(Arc::strong_count(&wakes), 1);
    assert!(binding.0.borrow().closed);
}

#[test]
fn registration_failure_and_callback_failure_terminate_without_retry() {
    let events = RefCell::new(Vec::new());
    ready(forward(
        std::future::ready(Err::<Subscription<u8>, _>(())),
        |event| {
            record(&events, event);
            std::future::ready(Ok(()))
        },
    ));
    assert_eq!(
        *events.borrow(),
        vec![Seen::End(SubscriptionEnd::BackendUnavailable)]
    );
    let (mut writer, reader) = store();
    let calls = Cell::new(0);
    ready(forward(std::future::ready(reader.subscribe()), |_| {
        calls.set(calls.get() + 1);
        std::future::ready(Err("callback failed".into()))
    }));
    writer.publish(1).unwrap();
    assert_eq!(calls.get(), 1);
}
