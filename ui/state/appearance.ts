/** Projects shared Rust appearance settings and coordinates serialized UI requests. */
import { cleanup, colorScheme, computed, effect, locale, Picker, setColorScheme } from "nsh";
import { createDiagnosticState } from "./diagnostics";
import { createWindowState, type WindowSession } from "./window";
import sessionConfig from "../config/session.json";
import { createSettingsOwner } from "./app";
import { settingsEffects } from "./transport";
import type { Immutable } from "./contracts";
import type { SettingsData } from "./settings";
import { isLocalePreference, resolveLocale, translate, type LocalePreference, type MessageKey, type Translate } from "../i18n";
import { defaultThemeId, isModernMac, resolveTheme, type ComponentFamily, type CustomTheme, type StyleTheme } from "../theme";
import backgroundConfig from "../config/background.json";

export type ColorMode = "system" | "light" | "dark";
export type SettingsSection = "general" | "theme" | "diagnostics";
export const settingsSections = sessionConfig.sections as readonly SettingsSection[];
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

export function createAppearanceStore(session: WindowSession = createWindowState().session) {
  const settingsOwner = createSettingsOwner({
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
  const diagnostics = createDiagnosticState();
  const settings = settingsOwner.view;
  const snapshot = settings.value;
  const settingsSection = () => session.value().section as SettingsSection;
  const operation = () => session.value().operations.settings;
  const status = (): Status => (operation()?.stage ?? "loading") as Status;
  const validationLabel = () => validationLabels[operation()?.error ?? ""];
  const directoryProblem = (): MessageKey | undefined => {
    const error = session.value().operations.directory?.error;
    return error ? error === "style_theme_directory_unavailable" ? "theme.directoryUnavailable" : "theme.directoryOpenFailed" : undefined;
  };
  const openingDirectory = () => session.value().operations.directory?.active === true;
  const pickingBackground = () => session.value().operations.picker?.active === true;
  const backgroundProblem = () => session.value().backgroundProblem as MessageKey | undefined;
  const backgroundOpacityDraft = () => session.value().opacityPreview ?? undefined;
  const contentTransparencyDraft = () => session.value().transparencyPreview ?? undefined;
  const backgroundImageId = computed(() => snapshot().backgroundImage?.id);
  const mode = () => snapshot().colorMode as ColorMode;
  const language = () => snapshot().localePreference as LocalePreference;
  const componentFamily = () => snapshot().componentFamily as ComponentFamily;
  const appKitAvailable = isModernMac() && settingsEffects.appKitThemeAvailable();
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

  type Request = () => Promise<Immutable<SettingsData>>;
  type Stage = "loading" | "saving" | "scanning";
  const requestErrors = { loading: "load-error", saving: "save-error", scanning: "scan-error" } as const;
  let requestInFlight = false;
  let disposed = false;
  let lastRequest: { run: Request; stage: Stage } | undefined;

  cleanup(() => {
    disposed = true;
  });

  function persistedStatus(next: Immutable<SettingsData>): Status {
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

    return nextStatus;
  }

  // This local flag serializes the effect executor; visible operation state lives in Rust.
  async function sync(run: Request, stage: Stage = "saving"): Promise<boolean> {
    if (requestInFlight || disposed) return false;
    requestInFlight = true;
    lastRequest = { run, stage };
    let ticket: string | undefined;
    try {
      ticket = await session.begin("settings", stage);
      const next = await run();
      if (disposed) return false;
      if (!isColorMode(next.colorMode) || !isLocalePreference(next.localePreference) || !isComponentFamily(next.componentFamily)) throw new Error("Invalid settings snapshot");
      await session.finish("settings", ticket, persistedStatus(next));
      return true;
    } catch (cause) {
      if (!disposed && ticket) {
        const code = cause instanceof Error ? cause.message : String(cause);
        await session.finish("settings", ticket, validationLabels[code] ? "invalid" : requestErrors[stage], code, null, true);
      }
      return false;
    } finally { requestInFlight = false; }
  }

  async function openThemeDirectory() {
    if (openingDirectory() || disposed) return;
    const ticket = await session.begin("directory", "running");
    try {
      await settingsEffects.openThemeDirectory();
      if (!disposed) await session.finish("directory", ticket, "done");
    } catch (cause) {
      if (!disposed) await session.finish("directory", ticket, "error", cause instanceof Error ? cause.message : String(cause), null, true);
    }
  }

  async function chooseBackgroundImage() {
    if (pickingBackground() || busy() || disposed) return;
    const ticket = await session.begin("picker", "running");
    await session.change({ kind: "background_problem", value: null });
    try {
      const [picked] = await Picker.pickImages({ maxDimension: backgroundConfig.pickerMaxDimension });
      if (!picked || disposed) return;
      if (picked.file.size > backgroundConfig.maxImageBytes) {
        await session.change({ kind: "background_problem", value: "background.tooLarge" });
        return;
      }
      const bytes = await picked.file.bytes();
      if (!disposed) await sync(() => settings.dispatch({ type: "importBackground", name: picked.file.name, bytes }));
    } catch {
      if (!disposed) await session.change({ kind: "background_problem", value: "background.pickFailed" });
    } finally { if (!disposed) await session.finish("picker", ticket, "idle"); }
  }

  async function setBackgroundImageOpacity(value: number) {
    const ticket = await session.preview(true, value);
    try { return await sync(() => settings.dispatch({ type: "opacity", value })); }
    finally { if (!disposed) await session.preview(true, null, ticket); }
  }

  async function setContentBackgroundTransparency(value: number) {
    const ticket = await session.preview(false, value);
    try { return await sync(() => settings.dispatch({ type: "transparency", value })); }
    finally { if (!disposed) await session.preview(false, null, ticket); }
  }

  effect(() => {
    const synchronization = settings.status();
    const incoming = snapshot();
    if (disposed) return;
    const stage = synchronization === "live" ? persistedStatus(incoming) : "load-error";
    if (synchronization !== "connecting" && synchronization !== "disposed") void session.change({ kind: "observe_settings", stage });
  });

  // The logger is idempotent; request feedback and the result are window-owned.
  effect(() => { void sync(() => settings.dispatch({ type: "startDiagnostics" }), "loading"); });

  const exportState = () => {
    const state = session.value().operations.export;
    const code = `diagnostics.error.${state?.error}` as MessageKey;
    return { kind: (state?.stage ?? "idle") as "idle" | "running" | "done" | "error",
      name: state?.value ?? undefined,
      text: state?.error ? code in diagnosticErrorKeys ? code : "diagnostics.error.diagnostics_export_failed" as MessageKey : undefined };
  };
  async function exportDiagnostics(days: number) {
    if (exportState().kind === "running" || disposed) return;
    const end = new Date();
    const start = new Date(end.getTime() - (days - 1) * 86_400_000);
    const day = (date: Date) => `${date.getFullYear()}-${String(date.getMonth() + 1).padStart(2, "0")}-${String(date.getDate()).padStart(2, "0")}`;
    await session.exportDiagnostics(day(start), day(end));
  }
  effect(() => setColorScheme(mode()));

  return {
    state: settings,
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
    previewBackgroundImageOpacity: (value: number) => void session.preview(true, value),
    setBackgroundImageOpacity,
    contentBackgroundTransparency: () => contentTransparencyDraft() ?? snapshot().contentBackgroundTransparency,
    previewContentBackgroundTransparency: (value: number) => void session.preview(false, value),
    setContentBackgroundTransparency,
    chooseBackgroundImage,
    clearBackgroundImage: () => sync(() => settings.dispatch({ type: "clearBackground" })),
    pickingBackground,
    backgroundProblem,
    reportBackgroundLoadFailure: (failed: boolean) => void session.change({ kind: "background_problem", value: failed ? "background.loadFailed" : null }),
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
    choose: (next: ColorMode) => sync(() => settings.dispatch({ type: "colorMode", value: next })),
    chooseTheme: (id: string) => sync(() => settings.dispatch({ type: "theme", value: id })),
    chooseComponentFamily: (family: ComponentFamily) => sync(() => settings.dispatch({ type: "componentFamily", value: family })),
    setShowStatusBar: (visible: boolean) => sync(() => settings.dispatch({ type: "statusBar", value: visible })),
    setSidebarDocked: (docked: boolean) => sync(() => settings.dispatch({ type: "sidebarDocked", value: docked })),
    chooseLanguage: (preference: LocalePreference) => sync(() => settings.dispatch({ type: "locale", value: preference })),
    diagnosticLevel: () => snapshot().diagnosticLevel as DiagnosticLevel,
    diagnosticProblem: () => diagnostics.value().problem ?? null,
    diagnosticDropped: () => Number(diagnostics.value().dropped),
    chooseDiagnosticLevel: (level: DiagnosticLevel) => sync(() => settings.dispatch({ type: "diagnosticLevel", value: level })),
    exportState,
    exportDiagnostics,
    settingsSection,
    setSettingsSection: (value: SettingsSection) => void session.change({ kind: "section", value }),
    setAccent: (accent: string | null) => sync(() => settings.dispatch({ type: "accent", value: accent })),
    saveTheme: (theme: CustomTheme) => sync(() => settings.dispatch({ type: "saveTheme", value: theme })),
    cloneStyleTheme: (id: string, name: string) => sync(() => settings.dispatch({ type: "cloneTheme", id, name })),
    removeTheme: (id: string) => sync(() => settings.dispatch({ type: "removeTheme", value: id })),
    openThemeDirectory,
    rescanThemes: () => sync(() => settings.dispatch({ type: "rescanThemes" }), "scanning"),
    retry: async () => {
      if (["error", "closed", "cancelled"].includes(settings.status())) await settingsOwner.connect();
      return lastRequest && sync(lastRequest.run, lastRequest.stage);
    },
  };
}

export type AppearanceStore = ReturnType<typeof createAppearanceStore>;
