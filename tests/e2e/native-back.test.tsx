/** Uses the official host input driver; native runs get a fresh data root per open. */
import { expect, open, test } from "nsh/test";
import options from "./fixtures/session.json";
test("e2e native back closes modals first and commits only after asynchronous guards", async () => {
  const app = await open("./fixtures/NativeRouting.tsx", { ...options, platform: "macos", colorScheme: "light" });
  try {
    await app.waitFor("text:Path:/");
    await app.press("Open settings"); await app.waitFor("text:Path:/settings");
    await app.press("Open modal"); await app.waitFor("text:Modal content");
    await app.back(); await app.waitFor("text:Modal content", { state: "gone" });
    expect(app.texts()).toContain("Path:/settings");
    expect(app.texts()).toContain("Pending:false");
    await app.back(); await app.waitFor("text:Pending:true");
    expect(app.texts()).toContain("Path:/settings");
    await app.press("Cancel return"); await app.waitFor("text:Pending:false");
    expect(app.texts()).toContain("Path:/settings");
    await app.back(); await app.waitFor("text:Pending:true");
    await app.press("Allow return"); await app.waitFor("text:Path:/");
    await app.back(); expect(app.texts()).toContain("Path:/");
    expect(app.texts()).toContain("Pending:false");
  } finally { app.dispose(); }
});
