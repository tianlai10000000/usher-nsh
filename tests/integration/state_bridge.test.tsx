/** Verifies native subscriptions and NSH selectors through the actual application bridge. */
import { expect, open, test } from "nsh/test";

const entry = "./ui/StateSubscription.tsx";
test("bridge: foreign commands update subscribed consumers and native owner cleanup stops delivery", async () => {
  const app = await open(entry);
  try {
    await app.waitFor("text:Sync: live");
    await app.waitFor("text:Observed: system");
    const instance = app.texts().find(text => text.startsWith("Instance:"));
    await app.press("id:foreign-dark");
    await app.waitFor("text:Primary: dark");
    await app.waitFor("text:Borrowed: dark");
    await app.waitFor("text:Observed: dark");
    await app.press("id:unmount");
    const deliveries = app.texts().find(text => text.startsWith("Deliveries:"));
    await app.press("id:foreign-light");
    await app.waitFor("text:Foreign: light");
    await app.waitFor("text:Primary: light");
    await app.waitFor("text:Borrowed: light");
    expect(app.texts()).toContain("Observed: dark");
    expect(app.texts()).toContain(deliveries);
    await app.press("id:remount");
    await app.waitFor("text:Observed: light");
    expect(app.texts()).toContain(instance);
  } finally { app.dispose(); }
});

test("bridge: destroying an engine leaves a new engine with an isolated session", async () => {
  const first = await open(entry);
  await first.waitFor("text:Sync: live");
  await first.press("id:foreign-dark");
  await first.waitFor("text:Primary: dark");
  const identity = first.texts().find(text => text.startsWith("Instance:"));
  first.dispose();
  const second = await open(entry);
  try {
    await second.waitFor("text:Sync: live");
    await second.waitFor("text:Observed: system");
    expect(second.texts()).toContain("Primary: system");
    expect(second.texts().find(text => text.startsWith("Instance:"))).not.toBe(identity);
  } finally { second.dispose(); }
});
