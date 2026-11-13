/** Creates one settings projection for its application owner and lends it to descendants. */
import type { StateView } from "./contracts";
import type { SettingsCommand, SettingsData } from "./settings";
import { createProjection } from "./projection";
import { createSettingsTransport } from "./transport";
import { createScope, nshRuntime, onDispose } from "./runtime/nsh";

const settingsScope = createScope<StateView<SettingsData, SettingsCommand>>();
export function createSettingsOwner(initial: SettingsData) {
  const owner = createProjection(initial, createSettingsTransport(), nshRuntime);
  onDispose(() => owner.dispose());
  void owner.connect();
  return owner;
}
export function createSettingsState(initial: SettingsData) { return createSettingsOwner(initial).view; }
export const useSettingsStore = () => settingsScope.use();

export const SettingsProvider = settingsScope.Provider;
