/** Application-owned settings data and semantic commands, independent of bridge types. */
import type { LocalePreference } from "../../i18n/index";
import type { CustomTheme, StyleTheme } from "../appearance/contracts";
export type ColorMode = "system" | "light" | "dark";
export type ComponentFamily = "nsh" | "appkit";
export type DiagnosticOutput = "console" | "file" | "structured";
export type DiagnosticLevel = "OFF" | "ERROR" | "WARN" | "INFO" | "DEBUG" | "TRACE";
export interface SettingsData {
  colorMode: ColorMode;
  showStatusBar: boolean;
  sidebarDocked: boolean;
  backgroundImage?: { id: string; name: string } | null;
  backgroundImageOpacity: number;
  contentBackgroundTransparency: number;
  componentFamily: ComponentFamily;
  themeId: string;
  accent?: string | null;
  localePreference: LocalePreference;
  diagnosticLevel: DiagnosticLevel;
  diagnosticConsole: boolean;
  diagnosticFile: boolean;
  diagnosticStructured: boolean;
  customThemes: CustomTheme[];
  styleThemes: StyleTheme[];
  themeFileIssues: { fileName: string; code: string }[];
  revision: number;
  saved: boolean;
  problem?: string | null;
}
export type SettingsCommand =
  | { type: "colorMode"; value: ColorMode }
  | { type: "statusBar" | "sidebarDocked"; value: boolean }
  | { type: "opacity" | "transparency"; value: number }
  | { type: "componentFamily"; value: ComponentFamily }
  | { type: "theme" | "locale" | "removeTheme"; value: string }
  | { type: "diagnosticLevel"; value: DiagnosticLevel }
  | { type: "diagnosticOutput"; output: DiagnosticOutput; value: boolean }
  | { type: "accent"; value: string | null }
  | { type: "saveTheme"; value: CustomTheme }
  | { type: "cloneTheme"; id: string; name: string }
  | { type: "importBackground"; name: string; bytes: Uint8Array }
  | { type: "clearBackground" | "rescanThemes" | "startDiagnostics" };
