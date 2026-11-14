/** Projects diagnostic preferences and health and coordinates startup and export actions. */
import diagnosticConfig from "../../config/diagnostics.json";
import { createDiagnosticState } from "./health";
import type { DiagnosticData } from "./health";
import type { MessageKey } from "../../i18n/index";
import type { Readable, Immutable } from "../../state-runtime/contracts";
import type { WindowSession } from "../window/index";
import type { DiagnosticLevel, DiagnosticOutput } from "../settings/contracts";
import { operationLifecycle, type OperationLifecycle, type SettingsView, type SettingsOperations } from "../settings/operations";
export const diagnosticLevels = diagnosticConfig.levels as DiagnosticLevel[];
export type { DiagnosticLevel, DiagnosticOutput } from "../settings/contracts";
const diagnosticErrorKeys: Record<string, true> = {
  "diagnostics.error.diagnostics_directory_unavailable": true,
  "diagnostics.error.diagnostics_export_empty": true,
  "diagnostics.error.diagnostics_export_failed": true,
  "diagnostics.error.diagnostics_export_invalid_range": true,
  "diagnostics.error.diagnostics_export_cancelled": true,
};

export function createDiagnosticsModule(
  settings: SettingsView,
  session: Pick<WindowSession, "value" | "exportDiagnostics">,
  operations: Pick<SettingsOperations, "sync" | "disposed">,
  health: { value: Readable<Immutable<DiagnosticData>> } = createDiagnosticState(),
  lifecycle: OperationLifecycle = operationLifecycle,
  now: () => Date = () => new Date(),
) {
  const snapshot = settings.value;
  const { sync } = operations;
  lifecycle.observe(() => { void sync(() => settings.dispatch({ type: "startDiagnostics" }), "loading"); });
  const exportState = () => {
    const state = session.value().operations.export;
    const code = `diagnostics.error.${state?.error}` as MessageKey;
    return { kind: (state?.stage ?? "idle") as "idle" | "running" | "done" | "error",
      name: state?.value ?? undefined,
      text: state?.error ? code in diagnosticErrorKeys ? code : "diagnostics.error.diagnostics_export_failed" as MessageKey : undefined };
  };
  async function exportDiagnostics(days: number) {
    if (exportState().kind === "running" || operations.disposed()) return;
    const end = now();
    const start = new Date(end.getTime() - (days - 1) * 86_400_000);
    const day = (date: Date) => `${date.getFullYear()}-${String(date.getMonth() + 1).padStart(2, "0")}-${String(date.getDate()).padStart(2, "0")}`;
    await session.exportDiagnostics(day(start), day(end));
  }

  return {
    diagnosticLevel: () => snapshot().diagnosticLevel as DiagnosticLevel,
    diagnosticOutput: (output: DiagnosticOutput) => ({ console: snapshot().diagnosticConsole, file: snapshot().diagnosticFile, structured: snapshot().diagnosticStructured })[output],
    chooseDiagnosticOutput: (output: DiagnosticOutput, value: boolean) => sync(() => settings.dispatch({ type: "diagnosticOutput", output, value })),
    diagnosticProblem: () => health.value().problem ?? null,
    diagnosticDropped: () => Number(health.value().dropped),
    chooseDiagnosticLevel: (level: DiagnosticLevel) => sync(() => settings.dispatch({ type: "diagnosticLevel", value: level })),
    exportState, exportDiagnostics,
  };
}
