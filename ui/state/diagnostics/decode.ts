/** Decodes diagnostic preferences and the independent health snapshot stream. */
import * as d from "../../state-runtime/decode";
import config from "../../config/diagnostics.json";
import type { DiagnosticData } from "./health";
import type { DiagnosticLevel } from "../settings/contracts";
export const decodeDiagnosticLevel = (value: unknown): DiagnosticLevel => d.oneOf(value, config.levels as DiagnosticLevel[]);
export function decodeDiagnosticData(value: unknown): DiagnosticData {
  const state = d.object(value); d.decimal(state.dropped); d.optional(state.problem, d.string);
  return value as DiagnosticData;
}
export const decodeDiagnosticEnvelope = (wire: unknown) => d.envelope(wire, decodeDiagnosticData);
