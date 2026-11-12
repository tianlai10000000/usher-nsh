import { expect, open, test } from "nsh/test";

test("shares navigation history across toolbar, keyboard, and breadcrumbs", async () => {
  const app = await open("../../App.tsx", { platform: "macos", width: 720, height: 740, locale: "en-US" });
  await app.waitFor("id:breadcrumb-current-home");
  expect(app.enabled("id:nav-back")).toBe(false);
  await app.waitUntil(() => app.enabled("id:sidebar-toggle"));
  await app.press("id:sidebar-toggle");
  await app.waitFor("id:application-sidebar", { state: "gone" });
  await app.press("id:sidebar-toggle");
  await app.waitFor("id:application-sidebar");
  await app.press("Click me");

  await app.press("id:route-settings");
  await app.waitFor("id:breadcrumb-current-settings");
  expect(app.shows("id:breadcrumb-home")).toBe(true);
  await app.press("id:nav-back");
  await app.waitFor("text:Clicks: 1");
  await app.keyboard.press("mod+]");
  await app.waitFor("id:breadcrumb-current-settings");
  await app.press("id:breadcrumb-home");
  await app.waitFor("id:breadcrumb-current-home");
  await app.press("id:nav-back");
  await app.waitFor("id:breadcrumb-current-settings");
  expect(app.enabled("id:nav-forward")).toBe(true);

  await app.press("id:route-home");
  expect(app.enabled("id:nav-forward")).toBe(false);
  await app.press("id:route-settings");
  await app.setViewport(380, 740);
  expect(app.shows("id:breadcrumb-home")).toBe(false);
  expect(app.shows("id:breadcrumb-current-settings")).toBe(true);
  expect(app.enabled("id:nav-back")).toBe(true);
  expect(app.shows("id:application-sidebar")).toBe(false);
  await app.press("id:sidebar-toggle");
  await app.waitFor("id:sidebar-drawer");
  await app.press("id:route-home");
  await app.waitFor("id:sidebar-drawer", { state: "gone" });
  await app.waitFor("text:Clicks: 1");
});

test("toggles the bottom status bar across pages", async () => {
  const app = await open("../../App.tsx", { platform: "macos", width: 900, height: 740, locale: "en-US" });
  await app.waitFor("id:bottom-status-bar");
  await app.press("id:route-settings");
  await app.waitUntil(() => app.enabled("id:show-status-bar"));
  await app.press("id:show-status-bar");
  await app.waitFor("id:bottom-status-bar", { state: "gone" });
  await app.press("id:route-home");
  expect(app.shows("id:bottom-status-bar")).toBe(false);

  await app.press("id:route-settings");
  await app.press("id:show-status-bar");
  await app.waitFor("id:bottom-status-bar");
  await app.waitFor("id:status-bar-version");
  await app.waitFor("id:status-bar-settings");
});
