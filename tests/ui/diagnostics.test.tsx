/** Exercises production diagnostic controls, availability and export feedback. */
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
  await app.waitUntil(() => app.enabled("id:diagnostic-level-picker"));

  await app.press("id:diagnostic-level-picker");
  await app.press("text:Debug");
  await app.waitUntil(ready);
  await app.press("id:diagnostic-level-picker");
  await app.press("text:Off");
  await app.waitUntil(ready);

  await app.scroll("id:main-content", 500);
  await app.press("id:diagnostic-range-picker");
  await app.press("text:Today");
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


test("diagnostics exposes connected outputs and marks unavailable collectors", async () => {
  const app = await open("../../App.tsx", { platform: "macos", width: 1200, height: 1000, locale: "en-US" });
  await app.press("id:route-settings");
  await app.press("id:settings-section-diagnostics");
  await app.waitUntil(() => app.enabled("id:diagnostic-output-file"));
  const texts = app.texts();
  expect(app.shows("id:settings-status")).toBe(false);
  expect(texts.indexOf("Updates") < texts.indexOf("Diagnostics")).toBe(true);
  for (const output of ["console", "file", "structured"]) {
    await app.press(`id:diagnostic-output-${output}`);
    await app.waitUntil(() => app.enabled(`id:diagnostic-output-${output}`));
  }
  expect(app.enabled("id:diagnostic-output-terminal")).toBe(false);
  expect(app.enabled("id:diagnostic-output-memory")).toBe(false);
  expect(app.texts()).toContain("Not connected yet");
});

// Hold the visual busy state explicitly; persistence still uses the real settings facade.
test("diagnostics protects a pending save without showing a header status", async () => {
  const app = await open("./fixtures/DiagnosticsPending.tsx", { platform: "macos", width: 1100, height: 820, locale: "en-US" });
  await app.waitUntil(() => app.enabled("id:diagnostic-output-console"));
  await app.press("id:diagnostic-output-console");
  await app.waitFor("text:Saving:pending");
  for (const output of ["file", "console", "structured"]) {
    expect(app.enabled(`id:diagnostic-output-${output}`)).toBe(false);
  }
  expect(app.shows("id:settings-status")).toBe(false);
  await app.press("id:finish-save");
  await app.waitFor("text:Saving:ready");
  await app.waitUntil(() => app.enabled("id:diagnostic-output-console"));
  expect(app.texts()).toContain("Outputs:file=true,console=true,structured=false");
});
