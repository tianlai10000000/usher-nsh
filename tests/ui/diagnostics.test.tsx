import { expect, open, test } from "nsh/test";

// Headless surfaces have no data directory: settings live in the session only and the
// logger has no folder to write to. The page must still work and report that plainly.
test("changes the log level and reports an unavailable log folder on export", async () => {
  const app = await open("../../App.tsx", { platform: "macos", width: 900, colorScheme: "light", locale: "en-US" });
  const ready = () =>
    app.shows("text:Saved") ||
    app.shows("text:Settings are read-only. Changes apply for this session.");

  await app.press("id:route-settings");
  await app.waitUntil(ready);
  await app.press("id:settings-section-diagnostics");
  await app.waitFor("id:diagnostics-settings");
  await app.waitUntil(() => app.enabled("id:diagnostic-level-DEBUG"));

  await app.press("id:diagnostic-level-DEBUG");
  await app.waitUntil(ready);
  await app.press("id:diagnostic-level-OFF");
  await app.waitUntil(ready);

  await app.press("id:diagnostic-range-today");
  await app.press("id:diagnostic-export");
  await app.waitFor("id:diagnostic-export-error");
  expect(app.texts()).toContain("The log folder is unavailable.");
  expect(app.shows("id:diagnostic-export-done")).toBe(false);
});

test("shows the diagnostics card in Chinese", async () => {
  const app = await open("../../App.tsx", { platform: "macos", width: 900, colorScheme: "light", locale: "zh-CN" });
  await app.press("id:route-settings");
  await app.press("id:settings-section-diagnostics");
  await app.waitFor("id:diagnostics-settings");
  expect(app.texts()).toContain("日志级别");
  expect(app.texts()).toContain("导出日志");
});
