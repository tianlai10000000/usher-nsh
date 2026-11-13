/** Checks production diagnostic controls and responsive layout on the isolated macOS test host. */
import { expect, open, test } from "nsh/test";

test("native diagnostics displays output controls, level selection and compact layout", async () => {
  const app = await open("../../App.tsx", { platform: "macos", width: 1100, height: 820, locale: "zh-CN", colorScheme: "light" });
  try {
    await app.press("id:route-settings");
    await app.press("id:settings-section-diagnostics");
    await app.waitUntil(() => app.enabled("id:diagnostic-output-file"));
    expect(app.enabled("id:diagnostic-output-terminal")).toBe(false);
    expect(app.enabled("id:diagnostic-output-memory")).toBe(false);
    expect(app.texts()).toContain("尚未接入");
    await app.press("id:diagnostic-output-structured");
    await app.waitUntil(() => app.enabled("id:diagnostic-level-picker"));
    await app.press("id:diagnostic-level-picker");
    await app.press("text:调试");
    await app.screenshot("target/test-reports/diagnostics-native-wide.png");
    await app.scroll("id:main-content", 500);
    await app.waitFor("id:diagnostic-export");
    await app.screenshot("target/test-reports/diagnostics-native-export.png");
    await app.setViewport(560, 820);
    await app.waitFor("id:settings-layout-compact");
    await app.scroll("id:main-content", -1000);
    await app.screenshot("target/test-reports/diagnostics-native-compact.png");
  } finally { app.dispose(); }
});

// A controlled view-boundary delay makes a transient busy frame observable on the real renderer.
test("native diagnostics keeps output appearance stable during a pending save", async () => {
  const app = await open("../ui/fixtures/DiagnosticsPending.tsx", { platform: "macos", width: 1100, height: 820, locale: "zh-CN", colorScheme: "light" });
  try {
    await app.waitUntil(() => app.enabled("id:diagnostic-output-console"));
    expect(app.shows("id:settings-status")).toBe(false);
    await app.screenshot("target/test-reports/diagnostics-switch-before.png");
    await app.press("id:diagnostic-output-console");
    await app.waitFor("text:Saving:pending");
    for (const output of ["file", "console", "structured"]) {
      expect(app.enabled(`id:diagnostic-output-${output}`)).toBe(false);
    }
    await app.screenshot("target/test-reports/diagnostics-switch-pending.png");
    await app.press("id:finish-save");
    await app.waitFor("text:Saving:ready");
    await app.waitUntil(() => app.enabled("id:diagnostic-output-console"));
    expect(app.texts()).toContain("Outputs:file=true,console=true,structured=false");
    await app.screenshot("target/test-reports/diagnostics-switch-after.png");
  } finally { app.dispose(); }
});
