//! Public contracts using opaque fixtures and simulated backends, without application dependencies.
use std::collections::BTreeSet;
use usher_shortcuts::*;

fn spec(id: &str, scope: &str, key: &str) -> BindingSpec {
    BindingSpec {
        id: BindingId::new(id).unwrap(),
        scope: ScopeId::new(scope).unwrap(),
        resource: ResourceKey {
            domain: DomainId::new("d").unwrap(),
            stroke: CanonicalStroke::new(key).unwrap(),
        },
        policy: Policy {
            editing: EditingPolicy::Auto,
            repeat: false,
            pass_through: false,
            required: true,
        },
    }
}
fn desired(id: &str, revision: u64) -> DesiredBinding {
    DesiredBinding {
        binding: spec(id, "s", "ctrl+k"),
        owner: OwnerToken::new("o").unwrap(),
        revision,
    }
}
fn install(effects: Vec<Effect>) -> Ticket {
    assert_eq!(effects.len(), 1);
    match effects.into_iter().next().unwrap() {
        Effect::Install(t) => t,
        _ => panic!("expected install"),
    }
}
fn release(effects: Vec<Effect>) -> Ticket {
    assert_eq!(effects.len(), 1);
    match effects.into_iter().next().unwrap() {
        Effect::Release(t) => t,
        _ => panic!("expected release"),
    }
}
fn relations(disjoint: bool) -> ScopeRelations {
    let a = ScopeId::new("a").unwrap();
    let b = ScopeId::new("b").unwrap();
    ScopeRelations::new(
        BTreeSet::from([a.clone(), b.clone()]),
        if disjoint { vec![(a, b)] } else { vec![] },
    )
    .unwrap()
}
#[test]
fn catalog_rejects_undeclared_overlap_and_duplicate_ids() {
    assert!(matches!(
        Catalog::new(
            1,
            vec![spec("x", "a", "k"), spec("y", "b", "k")],
            relations(false)
        ),
        Err(Error::Conflict(..))
    ));
    assert!(matches!(
        Catalog::new(
            1,
            vec![spec("x", "a", "k"), spec("x", "b", "j")],
            relations(true)
        ),
        Err(Error::DuplicateBinding(_))
    ));
}
#[test]
fn catalog_reuses_disjoint_keys_but_rejects_simultaneous_activation() {
    let catalog = Catalog::new(
        1,
        vec![spec("x", "a", "k"), spec("y", "b", "k")],
        relations(true),
    )
    .unwrap();
    let enabled = BTreeSet::from([BindingId::new("x").unwrap(), BindingId::new("y").unwrap()]);
    let owner = OwnerToken::new("o").unwrap();
    let active = BTreeSet::from([ScopeId::new("a").unwrap()]);
    assert_eq!(
        catalog
            .plan(&active, &enabled, owner.clone())
            .unwrap()
            .len(),
        1
    );
    assert!(matches!(
        catalog.plan(
            &BTreeSet::from([ScopeId::new("a").unwrap(), ScopeId::new("b").unwrap()]),
            &enabled,
            owner
        ),
        Err(Error::Conflict(..))
    ));
}
#[test]
fn catalog_checks_unknown_inputs_and_exact_keys() {
    let catalog = Catalog::new(
        1,
        vec![spec("x", "a", "ctrl+k"), spec("y", "a", "ctrl+shift+k")],
        relations(false),
    )
    .unwrap();
    assert_eq!(catalog.bindings().count(), 2);
    assert!(matches!(
        catalog.plan(
            &BTreeSet::new(),
            &BTreeSet::from([BindingId::new("absent").unwrap()]),
            OwnerToken::new("o").unwrap()
        ),
        Err(Error::UnknownBinding(_))
    ));
    assert!(matches!(
        Catalog::new(1, vec![spec("x", "missing", "k")], relations(false)),
        Err(Error::UnknownScope(_))
    ));
    assert!(
        ScopeRelations::new(
            BTreeSet::from([ScopeId::new("a").unwrap()]),
            vec![(ScopeId::new("a").unwrap(), ScopeId::new("a").unwrap())]
        )
        .is_err()
    );
}
#[test]
fn runtime_ready_requires_receipt_and_reconcile_is_idempotent() {
    let mut runtime = Runtime::new(2).unwrap();
    let plan = desired("x", 1);
    let ticket = install(runtime.reconcile(1, vec![plan.clone()]).unwrap());
    assert!(!runtime.accepts(&ticket));
    assert!(runtime.reconcile(1, vec![plan.clone()]).unwrap().is_empty());
    runtime.acknowledge(&ticket, Receipt::Installed).unwrap();
    assert!(runtime.accepts(&ticket));
    assert!(runtime.reconcile(1, vec![plan]).unwrap().is_empty());
}
#[test]
fn runtime_serializes_replacement_and_ignores_old_callbacks() {
    let mut runtime = Runtime::new(2).unwrap();
    let old = install(runtime.reconcile(1, vec![desired("x", 1)]).unwrap());
    runtime.acknowledge(&old, Receipt::Installed).unwrap();
    let cleanup = release(runtime.reconcile(2, vec![desired("y", 2)]).unwrap());
    assert!(!runtime.accepts(&old));
    let new = install(runtime.acknowledge(&cleanup, Receipt::Released).unwrap());
    assert!(
        runtime
            .acknowledge(&old, Receipt::Installed)
            .unwrap()
            .is_empty()
    );
    runtime.acknowledge(&new, Receipt::Installed).unwrap();
    assert!(runtime.accepts(&new));
    assert!(!runtime.accepts(&old));
}
#[test]
fn runtime_late_install_is_cleaned_before_same_key_is_reused() {
    let mut runtime = Runtime::new(2).unwrap();
    let old = install(runtime.reconcile(1, vec![desired("x", 1)]).unwrap());
    assert!(
        runtime
            .reconcile(2, vec![desired("y", 2)])
            .unwrap()
            .is_empty()
    );
    assert!(runtime.retry(&old).is_err());
    let cleanup = release(runtime.acknowledge(&old, Receipt::Installed).unwrap());
    assert!(!runtime.accepts(&old));
    let new = install(runtime.acknowledge(&cleanup, Receipt::Released).unwrap());
    assert_eq!(new.desired().binding.id.as_str(), "y");
}
#[test]
fn runtime_failed_old_install_needs_no_release() {
    let mut runtime = Runtime::new(2).unwrap();
    let old = install(runtime.reconcile(1, vec![desired("x", 1)]).unwrap());
    runtime.reconcile(2, vec![desired("y", 2)]).unwrap();
    let new = install(
        runtime
            .acknowledge(&old, Receipt::InstallFailed(Failure::Rejected))
            .unwrap(),
    );
    assert_eq!(new.desired().binding.id.as_str(), "y");
}
#[test]
fn runtime_release_failure_retains_ownership_until_retry_succeeds() {
    let mut runtime = Runtime::new(2).unwrap();
    let old = install(runtime.reconcile(1, vec![desired("x", 1)]).unwrap());
    runtime.acknowledge(&old, Receipt::Installed).unwrap();
    let cleanup = release(runtime.reconcile(2, vec![desired("y", 2)]).unwrap());
    assert!(
        runtime
            .acknowledge(&cleanup, Receipt::ReleaseFailed(Failure::Unavailable))
            .unwrap()
            .is_empty()
    );
    assert!(
        runtime
            .reconcile(2, vec![desired("y", 2)])
            .unwrap()
            .is_empty()
    );
    assert_eq!(release(runtime.retry(&cleanup).unwrap()), cleanup);
    assert!(matches!(
        runtime
            .acknowledge(&cleanup, Receipt::Released)
            .unwrap()
            .as_slice(),
        [Effect::Install(_)]
    ));
}
#[test]
fn runtime_retry_budget_and_ticket_replacement() {
    let mut runtime = Runtime::new(2).unwrap();
    let first = install(runtime.reconcile(1, vec![desired("x", 1)]).unwrap());
    runtime
        .acknowledge(&first, Receipt::InstallFailed(Failure::Unavailable))
        .unwrap();
    let second = install(runtime.retry(&first).unwrap());
    assert_ne!(first, second);
    assert!(
        runtime
            .acknowledge(&first, Receipt::Installed)
            .unwrap()
            .is_empty()
    );
    runtime
        .acknowledge(&second, Receipt::InstallFailed(Failure::Unknown))
        .unwrap();
    assert_eq!(runtime.retry(&second), Err(Error::RetryExhausted));
    assert_eq!(runtime.snapshot()[0].attempts, 2);
}
#[test]
fn runtime_rejection_is_not_automatically_retried() {
    let mut runtime = Runtime::new(2).unwrap();
    let first = install(runtime.reconcile(1, vec![desired("x", 1)]).unwrap());
    runtime
        .acknowledge(&first, Receipt::InstallFailed(Failure::Rejected))
        .unwrap();
    assert!(
        runtime
            .reconcile(1, vec![desired("x", 1)])
            .unwrap()
            .is_empty()
    );
    assert!(runtime.retry(&first).is_err());
}
#[test]
fn runtime_disposal_during_install_cleans_late_success() {
    let mut runtime = Runtime::new(2).unwrap();
    let ticket = install(runtime.reconcile(1, vec![desired("x", 1)]).unwrap());
    assert!(runtime.dispose().unwrap().is_empty());
    assert!(!runtime.is_disposed());
    let cleanup = release(runtime.acknowledge(&ticket, Receipt::Installed).unwrap());
    assert!(!runtime.accepts(&ticket));
    runtime.acknowledge(&cleanup, Receipt::Released).unwrap();
    assert!(runtime.is_disposed());
    assert!(runtime.dispose().unwrap().is_empty());
    assert_eq!(runtime.reconcile(2, vec![]), Err(Error::Disposed));
}
#[test]
fn runtime_invalid_plan_preserves_installed_bindings() {
    let mut runtime = Runtime::new(2).unwrap();
    let ticket = install(runtime.reconcile(2, vec![desired("x", 2)]).unwrap());
    runtime.acknowledge(&ticket, Receipt::Installed).unwrap();
    assert_eq!(runtime.reconcile(1, vec![]), Err(Error::StaleRevision));
    assert!(matches!(
        runtime.reconcile(3, vec![desired("x", 3), desired("y", 3)]),
        Err(Error::Conflict(..))
    ));
    assert!(runtime.accepts(&ticket));
}
#[test]
fn runtime_partial_failure_does_not_disable_another_resource() {
    let mut runtime = Runtime::new(2).unwrap();
    let first = desired("x", 1);
    let mut second = desired("y", 1);
    second.binding.resource.stroke = CanonicalStroke::new("ctrl+j").unwrap();
    let tickets: Vec<_> = runtime
        .reconcile(1, vec![first, second])
        .unwrap()
        .into_iter()
        .map(|e| install(vec![e]))
        .collect();
    runtime
        .acknowledge(&tickets[0], Receipt::Installed)
        .unwrap();
    runtime
        .acknowledge(&tickets[1], Receipt::InstallFailed(Failure::Rejected))
        .unwrap();
    assert!(runtime.accepts(&tickets[0]));
    assert!(!runtime.accepts(&tickets[1]));
}
#[test]
fn runtime_owner_change_invalidates_trigger_before_cleanup() {
    let mut runtime = Runtime::new(2).unwrap();
    let old = install(runtime.reconcile(1, vec![desired("x", 1)]).unwrap());
    runtime.acknowledge(&old, Receipt::Installed).unwrap();
    let mut replacement = desired("x", 1);
    replacement.owner = OwnerToken::new("other").unwrap();
    assert!(matches!(
        runtime.reconcile(1, vec![replacement]).unwrap().as_slice(),
        [Effect::Release(_)]
    ));
    assert!(!runtime.accepts(&old));
}
#[test]
fn crate_manifest_has_no_runtime_or_test_dependencies() {
    let manifest = include_str!("../Cargo.toml");
    assert!(!manifest.contains("dependencies"));
}

#[test]
fn runtime_duplicate_release_failure_and_late_success_preserve_cleanup() {
    let mut runtime = Runtime::new(2).unwrap();
    let old = install(runtime.reconcile(1, vec![desired("x", 1)]).unwrap());
    runtime.acknowledge(&old, Receipt::Installed).unwrap();
    let cleanup = release(runtime.dispose().unwrap());
    runtime
        .acknowledge(&cleanup, Receipt::ReleaseFailed(Failure::Unavailable))
        .unwrap();
    assert!(
        runtime
            .acknowledge(&cleanup, Receipt::ReleaseFailed(Failure::Unavailable))
            .unwrap()
            .is_empty()
    );
    runtime.acknowledge(&cleanup, Receipt::Released).unwrap();
    assert!(runtime.is_disposed());
}

#[test]
fn runtime_disabled_binding_releases_without_reinstalling() {
    let mut runtime = Runtime::new(2).unwrap();
    let old = install(runtime.reconcile(1, vec![desired("x", 1)]).unwrap());
    runtime.acknowledge(&old, Receipt::Installed).unwrap();
    let cleanup = release(runtime.reconcile(1, vec![]).unwrap());
    assert!(!runtime.accepts(&old));
    assert!(
        runtime
            .acknowledge(&cleanup, Receipt::Released)
            .unwrap()
            .is_empty()
    );
    assert!(runtime.snapshot().is_empty());
}
