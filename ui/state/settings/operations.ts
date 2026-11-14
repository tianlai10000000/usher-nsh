/** Coordinates shared settings requests and window-owned feedback without owning preference data. */
import { onDispose, observe } from "../../state-runtime/nsh";
import type { Immutable, StateView } from "../../state-runtime/contracts";
import type { SettingsData, SettingsCommand } from "./contracts";
import type { WindowSession } from "../window/index";
import { type MessageKey } from "../../i18n/index";

/** Owner hooks reuse NSH in production and allow deterministic lifecycle tests. */
export interface OperationLifecycle {
  onDispose(dispose: () => void): void;
  observe(run: () => void): void;
}
export const operationLifecycle: OperationLifecycle = { onDispose, observe };
export type SettingsView = StateView<SettingsData, SettingsCommand>;
export type SettingsStatus =
  | "loading"
  | "saving"
  | "scanning"
  | "saved"
  | "load-error"
  | "save-error"
  | "scan-error"
  | "invalid"
  | "recovered"
  | "read-only";

const validationLabels: Partial<Record<string, MessageKey>> = {
  invalid_theme_name: "theme.invalidName",
  invalid_theme_color: "theme.invalidColor",
  invalid_theme_contrast: "theme.lowContrast",
  invalid_accent: "theme.invalidColor",
  unknown_theme: "theme.selectionUnavailable",
  style_theme_invalid_name: "theme.invalidName",
  style_theme_directory_unavailable: "theme.directoryUnavailable",
  style_theme_write_failed: "theme.fileWriteFailed",
  style_theme_id_exhausted: "theme.copyIdFailed",
  background_image_invalid: "background.invalidImage",
  background_image_too_large: "background.tooLarge",
  background_image_write_failed: "background.writeFailed",
  background_opacity_invalid: "background.invalidOpacity",
  content_background_transparency_invalid: "background.invalidContentTransparency",
};

/** One coordinator per settings owner; all modules borrow its serialized request capability. */
export function createSettingsOperations(
  settings: SettingsView,
  session: Pick<WindowSession, "value" | "begin" | "finish" | "change">,
  reconnect: () => Promise<void>,
  lifecycle: OperationLifecycle = operationLifecycle,
) {
  const snapshot = settings.value;
  const operation = () => session.value().operations.settings;
  const status = (): SettingsStatus => (operation()?.stage ?? "loading") as SettingsStatus;
  const validationLabel = () => validationLabels[operation()?.error ?? ""];
  const busy = () => session.value().operations.picker?.active === true || status() === "loading" || status() === "saving" || status() === "scanning";
  const retryable = () => status() === "load-error" || status() === "save-error" || status() === "scan-error";
  const statusKey = (): MessageKey => {
    const current = status();
    return validationLabel() ?? (current === "invalid" ? "status.save-error" : `status.${current}`);
  };
  type Request = () => Promise<Immutable<SettingsData>>;
  type Stage = "loading" | "saving" | "scanning";
  const requestErrors = { loading: "load-error", saving: "save-error", scanning: "scan-error" } as const;
  let requestInFlight = false;
  let disposed = false;
  let lastRequest: { run: Request; stage: Stage } | undefined;
  lifecycle.onDispose(() => { disposed = true; });
  function persistedStatus(next: Immutable<SettingsData>): SettingsStatus {
    let nextStatus: SettingsStatus = next.saved ? "saved" : "save-error";
    if (next.problem === "settings_invalid_recovered") {
      nextStatus = "recovered";
    } else if (
      next.problem === "settings_directory_unavailable" ||
      next.problem === "settings_read_failed" ||
      next.problem === "settings_backup_failed" ||
      next.problem === "settings_unsupported_version"
    ) {
      nextStatus = "read-only";
    }

    return nextStatus;
  }

  // This local flag serializes the effect executor; visible operation state lives in Rust.
  async function sync(run: Request, stage: Stage = "saving"): Promise<boolean> {
    if (requestInFlight || disposed) return false;
    requestInFlight = true;
    lastRequest = { run, stage };
    let ticket: string | undefined;
    try {
      ticket = await session.begin("settings", stage);
      if (disposed) return false;
      const next = await run();
      if (disposed) return false;
      await session.finish("settings", ticket, persistedStatus(next));
      return true;
    } catch (cause) {
      if (!disposed && ticket) {
        const code = cause instanceof Error ? cause.message : String(cause);
        await session.finish("settings", ticket, validationLabels[code] ? "invalid" : requestErrors[stage], code, null, true);
      }
      return false;
    } finally { requestInFlight = false; }
  }

  lifecycle.observe(() => {
    const synchronization = settings.status();
    const incoming = snapshot();
    if (disposed) return;
    const stage = synchronization === "live" ? persistedStatus(incoming) : "load-error";
    if (synchronization !== "connecting" && synchronization !== "disposed") void session.change({ kind: "observe_settings", stage });
  });

  return {
    sync, status, statusKey, busy, retryable,
    disposed: () => disposed,
    retry: async () => {
      if (disposed) return false;
      if (["error", "closed", "cancelled"].includes(settings.status())) await reconnect();
      return lastRequest && sync(lastRequest.run, lastRequest.stage);
    },
  };
}
export type SettingsOperations = ReturnType<typeof createSettingsOperations>;
