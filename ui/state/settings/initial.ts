/** Startup rendering placeholder; only live Rust snapshots may drive host preferences. */
import { defaultThemeId } from "../../theme/index";
import backgroundConfig from "../../config/background.json";
import type { SettingsData } from "./contracts";
export function initialSettings(): SettingsData {
  return {
    colorMode: "system",
    themeId: defaultThemeId,
    componentFamily: "nsh",
    showStatusBar: true,
    sidebarDocked: true,
    backgroundImage: null,
    backgroundImageOpacity: backgroundConfig.defaultOpacity,
    contentBackgroundTransparency: backgroundConfig.contentTransparency.default,
    localePreference: "system",
    diagnosticLevel: "INFO",
    diagnosticConsole: false,
    diagnosticFile: true,
    diagnosticStructured: false,
    customThemes: [],
    styleThemes: [],
    themeFileIssues: [],
    revision: 0,
    saved: true,
  };
}
