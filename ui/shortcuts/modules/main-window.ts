/** Binds process intents to the main window's Router and shared settings actions. */
import { cleanup, effect } from "nsh";
import { currentWindow } from "nsh/desktop";
import type { Navigation } from "../../router";
import type { AppState } from "../../state/app";
import { applicationCommands } from "./application";
import { toggleScheme } from "./appearance";
/** Bind only the main window after its settings projection is live; teardown fences old handlers. */
export function bindMainWindowShortcuts(router: Navigation, state: AppState) {
  const host = currentWindow();
  if (!host.main) return;
  effect(() => {
    if (state.settings.view.status() !== "live") return;
    const binding = applicationCommands.bind(async command => {
      if (command.module === "window" && command.action === "openSettings") {
        host.focus();
        return router.navigate("settings");
      }
      if (command.module === "appearance" && command.action === "toggleScheme") return toggleScheme(state.appearance);
    });
    void binding.ready;
    cleanup(binding.release);
  });
}
