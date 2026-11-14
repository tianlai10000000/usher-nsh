/** Verifies persistent rail navigation and conditional update visibility. */
import { expect, open, test } from "nsh/test";

test("the rail stays while the secondary menu toggles", async () => {
  const app = await open("../../App.tsx", { platform: "macos", width: 900, height: 740, locale: "en-US" });
  await app.waitFor("id:application-rail");
  await app.waitFor("id:application-sidebar");

  await app.waitUntil(() => app.enabled("id:sidebar-toggle"));
  await app.press("id:sidebar-toggle");
  await app.waitFor("id:application-sidebar", { state: "gone" });
  expect(app.shows("id:application-rail")).toBe(true);

  await app.press("id:sidebar-toggle");
  await app.waitFor("id:application-sidebar");
  expect(app.shows("id:application-rail")).toBe(true);
});

test("the rail bottom holds the update, settings and account buttons", async () => {
  const app = await open("../../App.tsx", { platform: "macos", width: 900, height: 740, locale: "en-US" });
  await app.waitFor("id:rail-settings");
  await app.waitFor("id:rail-account");
  // The development shell has no updater capability; no update entry should appear.
  expect(app.shows("id:rail-update")).toBe(false);
  expect(app.enabled("id:rail-account")).toBe(false);
  expect(app.shows("id:rail-update-badge")).toBe(false);

  await app.press("id:rail-settings");
  await app.waitFor("id:breadcrumb-current-settings");
});

test("the title bar carries the breadcrumbs and no settings button", async () => {
  const app = await open("../../App.tsx", { platform: "macos", width: 900, height: 740, locale: "en-US" });
  await app.waitFor("id:title-bar");
  await app.waitFor("id:breadcrumb-current-home");
  expect(app.shows("id:titlebar-settings")).toBe(false);
  await app.press("id:rail-settings");
  await app.waitFor("id:breadcrumb-current-settings");
  await app.waitFor("id:breadcrumb-home");
  await app.press("id:breadcrumb-home");
  await app.waitFor("id:breadcrumb-current-home");
});

test("the status bar spans the window below the menus and can be hidden", async () => {
  const app = await open("../../App.tsx", { platform: "macos", width: 900, height: 740, locale: "en-US" });
  await app.waitFor("id:bottom-status-bar");
  // The rail and the menu stay above it instead of running down beside it.
  expect(app.shows("id:application-rail")).toBe(true);
  await app.press("id:rail-settings");
  await app.waitUntil(() => app.enabled("id:show-status-bar"));
  await app.press("id:show-status-bar");
  await app.waitFor("id:bottom-status-bar", { state: "gone" });
  expect(app.shows("id:application-rail")).toBe(true);
});

test("the title bar cycles saved appearance preferences and follows settings changes", async () => {
  const app = await open("../../App.tsx", { platform: "macos", width: 900, height: 740, locale: "en-US", colorScheme: "light" });
  await app.waitUntil(() => app.enabled("id:titlebar-color-mode"));
  await app.press("id:rail-settings");
  await app.waitUntil(() => app.enabled("id:appearance-system"));
  const labels = app.texts().filter((text) => ["System", "Light", "Dark"].includes(text));
  expect(labels.slice(0, 3)).toEqual(["System", "Light", "Dark"]);
  await app.press("id:appearance-system");
  for (const [action, scheme] of [
    ["Switch to light mode", "Light"],
    ["Switch to dark mode", "Dark"],
    ["Switch to system appearance", "Light"],
    ["Switch to light mode", "Light"],
  ]) {
    await app.waitUntil(() => app.enabled(`label:${action}`));
    await app.press(`label:${action}`);
    await app.waitFor(`text:Current appearance: ${scheme}`);
  }
  await app.waitUntil(() => app.enabled("id:appearance-dark"));
  await app.press("id:appearance-dark");
  await app.waitUntil(() => app.enabled("label:Switch to system appearance"));
  await app.press("label:Switch to system appearance");
  await app.waitFor("text:Current appearance: Light");
  await app.waitUntil(() => app.enabled("label:Switch to light mode"));
});
