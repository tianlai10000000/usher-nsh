/** Projects language preferences and localized names without owning a second settings copy. */
import { locale } from "nsh";
import { resolveLocale, translate, type LocalePreference, type Translate } from "../../i18n/index";
import type { StyleTheme } from "../../theme/index";
import type { SettingsView, SettingsOperations } from "../settings/operations";
export function createPreferencesModule(settings: SettingsView, operations: Pick<SettingsOperations, "sync">) {
  const language = () => settings.value().localePreference as LocalePreference;
  const t: Translate = (key, params) => translate(language(), locale(), key, params);
  return {
    language, t,
    showStatusBar: () => settings.value().showStatusBar,
    sidebarDocked: () => settings.value().sidebarDocked,
    setShowStatusBar: (value: boolean) => operations.sync(() => settings.dispatch({ type: "statusBar", value })),
    setSidebarDocked: (value: boolean) => operations.sync(() => settings.dispatch({ type: "sidebarDocked", value })),
    themeName: (theme: StyleTheme) => theme.names[resolveLocale(language(), locale())] ?? theme.name,
    chooseLanguage: (value: LocalePreference) => operations.sync(() => settings.dispatch({ type: "locale", value })),
  };
}
