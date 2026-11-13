/** Application-owned settings data and semantic commands, independent of bridge types. */
import type { CustomTheme, StyleTheme } from "../theme/contracts";
export type ColorMode = "system" | "light" | "dark";
export type ComponentFamily = "nsh" | "appkit";
export type DiagnosticLevel = "OFF" | "ERROR" | "WARN" | "INFO" | "DEBUG" | "TRACE";
export interface SettingsData {
  colorMode: string;
  showStatusBar: boolean;
  sidebarDocked: boolean;
  backgroundImage?: { id: string; name: string } | null;
  backgroundImageOpacity: number;
  contentBackgroundTransparency: number;
  componentFamily: string;
  themeId: string;
  accent?: string | null;
  localePreference: string;
  diagnosticLevel: string;
  diagnosticProblem?: string | null;
  diagnosticDropped: number;
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
  | { type: "accent"; value: string | null }
  | { type: "saveTheme"; value: CustomTheme }
  | { type: "cloneTheme"; id: string; name: string }
  | { type: "importBackground"; name: string; bytes: Uint8Array }
  | { type: "clearBackground" | "rescanThemes" | "startDiagnostics" };
