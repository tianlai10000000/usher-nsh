/** Coordinates appearance commands and the theme-directory lifecycle through shared settings actions. */
import type { ColorMode } from "../settings/contracts";
import type { ComponentFamily, CustomTheme } from "../../theme/index";
import type { MessageKey } from "../../i18n/index";
import type { WindowSession } from "../window/index";
import type { SettingsView, SettingsOperations } from "../settings/operations";
export function createAppearanceActions(
  settings: SettingsView,
  session: Pick<WindowSession, "value" | "begin" | "finish">,
  operations: Pick<SettingsOperations, "sync" | "disposed">,
  openDirectory: () => Promise<void>,
) {
  const { sync } = operations;
  const openingDirectory = () => session.value().operations.directory?.active === true;
  const directoryProblem = (): MessageKey | undefined => {
    const error = session.value().operations.directory?.error;
    return error ? error === "style_theme_directory_unavailable" ? "theme.directoryUnavailable" : "theme.directoryOpenFailed" : undefined;
  };
  let opening = false;
  async function openThemeDirectory() {
    if (opening || openingDirectory() || operations.disposed()) return;
    opening = true;
    let ticket: string | undefined;
    try {
      ticket = await session.begin("directory", "running");
      if (operations.disposed()) return;
      await openDirectory();
      if (!operations.disposed()) await session.finish("directory", ticket, "done");
    } catch (cause) {
      if (!ticket) throw cause;
      if (ticket && !operations.disposed()) await session.finish("directory", ticket, "error", cause instanceof Error ? cause.message : String(cause), null, true);
    } finally { opening = false; }
  }

  return {
    directoryProblem, openingDirectory, openThemeDirectory,
    choose: (value: ColorMode) => sync(() => settings.dispatch({ type: "colorMode", value })),
    chooseTheme: (value: string) => sync(() => settings.dispatch({ type: "theme", value })),
    chooseComponentFamily: (value: ComponentFamily) => sync(() => settings.dispatch({ type: "componentFamily", value })),
    setAccent: (value: string | null) => sync(() => settings.dispatch({ type: "accent", value })),
    saveTheme: (value: CustomTheme) => sync(() => settings.dispatch({ type: "saveTheme", value })),
    cloneStyleTheme: (id: string, name: string) => sync(() => settings.dispatch({ type: "cloneTheme", id, name })),
    removeTheme: (value: string) => sync(() => settings.dispatch({ type: "removeTheme", value })),
    rescanThemes: () => sync(() => settings.dispatch({ type: "rescanThemes" }), "scanning"),
  };
}
