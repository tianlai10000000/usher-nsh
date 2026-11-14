/** Exercises focus ownership and complete VM replacement on the isolated macOS host. */
import { expect, open, test } from "nsh/test";
import config from "./config.json";
test("shortcut native reload restores a usable window and lazy page service", async () => {
  const app = await open("./Harness.tsx", { platform: "macos", locale: "en-US", ...config.windowSize });
  try {
    await app.waitFor("text:Startup:ready"); await app.press("id:go-settings"); await app.waitFor("text:Page:ready");
    const previous = app.texts().find(text => text.startsWith("Store:"));
    await app.keyboard.press("mod+r");
    await app.waitUntil(() => app.texts().some(text => text.startsWith("Store:") && text !== previous));
    await app.waitFor("text:Startup:ready"); await app.waitFor("text:Connection:live");
    await app.press("id:go-settings"); await app.waitFor("text:Page:ready");
    await app.keyboard.press(config.payload.bindings[0].keystroke); await app.waitFor("text:Path:/");
  } finally { app.dispose(); }
});
test("shortcut native focus routes one key to only the active window", async () => {
  const app = await open("./MultiWindow.tsx", { platform: "macos", ...config.windowSize });
  try {
    await app.waitFor("text:Main:/settings"); await app.press("id:open-secondary"); await app.waitFor("text:Secondary:/settings");
    await app.press("id:focus-secondary");
    await app.waitUntil(() => app.window(config.secondaryId).state()?.focused === true);
    await app.keyboard.press("mod+["); await app.waitFor("text:Secondary:/"); expect(app.texts()).toContain("Main:/settings");
    await app.press("id:focus-main"); await app.waitUntil(() => app.window("main").state()?.focused === true);
    await app.keyboard.press("mod+["); await app.waitFor("text:Main:/"); expect(app.texts()).toContain("Secondary:/");
    const previousScope = app.texts().find(text => text.startsWith("SecondaryScope:"));
    await app.window(config.secondaryId).pressClose();
    await app.keyboard.press("mod+]"); await app.waitFor("text:Main:/settings");
    await app.press("id:open-secondary");
    await app.waitUntil(() => app.texts().some(text => text.startsWith("SecondaryScope:") && text !== previousScope));
    await app.waitFor("text:Secondary:/settings"); await app.press("id:focus-secondary");
    await app.waitUntil(() => app.window(config.secondaryId).state()?.focused === true);
    await app.keyboard.press("mod+["); await app.waitFor("text:Secondary:/");
    expect(app.texts()).toContain("Main:/settings");
    await app.keyboard.press("mod+]"); await app.waitFor("text:Secondary:/settings");
  } finally { app.dispose(); }
});
