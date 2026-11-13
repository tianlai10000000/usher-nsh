/** Real route resolution, guarded commits, NotFound rendering and shared projection integration. */
import { expect, open, test } from "nsh/test";

test("router bridge fences old guards and renders NotFound through the registered page", async () => {
  const app = await open("./fixtures/RouterProbe.tsx", { locale: "en-US" });
  try {
    await app.waitFor("text:Path:/");
    await app.press("Guarded settings");
    await app.press("Unknown page"); await app.waitFor("id:route-not-found");
    await app.press("Allow old guard"); await app.waitFor("text:Path:/unknown");
    expect(app.shows("id:route-not-found")).toBe(true);
    await app.press("Back from probe"); await app.waitFor("text:Clicks: 0");
    expect(app.texts()).toContain("Path:/");
  } finally { app.dispose(); }
});

test("router bridge replaces entries and shares previews and sections with mounted pages", async () => {
  const app = await open("./fixtures/RouterProbe.tsx", { locale: "en-US" });
  try {
    await app.waitFor("text:Path:/");
    const initial = app.texts().find(text => text.startsWith("Key:"));
    await app.press("Preview from another consumer"); await app.waitFor("text:Preview:45");
    await app.press("Replace settings"); await app.waitFor("text:Path:/settings");
    expect(app.texts().find(text => text.startsWith("Key:"))).not.toBe(initial);
    await app.press("Section from another consumer"); await app.waitFor("id:theme-settings");
    await app.press("Back from probe");
    expect(app.texts()).toContain("Path:/settings");
    expect(app.texts()).toContain("Preview:45");
  } finally { app.dispose(); }
});

test("router bridge remount keeps one window store while replacing disposed projections", async () => {
  const app = await open("./fixtures/WindowLifecycle.tsx");
  try {
    await app.waitFor("text:Count:0");
    await app.press("Increase window counter"); await app.waitFor("text:Count:1");
    await app.press("Set retained preview"); await app.waitFor("text:Preview:45");
    await app.press("Navigate retained history"); await app.waitFor("text:Path:/settings");
    const entry = app.texts().find(text => text.startsWith("Entry:"));
    await app.press("Unmount composition"); await app.waitFor("text:Count:1", { state: "gone" });
    await app.press("Remount composition"); await app.waitFor("text:Count:1");
    expect(app.texts()).toContain("Preview:45"); expect(app.texts()).toContain("Path:/settings");
    expect(app.texts()).toContain(entry);
    await app.press("Increase window counter"); await app.waitFor("text:Count:2");
  } finally { app.dispose(); }
});

test("router bridge host links await guards and native back shares cancellation", async () => {
  const app = await open("./fixtures/RouterProbe.tsx", { locale: "en-US" });
  try {
    await app.waitFor("text:Path:/");
    await app.openWithUrl("usher://app/settings");
    expect(app.texts()).toContain("Path:/");
    await app.press("Allow old guard"); await app.waitFor("text:Path:/settings");
    await app.press("Unknown page"); await app.waitFor("text:Path:/unknown");
    await app.back();
    expect(app.texts()).toContain("Path:/unknown");
    await app.press("Cancel old guard");
    expect(app.texts()).toContain("Path:/unknown");
    await app.back(); await app.press("Allow old guard"); await app.waitFor("text:Path:/settings");
  } finally { app.dispose(); }
});
