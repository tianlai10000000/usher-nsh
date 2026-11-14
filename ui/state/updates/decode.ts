/** Decodes update state, discriminated phases and executor claims at every wire entry. */
import * as d from "../../state-runtime/decode";
import type { UpdateData, Claim } from "./contracts";
const failures = ["network", "busy", "rejected", "disk", "cancelled", "unknown"] as const;
function candidate(value: unknown) { const c = d.object(value); d.string(c.version); d.oneOf(c.kind, ["package", "shell"]); }
function outcome(value: unknown) {
  const state = d.object(value);
  const type = d.oneOf(state.type, ["up_to_date", "package_update", "shell_update", "manual_upgrade", "system_managed", "not_applicable", "failed"]);
  if (["package_update", "shell_update", "manual_upgrade"].includes(type)) candidate(state.candidate);
  if (type === "failed") d.oneOf(state.failure, failures);
  d.requireValue(state.candidate === undefined || ["package_update", "shell_update", "manual_upgrade"].includes(type));
  d.requireValue(state.failure === undefined || type === "failed");
  d.optional(state.reason, d.string);
}
export function decodeUpdates(value: unknown): UpdateData {
  const state = d.object(value);
  d.requireValue("domain" in state);
  if (state.domain !== null) {
    const domain = d.object(state.domain);
    d.string(domain.shell); d.string(domain.running); d.boolean(domain.auto_check);
    d.oneOf(domain.capability, ["in_app", "manual_only", "system_managed", "unsupported"]);
    const phase = d.object(domain.phase);
    const type = d.oneOf(phase.type, ["idle", "checking", "checked", "downloading", "ready_to_install", "installing", "awaiting_first_launch", "install_failed"]);
    d.requireValue(phase.outcome === undefined || type === "checked");
    d.requireValue(phase.candidate === undefined || ["downloading", "ready_to_install", "installing", "awaiting_first_launch", "install_failed"].includes(type));
    d.requireValue(phase.failure === undefined || type === "install_failed");
    if (type === "checked") outcome(phase.outcome);
    if (["downloading", "ready_to_install", "installing", "awaiting_first_launch", "install_failed"].includes(type)) candidate(phase.candidate);
    if (type === "install_failed") d.oneOf(phase.failure, failures);
    if (type === "downloading") { d.decimal(phase.downloaded); d.optional(phase.total, d.decimal); }
  }
  const info = d.object(state.info);
  d.optional(info.manual_check, d.boolean);
  for (const key of ["shell_build", "package_build", "last_checked", "last_result", "manual_url"]) d.optional(info[key], d.string);
  d.decimal(state.pending); d.oneOf(state.execution, ["available", "running"]);
  return value as UpdateData;
}
export function decodeClaim(value: unknown): Claim {
  const claim = d.object(value); d.string(claim.token);
  const effect = d.object(claim.effect);
  const type = d.oneOf(effect.type, ["check", "download", "install", "notify"]);
  if (type === "download" || type === "install") candidate(effect.candidate);
  else d.optional(effect.candidate, candidate);
  return value as Claim;
}
export const decodeUpdateEnvelope = (wire: unknown) => d.envelope(wire, decodeUpdates, true);
