/** Checks read-only health projection with a fake service, independently of layout ownership. */
import { expect, test } from "nsh/test";
import { combineHealth } from "../combined-health";
import { shortcutIndicator } from "../status";
import type { Health } from "../contracts";
import type { Shortcuts } from "../runtime/nsh";
import type { Translate } from "../../i18n";
test("shortcut status keeps startup and page failure separate and delegates retry", async () => {
  const health: Health = { startup: "ready", page: "failed", global: "notRequested", entry: "entry", modules: [], bindings: [] };
  let connection = "live"; let retries = 0;
  const source = { value: () => ({ ...health, window: health.startup }), connection: () => connection, retry: async () => { retries++; } } as unknown as Shortcuts;
  const translate: Translate = (key, values) => `${key}:${values?.status ?? ""}`;
  const status = shortcutIndicator(source, translate);
  expect(status.summary()).toContain("shortcuts.ready");
  expect(status.pageSummary()).toContain("shortcuts.failed");
  expect(status.warning()).toBe(true); expect(status.retryable()).toBe(true);
  await status.retry(); expect(retries).toBe(1);
  connection = "error";
  expect(status.summary()).toContain("shortcuts.disconnected");
  expect(status.pageSummary()).toContain("shortcuts.disconnected");
  expect(status.details().every(row => row.value.includes("shortcuts.disconnected"))).toBe(true);
});
test("shortcut status inspection does not initialize unloaded pages", () => {
  const health: Health = { startup: "ready", page: "notRequested", global: "notRequested", entry: null,
    modules: [{ id: "fixture", kind: "page", page: "settings", phase: "notLoaded", error: null }], bindings: [] };
  const source = { value: () => ({ ...health, window: health.startup }), connection: () => "live" } as unknown as Shortcuts;
  const status = shortcutIndicator(source, key => key);
  expect(status.details().find(row => row.id === "module-fixture")?.value).toBe("shortcuts.notLoaded");
  expect(status.warning()).toBe(false); expect(status.retryable()).toBe(false);
});

test("shortcut status global health participates in startup without changing page readiness", () => {
  const local: Health = { startup: "ready", page: "ready", global: "notRequested", entry: "page", modules: [], bindings: [] };
  const global: Health = { ...local, page: "notRequested", global: "installing" };
  const pending = combineHealth(local, global);
  expect(pending.startup).toBe("installing"); expect(pending.page).toBe("ready");
  expect(combineHealth(local, { ...global, global: "failed" }).startup).toBe("failed");
  expect(combineHealth(local, { ...global, global: "ready" }).startup).toBe("ready");
});
test("shortcut status global failure must not label the healthy window channel as failed", () => {
  const local: Health = { startup: "ready", page: "notRequested", global: "notRequested", entry: null, modules: [], bindings: [] };
  const global: Health = { ...local, global: "failed" };
  const source = { value: () => combineHealth(local, global), connection: () => "live", retry: async () => {} } as unknown as Shortcuts;
  const status = shortcutIndicator(source, key => key);
  expect(status.details().find(row => row.id === "startup")?.value).toBe("shortcuts.ready");
  expect(status.details().find(row => row.id === "global")?.value).toBe("shortcuts.failed");
  expect(status.warning()).toBe(true); expect(status.retryable()).toBe(true);
});

test("shortcut status late readers see current startup stages without owning registration", () => {
  const local: Health = { startup: "ready", page: "notRequested", global: "notRequested", entry: null, modules: [], bindings: [] };
  let global: Health = { ...local, global: "validating" };
  const source = { value: () => combineHealth(local, global), connection: () => "live" } as unknown as Shortcuts;
  const indicator = shortcutIndicator(source, key => key);
  expect(indicator.details().find(row => row.id === "global")?.value).toBe("shortcuts.validating");
  global = { ...global, global: "installing" };
  expect(indicator.details().find(row => row.id === "global")?.value).toBe("shortcuts.installing");
  global = { ...global, global: "ready" };
  const late = shortcutIndicator(source, key => key);
  expect(late.details().find(row => row.id === "global")?.value).toBe("shortcuts.ready");
  expect(indicator.retryable()).toBe(false);
  expect(late.details().find(row => row.id === "page")?.value).toBe("shortcuts.notRequested");
});
