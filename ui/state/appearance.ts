/** Projects shared Rust appearance settings and coordinates serialized UI requests. */
import { batch, cleanup, colorScheme, computed, effect, locale, Picker, setColorScheme, signal } from "nsh";
import { Settings, type SettingsSnapshot } from "usher/nsh";
import { isLocalePreference, resolveLocale, translate, type LocalePreference, type MessageKey, type Translate } from "../i18n";
import { defaultThemeId, isModernMac, resolveTheme, type ComponentFamily, type CustomTheme, type StyleTheme } from "../theme";
import backgroundConfig from "../config/background.json";

export type ColorMode = "system" | "light" | "dark";
export const settingsSections = ["general", "theme", "diagnostics"] as const;
export type SettingsSection = (typeof settingsSections)[number];
export function isSettingsSection(value: string): value is SettingsSection {
  return (settingsSections as readonly string[]).includes(value);
}
export const diagnosticLevels = ["OFF", "ERROR", "WARN", "INFO", "DEBUG", "TRACE"] as const;
export type DiagnosticLevel = (typeof diagnosticLevels)[number];
const diagnosticErrorKeys: Record<string, true> = {
  "diagnostics.error.diagnostics_directory_unavailable": true,
  "diagnostics.error.diagnostics_export_empty": true,
  "diagnostics.error.diagnostics_export_failed": true,
  "diagnostics.error.diagnostics_export_invalid_range": true,
  "diagnostics.error.diagnostics_export_cancelled": true,
};
type Status =
  | "loading"
  | "saving"
  | "scanning"
  | "saved"
  | "load-error"
  | "save-error"
  | "scan-error"
  | "invalid"
  | "recovered"
  | "read-only";

const validationLabels: Partial<Record<string, MessageKey>> = {
  invalid_theme_name: "theme.invalidName",
  invalid_theme_color: "theme.invalidColor",
  invalid_theme_contrast: "theme.lowContrast",
  invalid_accent: "theme.invalidColor",
  unknown_theme: "theme.selectionUnavailable",
  style_theme_invalid_name: "theme.invalidName",
  style_theme_directory_unavailable: "theme.directoryUnavailable",
  style_theme_write_failed: "theme.fileWriteFailed",
  style_theme_id_exhausted: "theme.copyIdFailed",
  background_image_invalid: "background.invalidImage",
  background_image_too_large: "background.tooLarge",
  background_image_write_failed: "background.writeFailed",
  background_opacity_invalid: "background.invalidOpacity",
  content_background_transparency_invalid: "background.invalidContentTransparency",
};

function isColorMode(value: string): value is ColorMode {
  return value === "system" || value === "light" || value === "dark";
}

export function isComponentFamily(value: string): value is ComponentFamily {
  return value === "nsh" || value === "appkit";
}

export function createAppearanceStore() {
  const snapshot = signal<SettingsSnapshot>({
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
    diagnosticDropped: 0,
    customThemes: [],
    styleThemes: [],
    themeFileIssues: [],
    revision: 0,
    saved: true,
  });
  // Kept here, not in the page, so leaving Settings and coming back shows the same section.
  const settingsSection = signal<SettingsSection>("general");
  const status = signal<Status>("loading");
  const validationLabel = signal<MessageKey | undefined>();
  const directoryProblem = signal<MessageKey | undefined>();
  const openingDirectory = signal(false);
  const pickingBackground = signal(false);
  const backgroundProblem = signal<MessageKey | undefined>();
  const backgroundOpacityDraft = signal<number | undefined>();
  const contentTransparencyDraft = signal<number | undefined>();
  const backgroundImageId = computed(() => snapshot().backgroundImage?.id);
  const mode = () => snapshot().colorMode as ColorMode;
  const language = () => snapshot().localePreference as LocalePreference;
  const componentFamily = () => snapshot().componentFamily as ComponentFamily;
  const appKitAvailable = isModernMac() && Settings.appKitThemeAvailable();
  const busy = () => pickingBackground() || status() === "loading" || status() === "saving" || status() === "scanning";
  const retryable = () => status() === "load-error" || status() === "save-error" || status() === "scan-error";
  const selectedCustomTheme = () => snapshot().customThemes.find((theme) => theme.id === snapshot().themeId);
  const selectedStyleTheme = () => snapshot().styleThemes.find((theme) => theme.id === snapshot().themeId);
  const palette = computed(() =>
    resolveTheme(selectedStyleTheme(), colorScheme(), {
      accent: snapshot().accent,
      componentFamily: componentFamily(),
      appKitAvailable,
    }),
  );
  const t: Translate = (key, params) => translate(language(), locale(), key, params);
  // A theme names itself, per locale, in its own file; the catalog holds no theme names.
  const themeName = (theme: StyleTheme) => theme.names[resolveLocale(language(), locale())] ?? theme.name;
  const statusKey = (): MessageKey => {
    const current = status();
    return validationLabel() ?? (current === "invalid" ? "status.save-error" : `status.${current}`);
  };

  type Request = () => Promise<SettingsSnapshot>;
  type Stage = "loading" | "saving" | "scanning";
  const requestErrors = { loading: "load-error", saving: "save-error", scanning: "scan-error" } as const;
  let requestInFlight = false;
  let disposed = false;
  let lastRequest: { run: Request; stage: Stage } | undefined;

  cleanup(() => {
    disposed = true;
  });

  // Serialize bridge calls so an earlier load or save cannot replace a newer selection.
  async function sync(run: Request, stage: Stage = "saving"): Promise<boolean> {
    if (requestInFlight || disposed) return false;

    requestInFlight = true;
    lastRequest = { run, stage };
    validationLabel.set(undefined);
    status.set(stage);
    try {
      const next = await run();
      if (disposed) return false;
      if (
        !isColorMode(next.colorMode) ||
        !isLocalePreference(next.localePreference) ||
        !isComponentFamily(next.componentFamily)
      ) {
        throw new Error("Invalid settings snapshot");
      }

      // The native store is authoritative even when it could only apply the mode in memory.
      let nextStatus: Status = next.saved ? "saved" : "save-error";
      if (next.problem === "settings_invalid_recovered") {
        nextStatus = "recovered";
      } else if (
        next.problem === "settings_directory_unavailable" ||
        next.problem === "settings_read_failed" ||
        next.problem === "settings_backup_failed" ||
        next.problem === "settings_unsupported_version"
      ) {
        nextStatus = "read-only";
      }

      batch(() => {
        snapshot.set(next);
        status.set(nextStatus);
      });
      return true;
    } catch (cause) {
      if (!disposed) {
        const code = cause instanceof Error ? cause.message : String(cause);
        const label = validationLabels[code];
        batch(() => {
          validationLabel.set(label);
          status.set(label ? "invalid" : requestErrors[stage]);
        });
      }
      return false;
    } finally {
      requestInFlight = false;
    }
  }

  async function openThemeDirectory() {
    if (openingDirectory() || disposed) return;
    openingDirectory.set(true);
    directoryProblem.set(undefined);
    try {
      await Settings.openThemeDirectory();
    } catch (cause) {
      if (!disposed) {
        const code = cause instanceof Error ? cause.message : String(cause);
        directoryProblem.set(code === "style_theme_directory_unavailable"
          ? "theme.directoryUnavailable"
          : "theme.directoryOpenFailed");
      }
    } finally {
      if (!disposed) openingDirectory.set(false);
    }
  }

  async function chooseBackgroundImage() {
    if (pickingBackground() || busy() || disposed) return;
    pickingBackground.set(true);
    backgroundProblem.set(undefined);
    try {
      const [picked] = await Picker.pickImages({ maxDimension: backgroundConfig.pickerMaxDimension });
      if (!picked || disposed) return;
      if (picked.file.size > backgroundConfig.maxImageBytes) {
        backgroundProblem.set("background.tooLarge");
        return;
      }
      const bytes = await picked.file.bytes();
      if (!disposed) await sync(() => Settings.importBackgroundImage(picked.file.name, bytes));
    } catch {
      if (!disposed) backgroundProblem.set("background.pickFailed");
    } finally {
      if (!disposed) pickingBackground.set(false);
    }
  }

  async function setBackgroundImageOpacity(value: number) {
    backgroundOpacityDraft.set(value);
    try { return await sync(() => Settings.setBackgroundImageOpacity(value)); }
    finally { if (!disposed) backgroundOpacityDraft.set(undefined); }
  }

  async function setContentBackgroundTransparency(value: number) {
    contentTransparencyDraft.set(value);
    try { return await sync(() => Settings.setContentBackgroundTransparency(value)); }
    finally { if (!disposed) contentTransparencyDraft.set(undefined); }
  }

  // Loading also starts the diagnostic logger at the saved level. Starting it again for the
  // same folder only changes the filter, so remounts never create a second writer.
  effect(() => {
    void sync(() => Settings.startDiagnostics(), "loading");
  });

  // Export runs beside the settings requests: it never blocks or is blocked by a save.
  const exportState = signal<{ kind: "idle" | "running" | "done" | "error"; text?: MessageKey; name?: string }>({
    kind: "idle",
  });
  async function exportDiagnostics(days: number) {
    if (exportState().kind === "running") return;
    exportState.set({ kind: "running" });
    const end = new Date();
    const start = new Date(end.getTime() - (days - 1) * 86_400_000);
    const day = (date: Date) =>
      `${date.getFullYear()}-${String(date.getMonth() + 1).padStart(2, "0")}-${String(date.getDate()).padStart(2, "0")}`;
    try {
      const name = await Settings.exportDiagnostics(day(start), day(end));
      if (!disposed) exportState.set({ kind: "done", name });
    } catch (cause) {
      const code = cause instanceof Error ? cause.message : String(cause);
      const key = `diagnostics.error.${code}` as MessageKey;
      if (!disposed) {
        exportState.set({ kind: "error", text: key in diagnosticErrorKeys ? key : "diagnostics.error.diagnostics_export_failed" });
      }
    }
  }
  effect(() => setColorScheme(mode()));

  return {
    mode,
    language,
    componentFamily,
    appKitAvailable,
    themeId: () => snapshot().themeId,
    showStatusBar: () => snapshot().showStatusBar,
    sidebarDocked: () => snapshot().sidebarDocked,
    backgroundImage: () => snapshot().backgroundImage,
    backgroundImageId,
    backgroundImageOpacity: () => backgroundOpacityDraft() ?? snapshot().backgroundImageOpacity,
    previewBackgroundImageOpacity: (value: number) => backgroundOpacityDraft.set(value),
    setBackgroundImageOpacity,
    contentBackgroundTransparency: () => contentTransparencyDraft() ?? snapshot().contentBackgroundTransparency,
    previewContentBackgroundTransparency: (value: number) => contentTransparencyDraft.set(value),
    setContentBackgroundTransparency,
    chooseBackgroundImage,
    clearBackgroundImage: () => sync(() => Settings.clearBackgroundImage()),
    pickingBackground,
    backgroundProblem,
    reportBackgroundLoadFailure: (failed: boolean) => backgroundProblem.set(failed ? "background.loadFailed" : undefined),
    accent: () => snapshot().accent ?? null,
    customThemes: () => snapshot().customThemes,
    styleThemes: () => snapshot().styleThemes,
    themeFileIssues: () => snapshot().themeFileIssues,
    selectedCustomTheme,
    selectedStyleTheme,
    themeName,
    palette,
    t,
    status,
    statusKey,
    busy,
    retryable,
    directoryProblem,
    openingDirectory,
    choose: (next: ColorMode) => sync(() => Settings.setColorMode(next)),
    chooseTheme: (id: string) => sync(() => Settings.setTheme(id)),
    chooseComponentFamily: (family: ComponentFamily) => sync(() => Settings.setComponentFamily(family)),
    setShowStatusBar: (visible: boolean) => sync(() => Settings.setShowStatusBar(visible)),
    setSidebarDocked: (docked: boolean) => sync(() => Settings.setSidebarDocked(docked)),
    chooseLanguage: (preference: LocalePreference) => sync(() => Settings.setLocale(preference)),
    diagnosticLevel: () => snapshot().diagnosticLevel as DiagnosticLevel,
    diagnosticProblem: () => snapshot().diagnosticProblem ?? null,
    diagnosticDropped: () => snapshot().diagnosticDropped,
    chooseDiagnosticLevel: (level: DiagnosticLevel) => sync(() => Settings.setDiagnosticLevel(level)),
    exportState,
    exportDiagnostics,
    settingsSection,
    setAccent: (accent: string | null) => sync(() => Settings.setAccent(accent)),
    saveTheme: (theme: CustomTheme) => sync(() => Settings.saveCustomTheme(theme)),
    cloneStyleTheme: (id: string, name: string) => sync(() => Settings.cloneStyleTheme(id, name)),
    removeTheme: (id: string) => sync(() => Settings.removeCustomTheme(id)),
    openThemeDirectory,
    rescanThemes: () => sync(() => Settings.rescanThemes(), "scanning"),
    retry: () => lastRequest && sync(lastRequest.run, lastRequest.stage),
  };
}

export type AppearanceStore = ReturnType<typeof createAppearanceStore>;
