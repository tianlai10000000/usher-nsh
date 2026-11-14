/** Composes appearance selectors and actions over the borrowed settings transaction. */
import { createAppearanceProjection } from "./projection";
import { createAppearanceActions } from "./actions";
import { isModernMac } from "../../theme";
import { settingsEffects } from "../settings/transport";
import type { SettingsView, SettingsOperations } from "../settings/operations";
import type { WindowSession } from "../window";
export { createAppearanceProjection } from "./projection";
export type { AppearanceHost } from "./host";
export function createAppearanceModule(settings: SettingsView, session: WindowSession, operations: SettingsOperations) {
  const snapshot = settings.value;
  const appKitAvailable = isModernMac() && settingsEffects.appKitThemeAvailable();
  return {
    ...createAppearanceProjection(settings, appKitAvailable),
    ...createAppearanceActions(settings, session, operations, settingsEffects.openThemeDirectory),
    appKitAvailable,
    themeId: () => snapshot().themeId,
    accent: () => snapshot().accent ?? null,
    customThemes: () => snapshot().customThemes,
    styleThemes: () => snapshot().styleThemes,
    themeFileIssues: () => snapshot().themeFileIssues,
  };
}
