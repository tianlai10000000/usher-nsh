//! Update-domain integration contracts: real stores, controlled executors and isolated repositories.
#[path = "../../crates/usher-state/tests/support/memory.rs"]
mod memory;
use std::sync::{Arc, Barrier, atomic::Ordering};
use usher_nsh::{
    app_state::{ColorMode, Command as SettingsCommand, SettingsRegistry},
    update_service::*,
};
use usher_state::{BackendFactory, SubscriptionEnd, backends::WatchBackend};
use usher_test_support::task::ready;
use usher_update::state::{
    Capability, CheckResult, Effect, Failure, Machine, Offer, Outcome, Phase, Settings, Trigger,
};

fn fixture() -> serde_json::Value {
    let value: serde_json::Value =
        serde_json::from_str(include_str!("fixtures/updates.json")).unwrap();
    assert_eq!(value["schema_version"], 1);
    usher_test_support::fixture::assert_unique_ids(
        value["cases"]
            .as_array()
            .unwrap()
            .iter()
            .map(|v| v.as_str().unwrap()),
    );
    value
}
/// The settings every machine here starts from, matching the real default: automatic checks on.
/// A test that needs a found update to stop at "found" instead of downloading by itself turns
/// them off through `Command::AutoCheck(false)`, as the user would.
fn machine_settings() -> Settings {
    let current: usher_update::Version = fixture()["current"].as_str().unwrap().parse().unwrap();
    Settings {
        shell: current.clone(),
        running: current,
        capability: Capability::InApp,
        auto_check: true,
        failure_limit: fixture()["failure_limit"].as_u64().unwrap() as u32,
    }
}
fn machine() -> Machine {
    Machine::new(machine_settings())
}
/// An offer built from a fixture key. `package` says whether the release carries a code package.
fn offer(key: &str, package: bool) -> Offer {
    Offer {
        version: fixture()[key].as_str().unwrap().parse().unwrap(),
        has_package: package,
    }
}
fn both(run: impl Fn(&UpdateService)) {
    for backend in [
        &WatchBackend as &dyn BackendFactory<State>,
        &memory::MemoryFactory::default(),
    ] {
        run(&UpdateService::new(machine(), backend).unwrap());
    }
}
fn check(service: &UpdateService, lease: Lease) -> Claim {
    service.request(Command::Check(Trigger::Manual)).unwrap();
    service.claim(lease).unwrap().unwrap()
}
fn available(service: &UpdateService, lease: Lease) {
    let claim = check(service, lease);
    service
        .complete(
            &claim,
            Completion::Checked(CheckResult::Release(Some(offer("candidate", false)))),
        )
        .unwrap();
    let notice = service.claim(lease).unwrap().unwrap();
    assert!(matches!(notice.effect(), Effect::Notify(_)));
    service.complete(&notice, Completion::Notified).unwrap();
}
fn install(service: &UpdateService, lease: Lease) -> Claim {
    available(service, lease);
    service.request(Command::Download).unwrap();
    let download = service.claim(lease).unwrap().unwrap();
    // NSH verifies a download inside the download, so there is no verify step to claim.
    service.complete(&download, Completion::Downloaded).unwrap();
    service.request(Command::Install).unwrap();
    service.claim(lease).unwrap().unwrap()
}

#[test]
fn updates_backend_contract_single_execution_shared_subscriptions_and_coalesced_checks() {
    both(|service| {
        let mut first = service.subscribe().unwrap();
        let mut second = service.subscribe().unwrap();
        let lease = service.acquire().unwrap();
        assert_eq!(service.acquire(), Err(Error::Busy));
        service
            .request(Command::Check(Trigger::Background))
            .unwrap();
        let claim = service.claim(lease).unwrap().unwrap();
        service.request(Command::Check(Trigger::Manual)).unwrap();
        assert!(service.claim(lease).unwrap().is_none());
        assert_eq!(
            ready(first.next()).unwrap().state().domain.phase,
            Phase::Checking
        );
        assert_eq!(
            ready(second.next()).unwrap().state().execution,
            Execution::Running
        );
        service
            .complete(&claim, Completion::Checked(CheckResult::Release(None)))
            .unwrap();
        assert_eq!(
            service.complete(&claim, Completion::Checked(CheckResult::Release(None))),
            Err(Error::Stale)
        );
        let notice = service.claim(lease).unwrap().unwrap();
        assert!(
            matches!(notice.effect(), Effect::Notify(_)),
            "manual intent joined the background check"
        );
        service.complete(&notice, Completion::Notified).unwrap();
        assert!(service.claim(lease).unwrap().is_none());
        assert_eq!(
            ready(first.next()).unwrap().state(),
            ready(second.next()).unwrap().state()
        );
    });
}
#[test]
fn updates_backend_contract_download_progress_and_receipt_kinds_are_validated() {
    both(|service| {
        let lease = service.acquire().unwrap();
        available(service, lease);
        service.request(Command::Download).unwrap();
        let claim = service.claim(lease).unwrap().unwrap();
        assert_eq!(
            service.complete(&claim, Completion::Installed),
            Err(Error::Invalid)
        );
        service.progress(&claim, 4, Some(10)).unwrap();
        let sequence = service.snapshot().unwrap().sequence();
        for (received, total) in [(3, Some(10)), (11, Some(10)), (5, None), (5, Some(11))] {
            assert_eq!(
                service.progress(&claim, received, total),
                Err(Error::Invalid)
            );
        }
        assert_eq!(service.snapshot().unwrap().sequence(), sequence);
        service.progress(&claim, 10, Some(10)).unwrap();
        service.complete(&claim, Completion::Downloaded).unwrap();
        assert!(service.progress(&claim, 10, Some(10)).is_err());
        // The download is done and NSH accepted it: the update waits to be installed.
        assert!(matches!(
            service.snapshot().unwrap().state().domain.phase,
            Phase::ReadyToInstall(_)
        ));
        assert!(service.claim(lease).unwrap().is_none());
    });
}
#[test]
fn updates_backend_contract_an_update_installs_through_claims_and_waits_for_the_first_launch() {
    // The whole path through the service: check, download, install, first launch. Before the
    // handover contract was removed this was only reached through the handover tests.
    both(|service| {
        let lease = service.acquire().unwrap();
        let install = install(service, lease);
        assert!(matches!(install.effect(), Effect::StartInstall { .. }));
        assert!(matches!(
            service.snapshot().unwrap().state().domain.phase,
            Phase::Installing(_)
        ));
        // A result of the wrong kind for this claim is refused and leaves it open.
        assert_eq!(
            service.complete(&install, Completion::Downloaded),
            Err(Error::Invalid)
        );
        service.complete(&install, Completion::Installed).unwrap();
        assert!(matches!(
            service.snapshot().unwrap().state().domain.phase,
            Phase::AwaitingFirstLaunch(_)
        ));
        // The same claim cannot complete twice: the second result is a stale one.
        assert_eq!(
            service.complete(&install, Completion::Installed),
            Err(Error::Stale)
        );
        service.request(Command::ConfirmFirstLaunch).unwrap();
        assert_eq!(
            service.snapshot().unwrap().state().domain.phase,
            Phase::Idle
        );
    });
}
#[test]
fn updates_backend_contract_a_failed_install_is_a_failed_step_and_the_update_can_be_retried() {
    both(|service| {
        let lease = service.acquire().unwrap();
        let install = install(service, lease);
        service
            .complete(&install, Completion::Failed(Failure::Disk))
            .unwrap();
        assert!(matches!(
            service.snapshot().unwrap().state().domain.phase,
            Phase::InstallFailed {
                failure: Failure::Disk,
                ..
            }
        ));
        // A disk failure may pass, so the update is offered again: asking downloads it anew.
        service.request(Command::Download).unwrap();
        let retry = service.claim(lease).unwrap().unwrap();
        assert!(matches!(retry.effect(), Effect::StartDownload { .. }));
        assert_ne!(install.id(), retry.id());
    });
}
#[test]
fn updates_backend_contract_a_rejected_download_is_a_failed_step_not_a_stuck_claim() {
    // NSH refused the package. The claim is settled, the step fails with its class, and the
    // executor is free to take the next effect: nothing is left half done.
    both(|service| {
        let lease = service.acquire().unwrap();
        available(service, lease);
        service.request(Command::Download).unwrap();
        let claim = service.claim(lease).unwrap().unwrap();
        service
            .complete(&claim, Completion::Failed(Failure::Rejected))
            .unwrap();
        assert!(matches!(
            service.snapshot().unwrap().state().domain.phase,
            Phase::InstallFailed {
                failure: Failure::Rejected,
                ..
            }
        ));
        assert_eq!(
            service.snapshot().unwrap().state().execution,
            Execution::Available
        );
        // Retrying cannot help, so the machine does not queue another download.
        service.request(Command::Download).unwrap();
        assert!(service.claim(lease).unwrap().is_none());
    });
}
#[test]
fn updates_backend_contract_a_foreign_scope_cannot_claim_and_the_lease_stays_with_its_holder() {
    both(|service| {
        let lease = service.acquire().unwrap();
        service.request(Command::Check(Trigger::Manual)).unwrap();
        let other = UpdateService::new(machine(), &WatchBackend).unwrap();
        let foreign = other.acquire().unwrap();
        // A lease from another service is not valid here.
        assert_eq!(service.claim(foreign), Err(Error::Stale));
        // And there is no way to take the lease away from its holder: no revoke, no handover.
        // A second executor is refused until the service closes (docs/update.md, 8.2, option B).
        assert_eq!(service.acquire(), Err(Error::Busy));
        assert!(service.claim(lease).unwrap().is_some());
    });
}
#[test]
fn updates_backend_contract_a_not_newer_or_pre_release_offer_is_up_to_date_and_a_background_failure_is_silent()
 {
    both(|service| {
        let lease = service.acquire().unwrap();
        service.request(Command::AutoCheck(false)).unwrap();
        service
            .request(Command::Check(Trigger::Background))
            .unwrap();
        // With automatic checks off a background check is refused outright...
        assert!(service.claim(lease).unwrap().is_none());
        // ...and the user's check runs. A failure the user asked about is told to them.
        let claim = check(service, lease);
        service
            .complete(&claim, Completion::Failed(Failure::Network))
            .unwrap();
        let notice = service.claim(lease).unwrap().unwrap();
        assert!(matches!(notice.effect(), Effect::Notify(_)));
        service.complete(&notice, Completion::Notified).unwrap();
        // One channel, formal releases only: neither the running version nor a pre-release is an
        // update.
        for key in ["current", "prerelease"] {
            let claim = check(service, lease);
            service
                .complete(
                    &claim,
                    Completion::Checked(CheckResult::Release(Some(offer(key, true)))),
                )
                .unwrap();
            assert_eq!(
                service.snapshot().unwrap().state().domain.phase,
                Phase::Checked(Outcome::UpToDate)
            );
            let notice = service.claim(lease).unwrap().unwrap();
            service.complete(&notice, Completion::Notified).unwrap();
        }
    });
}
#[test]
fn updates_backend_contract_the_machines_log_records_reach_the_caller_in_order_and_are_bounded() {
    both(|service| {
        let lease = service.acquire().unwrap();
        let claim = check(service, lease);
        service
            .complete(&claim, Completion::Failed(Failure::Network))
            .unwrap();
        // A log record is not work for the executor: it never appears as a claim.
        let notice = service.claim(lease).unwrap().unwrap();
        assert!(matches!(notice.effect(), Effect::Notify(_)));
        service.complete(&notice, Completion::Notified).unwrap();
        assert!(service.claim(lease).unwrap().is_none());
        let records = service.take_logs().unwrap();
        let events: Vec<_> = records.iter().map(|record| record.event).collect();
        assert_eq!(events, ["check_started", "check_failed"]);
        // Handing them over empties the backlog.
        assert!(service.take_logs().unwrap().is_empty());
        // Taking is not a state change: nothing is published for it.
        let sequence = service.snapshot().unwrap().sequence();
        service.take_logs().unwrap();
        assert_eq!(service.snapshot().unwrap().sequence(), sequence);
    });
}
#[test]
fn updates_backend_contract_unread_log_records_cannot_grow_without_limit() {
    // A logger that is off or never collects must not make the service grow forever.
    let service = UpdateService::new(machine(), &WatchBackend).unwrap();
    let lease = service.acquire().unwrap();
    for _ in 0..200 {
        let claim = check(&service, lease);
        service
            .complete(&claim, Completion::Checked(CheckResult::Release(None)))
            .unwrap();
        let notice = service.claim(lease).unwrap().unwrap();
        service.complete(&notice, Completion::Notified).unwrap();
    }
    let records = service.take_logs().unwrap();
    assert!(records.len() <= 64, "kept {}", records.len());
    assert!(!records.is_empty());
    // The newest survive: the last record is from the last check.
    assert_eq!(records.last().unwrap().event, "check_finished");
}
#[test]
fn updates_backend_contract_auto_check_is_a_command_that_changes_the_next_check_only() {
    both(|service| {
        let lease = service.acquire().unwrap();
        assert!(service.snapshot().unwrap().state().domain.auto_check());
        service.request(Command::AutoCheck(false)).unwrap();
        assert!(!service.snapshot().unwrap().state().domain.auto_check());
        service
            .request(Command::Check(Trigger::Background))
            .unwrap();
        assert!(
            service.claim(lease).unwrap().is_none(),
            "off: a background check is refused"
        );
        service.request(Command::AutoCheck(true)).unwrap();
        assert!(service.snapshot().unwrap().state().domain.auto_check());
        // Now a background check goes through and a found package downloads without asking.
        service
            .request(Command::Check(Trigger::Background))
            .unwrap();
        let claim = service.claim(lease).unwrap().unwrap();
        service
            .complete(
                &claim,
                Completion::Checked(CheckResult::Release(Some(offer("candidate", true)))),
            )
            .unwrap();
        let mut seen = Vec::new();
        while let Some(next) = service.claim(lease).unwrap() {
            seen.push(matches!(next.effect(), Effect::StartDownload { .. }));
            if matches!(next.effect(), Effect::Notify(_)) {
                service.complete(&next, Completion::Notified).unwrap();
            } else {
                break;
            }
        }
        assert!(
            seen.contains(&true),
            "the package downloaded by itself: {seen:?}"
        );
    });
}
#[test]
fn updates_backend_contract_close_ends_subscriptions_and_retained_readers_cannot_write() {
    both(|service| {
        let lease = service.acquire().unwrap();
        let claim = check(service, lease);
        let reader = service.reader();
        let mut sub = service.subscribe().unwrap();
        service.close();
        service.close();
        assert_eq!(ready(sub.next()).unwrap_err(), SubscriptionEnd::Closed);
        assert!(reader.snapshot().is_ok());
        assert_eq!(
            service.complete(&claim, Completion::Checked(CheckResult::Release(None))),
            Err(Error::Closed)
        );
        assert_eq!(service.acquire(), Err(Error::Closed));
        assert!(service.subscribe().is_err());
    });
}
#[test]
fn updates_publication_failure_never_releases_or_loses_a_claim_or_domain_transition() {
    let backend = memory::MemoryFactory::default();
    let service = UpdateService::new(machine(), &backend).unwrap();
    let lease = service.acquire().unwrap();
    backend.0.fail_publish.store(true, Ordering::SeqCst);
    assert_eq!(
        service.request(Command::Check(Trigger::Manual)),
        Err(Error::Unavailable)
    );
    assert_eq!(
        service.snapshot().unwrap().state().domain.phase,
        Phase::Idle
    );
    assert!(service.claim(lease).unwrap().is_none());
    backend.0.fail_publish.store(false, Ordering::SeqCst);
    service.request(Command::Check(Trigger::Manual)).unwrap();
    backend.0.fail_publish.store(true, Ordering::SeqCst);
    assert_eq!(service.claim(lease), Err(Error::Unavailable));
    backend.0.fail_publish.store(false, Ordering::SeqCst);
    let claim = service.claim(lease).unwrap().unwrap();
    backend.0.fail_publish.store(true, Ordering::SeqCst);
    assert_eq!(
        service.complete(&claim, Completion::Checked(CheckResult::Release(None))),
        Err(Error::Unavailable)
    );
    assert_eq!(
        service.snapshot().unwrap().state().domain.phase,
        Phase::Checking
    );
    backend.0.fail_publish.store(false, Ordering::SeqCst);
    service
        .complete(&claim, Completion::Checked(CheckResult::Release(None)))
        .unwrap();
}
#[test]
fn updates_concurrent_windows_claim_one_effect_and_receive_the_same_state() {
    let service = UpdateService::new(machine(), &WatchBackend).unwrap();
    let lease = service.acquire().unwrap();
    service.request(Command::Check(Trigger::Manual)).unwrap();
    let barrier = Barrier::new(2);
    let claims = std::thread::scope(|scope| {
        let first = scope.spawn(|| {
            barrier.wait();
            service.claim(lease).unwrap()
        });
        let second = scope.spawn(|| {
            barrier.wait();
            service.claim(lease).unwrap()
        });
        [first.join().unwrap(), second.join().unwrap()]
    });
    assert_eq!(claims.iter().filter(|claim| claim.is_some()).count(), 1);
}
#[test]
fn updates_registry_shares_only_host_settings_scope_and_preserves_persisted_bytes() {
    let directory = tempfile::tempdir().unwrap();
    let settings = SettingsRegistry::default();
    let path = directory.path().join("settings.toml");
    let first = settings.for_path(path.clone()).unwrap();
    first
        .apply(SettingsCommand::SetColorMode(ColorMode::Dark))
        .unwrap();
    let before = std::fs::read(&path).unwrap();
    let registry = UpdateRegistry::default();
    let main = registry.for_settings(&first, machine()).unwrap();
    let second = registry
        .for_settings(&settings.for_path(path.clone()).unwrap(), machine())
        .unwrap();
    assert!(Arc::ptr_eq(&main, &second));
    let isolated_settings = settings
        .for_path(directory.path().join("other/settings.toml"))
        .unwrap();
    let isolated = registry
        .for_settings(&isolated_settings, machine())
        .unwrap();
    assert_ne!(
        main.snapshot().unwrap().store_id(),
        isolated.snapshot().unwrap().store_id()
    );
    let lease = main.acquire().unwrap();
    let claim = check(&main, lease);
    main.complete(&claim, Completion::Failed(Failure::Network))
        .unwrap();
    assert_eq!(std::fs::read(&path).unwrap(), before);
    assert_eq!(
        isolated.snapshot().unwrap().state().domain.phase,
        Phase::Idle
    );
    // The same scope cannot be re-initialised with a different machine.
    let mut other = machine_settings();
    other.shell = "9.9.9".parse().unwrap();
    assert!(matches!(
        registry.for_settings(&first, Machine::new(other)),
        Err(Error::Configuration)
    ));
    registry.close();
    assert_eq!(main.acquire(), Err(Error::Closed));
    assert!(matches!(
        registry.for_settings(&first, machine()),
        Err(Error::Closed)
    ));
    assert_eq!(std::fs::read(path).unwrap(), before);
}
#[test]
fn updates_registry_drop_closes_retained_services_and_new_sessions_do_not_replay_work() {
    let settings = usher_nsh::app_state::SettingsService::session().unwrap();
    let service;
    {
        let registry = UpdateRegistry::default();
        service = registry.for_settings(&settings, machine()).unwrap();
        service.request(Command::Check(Trigger::Manual)).unwrap();
    }
    assert_eq!(service.acquire(), Err(Error::Closed));
    let fresh = UpdateService::new(machine(), &WatchBackend).unwrap();
    let lease = fresh.acquire().unwrap();
    assert!(fresh.claim(lease).unwrap().is_none());
    assert_ne!(
        service.reader().snapshot().unwrap().store_id(),
        fresh.snapshot().unwrap().store_id()
    );
}
