/** Projects authoritative update phases into localized presentation and available actions. */
import type { UpdateData } from "./contracts";
import type { MessageKey } from "../../i18n";
export function presentation(state: UpdateData, development = false) {
  const domain = state.domain;
  const phase = domain?.phase;
  const outcome = phase?.outcome;
  const candidate = phase?.candidate ?? outcome?.candidate;
  const unavailable = !domain ||
    ["unsupported", "system_managed"].includes(domain.capability);
  const ready = phase?.type === "ready_to_install";
  const failure = phase?.failure ?? outcome?.failure;
  const manualCheckFailure = outcome?.type === "failed" &&
    state.info.manual_check === true;
  const quietFailure = outcome?.type === "failed" && !state.info.manual_check; // Background errors are shown in settings, not as attention badges.
  const progress =
    phase?.type === "downloading" && phase.total && phase.total !== "0"
      ? Math.min(1, Number(phase.downloaded ?? "0") / Number(phase.total))
      : null;
  let key: MessageKey = "update.preparing";
  if (domain) {
    if (domain.capability === "unsupported") {
      key = development ? "update.development" : "update.unsupported";
    } else if (domain.capability === "system_managed") {
      key = "update.systemManaged";
    } else if (failure) key = `update.failure.${failure}`;
    else if (ready) key = "update.ready";
    else if (phase?.type === "downloading") key = "update.downloading";
    else if (phase?.type === "checking") key = "update.checking";
    else if (
      phase?.type === "installing" || phase?.type === "awaiting_first_launch"
    ) key = "update.installing";
    else if (outcome?.type === "manual_upgrade") key = "update.manual";
    else if (outcome?.type === "shell_update") key = "update.shellNeeded";
    else if (outcome?.type === "package_update") key = "update.available";
    else if (outcome?.type === "up_to_date") key = "update.upToDate";
    else key = "update.check";
  }
  const working = [
    "checking",
    "downloading",
    "installing",
    "awaiting_first_launch",
  ].includes(phase?.type ?? "");
  const found = outcome?.type === "package_update" ||
    outcome?.type === "shell_update";
  return {
    key,
    indicatorKey: found || outcome?.type === "manual_upgrade"
      ? "update.indicatorAvailable" as const
      : key,
    visible: !unavailable && (working || ready || !!found ||
      outcome?.type === "manual_upgrade" || manualCheckFailure ||
      (!!candidate && !!failure)),
    candidate,
    failure,
    progress,
    ready,
    unavailable,
    working,
    canCheck: !unavailable && !working && !ready,
    canDownload: !!found ||
      (phase?.type === "install_failed" && failure !== "rejected"),
    canRetryCheck: !!failure && !working,
    shape: phase?.type === "checking"
      ? "checking" as const
      : phase?.type === "downloading"
      ? "progress" as const
      : ready || found || outcome?.type === "manual_upgrade" ||
          (!!failure && !quietFailure)
      ? "emphasis" as const
      : "quiet" as const,
    disabled: unavailable || working,
    warning: !!failure && !quietFailure,
  };
}
