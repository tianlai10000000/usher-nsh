/** Adapts native settings commands and engine-owned notifications to project contracts. */
import { subscribeDecoded } from "../../state-runtime/decode";
import { decodeSettingsEnvelope } from "./decode";
import { Settings, SettingsState } from "usher/nsh";
import type { StateTransport } from "../../state-runtime/contracts";
import type { SettingsCommand, SettingsData } from "./contracts";

export interface SettingsWire {
  snapshot(): Promise<unknown>;
  subscribe(receive: (event: unknown) => void): () => void;
}
export function createSettingsTransport(
  wire: SettingsWire = SettingsState,
  execute: (command: SettingsCommand) => Promise<void> = executeSettings,
): StateTransport<SettingsData, SettingsCommand> {
  return {
    subscribe: observer => subscribeDecoded(receive => wire.subscribe(receive), observer, decodeSettingsEnvelope),
    read: async () => decodeSettingsEnvelope(await wire.snapshot()),
    async dispatch(command) {
      await execute(command);
      // Legacy replies have no publication sequence. Never attach a newer sequence
      // to an old reply: read a fresh, internally consistent envelope instead.
      return decodeSettingsEnvelope(await wire.snapshot());
    },
  };
}

async function executeSettings(command: SettingsCommand): Promise<void> {
  switch (command.type) {
    case "colorMode": await Settings.setColorMode(command.value); break;
    case "statusBar": await Settings.setShowStatusBar(command.value); break;
    case "sidebarDocked": await Settings.setSidebarDocked(command.value); break;
    case "opacity": await Settings.setBackgroundImageOpacity(command.value); break;
    case "transparency": await Settings.setContentBackgroundTransparency(command.value); break;
    case "componentFamily": await Settings.setComponentFamily(command.value); break;
    case "theme": await Settings.setTheme(command.value); break;
    case "locale": await Settings.setLocale(command.value); break;
    case "diagnosticOutput": await Settings.setDiagnosticOutput(command.output, command.value); break;
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
}

/** Non-state host operations remain separate from subscription and ordering. */
export const settingsEffects = {
  appKitThemeAvailable: () => Settings.appKitThemeAvailable(),
  openThemeDirectory: () => Settings.openThemeDirectory(),
};

