//! The update state machine. A reducer: `(state, event) -> (state, effects)`, no I/O.
//!
//! The caller performs the effects (network, disk, restart) and feeds results back as
//! events. That keeps every rule here testable without a network or a window. The design is
//! in docs/update.md, section 8.2; read that first. In short:
//!
//! - there are two kinds of update, a code package and a whole-app update, and one machine
//!   for both: the phases are shared, the kind is on the candidate;
//! - one check at a time: a second request while one runs only returns the running one;
//! - each effect carries a `ticket`; a result with an old ticket is dropped, so a late answer
//!   cannot overwrite a newer state;
//! - a code package is used only when its major.minor equals the installed shell's, because a
//!   package carries TSX only and the shell holds the Rust it calls (docs/update.md, D9);
//! - a version that keeps failing stops being offered by background checks;
//! - there is no cancel: NSH cannot cancel an update operation, so the machine does not
//!   pretend to (docs/update.md, V12);
//! - the install steps only move forward: Downloading → ReadyToInstall → Installing →
//!   AwaitingFirstLaunch. NSH verifies a download inside the download, so there is no separate
//!   verifying phase.
//!
//! Every state change and failure also yields an `Effect::Log` with typed fields and no free
//! text. The caller writes it; this crate does no I/O and does not know a logger exists.

use semver::Version;

/// How many versions' failure counts are remembered. The machine is cloned into every
/// published snapshot, so the memory is kept small; the oldest entry is dropped first.
pub const FAILURE_MEMORY: usize = 8;

/// How this installation can be updated, which is separate from what a release asks for.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum Capability {
    /// The app may download and install an update itself.
    InApp,
    /// The environment can only point the user at a download page (`canInstall` is false).
    ManualOnly,
    /// A store or package manager owns updates.
    SystemManaged,
    /// This platform or install form has no update path.
    Unsupported,
}

/// What an update replaces.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum Kind {
    /// The app's TSX, as a signed package: runs from the next start, no replacement.
    Package,
    /// The whole installed app: replaced and restarted.
    Shell,
}

/// What a check found: the newest release the source offers.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Offer {
    pub version: Version,
    /// Whether the release carries a code package this runtime can run.
    pub has_package: bool,
    /// Whole-app installation permission reported by this check, not startup configuration.
    pub can_install: bool,
}

/// A release that can be offered, and how it would be installed.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Candidate {
    pub version: Version,
    pub kind: Kind,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum Outcome {
    UpToDate,
    /// A newer code package this shell can run.
    PackageUpdate(Candidate),
    /// A newer whole-app update that can be installed from inside the app.
    ShellUpdate {
        candidate: Candidate,
        /// Why a code package was not used: the release has none, or its major.minor differs
        /// from the installed shell's ("the app needs updating").
        reason: ShellReason,
    },
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
pub enum ShellReason {
    /// The release has no code package.
    NoPackage,
    /// The release's code package belongs to another major.minor than the installed shell.
    ShellTooOld,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum ManualReason {
    /// This environment cannot install by itself.
    Environment,
}

/// Why an update step did not work, as the panel and the log tell it. See docs/update.md 8.2
/// for what each class means and how it is counted.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum Failure {
    /// Could not reach the source, timed out, or the connection broke. Retry helps.
    Network,
    /// Another update operation is still running (NSH runs one at a time). Retry helps, later.
    Busy,
    /// NSH refused the package or archive: signature, app id, build number, runtime, hash.
    /// Retrying will not help.
    Rejected,
    /// Writing a file or replacing the app failed.
    Disk,
    /// The user gave up on the app's side. Not an NSH cancel, and not a failure.
    Cancelled,
    /// None of the above.
    Unknown,
}

impl Failure {
    /// Whether this failure counts toward a version's failure budget. Transient and
    /// user-made outcomes do not: going offline twice must not hide a good update.
    pub fn counts(self) -> bool {
        matches!(self, Self::Rejected | Self::Disk | Self::Unknown)
    }

    pub fn as_str(self) -> &'static str {
        match self {
            Self::Network => "network",
            Self::Busy => "busy",
            Self::Rejected => "rejected",
            Self::Disk => "disk",
            Self::Cancelled => "cancelled",
            Self::Unknown => "unknown",
        }
    }

    fn level(self) -> Level {
        match self {
            Self::Network | Self::Busy => Level::Warn,
            Self::Cancelled => Level::Info,
            Self::Rejected | Self::Disk | Self::Unknown => Level::Error,
        }
    }
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
    /// Downloaded and accepted by NSH. A code package waits for a restart; a whole-app update
    /// waits for the user to confirm the replacement.
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
    DownloadRequested,
    DownloadProgress {
        ticket: u64,
        downloaded: u64,
        total: Option<u64>,
    },
    /// The download finished and NSH accepted it.
    DownloadFinished {
        ticket: u64,
    },
    InstallRequested,
    InstallHandedOff {
        ticket: u64,
    },
    /// A download or install step failed.
    StepFailed {
        ticket: u64,
        failure: Failure,
    },
    /// The new version started and confirmed itself.
    FirstLaunchConfirmed,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum CheckResult {
    /// The newest release the source offers, if any.
    Release(Option<Offer>),
    Failed(Failure),
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum Effect {
    /// Ask the source. The caller checks for a code package and for a whole-app update and
    /// reports the newest offer.
    StartCheck {
        ticket: u64,
    },
    StartDownload {
        ticket: u64,
        candidate: Candidate,
    },
    /// Save settings and work state, then restart into the package or replace the app.
    StartInstall {
        ticket: u64,
        candidate: Candidate,
    },
    /// Tell the user. Background checks stay quiet unless something needs them.
    Notify(Notice),
    /// A record for the log. The machine does no I/O; the caller writes it, or drops it when
    /// logging is off.
    Log(Record),
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum Notice {
    UpToDate,
    UpdateAvailable(Version),
    /// The app itself has to be updated: the release's code package does not fit this shell,
    /// or it has none.
    ShellUpdateNeeded(Version),
    ManualUpgradeNeeded(Version),
    CheckFailed(Failure),
}

#[derive(Clone, Copy, Debug, Eq, Ord, PartialEq, PartialOrd)]
pub enum Level {
    Info,
    Warn,
    Error,
}

/// What to log, without free text: every field is a fixed word, a number or a version.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Record {
    pub level: Level,
    /// A stable event name, snake_case.
    pub event: &'static str,
    pub ticket: Option<u64>,
    pub version: Option<Version>,
    pub kind: Option<Kind>,
    pub failure: Option<Failure>,
    /// How many times this version has failed, when the record is about a counted failure.
    pub failures: Option<u32>,
}

impl Record {
    fn new(level: Level, event: &'static str) -> Self {
        Self {
            level,
            event,
            ticket: None,
            version: None,
            kind: None,
            failure: None,
            failures: None,
        }
    }
}

/// How a machine is set up. All of it comes from configuration or from the installed app; none
/// of it is written in the rules.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Settings {
    /// The installed app's version (`shellInfo().version`): what a code package must match.
    pub shell: Version,
    /// The version running now: the shell's, or a code package's when one runs.
    pub running: Version,
    pub capability: Capability,
    /// Whether background checks run and a code package downloads without asking.
    pub auto_check: bool,
    /// After this many counted failures a version is not offered to background checks.
    pub failure_limit: u32,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Machine {
    pub phase: Phase,
    shell: Version,
    running: Version,
    pub capability: Capability,
    auto_check: bool,
    failure_limit: u32,
    /// Counted failures per version, oldest first, at most `FAILURE_MEMORY` entries.
    failures: Vec<(Version, u32)>,
    ticket: u64,
    /// The check being run: its ticket and who asked.
    running_check: Option<(u64, Trigger)>,
}

impl Machine {
    pub fn new(settings: Settings) -> Self {
        Self {
            phase: Phase::Idle,
            shell: settings.shell,
            running: settings.running,
            capability: settings.capability,
            auto_check: settings.auto_check,
            failure_limit: settings.failure_limit,
            failures: Vec::new(),
            ticket: 0,
            running_check: None,
        }
    }

    pub fn shell(&self) -> &Version {
        &self.shell
    }

    pub fn running(&self) -> &Version {
        &self.running
    }

    pub fn auto_check(&self) -> bool {
        self.auto_check
    }

    /// Turns background checks and automatic package downloads on or off. A check or download
    /// already in progress is not affected.
    pub fn set_auto_check(&mut self, on: bool) {
        self.auto_check = on;
    }

    /// The ticket a result must carry to be accepted now.
    pub fn ticket(&self) -> u64 {
        self.ticket
    }

    /// How many times `version` has failed in a way that counts.
    pub fn failures_of(&self, version: &Version) -> u32 {
        self.failures
            .iter()
            .find(|(seen, _)| seen == version)
            .map_or(0, |(_, count)| *count)
    }

    /// Whether `version` has used up its failure budget and background checks should not
    /// offer it again.
    pub fn exhausted(&self, version: &Version) -> bool {
        self.failures_of(version) >= self.failure_limit
    }

    /// Whether a candidate is locked: from the start of its download until it is installed.
    pub fn candidate_locked(&self) -> bool {
        matches!(
            self.phase,
            Phase::Downloading { .. }
                | Phase::ReadyToInstall(_)
                | Phase::Installing(_)
                | Phase::AwaitingFirstLaunch(_)
        )
    }

    fn next_ticket(&mut self) -> u64 {
        self.ticket += 1;
        self.ticket
    }

    pub fn handle(&mut self, event: Event) -> Vec<Effect> {
        match event {
            Event::CheckRequested(trigger) => self.check_requested(trigger),
            Event::CheckFinished { ticket, result } => self.check_finished(ticket, result),
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
            Event::DownloadFinished { ticket } => self.download_finished(ticket),
            Event::InstallRequested => self.install_requested(),
            Event::InstallHandedOff { ticket } => {
                if ticket == self.ticket
                    && let Phase::Installing(candidate) = &self.phase
                {
                    let candidate = candidate.clone();
                    let log = log_step("install_handed_off", ticket, &candidate);
                    self.phase = Phase::AwaitingFirstLaunch(candidate);
                    return vec![Effect::Log(log)];
                }
                vec![]
            }
            Event::StepFailed { ticket, failure } => self.step_failed(ticket, failure),
            Event::FirstLaunchConfirmed => {
                if let Phase::AwaitingFirstLaunch(candidate) = &self.phase {
                    let mut record = Record::new(Level::Info, "first_launch_confirmed");
                    record.version = Some(candidate.version.clone());
                    record.kind = Some(candidate.kind);
                    self.phase = Phase::Idle;
                    return vec![Effect::Log(record)];
                }
                vec![]
            }
        }
    }

    fn check_requested(&mut self, trigger: Trigger) -> Vec<Effect> {
        if self.candidate_locked() {
            // An update is already in flight; a check would only confuse it.
            return vec![];
        }
        if trigger == Trigger::Background && !self.auto_check {
            // Background checks are off. A manual check always goes through.
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
        let mut record = Record::new(Level::Info, "check_started");
        record.ticket = Some(ticket);
        vec![Effect::StartCheck { ticket }, Effect::Log(record)]
    }

    fn check_finished(&mut self, ticket: u64, result: CheckResult) -> Vec<Effect> {
        let Some((running, trigger)) = self.running_check else {
            return vec![];
        };
        if ticket != running || ticket != self.ticket {
            // A superseded check: its answer is for a question nobody is asking any more.
            return vec![];
        }
        self.running_check = None;
        let outcome = match result {
            CheckResult::Failed(failure) => Outcome::Failed(failure),
            CheckResult::Release(None) => Outcome::UpToDate,
            CheckResult::Release(Some(offer)) => self.classify(offer, trigger),
        };
        let mut effects = Vec::new();
        let mut record = Record::new(Level::Info, "check_finished");
        record.ticket = Some(ticket);
        match &outcome {
            Outcome::Failed(failure) => {
                record.level = failure.level();
                record.event = "check_failed";
                record.failure = Some(*failure);
            }
            Outcome::PackageUpdate(candidate)
            | Outcome::ShellUpdate { candidate, .. }
            | Outcome::ManualUpgrade { candidate, .. } => {
                record.event = "update_found";
                record.version = Some(candidate.version.clone());
                record.kind = Some(candidate.kind);
            }
            _ => {}
        }
        effects.push(Effect::Log(record));
        let notice = notice_for(&outcome, trigger);
        let download = self.auto_download(&outcome);
        self.phase = Phase::Checked(outcome);
        if download.is_none() {
            effects.extend(notice.map(Effect::Notify));
        }
        if let Some(candidate) = download {
            effects.extend(self.start_download(candidate));
        }
        effects
    }

    /// Decides what to do with the newest release the source offered (docs/update.md, 8.2).
    fn classify(&self, offer: Offer, trigger: Trigger) -> Outcome {
        // The floor is the newer of what runs and the installed shell. Normally they agree or the
        // running package is ahead, but this must not rely on that: offering something older
        // than the installed shell would send the user to a downgrade.
        let floor = if self.running >= self.shell {
            &self.running
        } else {
            &self.shell
        };
        if offer.version <= *floor || !offer.version.pre.is_empty() {
            // Never a downgrade, and never a pre-release: there is one channel, the formal
            // releases (docs/update.md, D7). The source should not offer one; this is the
            // second line.
            return Outcome::UpToDate;
        }
        if trigger == Trigger::Background && self.exhausted(&offer.version) {
            // It kept failing. A background check does not bring it back; a manual one does.
            return Outcome::UpToDate;
        }
        let same_line =
            offer.version.major == self.shell.major && offer.version.minor == self.shell.minor;
        let (kind, shell_reason) = if offer.has_package && same_line {
            (Kind::Package, None)
        } else if offer.has_package {
            (Kind::Shell, Some(ShellReason::ShellTooOld))
        } else {
            (Kind::Shell, Some(ShellReason::NoPackage))
        };
        let candidate = Candidate {
            version: offer.version,
            kind,
        };
        let capability = if self.capability == Capability::InApp && !offer.can_install {
            Capability::ManualOnly
        } else {
            self.capability
        };
        match capability {
            Capability::InApp => match shell_reason {
                None => Outcome::PackageUpdate(candidate),
                Some(reason) => Outcome::ShellUpdate { candidate, reason },
            },
            Capability::ManualOnly => {
                if kind == Kind::Package {
                    // A package replaces nothing and needs no install permission, so a read-only
                    // install location does not stop it.
                    Outcome::PackageUpdate(candidate)
                } else {
                    Outcome::ManualUpgrade {
                        candidate,
                        reason: ManualReason::Environment,
                    }
                }
            }
            Capability::SystemManaged => Outcome::SystemManaged,
            Capability::Unsupported => Outcome::NotApplicable,
        }
    }

    /// The candidate to download without asking, if this outcome calls for it: a code package
    /// while automatic updates are on. A whole-app update is never started without the user.
    fn auto_download(&self, outcome: &Outcome) -> Option<Candidate> {
        match outcome {
            Outcome::PackageUpdate(candidate) if self.auto_check => Some(candidate.clone()),
            _ => None,
        }
    }

    fn download_requested(&mut self) -> Vec<Effect> {
        let candidate = match &self.phase {
            Phase::Checked(
                Outcome::PackageUpdate(candidate) | Outcome::ShellUpdate { candidate, .. },
            ) => candidate.clone(),
            // A download that failed can be asked for again.
            Phase::InstallFailed { candidate, failure }
                if !matches!(failure, Failure::Rejected) =>
            {
                candidate.clone()
            }
            _ => return vec![],
        };
        self.start_download(candidate)
    }

    fn start_download(&mut self, candidate: Candidate) -> Vec<Effect> {
        let ticket = self.next_ticket();
        let record = log_step("download_started", ticket, &candidate);
        self.phase = Phase::Downloading {
            candidate: candidate.clone(),
            downloaded: 0,
            total: None,
        };
        vec![
            Effect::StartDownload { ticket, candidate },
            Effect::Log(record),
        ]
    }

    fn download_finished(&mut self, ticket: u64) -> Vec<Effect> {
        if ticket != self.ticket {
            return vec![];
        }
        let Phase::Downloading { candidate, .. } = &self.phase else {
            return vec![];
        };
        let candidate = candidate.clone();
        let record = log_step("download_finished", ticket, &candidate);
        self.phase = Phase::ReadyToInstall(candidate);
        vec![Effect::Log(record)]
    }

    fn install_requested(&mut self) -> Vec<Effect> {
        let Phase::ReadyToInstall(candidate) = &self.phase else {
            return vec![];
        };
        let candidate = candidate.clone();
        let ticket = self.ticket;
        let record = log_step("install_started", ticket, &candidate);
        self.phase = Phase::Installing(candidate.clone());
        vec![
            Effect::StartInstall { ticket, candidate },
            Effect::Log(record),
        ]
    }

    fn step_failed(&mut self, ticket: u64, failure: Failure) -> Vec<Effect> {
        if ticket != self.ticket {
            return vec![];
        }
        let candidate = match &self.phase {
            Phase::Downloading { candidate, .. } | Phase::Installing(candidate) => {
                candidate.clone()
            }
            _ => return vec![],
        };
        let mut record = log_step("step_failed", ticket, &candidate);
        record.level = failure.level();
        record.failure = Some(failure);
        if failure.counts() {
            record.failures = Some(self.count_failure(&candidate.version));
        }
        self.phase = Phase::InstallFailed { candidate, failure };
        vec![Effect::Log(record)]
    }

    /// Adds one counted failure to `version` and returns its new total. The memory is bounded:
    /// a new version past `FAILURE_MEMORY` entries pushes the oldest out.
    fn count_failure(&mut self, version: &Version) -> u32 {
        if let Some(entry) = self.failures.iter_mut().find(|(seen, _)| seen == version) {
            entry.1 = entry.1.saturating_add(1);
            return entry.1;
        }
        if self.failures.len() == FAILURE_MEMORY {
            self.failures.remove(0);
        }
        self.failures.push((version.clone(), 1));
        1
    }
}

fn log_step(event: &'static str, ticket: u64, candidate: &Candidate) -> Record {
    let mut record = Record::new(Level::Info, event);
    record.ticket = Some(ticket);
    record.version = Some(candidate.version.clone());
    record.kind = Some(candidate.kind);
    record
}

fn notice_for(outcome: &Outcome, trigger: Trigger) -> Option<Notice> {
    match (outcome, trigger) {
        (Outcome::UpToDate, Trigger::Manual) => Some(Notice::UpToDate),
        (Outcome::UpToDate, Trigger::Background) => None,
        (Outcome::PackageUpdate(c), _) => Some(Notice::UpdateAvailable(c.version.clone())),
        (
            Outcome::ShellUpdate {
                candidate,
                reason: ShellReason::ShellTooOld,
            },
            _,
        ) => Some(Notice::ShellUpdateNeeded(candidate.version.clone())),
        (Outcome::ShellUpdate { candidate, .. }, _) => {
            Some(Notice::UpdateAvailable(candidate.version.clone()))
        }
        (Outcome::ManualUpgrade { candidate, .. }, _) => {
            Some(Notice::ManualUpgradeNeeded(candidate.version.clone()))
        }
        // A background check that merely failed (offline, busy) stays silent.
        (Outcome::Failed(failure), Trigger::Manual) => Some(Notice::CheckFailed(*failure)),
        (Outcome::Failed(_), Trigger::Background) => None,
        (Outcome::SystemManaged | Outcome::NotApplicable, _) => None,
    }
}
