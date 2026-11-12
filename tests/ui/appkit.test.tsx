import { expect, open, test } from "nsh/test";

test("mounts AppKit settings over the shared preferences when available", async () => {
  const app = await open("../../App.tsx", { platform: "macos", width: 900, locale: "en-US", colorScheme: "light" });
  await app.setViewport(900, 800);
  await app.press("id:route-settings");
  await app.waitUntil(() => app.enabled("id:appearance-dark"));

  if (!app.shows("id:component-family-appkit")) {
    expect(app.shows("id:title-bar")).toBe(true);
    return;
  }

  await app.press("id:appearance-dark");
  await app.waitFor("text:Current appearance: Dark");
  await app.press("id:component-family-appkit");
  await app.waitFor("id:appearance-native");
  await app.waitFor("id:show-status-bar-native");
  await app.waitFor("id:locale-native");
  await app.waitFor("id:settings-section-native");
  await app.press("id:settings-section-theme").catch(() => {});
  expect(app.shows("id:title-bar")).toBe(true);
  expect(app.texts()).toContain("Current appearance: Dark");

  await app.press("id:nav-back");
  await app.waitFor("id:breadcrumb-current-home");
  await app.press("id:nav-forward");
  await app.waitFor("id:appearance-native");
});
