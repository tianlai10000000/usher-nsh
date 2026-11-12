import { expect, open, test } from "nsh/test";

test("shares the appearance across Home and Settings", async () => {
  const app = await open("../../App.tsx", { platform: "macos", width: 900, colorScheme: "light", locale: "en-US" });
  // Native hosts save to disk. Headless surfaces may have session-only settings.
  const ready = () =>
    app.shows("text:Saved") ||
    app.shows("text:Settings are read-only. Changes apply for this session.");
  await app.waitFor("text:Current appearance: Light");
  await app.press("Click me");
  await app.waitFor("text:Clicks: 1");

  await app.press("id:route-settings");
  await app.waitUntil(ready);
  await app.press("id:appearance-dark");
  await app.waitUntil(ready);
  await app.waitFor("text:Current appearance: Dark");

  await app.press("id:route-home");
  await app.waitFor("text:Current appearance: Dark");
  expect(app.texts()).toContain("Clicks: 1");

  await app.press("id:route-settings");
  await app.press("id:appearance-system");
  await app.waitUntil(ready);
  await app.press("id:route-home");
  await app.waitFor("text:Current appearance: Light");
});

test("applies themes and language changes without resetting Home", async () => {
  const app = await open("../../App.tsx", { platform: "macos", width: 900, colorScheme: "light", locale: "en-US" });
  await app.press("Click me");
  await app.press("id:route-settings");
  await app.press("id:settings-section-theme");
  await app.waitUntil(() => app.enabled("id:theme-apple-glass"));
  await app.press("id:theme-apple-glass");
  await app.waitFor("id:glass-status");
  // The floating toolbar must remain interactive above the scrolling content.
  await app.press("id:route-home");
  expect(app.texts()).toContain("Clicks: 1");
  await app.press("id:route-settings");
  await app.waitFor("id:glass-status");

  await app.press("id:settings-section-theme");
  await app.type("id:accent-input", "#B42318");
  await app.press("id:accent-apply");
  await app.waitUntil(() => app.enabled("id:theme-add"));
  await app.press("id:theme-add");
  await app.type("id:theme-name", "Studio");
  await app.press("id:palette-dark");
  await app.press("id:theme-save");
  await app.waitFor("text:Studio");
  await app.waitFor("id:theme-edit");
  expect(app.shows("id:glass-status")).toBe(false);

  await app.press("id:settings-section-general");
  await app.press("id:locale-zh-Hans");
  await app.waitFor("text:设置");
  await app.press("id:route-home");
  await app.waitFor("text:你好，世界！");
  expect(app.texts()).toContain("点击次数：1");

  await app.press("id:route-settings");
  await app.press("id:settings-section-general");
  await app.press("id:locale-en");
  await app.waitFor("text:Settings");
  await app.press("id:settings-section-theme");
  await app.press("id:theme-remove");
  await app.waitUntil(() => !app.shows("text:Studio"));
});
