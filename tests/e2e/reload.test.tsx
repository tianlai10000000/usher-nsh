/** Uses the official macOS Cmd+R path to replace the VM, not a component remount. */
import { expect, open, test } from "nsh/test";
import options from "./fixtures/session.json";
test("native VM reload restores persisted preferences and rejects obsolete callbacks", async () => {
  const app = await open("./fixtures/NativeReload.tsx", { ...options, platform: "macos", colorScheme: "light" });
  try {
    await app.waitFor("text:Sync:live/live");
    await app.press("Prepare retained and transient data");
    await app.waitFor("text:Path:/settings");
    await app.waitFor("text:Color:dark");
    expect(app.texts()).toContain("Saved:true");
    expect(app.texts()).toContain("Count:1");
    expect(app.texts()).toContain("Preview:45");
    await app.press("Begin pending guard"); await app.waitFor("text:Pending:true");
    await app.press("Save callback checkpoint"); await app.waitFor("text:Checkpoint:saved");
    const previous = app.texts().find(text => text.startsWith("Store:"));
    await app.keyboard.press("mod+r");
    await app.waitUntil(() => app.texts().some(text => text.startsWith("Store:") && text !== previous));
    await app.waitFor("text:Sync:live/live"); await app.waitFor("text:Old callback:rejected");
    await app.waitFor("text:Color:dark");
    for (const text of ["Count:0", "Section:general", "Preview:none", "Path:/", "Pending:false", "Saved:true"]) expect(app.texts()).toContain(text);
  } finally { app.dispose(); }
});
