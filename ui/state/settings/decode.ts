/** Validates the settings aggregate by delegating each business slice to its domain schema. */
import * as d from "../../state-runtime/decode";
import type { SettingsData } from "./contracts";
import { decodeMode, decodeFamily, decodeStyleTheme, decodeCustomTheme } from "../appearance/decode";
import { decodeLanguage } from "../preferences/decode";
import { decodeDiagnosticLevel } from "../diagnostics/decode";
import { decodeBackground } from "../background/decode";
export function decodeSettings(value: unknown): SettingsData {
  const state = d.object(value);
  decodeMode(state.colorMode); decodeFamily(state.componentFamily);
  decodeLanguage(state.localePreference); decodeDiagnosticLevel(state.diagnosticLevel);
  for (const key of ["showStatusBar", "sidebarDocked", "diagnosticConsole", "diagnosticFile", "diagnosticStructured", "saved"]) d.boolean(state[key]);
  d.string(state.themeId); d.optional(state.accent, value => d.requireValue(/^#[0-9a-f]{6}$/i.test(d.string(value))));
  decodeBackground(state);
  d.integer(state.revision);
  d.optional(state.problem, d.string);
  d.array(state.styleThemes).forEach(decodeStyleTheme);
  d.array(state.customThemes).forEach(decodeCustomTheme);
  d.array(state.themeFileIssues).forEach(value => { const issue = d.object(value); d.string(issue.fileName); d.string(issue.code); });
  // Legacy native DTO health fields are deliberately excluded from the preference projection.
  const { diagnosticDropped: _dropped, diagnosticProblem: _problem, ...preferences } = state;
  return preferences as unknown as SettingsData;
}
export const decodeSettingsEnvelope = (wire: unknown) => d.envelope(wire, decodeSettings);
