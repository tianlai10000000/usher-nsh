//! Real window store, guarded route commits and retained session integration contracts.
use std::{cell::Cell, future::poll_fn, task::Poll};
#[path = "../../crates/usher-state/tests/support/memory.rs"]
mod memory;
use usher_nsh::window_service::{Decision, WindowService};
use usher_nsh::window_session::{
    OperationKind as Kind, Receipt, SessionCommand as Session, Stage, Ticket,
};
use usher_router::{Catalog, NavigationCommand as Command, NavigationResult as ResultKind, Target};
use usher_test_support::task::{poll, ready};
fn catalog() -> Catalog {
    serde_json::from_str(include_str!(
        "../../crates/usher-router/tests/fixtures/catalog.json"
    ))
    .unwrap()
}
fn service() -> std::sync::Arc<WindowService> {
    WindowService::new(catalog()).unwrap()
}
fn push(path: &str) -> Command {
    Command::Push(Target::path(path))
}
fn finish(kind: Kind, ticket: Ticket, stage: Stage, failed: bool) -> Session {
    Session::Finish {
        operation: kind,
        ticket,
        stage,
        error: None,
        value: None,
        failed,
    }
}

#[test]
fn router_catalog_registration_is_atomic_and_window_scopes_are_isolated() {
    let first = service();
    let second = service();
    first.register(&catalog()).unwrap();
    let mut invalid = catalog();
    invalid.routes[0].path = "/changed".into();
    assert!(first.register(&invalid).is_err());
    first.navigate(push("/about")).unwrap();
    assert_ne!(
        first.snapshot().unwrap().store_id(),
        second.snapshot().unwrap().store_id()
    );
    assert_eq!(
        second
            .snapshot()
            .unwrap()
            .state()
            .history
            .current()
            .location
            .id
            .as_deref(),
        Some("home")
    );
    assert_eq!(
        first.resolve(&Target::path("/")).unwrap().id.as_deref(),
        Some("home")
    );
}
#[test]
fn router_cancel_failure_and_invalid_targets_preserve_committed_history() {
    let service = service();
    let before = service.snapshot().unwrap().state().history.clone();
    for (decision, result) in [
        (Decision::Cancel, ResultKind::Cancelled),
        (Decision::Fail, ResultKind::Failed),
    ] {
        let ticket = service.begin(push("/about")).unwrap();
        assert_eq!(service.complete(&ticket, decision).unwrap(), Some(result));
        assert_eq!(service.snapshot().unwrap().state().history, before);
    }
    assert!(service.begin(push("https://outside")).is_err());
    assert_eq!(service.snapshot().unwrap().state().history, before);
}
#[test]
fn router_out_of_order_guards_cannot_commit_after_the_latest_navigation() {
    let service = service();
    let releases = [Cell::new(false), Cell::new(false)];
    let first = service.begin(push("/about")).unwrap();
    let mut older = Box::pin(async {
        poll_fn(|_| {
            if releases[0].get() {
                Poll::Ready(())
            } else {
                Poll::Pending
            }
        })
        .await;
        service.complete(&first, Decision::Allow)
    });
    let wakes = std::sync::Arc::default();
    assert!(poll(older.as_mut(), &wakes).is_pending());
    let second = service.begin(push("/files/final")).unwrap();
    let mut newer = Box::pin(async {
        poll_fn(|_| {
            if releases[1].get() {
                Poll::Ready(())
            } else {
                Poll::Pending
            }
        })
        .await;
        service.complete(&second, Decision::Allow)
    });
    assert!(poll(newer.as_mut(), &wakes).is_pending());
    // Unrelated window data does not invalidate a pending route's history revision.
    service.session(Session::Increment).unwrap();
    releases[1].set(true);
    assert_eq!(ready(newer).unwrap(), Some(ResultKind::Committed));
    releases[0].set(true);
    assert_eq!(ready(older).unwrap(), Some(ResultKind::Superseded));
    assert_eq!(
        service
            .snapshot()
            .unwrap()
            .state()
            .history
            .current()
            .location
            .path,
        "/files/final"
    );
    assert_eq!(service.snapshot().unwrap().state().session.clicks, 1);
}
#[test]
fn router_redirects_commit_once_preserve_replace_and_reject_loops_and_limits() {
    let service = service();
    let ticket = service
        .begin(Command::Replace(Target::path("/about")))
        .unwrap();
    assert_eq!(
        service
            .complete(&ticket, Decision::Redirect(Target::path("/files/final")))
            .unwrap(),
        None
    );
    assert_eq!(
        service.snapshot().unwrap().state().history.entries().len(),
        1
    );
    assert_eq!(
        service.complete(&ticket, Decision::Allow).unwrap(),
        Some(ResultKind::Committed)
    );
    assert_eq!(
        service.snapshot().unwrap().state().history.entries().len(),
        1
    );
    let ticket = service.begin(push("/about")).unwrap();
    assert_eq!(
        service
            .complete(&ticket, Decision::Redirect(Target::path("/about")))
            .unwrap(),
        Some(ResultKind::Failed)
    );
    let ticket = service.begin(push("/about")).unwrap();
    for index in 0..catalog().redirect_limit {
        assert_eq!(
            service
                .complete(
                    &ticket,
                    Decision::Redirect(Target::path(format!("/files/{index}")))
                )
                .unwrap(),
            None
        );
    }
    assert_eq!(
        service
            .complete(&ticket, Decision::Redirect(Target::path("/files/overflow")))
            .unwrap(),
        Some(ResultKind::Failed)
    );
    assert_eq!(
        service
            .snapshot()
            .unwrap()
            .state()
            .history
            .current()
            .location
            .path,
        "/files/final"
    );
}
#[test]
fn router_session_fields_survive_navigation_and_invalid_mutations_are_atomic() {
    let service = service();
    service
        .session(Session::Section {
            value: "theme".into(),
        })
        .unwrap();
    service
        .session(Session::Preview {
            opacity: true,
            value: Some(45),
            expected: None,
        })
        .unwrap();
    service.session(Session::Increment).unwrap();
    service.navigate(push("/about")).unwrap();
    service.navigate(Command::Go(-1)).unwrap();
    let before = service.snapshot().unwrap();
    assert_eq!(before.state().session.section, "theme");
    assert_eq!(before.state().session.opacity_preview, Some(45));
    assert_eq!(before.state().session.clicks, 1);
    let older = service
        .session(Session::Preview {
            opacity: true,
            value: Some(40),
            expected: None,
        })
        .unwrap();
    let latest = service
        .session(Session::Preview {
            opacity: true,
            value: Some(50),
            expected: None,
        })
        .unwrap();
    assert_eq!(
        service
            .session(Session::Preview {
                opacity: true,
                value: None,
                expected: Some(ticket(older))
            })
            .unwrap(),
        Receipt::Superseded
    );
    assert_eq!(
        service.snapshot().unwrap().state().session.opacity_preview,
        Some(50)
    );
    service
        .session(Session::Preview {
            opacity: true,
            value: Some(45),
            expected: Some(ticket(latest)),
        })
        .unwrap();
    let before = service.snapshot().unwrap();
    assert!(
        service
            .session(Session::Section {
                value: "unknown".into()
            })
            .is_err()
    );
    assert!(
        service
            .session(Session::Preview {
                opacity: true,
                value: Some(u32::MAX),
                expected: None,
            })
            .is_err()
    );
    assert_eq!(service.snapshot().unwrap().sequence(), before.sequence());
    service
        .session(Session::Preview {
            opacity: true,
            value: None,
            expected: None,
        })
        .unwrap();
    assert_eq!(
        service.snapshot().unwrap().state().session.opacity_preview,
        None
    );
}
#[test]
fn router_operation_tickets_fence_late_feedback_and_exports_have_one_executor() {
    let service = service();
    let old = service
        .session(Session::Begin {
            operation: Kind::Settings,
            stage: Stage::Saving,
        })
        .unwrap();
    let current = service
        .session(Session::Begin {
            operation: Kind::Settings,
            stage: Stage::Saving,
        })
        .unwrap();
    assert_eq!(
        service
            .session(finish(Kind::Settings, ticket(old), Stage::Saved, false))
            .unwrap(),
        Receipt::Superseded
    );
    service
        .session(finish(
            Kind::Settings,
            ticket(current),
            Stage::Invalid,
            true,
        ))
        .unwrap();
    service
        .session(Session::ObserveSettings {
            stage: Stage::Saved,
        })
        .unwrap();
    assert_eq!(
        service.snapshot().unwrap().state().session.operations[&Kind::Settings].stage,
        Stage::Invalid
    );
    let export = service
        .session(Session::Begin {
            operation: Kind::Export,
            stage: Stage::Running,
        })
        .unwrap();
    assert!(
        service
            .session(Session::Begin {
                operation: Kind::Export,
                stage: Stage::Running
            })
            .is_err()
    );
    service
        .session(finish(Kind::Export, ticket(export), Stage::Done, false))
        .unwrap();
    assert_eq!(
        service.snapshot().unwrap().state().session.operations[&Kind::Export].stage,
        Stage::Done
    );
}
#[test]
fn router_close_finishes_subscriptions_and_rejects_pending_guards_and_sessions() {
    let service = service();
    let mut subscription = service.subscribe().unwrap();
    let ticket = service.begin(push("/about")).unwrap();
    service.close();
    service.close();
    assert!(ready(subscription.next()).is_ok());
    assert_eq!(
        ready(subscription.next()).unwrap_err(),
        usher_state::SubscriptionEnd::Closed
    );
    assert!(service.complete(&ticket, Decision::Allow).is_err());
    assert!(service.session(Session::Increment).is_err());
    assert!(service.subscribe().is_err());
}

#[test]
fn router_application_catalog_and_wire_snapshot_match_the_page_contract() {
    let catalog: Catalog =
        serde_json::from_str(include_str!("../../ui/pages/routes.json")).unwrap();
    let router =
        usher_router::Router::new(catalog.clone(), &usher_router::matcher::Matchit).unwrap();
    assert_eq!(
        router.typescript(),
        include_str!("../../ui/pages/routes.generated.ts")
    );
    let english: serde_json::Value =
        serde_json::from_str(include_str!("../../i18n/messages/en.json")).unwrap();
    let chinese: serde_json::Value =
        serde_json::from_str(include_str!("../../i18n/messages/zh-Hans.json")).unwrap();
    for route in &catalog.routes {
        assert!(english[&route.title_key].is_string());
        assert!(chinese[&route.title_key].is_string());
    }
    let service = WindowService::new(catalog).unwrap();
    let value = serde_json::to_value(service.snapshot().unwrap().state()).unwrap();
    assert_eq!(value["history"]["entries"][0]["location"]["id"], "home");
    assert!(value["history"]["entries"][0]["key"].is_string());
    assert_eq!(value["session"]["section"], "general");
    assert!(value["session"]["operations"].is_object());
}

#[test]
fn router_catalog_migration_preserves_session_and_fences_pending_guards() {
    let service = service();
    service.navigate(push("/about")).unwrap();
    service.session(Session::Increment).unwrap();
    let ticket = service.begin(push("/users/alice")).unwrap();
    let before = service.snapshot().unwrap();
    let mut subscription = service.subscribe().unwrap();
    let mut replacement = catalog();
    replacement.identity.push_str("-replacement");
    replacement
        .routes
        .iter_mut()
        .find(|r| r.id == "about")
        .unwrap()
        .path = "/moved".into();
    service.replace_catalog(replacement.clone()).unwrap();
    let published = ready(subscription.next()).unwrap();
    let after = service.snapshot().unwrap();
    assert_eq!(published.sequence(), after.sequence());
    assert_eq!(published.state(), after.state());
    assert_eq!(after.store_id(), before.store_id());
    assert_eq!(after.sequence(), before.sequence() + 1);
    assert_eq!(after.state().session, before.state().session);
    assert_eq!(
        after.state().history.current().key,
        before.state().history.current().key
    );
    assert_eq!(after.state().history.current().location.id, None);
    assert_eq!(after.state().history.current().location.path, "/about");
    assert_eq!(after.state().navigation_result, ResultKind::Superseded);
    assert_eq!(
        service.complete(&ticket, Decision::Allow).unwrap(),
        Some(ResultKind::Superseded)
    );
    service.replace_catalog(replacement).unwrap();
    assert_eq!(service.snapshot().unwrap().sequence(), after.sequence());
    assert!(service.replace_catalog(catalog()).is_err());
    service.navigate(Command::Go(-1)).unwrap();
    assert_eq!(
        service
            .snapshot()
            .unwrap()
            .state()
            .history
            .current()
            .location
            .id
            .as_deref(),
        Some("home")
    );
}

#[test]
fn router_catalog_invalid_replacement_preserves_pending_ticket_and_snapshot() {
    let service = service();
    let ticket = service.begin(push("/about")).unwrap();
    let before = service.snapshot().unwrap();
    for change_identity in [false, true] {
        let mut invalid = catalog();
        if change_identity {
            invalid.identity.push_str("-invalid");
        }
        invalid.routes[1].page = "missing".into();
        assert!(service.replace_catalog(invalid).is_err());
        assert_eq!(service.snapshot().unwrap().state(), before.state());
        assert_eq!(service.snapshot().unwrap().sequence(), before.sequence());
    }
    assert_eq!(
        service.complete(&ticket, Decision::Allow).unwrap(),
        Some(ResultKind::Committed)
    );
}

fn ticket(receipt: Receipt) -> Ticket {
    match receipt {
        Receipt::Ticket(ticket) => ticket,
        other => panic!("expected ticket, got {other:?}"),
    }
}

#[test]
fn router_publication_fault_preserves_session_tickets_and_allows_retry() {
    use std::sync::atomic::Ordering;
    use usher_nsh::window_service::Error;
    use usher_state::StoreError;
    let backend = memory::MemoryFactory::default();
    let service = WindowService::with_backend(catalog(), &backend).unwrap();
    let before = service.snapshot().unwrap();
    let begin = Session::Begin {
        operation: Kind::Settings,
        stage: Stage::Saving,
    };
    backend.0.fail_publish.store(true, Ordering::SeqCst);
    let error = service.session(begin.clone()).unwrap_err();
    assert_eq!(error, Error::Store(StoreError::BackendUnavailable));
    assert_eq!(error.code(), "router_unavailable");
    assert!(std::error::Error::source(&error).is_some());
    assert_eq!(service.snapshot().unwrap().sequence(), before.sequence());
    assert_eq!(service.snapshot().unwrap().state(), before.state());
    backend.0.fail_publish.store(false, Ordering::SeqCst);
    let receipt = service.session(begin.clone()).unwrap();
    let reference = WindowService::new(catalog()).unwrap();
    assert_eq!(receipt, reference.session(begin).unwrap());
    service
        .session(finish(Kind::Settings, ticket(receipt), Stage::Saved, false))
        .unwrap();
}

#[test]
fn router_publication_fault_keeps_guard_outcome_and_redirect_tracking_retryable() {
    use std::sync::atomic::Ordering;
    let backend = memory::MemoryFactory::default();
    let service = WindowService::with_backend(catalog(), &backend).unwrap();
    let ticket = service.begin(push("/about")).unwrap();
    let before = service.snapshot().unwrap();
    backend.0.fail_publish.store(true, Ordering::SeqCst);
    assert!(service.complete(&ticket, Decision::Allow).is_err());
    assert_eq!(service.snapshot().unwrap().state(), before.state());
    backend.0.fail_publish.store(false, Ordering::SeqCst);
    assert_eq!(
        service.complete(&ticket, Decision::Allow).unwrap(),
        Some(ResultKind::Committed)
    );

    let ticket = service.begin(push("/users/alice")).unwrap();
    assert_eq!(
        service
            .complete(&ticket, Decision::Redirect(Target::path("/")))
            .unwrap(),
        None
    );
    let before = service.snapshot().unwrap();
    backend.0.fail_publish.store(true, Ordering::SeqCst);
    assert!(
        service
            .complete(&ticket, Decision::Redirect(Target::path("/users/alice")))
            .is_err()
    );
    assert_eq!(service.snapshot().unwrap().state(), before.state());
    backend.0.fail_publish.store(false, Ordering::SeqCst);
    assert_eq!(
        service.complete(&ticket, Decision::Allow).unwrap(),
        Some(ResultKind::Committed)
    );
    assert_eq!(
        service
            .snapshot()
            .unwrap()
            .state()
            .history
            .current()
            .location
            .path,
        "/"
    );
}

#[test]
fn router_publication_fault_does_not_retire_a_catalog_or_cancel_its_guard() {
    use std::sync::atomic::Ordering;
    let backend = memory::MemoryFactory::default();
    let original = catalog();
    let service = WindowService::with_backend(original.clone(), &backend).unwrap();
    let ticket = service.begin(push("/about")).unwrap();
    let before = service.snapshot().unwrap();
    let mut replacement = original.clone();
    replacement.identity.push_str("-replacement");
    backend.0.fail_publish.store(true, Ordering::SeqCst);
    assert!(service.replace_catalog(replacement.clone()).is_err());
    assert_eq!(service.snapshot().unwrap().state(), before.state());
    service.register(&original).unwrap();
    backend.0.fail_publish.store(false, Ordering::SeqCst);
    service.replace_catalog(replacement.clone()).unwrap();
    service.register(&replacement).unwrap();
    assert_eq!(
        service.complete(&ticket, Decision::Allow).unwrap(),
        Some(ResultKind::Superseded)
    );
}
