/** Verifies responsive settings sections, native control selection and framework tab navigation. */
import { expect, open, test } from "nsh/test";

const ready = (app: Awaited<ReturnType<typeof open>>) => () =>
  app.shows("text:Saved") || app.shows("text:Settings are read-only. Changes apply for this session.");

test("wide windows list the sections on the left and show one at a time", async () => {
  const app = await open("../../App.tsx", { platform: "macos", width: 1100, height: 800, locale: "en-US" });
  await app.setViewport(1100, 800);
  await app.press("id:route-settings");
  await app.waitUntil(ready(app));
  await app.waitFor("id:settings-layout-wide");
  expect(app.shows("id:settings-layout-compact")).toBe(false);

  // General is first; the others are not on screen yet.
  await app.waitFor("id:settings-general");
  expect(app.shows("id:diagnostics-settings")).toBe(false);

  await app.press("id:settings-section-diagnostics");
  await app.waitFor("id:diagnostics-settings");
  expect(app.shows("id:settings-general")).toBe(false);

  await app.press("id:settings-section-general");
  await app.waitFor("id:settings-general");
  expect(app.shows("id:diagnostics-settings")).toBe(false);
});

test("narrow windows replace the list with one drop-down that still switches sections", async () => {
  const app = await open("../../App.tsx", { platform: "macos", width: 560, height: 800, locale: "en-US" });
  await app.setViewport(560, 800);
  // Below the wide breakpoint the menu is a drawer, so the page has the whole window.
  await app.press("id:rail-settings");
  await app.waitUntil(ready(app));
  await app.waitFor("id:settings-layout-compact");
  expect(app.shows("id:settings-layout-wide")).toBe(false);
  expect(app.shows("id:settings-navigation")).toBe(false);

  await app.waitFor("id:settings-section-picker");
  await app.press("id:settings-section-picker");
  await app.press("text:Diagnostics");
  await app.waitFor("id:diagnostics-settings");
  expect(app.shows("id:settings-general")).toBe(false);
});

test("settings tabs use framework keyboard selection and one Tab stop", async () => {
  const app = await open("./fixtures/SettingsKeyboardPreview.tsx", {
    platform: "macos", width: 1100, height: 800, locale: "en-US",
  });
  await app.waitUntil(ready(app));
  await app.waitFor("id:settings-tabs");
  await app.keyboard.press("tab");
  await app.keyboard.press("down");
  await app.waitFor("id:theme-settings");
  await app.keyboard.press("end");
  await app.waitFor("id:settings-updates");
  await app.keyboard.press("home");
  await app.waitFor("id:settings-general");
  await app.keyboard.press("up");
  await app.waitFor("id:settings-updates");
  await app.keyboard.press("tab");
  await app.keyboard.press("down");
  expect(app.shows("id:settings-updates")).toBe(true);
  await app.keyboard.press("shift+tab");
  await app.keyboard.press("up");
  await app.waitFor("id:diagnostics-settings");
});

test("the layout follows the window when it is resized, and the section is kept", async () => {
  const app = await open("../../App.tsx", { platform: "macos", width: 1100, height: 800, locale: "en-US" });
  await app.setViewport(1100, 800);
  await app.press("id:route-settings");
  await app.waitUntil(ready(app));
  await app.press("id:settings-section-diagnostics");
  await app.waitFor("id:diagnostics-settings");

  await app.setViewport(560, 800);
  await app.waitFor("id:settings-layout-compact");
  await app.waitFor("id:diagnostics-settings");

  await app.setViewport(1100, 800);
  await app.waitFor("id:settings-layout-wide");
  await app.waitFor("id:diagnostics-settings");
});

test("the section survives leaving Settings and coming back", async () => {
  const app = await open("../../App.tsx", { platform: "macos", width: 1100, height: 800, locale: "en-US" });
  await app.setViewport(1100, 800);
  await app.press("id:route-settings");
  await app.waitUntil(ready(app));
  await app.press("id:settings-section-theme");
  await app.waitFor("id:theme-native").catch(() => {});
  await app.press("id:route-home");
  await app.press("id:route-settings");
  await app.waitFor("id:settings-layout-wide");
  expect(app.shows("id:settings-general")).toBe(false);
});

test("the liquid glass family uses AppKit's own controls for the sections", async () => {
  const app = await open("../../App.tsx", { platform: "macos", width: 1100, height: 800, locale: "en-US" });
  await app.setViewport(1100, 800);
  await app.press("id:route-settings");
  await app.waitUntil(() => app.enabled("id:appearance-dark"));
  if (!app.shows("id:component-family-appkit")) return; // AppKit controls need macOS 26+.

  await app.press("id:component-family-appkit");
  await app.waitFor("id:settings-section-native");
  expect(app.shows("id:settings-section-diagnostics")).toBe(false);

  await app.setViewport(560, 800);
  await app.waitFor("id:settings-section-native-picker");
  expect(app.shows("id:settings-section-picker")).toBe(false);
});
