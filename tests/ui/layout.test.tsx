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
  await app.waitFor("id:rail-update");
  await app.waitFor("id:rail-settings");
  await app.waitFor("id:rail-account");
  // No update or account feature is connected yet, so those two buttons are inert.
  expect(app.enabled("id:rail-update")).toBe(false);
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

test("the title bar switches between light and dark", async () => {
  const app = await open("../../App.tsx", { platform: "macos", width: 900, height: 740, locale: "en-US", colorScheme: "light" });
  await app.waitUntil(() => app.enabled("id:titlebar-color-mode"));
  await app.press("id:rail-settings");
  await app.waitFor("text:Current appearance: Light");
  await app.press("id:titlebar-color-mode");
  await app.waitFor("text:Current appearance: Dark");
  await app.waitUntil(() => app.enabled("id:titlebar-color-mode"));
  await app.press("id:titlebar-color-mode");
  await app.waitFor("text:Current appearance: Light");
});
