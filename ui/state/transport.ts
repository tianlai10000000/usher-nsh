/** Adapts native settings commands and engine-owned notifications to project contracts. */
import { DiagnosticState, Settings, SettingsState } from "usher/nsh";
import type { StateTransport } from "./contracts";
import type { SettingsCommand, SettingsData } from "./settings";

export function createSettingsTransport(): StateTransport<SettingsData, SettingsCommand> {
  return {
    subscribe(observer) {
      let active = true;
      const stop = SettingsState.subscribe(event => {
        if (!active) return;
        if (event.snapshot && !event.end) observer.snapshot(event.snapshot);
        else if (!event.snapshot && (event.end === "closed" || event.end === "cancelled" || event.end === "error")) observer.end(event.end);
        else observer.end("error");
      });
      return () => { if (active) { active = false; stop(); } };
    },
    read: () => SettingsState.snapshot(),
    async dispatch(command) {
      switch (command.type) {
        case "colorMode": await Settings.setColorMode(command.value); break;
        case "statusBar": await Settings.setShowStatusBar(command.value); break;
        case "sidebarDocked": await Settings.setSidebarDocked(command.value); break;
        case "opacity": await Settings.setBackgroundImageOpacity(command.value); break;
        case "transparency": await Settings.setContentBackgroundTransparency(command.value); break;
        case "componentFamily": await Settings.setComponentFamily(command.value); break;
        case "theme": await Settings.setTheme(command.value); break;
        case "locale": await Settings.setLocale(command.value); break;
        case "diagnosticLevel": await Settings.setDiagnosticLevel(command.value); break;
        case "accent": await Settings.setAccent(command.value); break;
        case "saveTheme": await Settings.saveCustomTheme(command.value); break;
        case "removeTheme": await Settings.removeCustomTheme(command.value); break;
        case "cloneTheme": await Settings.cloneStyleTheme(command.id, command.name); break;
        case "importBackground": await Settings.importBackgroundImage(command.name, command.bytes); break;
        case "clearBackground": await Settings.clearBackgroundImage(); break;
        case "rescanThemes": await Settings.rescanThemes(); break;
        case "startDiagnostics": await Settings.startDiagnostics(); break;
        default: { const exhaustive: never = command; throw new Error(`state_unknown_command: ${exhaustive}`); }
      }
      // Legacy replies have no publication sequence. Never attach a newer sequence
      // to an old reply: read a fresh, internally consistent envelope instead.
      return SettingsState.snapshot();
    },
  };
}

/** Non-state host operations remain separate from subscription and ordering. */
export const settingsEffects = {
  appKitThemeAvailable: () => Settings.appKitThemeAvailable(),
  openThemeDirectory: () => Settings.openThemeDirectory(),
  exportDiagnostics: (from: string, to: string) => Settings.exportDiagnostics(from, to),
};

/** Diagnostic updates have their own version stream; legacy settings DTO fields remain compatible. */
export function createDiagnosticTransport(): StateTransport<import("./diagnostics").DiagnosticData, never> {
  return {
    read: async () => DiagnosticState.snapshot(),
    dispatch: async () => { throw new Error("diagnostics_read_only"); },
    subscribe(observer) {
      let active = true;
      const stop = DiagnosticState.subscribe(event => {
        if (!active) return;
        if (event.snapshot && !event.end) observer.snapshot(event.snapshot);
        else observer.end(event.end === "closed" || event.end === "cancelled" ? event.end : "error");
      });
      return () => { if (active) { active = false; stop(); } };
    },
  };
}
