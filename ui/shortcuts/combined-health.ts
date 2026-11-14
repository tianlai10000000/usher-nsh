/** Combines borrowed global and window health without transferring either registration lifetime. */
import type { Immutable } from "../state-runtime/contracts";
import type { Health, Phase } from "./contracts";
export function combineHealth(local: Immutable<Health>, global: Immutable<Health>): Immutable<Health> & { readonly window: Phase } {
  const phases = [local.startup, global.global];
  const startup: Phase = phases.includes("failed") ? "failed" : phases.includes("recovering") ? "recovering"
    : phases.some(phase => phase === "installing" || phase === "validating" || phase === "loading") ? "installing"
    : local.startup;
  return { ...local, window: local.startup, startup, global: global.global, modules: [...local.modules, ...global.modules], bindings: [...local.bindings, ...global.bindings] };
}
