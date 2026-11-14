/** Exercises module boundaries with deterministic host, transport and owner doubles. */
import { expect, test } from "nsh/test";
import { businessModules } from "./support/business-modules";
import { deferred } from "./support/controlled";
import { createBackgroundModule, type BackgroundPicker } from "../background/index";
import { createDiagnosticsModule } from "../diagnostics/index";
import { createAppearanceActions } from "../appearance/actions";
import { createPreferencesModule } from "../preferences/index";
import backgroundConfig from "../../config/background.json";

function diagnostics(mock: ReturnType<typeof businessModules>) {
  return createDiagnosticsModule(mock.settings, mock.session, mock.operations,
    { value: () => ({ dropped: "7", problem: "mock-health" }) },
    { onDispose: mock.lifecycle.onDispose, observe: () => {} },
    () => new Date(2026, 10, 14, 12));
}

test("business modules share one request lock and preserve retry after failure", async () => {
  const mock = businessModules();
  const gate = mock.delayDispatch();
  const appearance = createAppearanceActions(mock.settings, mock.session, mock.operations, async () => {});
  const logs = diagnostics(mock);
  const pending = appearance.choose("dark");
  expect(await logs.chooseDiagnosticLevel("DEBUG")).toBe(false);
  gate.resolve();
  expect(await pending).toBe(true);
  expect(mock.commands.length).toBe(1);
  expect(mock.commands[0].type).toBe("colorMode");
  let attempts = 0;
  expect(await mock.operations.sync(async () => {
    if (++attempts === 1) throw new Error("disk_unavailable");
    return mock.settings.value();
  })).toBe(false);
  expect(mock.operations.status()).toBe("save-error");
  mock.setSynchronization("error");
  expect(await mock.operations.retry()).toBe(true);
  expect(mock.reconnects()).toBe(1);
  expect(attempts).toBe(2);
  expect(mock.operations.status()).toBe("saved");
});

test("business modules suppress work and late receipts after owner disposal", async () => {
  const before = businessModules();
  const begin = before.delayBegin();
  let calls = 0;
  const pending = before.operations.sync(async () => { calls++; return before.settings.value(); });
  before.dispose(); begin.resolve();
  expect(await pending).toBe(false);
  expect(calls).toBe(0);
  expect(before.finishes.length).toBe(0);
  const after = businessModules();
  const response = deferred<ReturnType<typeof after.settings.value>>();
  const started = deferred<void>();
  const running = after.operations.sync(() => { started.resolve(); return response.promise; });
  await started.promise;
  after.dispose(); response.resolve(after.settings.value());
  expect(await running).toBe(false);
  expect(after.finishes.length).toBe(0);
  expect(await after.operations.retry()).toBe(false);
});

test("business modules preserve read-only and validation feedback in window state", async () => {
  const mock = businessModules();
  mock.setData({ saved: false, problem: "settings_unsupported_version" }); mock.notify();
  expect(mock.operations.status()).toBe("read-only");
  await mock.operations.sync(async () => { throw new Error("invalid_accent"); });
  expect(mock.operations.status()).toBe("invalid");
  expect(mock.operations.statusKey()).toBe("theme.invalidColor");
  mock.setData({ saved: true, problem: null }); mock.notify();
  expect(mock.operations.status()).toBe("saved");
});

test("business modules background cancellation and oversized picks never import bytes", async () => {
  const mock = businessModules();
  let reads = 0;
  let picks = 0;
  const picker: BackgroundPicker = { pickImages: async () => {
    if (++picks === 1) return [];
    return [{ file: { name: "large.png", size: backgroundConfig.maxImageBytes + 1,
      bytes: async () => { reads++; return new Uint8Array(); } } }];
  } };
  const background = createBackgroundModule(mock.settings, mock.session, mock.operations, picker);
  await background.chooseBackgroundImage();
  expect(background.pickingBackground()).toBe(false);
  await background.chooseBackgroundImage();
  expect(background.backgroundProblem()).toBe("background.tooLarge");
  expect(reads).toBe(0);
  expect(mock.commands.length).toBe(0);
  expect(background.pickingBackground()).toBe(false);
});

test("business modules background import failure recovers and preview cleanup uses its ticket", async () => {
  const mock = businessModules();
  let fail = true;
  const background = createBackgroundModule(mock.settings, mock.session, mock.operations, {
    pickImages: async () => [{ file: { name: "image.png", size: 1, bytes: async () => {
      if (fail) throw new Error("read_failed");
      return new Uint8Array([1]);
    } } }],
  });
  await background.chooseBackgroundImage();
  expect(background.backgroundProblem()).toBe("background.pickFailed");
  fail = false;
  await background.chooseBackgroundImage();
  expect(mock.commands[0].type).toBe("importBackground");
  expect(background.backgroundProblem()).toBe(null);
  const gate = mock.delayDispatch();
  const pending = background.setBackgroundImageOpacity(30);
  await mock.session.preview(true, 80);
  gate.resolve(); await pending;
  expect(background.backgroundImageOpacity()).toBe(80);
  expect(mock.previews[mock.previews.length - 1].expected !== undefined).toBe(true);
});

test("business modules background ignores picker results arriving after disposal", async () => {
  const mock = businessModules();
  const result = deferred<Awaited<ReturnType<BackgroundPicker["pickImages"]>>>();
  const started = deferred<void>();
  let reads = 0;
  const background = createBackgroundModule(mock.settings, mock.session, mock.operations, {
    pickImages: () => { started.resolve(); return result.promise; },
  });
  const pending = background.chooseBackgroundImage();
  await started.promise;
  mock.dispose();
  result.resolve([{ file: { name: "late.png", size: 1, bytes: async () => { reads++; return new Uint8Array(); } } }]);
  await pending;
  expect(reads).toBe(0);
  expect(mock.commands.length).toBe(0);
  expect(mock.finishes.length).toBe(0);
});

test("business modules diagnostics health and export stay separate from appearance requests", async () => {
  const mock = businessModules();
  const logs = diagnostics(mock);
  const gate = mock.delayExport();
  expect(logs.diagnosticDropped()).toBe(7);
  expect(logs.diagnosticProblem()).toBe("mock-health");
  const pending = logs.exportDiagnostics(3);
  await logs.exportDiagnostics(3);
  expect(mock.exports).toEqual([["2026-11-12", "2026-11-14"]]);
  expect(logs.exportState().kind).toBe("running");
  expect(await mock.operations.sync(() => mock.settings.dispatch({ type: "colorMode", value: "dark" }))).toBe(true);
  gate.resolve(); await pending;
  expect(logs.exportState().kind).toBe("done");
  mock.sessionData.operations.export = { ticket: "failure", stage: "error", active: false, failed: true, error: "unknown" };
  expect(logs.exportState().text).toBe("diagnostics.error.diagnostics_export_failed");
  mock.dispose(); await logs.exportDiagnostics(3);
  expect(mock.exports.length).toBe(1);
});

test("business modules directory failures release the operation for retry", async () => {
  const mock = businessModules();
  let tries = 0;
  const appearance = createAppearanceActions(mock.settings, mock.session, mock.operations, async () => {
    if (++tries === 1) throw new Error("style_theme_directory_unavailable");
  });
  await appearance.openThemeDirectory();
  expect(appearance.directoryProblem()).toBe("theme.directoryUnavailable");
  expect(appearance.openingDirectory()).toBe(false);
  await appearance.openThemeDirectory();
  expect(appearance.directoryProblem()).toBe(undefined);
  expect(tries).toBe(2);
});

test("business modules picker admission prevents duplicates before session publication", async () => {
  const mock = businessModules();
  const gate = deferred<void>();
  let admissions = 0;
  let picks = 0;
  const session = {
    ...mock.session,
    begin: async (...args: Parameters<typeof mock.session.begin>) => {
      admissions++;
      await gate.promise;
      return mock.session.begin(...args);
    },
  };
  const background = createBackgroundModule(mock.settings, session, mock.operations, {
    pickImages: async () => { picks++; return []; },
  });
  const first = background.chooseBackgroundImage();
  await background.chooseBackgroundImage();
  expect(admissions).toBe(1);
  mock.dispose(); gate.resolve(); await first;
  expect(picks).toBe(0);
});

test("business modules background drops bytes and directory actions after disposal", async () => {
  const mock = businessModules();
  const bytes = deferred<Uint8Array>();
  const reading = deferred<void>();
  const background = createBackgroundModule(mock.settings, mock.session, mock.operations, {
    pickImages: async () => [{ file: { name: "late.png", size: 1, bytes: () => {
      reading.resolve(); return bytes.promise;
    } } }],
  });
  const pending = background.chooseBackgroundImage();
  await reading.promise;
  mock.dispose(); bytes.resolve(new Uint8Array([1])); await pending;
  expect(mock.commands.length).toBe(0);
  const directory = businessModules();
  const begin = directory.delayBegin();
  let opened = false;
  const actions = createAppearanceActions(directory.settings, directory.session, directory.operations, async () => { opened = true; });
  const opening = actions.openThemeDirectory();
  directory.dispose(); begin.resolve(); await opening;
  expect(opened).toBe(false);
  expect(directory.finishes.length).toBe(0);
});


test("business modules language accessors follow the shared snapshot and actions use the coordinator", async () => {
  const mock = businessModules();
  const preferences = createPreferencesModule(mock.settings, mock.operations);
  expect(preferences.language()).toBe("system");
  await preferences.chooseLanguage("zh-Hans");
  expect(mock.commands).toEqual([{ type: "locale", value: "zh-Hans" }]);
  mock.setData({ localePreference: "zh-Hans" });
  expect(preferences.language()).toBe("zh-Hans");
  mock.setData({ localePreference: "en" });
  expect(preferences.language()).toBe("en");
});

test("business modules preserve admission errors and release local action guards", async () => {
  const mock = businessModules();
  let fail = true;
  let effects = 0;
  const session = {
    ...mock.session,
    begin: async (...args: Parameters<typeof mock.session.begin>) => {
      if (fail) throw new Error("window_closed");
      return mock.session.begin(...args);
    },
  };
  const directory = createAppearanceActions(mock.settings, session, mock.operations, async () => { effects++; });
  const background = createBackgroundModule(mock.settings, session, mock.operations, {
    pickImages: async () => { effects++; return []; },
  });
  expect(await directory.openThemeDirectory().then(() => false, () => true)).toBe(true);
  expect(await background.chooseBackgroundImage().then(() => false, () => true)).toBe(true);
  expect(effects).toBe(0);
  fail = false;
  await directory.openThemeDirectory();
  await background.chooseBackgroundImage();
  expect(effects).toBe(2);
});
