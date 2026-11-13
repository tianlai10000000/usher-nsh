/** Checks the real rendering boundary with injected page failures and observable navigation. */
import { expect, open, test } from "nsh/test";

test("router view retries a failed page without changing history and can return from a persistent failure", async () => {
  const app = await open("./fixtures/RenderFailure.tsx", { locale: "en-US", platform: "macos" });
  try {
    await app.waitFor("id:route-error");
    const initial = app.texts().find(text => text.startsWith("Entry:"));
    expect(app.texts()).toContain("Path:/");
    await app.press("Retry");
    await app.waitFor("text:Recovered home page");
    expect(app.shows("id:route-error")).toBe(false);
    expect(app.texts()).toContain(initial);

    await app.press("Open failing page");
    await app.waitFor("id:route-error");
    expect(app.texts()).toContain("Path:/settings");
    expect(app.texts().find(text => text.startsWith("Entry:"))).not.toBe(initial);
    await app.press("Back");
    await app.waitFor("text:Recovered home page");
    expect(app.texts()).toContain("Path:/");
    expect(app.texts()).toContain(initial);
  } finally {
    app.dispose();
  }
});
