//! Validates transport input and projects domain state without exposing mutable Rust objects.
use super::{Claim, Command, Completion, Error, Execution, State};
use serde::Deserialize;
use serde_json::{Value, json};
use usher_update::state::{
    self as domain, Candidate, Capability, CheckResult, Effect, Failure, Kind, Notice, Outcome,
    Phase, Trigger,
};

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct Configuration {
    pub shell: Option<String>,
    pub running: Option<String>,
    pub capability: String,
    pub failure_limit: String,
    pub shell_build: Option<String>,
    pub package_build: Option<String>,
}
impl Configuration {
    pub fn info(&self) -> Result<super::RuntimeInfo, Error> {
        for value in [&self.shell_build, &self.package_build]
            .into_iter()
            .flatten()
        {
            decimal(value)?;
        }
        Ok(super::RuntimeInfo {
            shell_build: self.shell_build.clone(),
            package_build: self.package_build.clone(),
            ..Default::default()
        })
    }
    pub fn settings(&self, auto_check: bool) -> Result<domain::Settings, Error> {
        let capability = match self.capability.as_str() {
            "in_app" => Capability::InApp,
            "manual_only" => Capability::ManualOnly,
            "system_managed" => Capability::SystemManaged,
            "unsupported" => Capability::Unsupported,
            _ => return Err(Error::Invalid),
        };
        let shell: usher_update::Version = match self.shell.as_deref().filter(|s| !s.is_empty()) {
            Some(value) => value.parse().map_err(|_| Error::Invalid)?,
            None if capability == Capability::Unsupported => env!("CARGO_PKG_VERSION")
                .parse()
                .map_err(|_| Error::Invalid)?,
            None => return Err(Error::Invalid),
        };
        let running = self
            .running
            .as_deref()
            .filter(|s| !s.is_empty())
            .map(|value| value.parse().map_err(|_| Error::Invalid))
            .transpose()?
            .unwrap_or_else(|| shell.clone());
        let failure_limit = decimal(&self.failure_limit)?
            .try_into()
            .map_err(|_| Error::Invalid)?;
        if failure_limit == 0 {
            return Err(Error::Invalid);
        }
        Ok(domain::Settings {
            shell,
            running,
            capability,
            auto_check,
            failure_limit,
        })
    }
}

#[derive(Deserialize)]
#[serde(tag = "type", rename_all = "snake_case", deny_unknown_fields)]
pub(crate) enum Request {
    Check { background: bool },
    Download,
    Install,
    ConfirmFirstLaunch,
    AutoCheck { enabled: bool },
}
impl Request {
    pub fn command(self) -> Result<Command, Error> {
        Ok(match self {
            Self::Check { background } => Command::Check(if background {
                Trigger::Background
            } else {
                Trigger::Manual
            }),
            Self::Download => Command::Download,
            Self::Install => Command::Install,
            Self::ConfirmFirstLaunch => Command::ConfirmFirstLaunch,
            Self::AutoCheck { .. } => return Err(Error::Invalid),
        })
    }
}
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct Offer {
    version: String,
    has_package: bool,
    #[serde(default = "installable")]
    can_install: bool,
}
fn installable() -> bool {
    true
}
#[derive(Deserialize)]
#[serde(tag = "type", rename_all = "snake_case", deny_unknown_fields)]
pub(crate) enum Reply {
    Checked {
        offer: Option<Offer>,
        manual_url: Option<String>,
    },
    Downloaded,
    Installed,
    Failed {
        failure: String,
        detail: Option<String>,
    },
    Notified,
}
impl Reply {
    pub fn completion(self) -> Result<(Completion, Option<String>), Error> {
        let mut detail = None;
        let completion = match self {
            Self::Checked { offer, manual_url } => Completion::CheckedWithUrl(
                CheckResult::Release(
                    offer
                        .map(|offer| {
                            Ok(domain::Offer {
                                version: offer.version.parse().map_err(|_| Error::Invalid)?,
                                has_package: offer.has_package,
                                can_install: offer.can_install,
                            })
                        })
                        .transpose()?,
                ),
                manual_url.filter(|url| url.starts_with("https://")),
            ),
            Self::Downloaded => Completion::Downloaded,
            Self::Installed => Completion::Installed,
            Self::Notified => Completion::Notified,
            Self::Failed {
                failure,
                detail: message,
            } => {
                let failure = match failure.as_str() {
                    "network" => Failure::Network,
                    "busy" => Failure::Busy,
                    "rejected" => Failure::Rejected,
                    "disk" => Failure::Disk,
                    "cancelled" => Failure::Cancelled,
                    "unknown" => Failure::Unknown,
                    _ => return Err(Error::Invalid),
                };
                detail = message;
                Completion::Failed(failure)
            }
        };
        Ok((completion, detail))
    }
}
pub(crate) fn parse<T: serde::de::DeserializeOwned>(value: &str) -> Result<T, Error> {
    serde_json::from_str(value).map_err(|_| Error::Invalid)
}
pub(crate) fn decimal(value: &str) -> Result<u64, Error> {
    if value.is_empty() || !value.bytes().all(|byte| byte.is_ascii_digit()) {
        return Err(Error::Invalid);
    }
    value.parse().map_err(|_| Error::Invalid)
}
pub(crate) fn kind(kind: Kind) -> &'static str {
    match kind {
        Kind::Package => "package",
        Kind::Shell => "shell",
    }
}
fn candidate(value: &Candidate) -> Value {
    json!({"version": value.version.to_string(), "kind": kind(value.kind)})
}
fn outcome(value: &Outcome) -> Value {
    match value {
        Outcome::UpToDate => json!({"type": "up_to_date"}),
        Outcome::PackageUpdate(c) => json!({"type": "package_update", "candidate": candidate(c)}),
        Outcome::ShellUpdate {
            candidate: c,
            reason,
        } => {
            json!({"type": "shell_update", "candidate": candidate(c), "reason": match reason { domain::ShellReason::NoPackage => "no_package", domain::ShellReason::ShellTooOld => "shell_too_old" }})
        }
        Outcome::ManualUpgrade { candidate: c, .. } => {
            json!({"type": "manual_upgrade", "candidate": candidate(c), "reason": "environment"})
        }
        Outcome::SystemManaged => json!({"type": "system_managed"}),
        Outcome::NotApplicable => json!({"type": "not_applicable"}),
        Outcome::Failed(failure) => json!({"type": "failed", "failure": failure.as_str()}),
    }
}
fn phase(value: &Phase) -> Value {
    match value {
        Phase::Idle => json!({"type": "idle"}),
        Phase::Checking => json!({"type": "checking"}),
        Phase::Checked(result) => json!({"type": "checked", "outcome": outcome(result)}),
        Phase::Downloading {
            candidate: c,
            downloaded,
            total,
        } => {
            json!({"type": "downloading", "candidate": candidate(c), "downloaded": downloaded.to_string(), "total": total.map(|v| v.to_string())})
        }
        Phase::ReadyToInstall(c) => json!({"type": "ready_to_install", "candidate": candidate(c)}),
        Phase::Installing(c) => json!({"type": "installing", "candidate": candidate(c)}),
        Phase::AwaitingFirstLaunch(c) => {
            json!({"type": "awaiting_first_launch", "candidate": candidate(c)})
        }
        Phase::InstallFailed {
            candidate: c,
            failure,
        } => {
            json!({"type": "install_failed", "candidate": candidate(c), "failure": failure.as_str()})
        }
    }
}
pub(crate) fn state(value: &State) -> String {
    json!({
        "domain": value.domain.as_ref().map(|machine| json!({
            "shell": machine.shell().to_string(), "running": machine.running().to_string(),
            "auto_check": machine.auto_check(), "phase": phase(&machine.phase),
            "capability": match machine.capability { Capability::InApp => "in_app", Capability::ManualOnly => "manual_only", Capability::SystemManaged => "system_managed", Capability::Unsupported => "unsupported" },
        })),
        "info": { "shell_build": value.info.shell_build, "package_build": value.info.package_build, "last_checked": value.info.last_checked, "last_result": value.info.last_result, "manual_url": value.info.manual_url, "manual_check": value.info.manual_check },
        "pending": value.pending.to_string(),
        "execution": match value.execution { Execution::Available => "available", Execution::Running => "running" },
    }).to_string()
}
pub(crate) fn claim(value: &Claim) -> String {
    let effect = match value.effect() {
        Effect::StartCheck { ticket } => json!({"type": "check", "ticket": ticket.to_string()}),
        Effect::StartDownload {
            ticket,
            candidate: c,
        } => json!({"type": "download", "ticket": ticket.to_string(), "candidate": candidate(c)}),
        Effect::StartInstall {
            ticket,
            candidate: c,
        } => json!({"type": "install", "ticket": ticket.to_string(), "candidate": candidate(c)}),
        Effect::Notify(notice) => json!({"type": "notify", "notice": match notice {
            Notice::UpToDate => json!({"type": "up_to_date"}),
            Notice::UpdateAvailable(v) => json!({"type": "update_available", "version": v.to_string()}),
            Notice::ShellUpdateNeeded(v) => json!({"type": "shell_update_needed", "version": v.to_string()}),
            Notice::ManualUpgradeNeeded(v) => json!({"type": "manual_upgrade_needed", "version": v.to_string()}),
            Notice::CheckFailed(f) => json!({"type": "check_failed", "failure": f.as_str()}),
        }}),
        Effect::Log(_) => unreachable!("log records are not executable claims"),
    };
    json!({"token": value.token(), "effect": effect}).to_string()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::update_service::UpdateService;
    use usher_state::backends::WatchBackend;

    fn config() -> Configuration {
        parse(r#"{"shell":"1.0.0","running":"1.0.1","capability":"in_app","failure_limit":"2"}"#)
            .unwrap()
    }
    #[test]
    fn configuration_uses_host_preferences_and_rejects_malformed_identity() {
        assert!(!config().settings(false).unwrap().auto_check);
        let mut invalid = config();
        invalid.shell = None;
        assert_eq!(invalid.settings(true), Err(Error::Invalid));
        let mut unsupported = config();
        unsupported.shell = None;
        unsupported.running = None;
        unsupported.capability = "unsupported".into();
        assert_eq!(
            unsupported.settings(true).unwrap().capability,
            Capability::Unsupported
        );
        for invalid in ["0", "-1", "2.5", "18446744073709551616", "4294967296"] {
            let mut value = config();
            value.failure_limit = invalid.into();
            assert_eq!(value.settings(true), Err(Error::Invalid));
        }
        assert!(parse::<Configuration>(r#"{"shell":"1.0.0","running":null,"capability":"in_app","failure_limit":"2","auto_check":true}"#).is_err());
    }
    #[test]
    fn transport_round_trip_fences_foreign_and_duplicate_replies() {
        let service = UpdateService::new(&WatchBackend).unwrap();
        let before: Value =
            serde_json::from_str(&state(service.snapshot().unwrap().state())).unwrap();
        assert!(before["domain"].is_null());
        service
            .request(Command::Configure(config().settings(false).unwrap()))
            .unwrap();
        let lease = service.acquire().unwrap();
        service
            .request(
                parse::<Request>(r#"{"type":"check","background":false}"#)
                    .unwrap()
                    .command()
                    .unwrap(),
            )
            .unwrap();
        let work = service
            .claim(service.lease_token(&lease.token()).unwrap())
            .unwrap()
            .unwrap();
        let encoded: Value = serde_json::from_str(&claim(&work)).unwrap();
        let token = encoded["token"].as_str().unwrap();
        let (reply, detail) =
            parse::<Reply>(r#"{"type":"checked","offer":{"version":"1.0.2","has_package":true}}"#)
                .unwrap()
                .completion()
                .unwrap();
        assert!(detail.is_none());
        let foreign = UpdateService::new(&WatchBackend).unwrap();
        assert_eq!(
            foreign.lease_token(&lease.token()),
            Err(Error::NotConfigured)
        );
        foreign
            .request(Command::Configure(config().settings(false).unwrap()))
            .unwrap();
        assert_eq!(foreign.claim_token(token), Err(Error::Stale));
        service
            .complete(&service.claim_token(token).unwrap(), reply)
            .unwrap();
        assert_eq!(service.claim_token(token), Err(Error::Stale));
        let after: Value =
            serde_json::from_str(&state(service.snapshot().unwrap().state())).unwrap();
        assert_eq!(
            after["domain"]["phase"]["outcome"]["candidate"]["kind"],
            "package"
        );
    }
    #[test]
    fn runtime_metadata_is_shared_and_check_failures_remain_distinct() {
        let service = UpdateService::new(&WatchBackend).unwrap();
        let mut configuration = config();
        configuration.shell_build = Some("12".into());
        service
            .configure_runtime(
                configuration.settings(true).unwrap(),
                configuration.info().unwrap(),
            )
            .unwrap();
        let lease = service.acquire().unwrap();
        service
            .request(Command::Check(Trigger::Background))
            .unwrap();
        service.request(Command::Check(Trigger::Manual)).unwrap();
        let claim = service.claim(lease).unwrap().unwrap();
        let (reply, _) = parse::<Reply>(r#"{"type":"checked","offer":{"version":"1.1.0","has_package":false,"can_install":false},"manual_url":"https://example.test/update"}"#).unwrap().completion().unwrap();
        service.complete(&claim, reply).unwrap();
        let snapshot = service.snapshot().unwrap();
        let info = &snapshot.state().info;
        assert_eq!(info.shell_build.as_deref(), Some("12"));
        assert_eq!(info.last_result.as_deref(), Some("manual_upgrade"));
        assert!(info.manual_check);
        assert!(
            chrono::DateTime::parse_from_rfc3339(info.last_checked.as_deref().unwrap()).is_ok()
        );
        assert_eq!(
            info.manual_url.as_deref(),
            Some("https://example.test/update")
        );
        // Consume the notification before the next background check.
        let notice = service.claim(lease).unwrap().unwrap();
        service.complete(&notice, Completion::Notified).unwrap();
        service
            .request(Command::Check(Trigger::Background))
            .unwrap();
        let claim = service.claim(lease).unwrap().unwrap();
        service
            .complete(&claim, Completion::Failed(Failure::Network))
            .unwrap();
        let snapshot = service.snapshot().unwrap();
        assert_eq!(snapshot.state().info.last_result.as_deref(), Some("failed"));
        assert!(!snapshot.state().info.manual_check);
        service.request(Command::Check(Trigger::Manual)).unwrap();
        let claim = service.claim(lease).unwrap().unwrap();
        service
            .complete(&claim, Completion::Failed(Failure::Network))
            .unwrap();
        service.request(Command::AutoCheck(false)).unwrap();
        service
            .request(Command::Check(Trigger::Background))
            .unwrap();
        assert!(service.snapshot().unwrap().state().info.manual_check);
        configuration.shell_build = Some("invalid".into());
        assert_eq!(configuration.info(), Err(Error::Invalid));
    }
    #[test]
    fn unsigned_transport_values_never_round_through_javascript_numbers() {
        assert_eq!(decimal("18446744073709551615"), Ok(u64::MAX));
        for value in ["-1", "1.2", "1e3", "", "+2", " 2"] {
            assert_eq!(decimal(value), Err(Error::Invalid));
        }
        assert!(
            parse::<Reply>(r#"{"type":"failed","failure":"mystery"}"#)
                .unwrap()
                .completion()
                .is_err()
        );
        assert!(parse::<Request>(r#"{"type":"complete","ticket":"1"}"#).is_err());
    }
}
