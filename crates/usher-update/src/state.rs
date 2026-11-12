//! The update state machine. A reducer: `(state, event) -> (state, effects)`, no I/O.
//!
//! The caller performs the effects (network, disk, restart) and feeds results back as
//! events. That keeps every rule here testable without a network or a window:
//!
//! - one check at a time: a second request while one runs only returns the running one;
//! - each effect carries a `ticket`; a result with an old ticket is dropped, so a slow
//!   check cannot overwrite the result of a newer one after a channel switch or a cancel;
//! - the stable channel never offers a pre-release; switching channels never downgrades;
//! - a candidate is locked once its download starts;
//! - the install steps only move forward: Downloading → Verifying → ReadyToInstall →
//!   Installing → AwaitingFirstLaunch.

use semver::Version;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum Channel {
    Stable,
    Preview,
}

impl Channel {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Stable => "stable",
            Self::Preview => "preview",
        }
    }

    /// Whether this channel may be offered `version`.
    pub fn accepts(self, version: &Version) -> bool {
        self == Self::Preview || version.pre.is_empty()
    }
}

/// How this installation can be updated, which is separate from what a release asks for.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum Capability {
    /// The app may download, verify and hand over a package itself.
    InApp,
    /// The environment can only point the user at a download page.
    ManualOnly,
    /// A store or package manager owns updates.
    SystemManaged,
    /// This platform or install form has no update path.
    Unsupported,
}

/// A release that can be offered, bound to what it is for.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Candidate {
    pub version: Version,
    pub channel: Channel,
    /// Lowest running version that may update in place; older ones need the full installer.
    pub min_version: Option<Version>,
    /// Whether the signed manifest and the package were both authenticated.
    pub verified: bool,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum Outcome {
    UpToDate,
    /// Can be installed from inside the app.
    InAppUpdate(Candidate),
    /// A newer release exists, but the user has to install it.
    ManualUpgrade {
        candidate: Candidate,
        reason: ManualReason,
    },
    SystemManaged,
    NotApplicable,
    Failed(Failure),
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum ManualReason {
    /// The release asks for a manual install (its `min-version` is above this build).
    ReleasePolicy,
    /// This environment cannot install by itself.
    Environment,
    /// The release could not be authenticated, so it must not be installed in place.
    Unverified,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum Failure {
    Network,
    NotFound,
    RateLimited,
    InvalidResponse,
    Signature,
    Disk,
    Cancelled,
    Unknown,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum Phase {
    Idle,
    Checking,
    /// Holds the last outcome of a check.
    Checked(Outcome),
    Downloading {
        candidate: Candidate,
        downloaded: u64,
        total: Option<u64>,
    },
    Verifying(Candidate),
    ReadyToInstall(Candidate),
    Installing(Candidate),
    AwaitingFirstLaunch(Candidate),
    InstallFailed {
        candidate: Candidate,
        failure: Failure,
    },
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum Trigger {
    /// The user asked.
    Manual,
    /// A timer or startup.
    Background,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum Event {
    CheckRequested(Trigger),
    /// The result of the check started with `ticket`.
    CheckFinished {
        ticket: u64,
        result: CheckResult,
    },
    ChannelChanged(Channel),
    DownloadRequested,
    DownloadProgress {
        ticket: u64,
        downloaded: u64,
        total: Option<u64>,
    },
    DownloadFinished {
        ticket: u64,
    },
    VerifyFinished {
        ticket: u64,
        ok: bool,
    },
    InstallRequested,
    InstallHandedOff {
        ticket: u64,
    },
    InstallFailed {
        ticket: u64,
        failure: Failure,
    },
    Cancel,
    /// The new version started and confirmed itself.
    FirstLaunchConfirmed,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum CheckResult {
    /// The newest release the server offers for the asked channel, if any.
    Release(Option<Candidate>),
    Failed(Failure),
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum Effect {
    StartCheck {
        ticket: u64,
        channel: Channel,
    },
    StartDownload {
        ticket: u64,
        candidate: Candidate,
    },
    StartVerify {
        ticket: u64,
        candidate: Candidate,
    },
    /// Save settings and work state, then hand the package to the installer.
    StartInstall {
        ticket: u64,
        candidate: Candidate,
    },
    CancelRunning,
    /// Tell the user. Background checks stay quiet unless something needs them.
    Notify(Notice),
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum Notice {
    UpToDate,
    UpdateAvailable(Version),
    ManualUpgradeNeeded(Version),
    CheckFailed(Failure),
}

#[derive(Clone, Debug)]
pub struct Machine {
    pub phase: Phase,
    pub current: Version,
    pub channel: Channel,
    pub capability: Capability,
    ticket: u64,
    /// The check being run: its ticket and who asked.
    running_check: Option<(u64, Trigger)>,
}

impl Machine {
    pub fn new(current: Version, channel: Channel, capability: Capability) -> Self {
        Self {
            phase: Phase::Idle,
            current,
            channel,
            capability,
            ticket: 0,
            running_check: None,
        }
    }

    fn next_ticket(&mut self) -> u64 {
        self.ticket += 1;
        self.ticket
    }

    /// The ticket a result must carry to be accepted now.
    pub fn ticket(&self) -> u64 {
        self.ticket
    }

    /// Whether a candidate is locked: from the start of its download until it is installed.
    pub fn candidate_locked(&self) -> bool {
        matches!(
            self.phase,
            Phase::Downloading { .. }
                | Phase::Verifying(_)
                | Phase::ReadyToInstall(_)
                | Phase::Installing(_)
                | Phase::AwaitingFirstLaunch(_)
        )
    }

    fn busy_installing(&self) -> bool {
        self.candidate_locked()
    }

    pub fn handle(&mut self, event: Event) -> Vec<Effect> {
        match event {
            Event::CheckRequested(trigger) => self.check_requested(trigger),
            Event::CheckFinished { ticket, result } => self.check_finished(ticket, result),
            Event::ChannelChanged(channel) => self.channel_changed(channel),
            Event::DownloadRequested => self.download_requested(),
            Event::DownloadProgress {
                ticket,
                downloaded,
                total,
            } => {
                if ticket == self.ticket
                    && let Phase::Downloading { candidate, .. } = &self.phase
                {
                    self.phase = Phase::Downloading {
                        candidate: candidate.clone(),
                        downloaded,
                        total,
                    };
                }
                vec![]
            }
            Event::DownloadFinished { ticket } => {
                if ticket != self.ticket {
                    return vec![];
                }
                let Phase::Downloading { candidate, .. } = &self.phase else {
                    return vec![];
                };
                let candidate = candidate.clone();
                self.phase = Phase::Verifying(candidate.clone());
                vec![Effect::StartVerify { ticket, candidate }]
            }
            Event::VerifyFinished { ticket, ok } => {
                if ticket != self.ticket {
                    return vec![];
                }
                let Phase::Verifying(candidate) = &self.phase else {
                    return vec![];
                };
                let candidate = candidate.clone();
                self.phase = if ok {
                    Phase::ReadyToInstall(candidate)
                } else {
                    Phase::InstallFailed {
                        candidate,
                        failure: Failure::Signature,
                    }
                };
                vec![]
            }
            Event::InstallRequested => {
                let Phase::ReadyToInstall(candidate) = &self.phase else {
                    return vec![];
                };
                let candidate = candidate.clone();
                let ticket = self.ticket;
                self.phase = Phase::Installing(candidate.clone());
                vec![Effect::StartInstall { ticket, candidate }]
            }
            Event::InstallHandedOff { ticket } => {
                if ticket == self.ticket
                    && let Phase::Installing(candidate) = &self.phase
                {
                    self.phase = Phase::AwaitingFirstLaunch(candidate.clone());
                }
                vec![]
            }
            Event::InstallFailed { ticket, failure } => {
                if ticket == self.ticket {
                    let candidate = match &self.phase {
                        Phase::Downloading { candidate, .. } => Some(candidate),
                        Phase::Verifying(c) | Phase::Installing(c) => Some(c),
                        _ => None,
                    };
                    if let Some(candidate) = candidate.cloned() {
                        self.phase = Phase::InstallFailed { candidate, failure };
                    }
                }
                vec![]
            }
            Event::Cancel => self.cancel(),
            Event::FirstLaunchConfirmed => {
                if matches!(self.phase, Phase::AwaitingFirstLaunch(_)) {
                    self.phase = Phase::Idle;
                }
                vec![]
            }
        }
    }

    fn check_requested(&mut self, trigger: Trigger) -> Vec<Effect> {
        if self.busy_installing() {
            // An update is already in flight; a check would only confuse it.
            return vec![];
        }
        if let Some((_, started_by)) = &mut self.running_check {
            // One executor: the running check serves this request too. If the user asked
            // while a background check ran, the result is shown to them.
            if trigger == Trigger::Manual {
                *started_by = Trigger::Manual;
            }
            return vec![];
        }
        match self.capability {
            Capability::SystemManaged => {
                self.phase = Phase::Checked(Outcome::SystemManaged);
                return vec![];
            }
            Capability::Unsupported => {
                self.phase = Phase::Checked(Outcome::NotApplicable);
                return vec![];
            }
            Capability::InApp | Capability::ManualOnly => {}
        }
        let ticket = self.next_ticket();
        self.running_check = Some((ticket, trigger));
        self.phase = Phase::Checking;
        vec![Effect::StartCheck {
            ticket,
            channel: self.channel,
        }]
    }

    fn check_finished(&mut self, ticket: u64, result: CheckResult) -> Vec<Effect> {
        let Some((running, trigger)) = self.running_check else {
            return vec![];
        };
        if ticket != running || ticket != self.ticket {
            // A superseded check (channel switch or cancel): its answer is for a question
            // nobody is asking any more.
            return vec![];
        }
        self.running_check = None;
        let outcome = match result {
            CheckResult::Failed(failure) => Outcome::Failed(failure),
            CheckResult::Release(None) => Outcome::UpToDate,
            CheckResult::Release(Some(candidate)) => self.classify(candidate),
        };
        let notice = notice_for(&outcome, trigger);
        self.phase = Phase::Checked(outcome);
        notice.map(Effect::Notify).into_iter().collect()
    }

    /// Decides what to do with a release the server offered.
    fn classify(&self, candidate: Candidate) -> Outcome {
        if candidate.version <= self.current || !self.channel.accepts(&candidate.version) {
            // Never a downgrade, and never a pre-release on the stable channel.
            return Outcome::UpToDate;
        }
        if candidate.channel != self.channel {
            return Outcome::UpToDate;
        }
        let too_old = candidate
            .min_version
            .as_ref()
            .is_some_and(|min| &self.current < min);
        if too_old {
            return Outcome::ManualUpgrade {
                candidate,
                reason: ManualReason::ReleasePolicy,
            };
        }
        if !candidate.verified {
            return Outcome::ManualUpgrade {
                candidate,
                reason: ManualReason::Unverified,
            };
        }
        match self.capability {
            Capability::InApp => Outcome::InAppUpdate(candidate),
            Capability::ManualOnly => Outcome::ManualUpgrade {
                candidate,
                reason: ManualReason::Environment,
            },
            Capability::SystemManaged => Outcome::SystemManaged,
            Capability::Unsupported => Outcome::NotApplicable,
        }
    }

    fn channel_changed(&mut self, channel: Channel) -> Vec<Effect> {
        if channel == self.channel || self.candidate_locked() {
            // A locked candidate finishes first; the new channel applies to the next check.
            if self.candidate_locked() {
                self.channel = channel;
            }
            return vec![];
        }
        self.channel = channel;
        // Whatever was being checked answered for the old channel. Drop it, and forget a
        // result that belonged to it; a downgrade is never offered because `classify`
        // compares against the running version.
        self.running_check = None;
        self.ticket += 1;
        self.phase = Phase::Idle;
        vec![]
    }

    fn download_requested(&mut self) -> Vec<Effect> {
        let Phase::Checked(Outcome::InAppUpdate(candidate)) = &self.phase else {
            return vec![];
        };
        let candidate = candidate.clone();
        let ticket = self.next_ticket();
        self.phase = Phase::Downloading {
            candidate: candidate.clone(),
            downloaded: 0,
            total: None,
        };
        vec![Effect::StartDownload { ticket, candidate }]
    }

    fn cancel(&mut self) -> Vec<Effect> {
        match &self.phase {
            Phase::Checking => {
                self.running_check = None;
                self.ticket += 1;
                self.phase = Phase::Idle;
                vec![Effect::CancelRunning]
            }
            Phase::Downloading { candidate, .. } => {
                // The candidate is still the one the user chose; they can start again.
                let candidate = candidate.clone();
                self.ticket += 1;
                self.phase = Phase::Checked(Outcome::InAppUpdate(candidate));
                vec![Effect::CancelRunning]
            }
            // Past verification the package is handed over or ready: not cancellable here.
            _ => vec![],
        }
    }
}

fn notice_for(outcome: &Outcome, trigger: Trigger) -> Option<Notice> {
    match (outcome, trigger) {
        (Outcome::UpToDate, Trigger::Manual) => Some(Notice::UpToDate),
        (Outcome::UpToDate, Trigger::Background) => None,
        (Outcome::InAppUpdate(c), _) => Some(Notice::UpdateAvailable(c.version.clone())),
        (Outcome::ManualUpgrade { candidate, .. }, _) => {
            Some(Notice::ManualUpgradeNeeded(candidate.version.clone()))
        }
        // A background check that merely failed (offline, rate limited) stays silent.
        (Outcome::Failed(failure), Trigger::Manual) => Some(Notice::CheckFailed(*failure)),
        (Outcome::Failed(_), Trigger::Background) => None,
        (Outcome::SystemManaged | Outcome::NotApplicable, _) => None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn v(text: &str) -> Version {
        Version::parse(text).unwrap()
    }

    fn candidate(version: &str) -> Candidate {
        Candidate {
            version: v(version),
            channel: Channel::Stable,
            min_version: None,
            verified: true,
        }
    }

    fn machine(current: &str) -> Machine {
        Machine::new(v(current), Channel::Stable, Capability::InApp)
    }

    fn start_check(m: &mut Machine, trigger: Trigger) -> u64 {
        match m.handle(Event::CheckRequested(trigger)).as_slice() {
            [Effect::StartCheck { ticket, .. }] => *ticket,
            other => panic!("expected one check, got {other:?}"),
        }
    }

    fn finish(m: &mut Machine, ticket: u64, release: Option<Candidate>) -> Vec<Effect> {
        m.handle(Event::CheckFinished {
            ticket,
            result: CheckResult::Release(release),
        })
    }

    #[test]
    fn a_manual_check_reports_up_to_date_but_a_background_one_stays_quiet() {
        let mut m = machine("1.0.0");
        let t = start_check(&mut m, Trigger::Manual);
        assert_eq!(
            finish(&mut m, t, None),
            vec![Effect::Notify(Notice::UpToDate)]
        );
        let t = start_check(&mut m, Trigger::Background);
        assert_eq!(finish(&mut m, t, None), vec![]);
        assert_eq!(m.phase, Phase::Checked(Outcome::UpToDate));
    }

    #[test]
    fn a_second_request_while_checking_joins_the_running_check() {
        let mut m = machine("1.0.0");
        let t = start_check(&mut m, Trigger::Background);
        assert_eq!(m.handle(Event::CheckRequested(Trigger::Manual)), vec![]);
        assert_eq!(m.handle(Event::CheckRequested(Trigger::Manual)), vec![]);
        // The user asked meanwhile, so the answer is shown to them.
        assert_eq!(
            finish(&mut m, t, None),
            vec![Effect::Notify(Notice::UpToDate)]
        );
    }

    #[test]
    fn newer_in_app_candidates_can_be_downloaded_verified_and_installed_in_order() {
        let mut m = machine("1.0.0");
        let t = start_check(&mut m, Trigger::Manual);
        assert_eq!(
            finish(&mut m, t, Some(candidate("1.1.0"))),
            vec![Effect::Notify(Notice::UpdateAvailable(v("1.1.0")))]
        );
        let effects = m.handle(Event::DownloadRequested);
        let [Effect::StartDownload { ticket, .. }] = effects.as_slice() else {
            panic!("{effects:?}")
        };
        let ticket = *ticket;
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

        assert!(matches!(
            m.handle(Event::DownloadFinished { ticket }).as_slice(),
            [Effect::StartVerify { .. }]
        ));
        assert!(matches!(m.phase, Phase::Verifying(_)));
        m.handle(Event::VerifyFinished { ticket, ok: true });
        assert!(matches!(m.phase, Phase::ReadyToInstall(_)));
        assert!(matches!(
            m.handle(Event::InstallRequested).as_slice(),
            [Effect::StartInstall { .. }]
        ));
        m.handle(Event::InstallHandedOff { ticket });
        assert!(matches!(m.phase, Phase::AwaitingFirstLaunch(_)));
        m.handle(Event::FirstLaunchConfirmed);
        assert_eq!(m.phase, Phase::Idle);
    }

    #[test]
    fn steps_cannot_run_out_of_order() {
        let mut m = machine("1.0.0");
        assert_eq!(
            m.handle(Event::DownloadRequested),
            vec![],
            "no candidate yet"
        );
        assert_eq!(
            m.handle(Event::InstallRequested),
            vec![],
            "nothing is ready"
        );
        let t = start_check(&mut m, Trigger::Manual);
        finish(&mut m, t, Some(candidate("1.1.0")));
        assert_eq!(
            m.handle(Event::InstallRequested),
            vec![],
            "not downloaded yet"
        );
        m.handle(Event::DownloadRequested);
        assert_eq!(
            m.handle(Event::InstallRequested),
            vec![],
            "not verified yet"
        );
    }

    #[test]
    fn a_failed_verification_blocks_installation() {
        let mut m = machine("1.0.0");
        let t = start_check(&mut m, Trigger::Manual);
        finish(&mut m, t, Some(candidate("1.1.0")));
        m.handle(Event::DownloadRequested);
        let ticket = m.ticket();
        m.handle(Event::DownloadFinished { ticket });
        m.handle(Event::VerifyFinished { ticket, ok: false });
        assert!(matches!(
            m.phase,
            Phase::InstallFailed {
                failure: Failure::Signature,
                ..
            }
        ));
        assert_eq!(m.handle(Event::InstallRequested), vec![]);
    }

    #[test]
    fn the_stable_channel_never_offers_a_prerelease_and_never_a_downgrade() {
        let mut m = machine("1.0.0");
        for offered in ["1.1.0-beta.1", "0.9.0", "1.0.0"] {
            let t = start_check(&mut m, Trigger::Manual);
            finish(&mut m, t, Some(candidate(offered)));
            assert_eq!(m.phase, Phase::Checked(Outcome::UpToDate), "{offered}");
        }
        let mut preview = Machine::new(v("1.0.0"), Channel::Preview, Capability::InApp);
        let t = start_check(&mut preview, Trigger::Manual);
        let beta = Candidate {
            channel: Channel::Preview,
            ..candidate("1.1.0-beta.1")
        };
        finish(&mut preview, t, Some(beta));
        assert!(matches!(
            preview.phase,
            Phase::Checked(Outcome::InAppUpdate(_))
        ));
    }

    #[test]
    fn a_release_for_another_channel_is_not_offered() {
        let mut m = machine("1.0.0");
        let t = start_check(&mut m, Trigger::Manual);
        let wrong = Candidate {
            channel: Channel::Preview,
            ..candidate("1.1.0")
        };
        finish(&mut m, t, Some(wrong));
        assert_eq!(m.phase, Phase::Checked(Outcome::UpToDate));
    }

    #[test]
    fn switching_channel_drops_a_late_answer_for_the_old_one() {
        let mut m = machine("1.0.0");
        let old = start_check(&mut m, Trigger::Manual);
        m.handle(Event::ChannelChanged(Channel::Preview));
        assert_eq!(m.phase, Phase::Idle);
        // The old check answers late with a stable release: it must be ignored.
        assert_eq!(finish(&mut m, old, Some(candidate("1.1.0"))), vec![]);
        assert_eq!(m.phase, Phase::Idle);
        // The next check asks for the new channel and is accepted.
        let effects = m.handle(Event::CheckRequested(Trigger::Manual));
        assert!(matches!(
            effects.as_slice(),
            [Effect::StartCheck {
                channel: Channel::Preview,
                ..
            }]
        ));
    }

    #[test]
    fn cancelling_a_check_discards_its_late_answer() {
        let mut m = machine("1.0.0");
        let t = start_check(&mut m, Trigger::Manual);
        assert_eq!(m.handle(Event::Cancel), vec![Effect::CancelRunning]);
        assert_eq!(finish(&mut m, t, Some(candidate("1.1.0"))), vec![]);
        assert_eq!(m.phase, Phase::Idle);
    }

    #[test]
    fn cancelling_a_download_keeps_the_candidate_so_it_can_be_retried() {
        let mut m = machine("1.0.0");
        let t = start_check(&mut m, Trigger::Manual);
        finish(&mut m, t, Some(candidate("1.1.0")));
        m.handle(Event::DownloadRequested);
        let ticket = m.ticket();
        assert_eq!(m.handle(Event::Cancel), vec![Effect::CancelRunning]);
        // Late progress and completion from the cancelled download are ignored.
        m.handle(Event::DownloadProgress {
            ticket,
            downloaded: 50,
            total: None,
        });
        assert_eq!(m.handle(Event::DownloadFinished { ticket }), vec![]);
        assert!(matches!(m.phase, Phase::Checked(Outcome::InAppUpdate(_))));
        assert!(matches!(
            m.handle(Event::DownloadRequested).as_slice(),
            [Effect::StartDownload { .. }]
        ));
    }

    #[test]
    fn a_locked_candidate_is_not_replaced_by_a_new_check_or_channel_change() {
        let mut m = machine("1.0.0");
        let t = start_check(&mut m, Trigger::Manual);
        finish(&mut m, t, Some(candidate("1.1.0")));
        m.handle(Event::DownloadRequested);
        assert_eq!(m.handle(Event::CheckRequested(Trigger::Manual)), vec![]);
        m.handle(Event::ChannelChanged(Channel::Preview));
        assert!(
            matches!(m.phase, Phase::Downloading { .. }),
            "the download goes on"
        );
        assert_eq!(
            m.channel,
            Channel::Preview,
            "the new channel applies to the next check"
        );
    }

    #[test]
    fn a_release_that_needs_a_newer_base_asks_for_a_manual_upgrade() {
        let mut m = machine("1.0.0");
        let t = start_check(&mut m, Trigger::Manual);
        let gated = Candidate {
            min_version: Some(v("1.5.0")),
            ..candidate("2.0.0")
        };
        let effects = finish(&mut m, t, Some(gated));
        assert_eq!(
            effects,
            vec![Effect::Notify(Notice::ManualUpgradeNeeded(v("2.0.0")))]
        );
        assert!(matches!(
            m.phase,
            Phase::Checked(Outcome::ManualUpgrade {
                reason: ManualReason::ReleasePolicy,
                ..
            })
        ));
        assert_eq!(
            m.handle(Event::DownloadRequested),
            vec![],
            "no in-app install for it"
        );
        // A build at or above the minimum updates in place.
        let mut recent = machine("1.5.0");
        let t = start_check(&mut recent, Trigger::Manual);
        finish(
            &mut recent,
            t,
            Some(Candidate {
                min_version: Some(v("1.5.0")),
                ..candidate("2.0.0")
            }),
        );
        assert!(matches!(
            recent.phase,
            Phase::Checked(Outcome::InAppUpdate(_))
        ));
    }

    #[test]
    fn an_unverified_release_keeps_its_version_but_cannot_be_installed_in_place() {
        let mut m = machine("1.0.0");
        let t = start_check(&mut m, Trigger::Manual);
        finish(
            &mut m,
            t,
            Some(Candidate {
                verified: false,
                ..candidate("1.1.0")
            }),
        );
        assert!(matches!(
            m.phase,
            Phase::Checked(Outcome::ManualUpgrade {
                reason: ManualReason::Unverified,
                ..
            })
        ));
        assert_eq!(m.handle(Event::DownloadRequested), vec![]);
    }

    #[test]
    fn capability_decides_between_in_app_manual_and_system_managed() {
        let mut manual = Machine::new(v("1.0.0"), Channel::Stable, Capability::ManualOnly);
        let t = start_check(&mut manual, Trigger::Manual);
        finish(&mut manual, t, Some(candidate("1.1.0")));
        assert!(matches!(
            manual.phase,
            Phase::Checked(Outcome::ManualUpgrade {
                reason: ManualReason::Environment,
                ..
            })
        ));
        for (capability, outcome) in [
            (Capability::SystemManaged, Outcome::SystemManaged),
            (Capability::Unsupported, Outcome::NotApplicable),
        ] {
            let mut m = Machine::new(v("1.0.0"), Channel::Stable, capability);
            // No request goes out: the platform owns updates or has none.
            assert_eq!(m.handle(Event::CheckRequested(Trigger::Manual)), vec![]);
            assert_eq!(m.phase, Phase::Checked(outcome));
        }
    }

    #[test]
    fn failures_are_reported_to_a_manual_check_and_swallowed_by_a_background_one() {
        let mut m = machine("1.0.0");
        let t = start_check(&mut m, Trigger::Manual);
        let effects = m.handle(Event::CheckFinished {
            ticket: t,
            result: CheckResult::Failed(Failure::Network),
        });
        assert_eq!(
            effects,
            vec![Effect::Notify(Notice::CheckFailed(Failure::Network))]
        );
        let t = start_check(&mut m, Trigger::Background);
        let effects = m.handle(Event::CheckFinished {
            ticket: t,
            result: CheckResult::Failed(Failure::RateLimited),
        });
        assert_eq!(effects, vec![]);
        assert_eq!(
            m.phase,
            Phase::Checked(Outcome::Failed(Failure::RateLimited))
        );
        // Retrying after a failure works.
        assert!(matches!(
            m.handle(Event::CheckRequested(Trigger::Manual)).as_slice(),
            [Effect::StartCheck { .. }]
        ));
    }

    #[test]
    fn an_install_failure_is_recorded_and_stale_failures_are_ignored() {
        let mut m = machine("1.0.0");
        let t = start_check(&mut m, Trigger::Manual);
        finish(&mut m, t, Some(candidate("1.1.0")));
        m.handle(Event::DownloadRequested);
        let ticket = m.ticket();
        m.handle(Event::InstallFailed {
            ticket: ticket - 1,
            failure: Failure::Disk,
        });
        assert!(
            matches!(m.phase, Phase::Downloading { .. }),
            "a stale failure changes nothing"
        );
        m.handle(Event::InstallFailed {
            ticket,
            failure: Failure::Network,
        });
        assert!(matches!(
            m.phase,
            Phase::InstallFailed {
                failure: Failure::Network,
                ..
            }
        ));
    }
}
