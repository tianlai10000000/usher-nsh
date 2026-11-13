//! Behaviour of the update state machine, through its public interface only: events in, phase
//! and effects out. Each test names one rule from docs/update.md, section 8.2.

use semver::Version;
use usher_update::state::{
    Candidate, Capability, CheckResult, Effect, Event, FAILURE_MEMORY, Failure, Kind, Level,
    Machine, ManualReason, Notice, Offer, Outcome, Phase, Record, Settings, ShellReason, Trigger,
};

fn v(text: &str) -> Version {
    Version::parse(text).unwrap()
}

fn settings(shell: &str, running: &str) -> Settings {
    Settings {
        shell: v(shell),
        running: v(running),
        capability: Capability::InApp,
        auto_check: true,
        failure_limit: 2,
    }
}

/// A shell and a running version that are the same: nothing but the shell is installed.
fn machine(version: &str) -> Machine {
    Machine::new(settings(version, version))
}

/// Like `machine`, but with automatic checks off, so a code package stops at "found" instead of
/// downloading by itself. Tests about which path an update takes use this one; tests about the
/// automatic download use `machine`.
fn classifying(shell: &str, running: &str) -> Machine {
    let mut s = settings(shell, running);
    s.auto_check = false;
    Machine::new(s)
}

fn offer(version: &str, has_package: bool) -> Offer {
    Offer {
        version: v(version),
        has_package,
        can_install: true,
    }
}

fn candidate(version: &str, kind: Kind) -> Candidate {
    Candidate {
        version: v(version),
        kind,
    }
}

fn check(m: &mut Machine, trigger: Trigger) -> u64 {
    let effects = m.handle(Event::CheckRequested(trigger));
    match effects.iter().find_map(|effect| match effect {
        Effect::StartCheck { ticket } => Some(*ticket),
        _ => None,
    }) {
        Some(ticket) => ticket,
        None => panic!("expected a check to start, got {effects:?}"),
    }
}

fn finish(m: &mut Machine, ticket: u64, offer: Option<Offer>) -> Vec<Effect> {
    m.handle(Event::CheckFinished {
        ticket,
        result: CheckResult::Release(offer),
    })
}

fn notices(effects: &[Effect]) -> Vec<&Notice> {
    effects
        .iter()
        .filter_map(|effect| match effect {
            Effect::Notify(notice) => Some(notice),
            _ => None,
        })
        .collect()
}

fn logs(effects: &[Effect]) -> Vec<&Record> {
    effects
        .iter()
        .filter_map(|effect| match effect {
            Effect::Log(record) => Some(record),
            _ => None,
        })
        .collect()
}

fn started_download(effects: &[Effect]) -> Option<(u64, &Candidate)> {
    effects.iter().find_map(|effect| match effect {
        Effect::StartDownload { ticket, candidate } => Some((*ticket, candidate)),
        _ => None,
    })
}

// ---- Which path an update takes -----------------------------------------------------------------

#[test]
fn a_package_on_the_shells_major_minor_line_is_a_package_update() {
    let mut m = classifying("0.1.0", "0.1.0");
    let t = check(&mut m, Trigger::Manual);
    finish(&mut m, t, Some(offer("0.1.5", true)));
    assert_eq!(
        m.phase,
        Phase::Checked(Outcome::PackageUpdate(candidate("0.1.5", Kind::Package)))
    );
}

#[test]
fn a_package_for_another_major_minor_line_needs_the_app_updated() {
    // The package carries TSX only; the shell holds the Rust it calls. A different line means a
    // shell change, so the code package must not run on this shell.
    let mut m = machine("0.1.0");
    let t = check(&mut m, Trigger::Manual);
    let effects = finish(&mut m, t, Some(offer("0.2.0", true)));
    assert_eq!(
        m.phase,
        Phase::Checked(Outcome::ShellUpdate {
            candidate: candidate("0.2.0", Kind::Shell),
            reason: ShellReason::ShellTooOld,
        })
    );
    assert_eq!(
        notices(&effects),
        vec![&Notice::ShellUpdateNeeded(v("0.2.0"))]
    );
    // The major line counts too.
    let mut m = machine("0.9.0");
    let t = check(&mut m, Trigger::Manual);
    finish(&mut m, t, Some(offer("1.0.0", true)));
    assert!(matches!(
        m.phase,
        Phase::Checked(Outcome::ShellUpdate {
            reason: ShellReason::ShellTooOld,
            ..
        })
    ));
}

#[test]
fn a_release_without_a_package_is_a_whole_app_update() {
    let mut m = machine("0.1.0");
    let t = check(&mut m, Trigger::Manual);
    let effects = finish(&mut m, t, Some(offer("0.1.1", false)));
    assert_eq!(
        m.phase,
        Phase::Checked(Outcome::ShellUpdate {
            candidate: candidate("0.1.1", Kind::Shell),
            reason: ShellReason::NoPackage,
        })
    );
    // Not "the app needs updating": the user just has a whole-app update to choose.
    assert_eq!(
        notices(&effects),
        vec![&Notice::UpdateAvailable(v("0.1.1"))]
    );
}

#[test]
fn the_shell_line_is_what_the_package_must_match_not_the_running_version() {
    // Normally the running package is on the shell's line, so the two agree and a test cannot
    // tell them apart. They differ when the app was just replaced by a whole-app update but the
    // process still reports an older running version, or when an old package is still running
    // on a newer shell. The rule is about the shell: it is what holds the Rust the TSX calls.
    //
    // Shell 0.2.0 (updated whole-app), still running the 0.1.4 package from before:
    let mut m = classifying("0.2.0", "0.1.4");
    let t = check(&mut m, Trigger::Manual);
    finish(&mut m, t, Some(offer("0.2.1", true)));
    assert!(
        matches!(m.phase, Phase::Checked(Outcome::PackageUpdate(_))),
        "0.2.1 is on the shell's 0.2 line, so its package fits, whatever is running"
    );
    let t = check(&mut m, Trigger::Manual);
    finish(&mut m, t, Some(offer("0.1.9", true)));
    assert_eq!(
        m.phase,
        Phase::Checked(Outcome::UpToDate),
        "0.1.9 is newer than the running 0.1.4 but is not on the shell's line and is older than the shell"
    );
    // And the other way round: shell 0.1.0 with a (wrongly) higher running version.
    let mut m = classifying("0.1.0", "0.2.0");
    let t = check(&mut m, Trigger::Manual);
    finish(&mut m, t, Some(offer("0.2.1", true)));
    assert!(
        matches!(
            m.phase,
            Phase::Checked(Outcome::ShellUpdate {
                reason: ShellReason::ShellTooOld,
                ..
            })
        ),
        "0.2.1 matches the running 0.2 line but not the shell's 0.1: the shell is what counts"
    );
}

// ---- What is never offered ------------------------------------------------------------------------

#[test]
fn nothing_not_newer_than_what_runs_is_offered() {
    let mut m = Machine::new(settings("0.1.0", "0.1.4"));
    for offered in ["0.1.4", "0.1.3", "0.1.0", "0.0.9"] {
        let t = check(&mut m, Trigger::Manual);
        finish(&mut m, t, Some(offer(offered, true)));
        assert_eq!(m.phase, Phase::Checked(Outcome::UpToDate), "{offered}");
    }
}

#[test]
fn a_pre_release_is_never_offered() {
    // One channel, the formal releases. The source should not offer a pre-release; if it does,
    // the machine still refuses it.
    let mut m = machine("0.1.0");
    for offered in ["0.1.1-alpha.1", "0.2.0-rc.1", "1.0.0-beta.2"] {
        let t = check(&mut m, Trigger::Manual);
        finish(&mut m, t, Some(offer(offered, true)));
        assert_eq!(m.phase, Phase::Checked(Outcome::UpToDate), "{offered}");
    }
}

// ---- Checks -----------------------------------------------------------------------------------------

#[test]
fn a_manual_check_says_up_to_date_and_a_background_one_stays_quiet() {
    let mut m = machine("1.0.0");
    let t = check(&mut m, Trigger::Manual);
    assert_eq!(notices(&finish(&mut m, t, None)), vec![&Notice::UpToDate]);
    let t = check(&mut m, Trigger::Background);
    assert!(notices(&finish(&mut m, t, None)).is_empty());
    assert_eq!(m.phase, Phase::Checked(Outcome::UpToDate));
}

#[test]
fn a_second_request_while_checking_joins_the_running_check() {
    let mut m = machine("1.0.0");
    let t = check(&mut m, Trigger::Background);
    assert!(m.handle(Event::CheckRequested(Trigger::Manual)).is_empty());
    assert!(m.handle(Event::CheckRequested(Trigger::Manual)).is_empty());
    // The user asked meanwhile, so the answer is shown to them.
    assert_eq!(notices(&finish(&mut m, t, None)), vec![&Notice::UpToDate]);
}

#[test]
fn a_late_answer_to_a_superseded_check_is_ignored() {
    let mut m = machine("1.0.0");
    let first = check(&mut m, Trigger::Manual);
    finish(&mut m, first, None);
    let second = check(&mut m, Trigger::Manual);
    assert_ne!(first, second);
    // The first check answers again, late, with an update: it is not the current question.
    assert!(finish(&mut m, first, Some(offer("1.1.0", true))).is_empty());
    assert_eq!(m.phase, Phase::Checking);
}

#[test]
fn a_check_while_an_update_is_in_flight_does_nothing() {
    let mut m = machine("0.1.0");
    let t = check(&mut m, Trigger::Manual);
    finish(&mut m, t, Some(offer("0.1.1", true)));
    m.handle(Event::DownloadRequested);
    assert!(matches!(m.phase, Phase::Downloading { .. }));
    assert!(m.candidate_locked());
    assert!(m.handle(Event::CheckRequested(Trigger::Manual)).is_empty());
    assert!(matches!(m.phase, Phase::Downloading { .. }));
}

// ---- Automatic checks and downloads (D4, D10) ------------------------------------------------------

#[test]
fn with_automatic_checks_on_a_package_downloads_without_asking() {
    let mut m = machine("0.1.0");
    let t = check(&mut m, Trigger::Background);
    let effects = finish(&mut m, t, Some(offer("0.1.1", true)));
    let (_, started) = started_download(&effects).expect("the package downloads by itself");
    assert_eq!(started, &candidate("0.1.1", Kind::Package));
    assert!(matches!(m.phase, Phase::Downloading { .. }));
}

#[test]
fn a_whole_app_update_never_downloads_without_the_user() {
    let mut m = machine("0.1.0");
    let t = check(&mut m, Trigger::Background);
    let effects = finish(&mut m, t, Some(offer("0.2.0", true)));
    assert!(started_download(&effects).is_none());
    assert!(matches!(
        m.phase,
        Phase::Checked(Outcome::ShellUpdate { .. })
    ));
    let t = check(&mut m, Trigger::Background);
    let effects = finish(&mut m, t, Some(offer("0.1.1", false)));
    assert!(started_download(&effects).is_none());
    // The user chooses it.
    let effects = m.handle(Event::DownloadRequested);
    assert!(started_download(&effects).is_some());
}

#[test]
fn with_automatic_checks_off_there_are_no_background_checks_and_nothing_downloads_by_itself() {
    let mut s = settings("0.1.0", "0.1.0");
    s.auto_check = false;
    let mut m = Machine::new(s);
    // A timer or startup asks; the machine refuses.
    assert!(
        m.handle(Event::CheckRequested(Trigger::Background))
            .is_empty()
    );
    assert_eq!(m.phase, Phase::Idle);
    // The user asks: the check runs, and the package waits for their go-ahead.
    let t = check(&mut m, Trigger::Manual);
    let effects = finish(&mut m, t, Some(offer("0.1.1", true)));
    assert!(started_download(&effects).is_none());
    assert!(matches!(m.phase, Phase::Checked(Outcome::PackageUpdate(_))));
    assert!(started_download(&m.handle(Event::DownloadRequested)).is_some());
}

#[test]
fn switching_automatic_checks_applies_to_the_next_check_only() {
    let mut m = machine("0.1.0");
    m.set_auto_check(false);
    assert!(!m.auto_check());
    assert!(
        m.handle(Event::CheckRequested(Trigger::Background))
            .is_empty()
    );
    m.set_auto_check(true);
    assert!(check_started(&mut m));
}

fn check_started(m: &mut Machine) -> bool {
    m.handle(Event::CheckRequested(Trigger::Background))
        .iter()
        .any(|effect| matches!(effect, Effect::StartCheck { .. }))
}

// ---- The install path ----------------------------------------------------------------------------------

#[test]
fn an_update_moves_forward_through_its_steps_in_order() {
    let mut m = machine("0.1.0");
    let t = check(&mut m, Trigger::Manual);
    assert_eq!(
        notices(&finish(&mut m, t, Some(offer("0.1.1", false)))),
        vec![&Notice::UpdateAvailable(v("0.1.1"))]
    );
    let effects = m.handle(Event::DownloadRequested);
    let (ticket, _) = started_download(&effects).unwrap();
    assert!(m.candidate_locked());
    m.handle(Event::DownloadProgress {
        ticket,
        downloaded: 10,
        total: Some(100),
    });
    assert!(matches!(
        m.phase,
        Phase::Downloading {
            downloaded: 10,
            total: Some(100),
            ..
        }
    ));
    m.handle(Event::DownloadFinished { ticket });
    // NSH verified it inside the download: there is no separate verifying step.
    assert!(matches!(m.phase, Phase::ReadyToInstall(_)));
    let effects = m.handle(Event::InstallRequested);
    assert!(
        effects
            .iter()
            .any(|effect| matches!(effect, Effect::StartInstall { .. }))
    );
    assert!(matches!(m.phase, Phase::Installing(_)));
    m.handle(Event::InstallHandedOff { ticket });
    assert!(matches!(m.phase, Phase::AwaitingFirstLaunch(_)));
    m.handle(Event::FirstLaunchConfirmed);
    assert_eq!(m.phase, Phase::Idle);
}

#[test]
fn steps_cannot_run_out_of_order() {
    let mut m = machine("0.1.0");
    assert!(
        m.handle(Event::DownloadRequested).is_empty(),
        "no candidate yet"
    );
    assert!(
        m.handle(Event::InstallRequested).is_empty(),
        "nothing is ready"
    );
    let t = check(&mut m, Trigger::Manual);
    finish(&mut m, t, Some(offer("0.1.1", false)));
    assert!(
        m.handle(Event::InstallRequested).is_empty(),
        "not downloaded yet"
    );
    m.handle(Event::DownloadRequested);
    assert!(
        m.handle(Event::InstallRequested).is_empty(),
        "still downloading"
    );
}

#[test]
fn late_progress_and_finish_events_from_an_old_ticket_change_nothing() {
    let mut m = machine("0.1.0");
    let t = check(&mut m, Trigger::Manual);
    finish(&mut m, t, Some(offer("0.1.1", false)));
    let effects = m.handle(Event::DownloadRequested);
    let (ticket, _) = started_download(&effects).unwrap();
    let before = m.phase.clone();
    m.handle(Event::DownloadProgress {
        ticket: ticket - 1,
        downloaded: 50,
        total: None,
    });
    assert_eq!(
        m.handle(Event::DownloadFinished { ticket: ticket - 1 }),
        vec![]
    );
    assert_eq!(m.phase, before);
    assert_eq!(
        m.handle(Event::StepFailed {
            ticket: ticket - 1,
            failure: Failure::Disk
        }),
        vec![]
    );
    assert_eq!(m.phase, before);
}

// ---- Failures -----------------------------------------------------------------------------------------

#[test]
fn a_failed_check_is_told_to_a_manual_check_and_swallowed_by_a_background_one() {
    let mut m = machine("1.0.0");
    let t = check(&mut m, Trigger::Manual);
    let effects = m.handle(Event::CheckFinished {
        ticket: t,
        result: CheckResult::Failed(Failure::Network),
    });
    assert_eq!(
        notices(&effects),
        vec![&Notice::CheckFailed(Failure::Network)]
    );
    let t = check(&mut m, Trigger::Background);
    let effects = m.handle(Event::CheckFinished {
        ticket: t,
        result: CheckResult::Failed(Failure::Busy),
    });
    assert!(notices(&effects).is_empty());
    assert_eq!(m.phase, Phase::Checked(Outcome::Failed(Failure::Busy)));
    // Retrying after a failure works.
    assert!(check_started(&mut m));
}

#[test]
fn every_failure_has_a_stable_name_and_only_three_count() {
    let all = [
        (Failure::Network, "network", false),
        (Failure::Busy, "busy", false),
        (Failure::Rejected, "rejected", true),
        (Failure::Disk, "disk", true),
        (Failure::Cancelled, "cancelled", false),
        (Failure::Unknown, "unknown", true),
    ];
    for (failure, name, counts) in all {
        assert_eq!(failure.as_str(), name);
        assert_eq!(failure.counts(), counts, "{name}");
    }
}

/// Runs one download that fails with `failure` and returns the machine afterwards.
fn failed_download(m: &mut Machine, version: &str, failure: Failure) {
    let t = check(m, Trigger::Manual);
    finish(m, t, Some(offer(version, false)));
    let effects = m.handle(Event::DownloadRequested);
    let (ticket, _) = started_download(&effects).unwrap();
    m.handle(Event::StepFailed { ticket, failure });
}

#[test]
fn a_failed_step_is_recorded_and_the_candidate_can_be_tried_again() {
    let mut m = machine("0.1.0");
    failed_download(&mut m, "0.1.1", Failure::Network);
    assert!(matches!(
        m.phase,
        Phase::InstallFailed {
            failure: Failure::Network,
            ..
        }
    ));
    assert!(started_download(&m.handle(Event::DownloadRequested)).is_some());
}

#[test]
fn a_rejected_candidate_cannot_be_retried_because_retrying_cannot_help() {
    let mut m = machine("0.1.0");
    failed_download(&mut m, "0.1.1", Failure::Rejected);
    assert!(started_download(&m.handle(Event::DownloadRequested)).is_none());
    assert!(matches!(m.phase, Phase::InstallFailed { .. }));
}

#[test]
fn only_counted_failures_use_up_a_versions_budget() {
    let mut m = machine("0.1.0");
    let version = v("0.1.1");
    for failure in [
        Failure::Network,
        Failure::Busy,
        Failure::Cancelled,
        Failure::Network,
    ] {
        failed_download(&mut m, "0.1.1", failure);
    }
    assert_eq!(
        m.failures_of(&version),
        0,
        "going offline must not hide a good update"
    );
    assert!(!m.exhausted(&version));
    failed_download(&mut m, "0.1.1", Failure::Disk);
    assert_eq!(m.failures_of(&version), 1);
    failed_download(&mut m, "0.1.1", Failure::Unknown);
    assert_eq!(m.failures_of(&version), 2);
    assert!(m.exhausted(&version));
}

#[test]
fn a_version_that_keeps_failing_stops_being_offered_to_background_checks_only() {
    let mut m = machine("0.1.0");
    failed_download(&mut m, "0.1.1", Failure::Disk);
    failed_download(&mut m, "0.1.1", Failure::Disk);
    assert!(m.exhausted(&v("0.1.1")));
    // The background check does not bring it back, which is what ends the "same update, same
    // failure, again" loop.
    let t = check(&mut m, Trigger::Background);
    let effects = finish(&mut m, t, Some(offer("0.1.1", false)));
    assert_eq!(m.phase, Phase::Checked(Outcome::UpToDate));
    assert!(notices(&effects).is_empty());
    // The user asking does.
    let t = check(&mut m, Trigger::Manual);
    finish(&mut m, t, Some(offer("0.1.1", false)));
    assert!(matches!(
        m.phase,
        Phase::Checked(Outcome::ShellUpdate { .. })
    ));
    // A newer version is a different version, with its own budget.
    let t = check(&mut m, Trigger::Background);
    finish(&mut m, t, Some(offer("0.1.2", false)));
    assert!(matches!(
        m.phase,
        Phase::Checked(Outcome::ShellUpdate { .. })
    ));
}

#[test]
fn the_failure_memory_is_bounded_and_forgets_the_oldest_version_first() {
    // The machine is cloned into every published snapshot, so it must not grow without limit.
    let mut m = machine("0.1.0");
    for patch in 1..=(FAILURE_MEMORY as u64 + 3) {
        failed_download(&mut m, &format!("0.1.{patch}"), Failure::Disk);
    }
    assert_eq!(m.failures_of(&v("0.1.1")), 0, "the oldest is forgotten");
    assert_eq!(m.failures_of(&v("0.1.2")), 0);
    assert_eq!(m.failures_of(&v("0.1.3")), 0);
    assert_eq!(m.failures_of(&v("0.1.4")), 1);
    assert_eq!(m.failures_of(&v(&format!("0.1.{}", FAILURE_MEMORY + 3))), 1);
}

// ---- Capability --------------------------------------------------------------------------------------

#[test]
fn capability_decides_between_in_app_manual_and_system_managed() {
    let mut s = settings("0.1.0", "0.1.0");
    s.capability = Capability::ManualOnly;
    s.auto_check = false;
    let mut manual = Machine::new(s);
    let t = check(&mut manual, Trigger::Manual);
    finish(&mut manual, t, Some(offer("0.1.1", false)));
    assert!(matches!(
        manual.phase,
        Phase::Checked(Outcome::ManualUpgrade {
            reason: ManualReason::Environment,
            ..
        })
    ));
    // Nothing to download: the user is sent to the download page.
    assert!(manual.handle(Event::DownloadRequested).is_empty());
    // A code package replaces nothing, so a read-only install location does not stop it.
    let t = check(&mut manual, Trigger::Manual);
    finish(&mut manual, t, Some(offer("0.1.2", true)));
    assert!(matches!(
        manual.phase,
        Phase::Checked(Outcome::PackageUpdate(_))
    ));

    for (capability, outcome) in [
        (Capability::SystemManaged, Outcome::SystemManaged),
        (Capability::Unsupported, Outcome::NotApplicable),
    ] {
        let mut s = settings("0.1.0", "0.1.0");
        s.capability = capability;
        let mut m = Machine::new(s);
        // No request goes out: the platform owns updates or has none.
        assert!(m.handle(Event::CheckRequested(Trigger::Manual)).is_empty());
        assert_eq!(m.phase, Phase::Checked(outcome));
    }
}

// ---- Log records --------------------------------------------------------------------------------------

#[test]
fn a_failure_logs_at_the_level_its_class_calls_for_with_typed_fields() {
    let cases = [
        (Failure::Network, Level::Warn),
        (Failure::Busy, Level::Warn),
        (Failure::Cancelled, Level::Info),
        (Failure::Rejected, Level::Error),
        (Failure::Disk, Level::Error),
        (Failure::Unknown, Level::Error),
    ];
    for (failure, level) in cases {
        let mut m = machine("0.1.0");
        let t = check(&mut m, Trigger::Manual);
        finish(&mut m, t, Some(offer("0.1.1", false)));
        let effects = m.handle(Event::DownloadRequested);
        let (ticket, _) = started_download(&effects).unwrap();
        let effects = m.handle(Event::StepFailed { ticket, failure });
        let records = logs(&effects);
        assert_eq!(records.len(), 1, "{failure:?}");
        let record = records[0];
        assert_eq!(record.level, level, "{failure:?}");
        assert_eq!(record.event, "step_failed");
        assert_eq!(record.failure, Some(failure));
        assert_eq!(record.version, Some(v("0.1.1")));
        assert_eq!(record.kind, Some(Kind::Shell));
        assert_eq!(record.ticket, Some(ticket));
        // Only a counted failure carries how many times this version has failed.
        assert_eq!(record.failures.is_some(), failure.counts(), "{failure:?}");
    }
}

#[test]
fn a_failed_check_logs_its_failure_class() {
    let mut m = machine("0.1.0");
    let t = check(&mut m, Trigger::Background);
    let effects = m.handle(Event::CheckFinished {
        ticket: t,
        result: CheckResult::Failed(Failure::Network),
    });
    let records = logs(&effects);
    assert_eq!(records.len(), 1);
    assert_eq!(records[0].event, "check_failed");
    assert_eq!(records[0].level, Level::Warn);
    assert_eq!(records[0].failure, Some(Failure::Network));
}

#[test]
fn each_step_of_an_update_leaves_a_record_naming_it() {
    let mut m = machine("0.1.0");
    let mut seen: Vec<&'static str> = Vec::new();
    let mut take = |effects: Vec<Effect>| {
        seen.extend(logs(&effects).iter().map(|record| record.event));
    };
    take(m.handle(Event::CheckRequested(Trigger::Manual)));
    let t = m.ticket();
    take(finish(&mut m, t, Some(offer("0.1.1", false))));
    take(m.handle(Event::DownloadRequested));
    let t = m.ticket();
    take(m.handle(Event::DownloadFinished { ticket: t }));
    take(m.handle(Event::InstallRequested));
    take(m.handle(Event::InstallHandedOff { ticket: t }));
    take(m.handle(Event::FirstLaunchConfirmed));
    assert_eq!(
        seen,
        [
            "check_started",
            "update_found",
            "download_started",
            "download_finished",
            "install_started",
            "install_handed_off",
            "first_launch_confirmed",
        ]
    );
}

#[test]
fn automatic_package_download_stays_silent_until_ready() {
    let mut machine = machine("0.1.0");
    let ticket = check(&mut machine, Trigger::Background);
    let effects = finish(&mut machine, ticket, Some(offer("0.1.1", true)));
    assert!(notices(&effects).is_empty());
    let (ticket, _) = started_download(&effects).unwrap();
    machine.handle(Event::DownloadFinished { ticket });
    assert!(matches!(machine.phase, Phase::ReadyToInstall(_)));
}

#[test]
fn per_offer_install_permission_limits_shell_updates_but_not_compatible_packages() {
    for (version, package, expected_package) in [
        ("0.1.1", true, true),
        ("0.1.1", false, false),
        ("0.2.0", true, false),
    ] {
        let mut m = classifying("0.1.0", "0.1.0");
        let ticket = check(&mut m, Trigger::Manual);
        let mut release = offer(version, package);
        release.can_install = false;
        finish(&mut m, ticket, Some(release));
        if expected_package {
            assert!(matches!(m.phase, Phase::Checked(Outcome::PackageUpdate(_))));
        } else {
            assert!(matches!(
                m.phase,
                Phase::Checked(Outcome::ManualUpgrade { .. })
            ));
        }
        // Permission is checked per release; a later installable release is not permanently blocked.
        let ticket = check(&mut m, Trigger::Manual);
        finish(&mut m, ticket, Some(offer("0.2.1", false)));
        assert!(matches!(
            m.phase,
            Phase::Checked(Outcome::ShellUpdate { .. })
        ));
    }
}
