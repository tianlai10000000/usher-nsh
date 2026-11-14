/** Verifies appearance host ownership using an injected mock and real NSH reactivity. */
import { expect, open, test } from "nsh/test";

test("appearance host waits for a live snapshot and ignores disconnected preferences", async () => {
  const app = await open("./support/AppearanceHost.tsx");
  await app.waitFor("text:Calls:none");
  await app.press("id:dark");
  expect(app.texts()).toContain("Calls:none");
  await app.press("id:live");
  await app.waitFor("text:Calls:dark");
  await app.press("id:reconnect");
  await app.press("id:light");
  expect(app.texts()).toContain("Calls:dark");
  await app.press("id:live");
  await app.waitFor("text:Calls:dark,light");
  await app.press("id:error");
  await app.press("id:system");
  expect(app.texts()).toContain("Calls:dark,light");
});

test("appearance host exposes system changes without writeback and releases its NSH owner", async () => {
  const app = await open("./support/AppearanceHost.tsx");
  await app.press("id:live");
  await app.waitFor("text:Calls:system");
  await app.press("id:host-dark");
  await app.waitFor("text:Effective:dark");
  expect(app.texts()).toContain("Calls:system");
  await app.press("id:dark");
  await app.waitFor("text:Calls:system,dark");
  await app.press("id:detach");
  await app.waitFor("text:Effective:dark", { state: "gone" });
  await app.press("id:light");
  expect(app.texts()).toContain("Calls:system,dark");
});
