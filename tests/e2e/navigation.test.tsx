/** Product journeys through the actual App, Rust router/window store and subscriptions. */
import { expect, open, test } from "nsh/test";
import options from "./fixtures/session.json";
const ready = (app: Awaited<ReturnType<typeof open>>) => () => app.shows("text:Saved") || app.shows("text:Settings are read-only. Changes apply for this session.");

test("e2e retained window data follows real menus history and page remounts", async () => {
  const app = await open("../../App.tsx", { ...options, platform: "macos", colorScheme: "light" });
  try {
    await app.waitUntil(ready(app));
    await app.press("Click me"); await app.waitFor("text:Clicks: 1");
    await app.press("id:route-settings"); await app.waitFor("id:settings-general");
    await app.press("id:settings-section-diagnostics"); await app.waitFor("id:diagnostics-settings");
    await app.press("id:diagnostic-export"); await app.waitFor("id:diagnostic-export-error");
    await app.press("id:nav-back"); await app.waitFor("text:Clicks: 1");
    await app.keyboard.press("mod+]"); await app.waitFor("id:diagnostics-settings");
    expect(app.texts()).toContain("The log folder is unavailable.");
    await app.press("id:route-home"); await app.waitFor("text:Clicks: 1");
    expect(app.enabled("id:nav-forward")).toBe(false);
  } finally { app.dispose(); }
});

test("e2e separately opened engines isolate history counters and settings sections", async () => {
  const first = await open("../../App.tsx", { ...options, platform: "macos", colorScheme: "light" });
  const second = await open("../../App.tsx", { ...options, platform: "macos", colorScheme: "light" });
  try {
    await first.waitUntil(ready(first)); await second.waitUntil(ready(second));
    await first.press("Click me"); await first.waitFor("text:Clicks: 1");
    await first.press("id:route-settings"); await first.press("id:settings-section-theme");
    await first.waitFor("id:theme-settings");
    expect(second.texts()).toContain("Clicks: 0");
    expect(second.enabled("id:nav-back")).toBe(false);
    await second.press("id:route-settings"); await second.waitFor("id:settings-general");
    expect(first.shows("id:theme-settings")).toBe(true);
  } finally { first.dispose(); second.dispose(); }
});

test("e2e host launch links native back and invalid external inputs use the product router", async () => {
  const app = await open("../../App.tsx", { ...options, platform: "macos", colorScheme: "light", urlScheme: "usher://app/settings" });
  try {
    await app.waitFor("id:settings-general");
    await app.back(); await app.waitFor("text:Clicks: 0");
    for (const url of ["usher://other/settings", "usher://app/%2e%2e", "usher://app/settings#fragment", "usher://app/settings?unexpected=1"]) {
      await app.openWithUrl(url);
      expect(app.texts()).toContain("Clicks: 0");
    }
    await app.openWithUrl("usher://app/unknown"); await app.waitFor("id:route-not-found");
    await app.back(); await app.waitFor("text:Clicks: 0");
    await app.back(); expect(app.enabled("id:nav-back")).toBe(false);
    expect(app.texts()).toContain("Clicks: 0");
  } finally { app.dispose(); }
});
