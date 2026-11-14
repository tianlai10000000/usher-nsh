/** Projects authoritative shortcut health into localized layout data without owning registration. */
import { keymap } from "nsh";
import type { Translate, MessageKey } from "../i18n/index";
import type { ShortcutIndicator } from "../layout/contracts/shortcuts";
import type { Shortcuts } from "./runtime/nsh";
import type { Phase } from "./contracts";
const phaseKeys: Record<Phase, MessageKey> = {
  validating: "shortcuts.validating", loading: "shortcuts.loading", installing: "shortcuts.installing", ready: "shortcuts.ready", failed: "shortcuts.failed",
  unsupported: "shortcuts.unsupported", recovering: "shortcuts.recovering",
  notRequested: "shortcuts.notRequested", notLoaded: "shortcuts.notLoaded", inactive: "shortcuts.inactive",
};
export function shortcutIndicator(shortcuts: Shortcuts, t: Translate): ShortcutIndicator {
  const live = () => shortcuts.connection() === "live";
  const phase = (value: Phase) => t(phaseKeys[value]);
  return {
    summary: () => t("shortcuts.startupStatus", { status: live() ? phase(shortcuts.value().startup) : t("shortcuts.disconnected") }),
    pageSummary: () => t("shortcuts.pageStatus", { status: live() ? phase(shortcuts.value().page) : t("shortcuts.disconnected") }),
    warning: () => !live() || ["failed", "recovering"].includes(shortcuts.value().startup) || shortcuts.value().page === "failed",
    details: () => [
      { id: "startup", label: t("shortcuts.startup"), value: live() ? phase(shortcuts.value().window) : t("shortcuts.disconnected") },
      { id: "page", label: t("shortcuts.page"), value: live() ? phase(shortcuts.value().page) : t("shortcuts.disconnected") },
      { id: "global", label: t("shortcuts.global"), value: phase(shortcuts.value().global) },
      ...shortcuts.value().modules.map(module => ({ id: `module-${module.id}`, label: module.id, value: `${phase(module.phase)}${module.error ? ` · ${t("shortcuts.moduleError")}` : ""}` })),
      ...shortcuts.value().bindings.map(binding => ({ id: `binding-${binding.id}`, label: `${binding.id} · ${keymap.label(binding.keystroke)}`, value: t(`shortcuts.binding.${binding.state}` as MessageKey) })),
    ].map(row => live() ? row : { ...row, value: t("shortcuts.disconnected") }),
    retryable: () => !live() || ["failed", "recovering"].includes(shortcuts.value().startup) || shortcuts.value().page === "failed",
    retry: shortcuts.retry,
  };
}
