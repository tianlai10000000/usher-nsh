/** Derives appearance views from the shared settings snapshot using NSH computed values. */
import { computed } from "nsh";
import type { StateView } from "../../state-runtime/contracts";
import type { ColorMode, SettingsCommand, SettingsData } from "../settings/contracts";
import { resolveTheme, defaultThemeId, type ComponentFamily } from "../../theme/index";
import { bindAppearanceHost, type AppearanceHost } from "./host";

/** Borrow the settings projection; this module creates no writable business state or owner. */
export function createAppearanceProjection(
  settings: StateView<SettingsData, SettingsCommand>,
  appKitAvailable: boolean,
  host?: AppearanceHost,
) {
  const snapshot = settings.value;
  const mode = () => snapshot().colorMode as ColorMode;
  const componentFamily = () => snapshot().componentFamily as ComponentFamily;
  const effectiveScheme = bindAppearanceHost(mode, settings.status, host);
  const selectedCustomTheme = () => snapshot().customThemes.find(theme => theme.id === snapshot().themeId);
  const selectedStyleTheme = () => snapshot().styleThemes.find(theme => theme.id === snapshot().themeId);
  const palette = computed(() => resolveTheme(selectedStyleTheme() ?? snapshot().styleThemes.find(theme => theme.id === defaultThemeId), effectiveScheme(), {
    componentFamily: componentFamily(),
    appKitAvailable,
  }));
  return { mode, componentFamily, effectiveScheme, selectedCustomTheme, selectedStyleTheme, palette };
}
