//! Owns effect claims and typed completion validation for the one designated executor.
//!
//! There is no handover protocol here: NSH cannot cancel an update operation, so "revoke the old
//! executor and confirm it stopped" cannot be honoured (docs/update.md, 8.2, option B).
use super::{Error, Model, UpdateService};
use usher_state::StoreId;
use usher_update::state::{CheckResult, Effect, Event, Failure};

/// Opaque controller identity, bound to one store lifetime; not a timeout or a window ID.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct Lease {
    scope: StoreId,
    generation: u64,
}
/// The only executable delivery. Cloning a claim does not permit a second completion.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Claim {
    lease: Lease,
    id: u64,
    effect: Effect,
}
impl Lease {
    pub(crate) fn token(self) -> String {
        format!("{}:{}", self.scope.value(), self.generation)
    }
}
impl Claim {
    pub(crate) fn token(&self) -> String {
        format!("{}:{}", self.lease.token(), self.id)
    }

    pub fn effect(&self) -> &Effect {
        &self.effect
    }
    pub fn id(&self) -> u64 {
        self.id
    }
}
/// Results are matched against the claimed effect. The service supplies the original ticket.
#[derive(Clone, Debug)]
pub enum Completion {
    Checked(CheckResult),
    CheckedWithUrl(CheckResult, Option<String>),
    /// The download finished and NSH accepted it (NSH verifies inside the download).
    Downloaded,
    Installed,
    Failed(Failure),
    Notified,
}
impl Model {
    fn validate(&self, claim: &Claim) -> Result<(), Error> {
        if self.lease != Some(claim.lease) || self.active.as_ref() != Some(claim) {
            Err(Error::Stale)
        } else {
            Ok(())
        }
    }
}
impl UpdateService {
    /// Resolves only a live service-issued identity; callers cannot submit raw Rust fields.
    pub(crate) fn lease_token(&self, token: &str) -> Result<Lease, Error> {
        let guard = self.transaction.lock().map_err(|_| Error::Unavailable)?;
        if guard.closed {
            return Err(Error::Closed);
        }
        if guard.model.domain.is_none() {
            return Err(Error::NotConfigured);
        }
        guard
            .model
            .lease
            .filter(|lease| lease.token() == token)
            .ok_or(Error::Stale)
    }
    pub(crate) fn claim_token(&self, token: &str) -> Result<Claim, Error> {
        let guard = self.transaction.lock().map_err(|_| Error::Unavailable)?;
        if guard.closed {
            return Err(Error::Closed);
        }
        if guard.model.domain.is_none() {
            return Err(Error::NotConfigured);
        }
        guard
            .model
            .active
            .as_ref()
            .filter(|claim| claim.token() == token)
            .cloned()
            .ok_or(Error::Stale)
    }
    /// Elects a single designated controller; ordinary consumers should never acquire a lease.
    pub fn acquire(&self) -> Result<Lease, Error> {
        self.change(|model| {
            model.machine()?;
            if model.lease.is_some() {
                return Err(Error::Busy);
            }
            let lease = Lease {
                scope: self.id,
                generation: model.identity()?,
            };
            model.lease = Some(lease);
            Ok(lease)
        })
    }
    /// Atomically delivers one queued effect. None means no work or an existing in-flight claim.
    pub fn claim(&self, lease: Lease) -> Result<Option<Claim>, Error> {
        self.change(|model| {
            model.machine()?;
            if model.lease != Some(lease) {
                return Err(Error::Stale);
            }
            if model.active.is_some() {
                return Ok(None);
            }
            let Some(job) = model.queue.pop_front() else {
                return Ok(None);
            };
            let claim = Claim {
                lease,
                id: job.id,
                effect: job.effect,
            };
            model.active = Some(claim.clone());
            Ok(Some(claim))
        })
    }
    /// Publishes monotonic progress only for the currently claimed download.
    pub fn progress(
        &self,
        claim: &Claim,
        downloaded: u64,
        total: Option<u64>,
    ) -> Result<(), Error> {
        self.change(|model| {
            model.validate(claim)?;
            let Effect::StartDownload { ticket, .. } = claim.effect else {
                return Err(Error::Invalid);
            };
            let usher_update::state::Phase::Downloading {
                downloaded: previous,
                total: known,
                ..
            } = model.machine()?.phase
            else {
                return Err(Error::Invalid);
            };
            if downloaded < previous
                || total.is_some_and(|value| downloaded > value)
                || known.is_some_and(|value| total != Some(value))
            {
                return Err(Error::Invalid);
            }
            model.event(Event::DownloadProgress {
                ticket,
                downloaded,
                total,
            })
        })
    }
    /// Completes exactly the current claim; duplicate, wrong-kind and old-owner replies are rejected.
    pub fn complete(&self, claim: &Claim, completion: Completion) -> Result<(), Error> {
        self.change(|model| {
            model.validate(claim)?;
            let completion = match completion {
                Completion::CheckedWithUrl(result, url)
                    if matches!(claim.effect, Effect::StartCheck { .. }) =>
                {
                    model.info.manual_url = url;
                    Completion::Checked(result)
                }
                other => other,
            };
            let checking = matches!(claim.effect, Effect::StartCheck { .. });
            let event = match (&claim.effect, completion) {
                (Effect::StartCheck { ticket, .. }, Completion::Checked(result)) => {
                    Some(Event::CheckFinished {
                        ticket: *ticket,
                        result,
                    })
                }
                (Effect::StartCheck { ticket, .. }, Completion::Failed(failure)) => {
                    Some(Event::CheckFinished {
                        ticket: *ticket,
                        result: CheckResult::Failed(failure),
                    })
                }
                (Effect::StartDownload { ticket, .. }, Completion::Downloaded) => {
                    Some(Event::DownloadFinished { ticket: *ticket })
                }
                (Effect::StartInstall { ticket, .. }, Completion::Installed) => {
                    Some(Event::InstallHandedOff { ticket: *ticket })
                }
                (
                    Effect::StartDownload { ticket, .. } | Effect::StartInstall { ticket, .. },
                    Completion::Failed(failure),
                ) => Some(Event::StepFailed {
                    ticket: *ticket,
                    failure,
                }),
                (Effect::Notify(_), Completion::Notified) => None,
                _ => return Err(Error::Invalid),
            };
            model.active = None;
            if let Some(event) = event {
                model.event(event)?;
            }
            if checking {
                use usher_update::state::{Outcome, Phase};
                model.info.last_checked = Some(chrono::Utc::now().to_rfc3339());
                model.info.last_result = Some(
                    match &model.machine()?.phase {
                        Phase::Downloading { .. } | Phase::Checked(Outcome::PackageUpdate(_)) => {
                            "package_update"
                        }
                        Phase::Checked(Outcome::ShellUpdate { .. }) => "shell_update",
                        Phase::Checked(Outcome::ManualUpgrade { .. }) => "manual_upgrade",
                        Phase::Checked(Outcome::Failed(_)) => "failed",
                        _ => "up_to_date",
                    }
                    .into(),
                );
            }
            Ok(())
        })
    }
}
