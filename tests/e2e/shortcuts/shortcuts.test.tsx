/** Exercises the production shortcut service and keymap with an isolated lazy page factory. */
import { expect, open, test } from "nsh/test";
import config from "./config.json";
test("shortcut host lazily loads pages, preserves registrations across status visibility and fences remount", async () => {
  const app = await open("./Harness.tsx", { platform: "macos", locale: "en-US", ...config.windowSize });
  try {
    await app.waitFor("text:Startup:ready"); await app.waitFor("text:Connection:live");
    await app.press("id:read-loads"); const initial = Number(app.texts().find(t => t.startsWith("Loads:"))!.slice(6));
    await app.press("id:shortcut-status"); await app.waitFor("id:shortcut-details");
    expect(app.texts().some(t => t.includes("fixture-page: Not loaded"))).toBe(true);
    await app.keyboard.press("escape"); await app.press("id:read-loads"); expect(app.texts()).toContain(`Loads:${initial}`);
    await app.press("id:toggle-status"); await app.waitFor("id:bottom-status-bar", { state: "gone" });
    await app.press("id:go-settings"); await app.waitFor("text:Page:ready");
    await app.press("id:read-loads"); expect(app.texts()).toContain(`Loads:${initial + 1}`);
    await app.keyboard.press(config.payload.bindings[0].keystroke); await app.waitFor("text:Path:/");
    await app.press("id:go-settings"); await app.waitFor("text:Page:ready");
    await app.press("id:read-loads"); expect(app.texts()).toContain(`Loads:${initial + 1}`);
    await app.press("id:block"); await app.keyboard.press(config.payload.bindings[0].keystroke); expect(app.texts()).toContain("Path:/settings");
    await app.press("id:block"); await app.keyboard.press("mod+["); await app.waitFor("text:Path:/");
    await app.press("id:go-settings"); await app.waitFor("text:Page:ready"); await app.type("id:editor", "editing"); await app.keyboard.press(config.payload.bindings[0].keystroke); expect(app.texts()).toContain("Path:/settings");
    await app.press("id:toggle-status"); await app.waitFor("id:shortcut-status");
    await app.press("id:remount"); await app.waitFor("text:Startup:ready", { state: "gone" });
    await app.press("id:remount"); await app.waitFor("text:Startup:ready"); await app.waitFor("text:Connection:live");
    await app.press("id:go-settings"); await app.waitFor("text:Page:ready");
    await app.keyboard.press(config.payload.bindings[0].keystroke); await app.waitFor("text:Path:/");
  } finally { app.dispose(); }
});
