/** Validates real OS allocation, component-independent ownership and VM cleanup in an isolated app. */
import { expect, open, test } from "nsh/test";
import config from "./config.json";
test("global native startup registers once across borrower remount and VM reload", async () => {
  const app = await open("./Harness.tsx", { platform: "macos", locale: "en-US", ...config.windowSize });
  try {
    await app.waitFor("text:Global:ready"); await app.waitFor("text:Connection:live");
    expect(app.texts()).not.toContain("Page shortcuts: Not configured");
    if (config.previewPath) await app.screenshot(config.previewPath);
    await app.press("id:probe"); await app.waitFor("text:Native available:false");
    await app.press("id:remount"); await app.waitFor("id:bottom-status-bar", { state: "gone" });
    await app.press("id:probe"); expect(app.texts()).toContain("Native available:false");
    await app.press("id:remount"); await app.waitFor("id:shortcut-status"); expect(app.texts()).toContain("Global:ready");
    await app.keyboard.press("mod+r"); await app.waitFor("text:Native available:unknown");
    await app.waitFor("text:Global:ready"); await app.press("id:probe"); await app.waitFor("text:Native available:false");
  } finally { app.dispose(); }
});
test("global native occupied startup reports failure and recovers after the blocker releases", async () => {
  const app = await open("./Rejected.tsx", { platform: "macos", locale: "en-US", ...config.windowSize });
  try {
    await app.waitFor("text:Global:failed");
    await app.press("id:shortcut-status");
    await app.waitFor("text:Window shortcuts: Ready");
    await app.waitFor("text:Global shortcuts: Unavailable");
    await app.waitFor("id:shortcut-retry");
    await app.keyboard.press("escape");
    await app.press("id:release"); await app.press("id:probe"); await app.waitFor("text:Native available:true");
    await app.press("id:shortcut-status"); await app.press("id:shortcut-retry"); await app.waitFor("text:Global:ready");
    await app.waitFor("text:Global shortcuts: Ready");
    await app.waitFor("id:shortcut-retry", { state: "gone" });
    await app.keyboard.press("escape");
    await app.press("id:probe"); await app.waitFor("text:Native available:false");
  } finally { app.dispose(); }
});
test("global native closing releases its OS registration before the next test host", async () => {
  // NSH retains app hosts until the containing test ends; disposing its JS handle is not process exit.
  const next = await open("./Harness.tsx", { platform: "macos", ...config.windowSize });
  try {
    await next.waitFor("text:Global:ready");
    await next.press("id:probe"); await next.waitFor("text:Native available:false");
  } finally { next.dispose(); }
});

test("global native business intents use main-window navigation and persisted appearance actions", async () => {
  const app = await open("./Harness.tsx", { platform: "macos", locale: "en-US", colorScheme: "light", ...config.windowSize });
  try {
    await app.waitFor("text:Global:ready");
    await app.press("id:guard"); await app.press("id:command-settings"); expect(app.texts()).toContain("Path:/");
    await app.press("id:guard"); await app.press("id:command-settings"); await app.waitFor("text:Path:/settings");
    await app.press("id:system"); await app.waitFor("text:Mode:system"); await app.waitFor("text:Effective:light");
    await app.press("id:command-theme"); await app.waitFor("text:Mode:dark");
    await app.press("id:command-theme"); await app.waitFor("text:Mode:light");
    await app.press("id:remount"); await app.waitFor("id:bottom-status-bar", { state: "gone" });
    await app.press("id:remount"); await app.waitFor("text:Mode:light"); await app.waitFor("text:Path:/settings");
  } finally { app.dispose(); }
});
