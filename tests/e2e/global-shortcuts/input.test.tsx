/** Requires system input permission and verifies the OS hotkey path while the target app is backgrounded. */
import { expect, open, test } from "nsh/test";
import config from "./config.json";
test("global system input triggers startup settings and theme once while backgrounded", async () => {
  const app = await open("./Harness.tsx", { platform: "macos", locale: "en-US", colorScheme: "light", ...config.windowSize });
  try {
    await app.waitFor("text:Global:ready"); await app.waitFor("text:Connection:live"); await app.waitFor("text:Effective:light");
    expect(app.texts()).toContain("Events:0");
    await app.press("id:input-settings"); await app.waitFor("text:Path:/settings"); await app.waitFor("text:Events:1");
    expect(app.texts().some(text => text.startsWith("Input:") && text.includes('"wasBackground":true'))).toBe(true);
    await app.waitUntil(() => app.window("main").state()?.focused === true);
    await app.press("id:input-appearance"); await app.waitFor("text:Mode:dark"); await app.waitFor("text:Events:2");
    expect(app.texts()).toContain("Active:false");
    await app.waitFor("text:ThemeActive:false");
    await app.press("id:input-appearance"); await app.waitFor("text:Mode:light"); await app.waitFor("text:Events:3");
    await app.press("id:input-focus"); await app.waitFor("text:Events:4");
    await app.waitUntil(() => app.window("main").state()?.focused === true);
    await app.press("id:remount"); await app.waitFor("id:bottom-status-bar", { state: "gone" });
    await app.press("id:input-settings"); await app.waitFor("text:Events:5");
    await app.press("id:remount"); await app.waitFor("text:Path:/settings"); await app.waitFor("id:shortcut-status");
    expect(app.texts()).toContain("Events:5");
  } finally { app.dispose(); }
});
