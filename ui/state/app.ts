/** Composes business modules over one settings owner and one borrowed window session. */
import { createAppearanceModule } from "./appearance/index";
import { createBackgroundModule } from "./background/index";
import { createDiagnosticsModule } from "./diagnostics/index";
import { createPreferencesModule } from "./preferences/index";
import { createSettingsOperations } from "./settings/operations";
import { initialSettings } from "./settings/initial";
import { createWindowState } from "./window/index";
import sessionConfig from "../config/session.json";
import { createSettingsOwner } from "./settings/store";
import type { ComponentFamily } from "../theme/index";
export type { ColorMode } from "./settings/contracts";
export { diagnosticLevels } from "./diagnostics/index";
export type { DiagnosticLevel, DiagnosticOutput } from "./settings/contracts";
export type SettingsSection = "general" | "theme" | "diagnostics" | "updates";
export const settingsSections = sessionConfig.sections as readonly SettingsSection[];
export function isSettingsSection(value: string): value is SettingsSection {
  return (settingsSections as readonly string[]).includes(value);
}
export function isComponentFamily(value: string): value is ComponentFamily {
  return value === "nsh" || value === "appkit";
}

/** Create once in the window root; descendants borrow modules and never own their subscriptions. */
export function createAppState(window = createWindowState()) {
  const session = window.session;
  const owner = createSettingsOwner(initialSettings());
  const settings = owner.view;
  const operations = createSettingsOperations(settings, session, owner.connect);
  const appearance = createAppearanceModule(settings, session, operations);
  const background = createBackgroundModule(settings, session, operations);
  const preferences = createPreferencesModule(settings, operations);
  const diagnostics = createDiagnosticsModule(settings, session, operations);
  // These modules expose accessor functions, not eagerly evaluated reactive props.
  return {
    window, appearance, background, preferences, diagnostics,
    settings: {
      view: settings,
      settingsSection: () => session.value().section as SettingsSection,
      setSettingsSection: (value: SettingsSection) => void session.change({ kind: "section", value }),
      status: operations.status,
      statusKey: operations.statusKey,
      busy: operations.busy,
      retryable: operations.retryable,
      retry: operations.retry,
    },
  };
}
export type AppState = ReturnType<typeof createAppState>;
