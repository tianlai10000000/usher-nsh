/** Real Rust-to-JS subscription tests on isolated headless engines, without module mocks. */
import { expect, open, test } from "nsh/test";

test("bridge native commits reach independent subscribers and derived context views", async () => {
  const app = await open("./fixtures/StateBridge.tsx");
  await app.waitFor("text:A:system:live");
  await app.waitFor("text:B:system:live");
  await app.press("Dark from Rust command");
  await app.waitFor("text:A:dark:live");
  await app.waitFor("text:B:dark:live");
  expect(app.texts()).toContain("Borrowed:dark");
  // The command discards its legacy reply; only subscription delivery can update consumers.
  expect(app.texts()).toContain("LastB:dark");
});

test("bridge owner disposal stops callbacks and remount receives current state", async () => {
  const app = await open("./fixtures/StateBridge.tsx");
  await app.waitFor("text:B:system:live");
  await app.press("Dark from Rust command");
  await app.waitFor("text:LastB:dark");
  await app.press("Toggle B");
  await app.waitFor("text:Released:1");
  await app.press("Light from Rust command");
  await app.waitFor("text:A:light:live");
  expect(app.texts()).toContain("LastB:dark");
  await app.press("Toggle B");
  await app.waitFor("text:B:light:live");
  await app.waitFor("text:LastB:light");
  expect(app.texts()).toContain("Released:1");
});

test("bridge cancellation during registration never delivers a late initial callback", async () => {
  const app = await open("./fixtures/StateBridge.tsx");
  await app.waitFor("text:A:system:live");
  await app.press("Cancel registration");
  await app.press("Dark from Rust command");
  await app.waitFor("text:A:dark:live");
  expect(app.texts()).toContain("Cancelled:0");
});

test("bridge engine destruction ends pending subscriptions and new sessions are isolated", async () => {
  const first = await open("./fixtures/StateBridge.tsx");
  await first.waitFor("text:A:system:live");
  await first.press("Dark from Rust command");
  await first.waitFor("text:A:dark:live");
  first.dispose();
  const second = await open("./fixtures/StateBridge.tsx");
  await second.waitFor("text:A:system:live");
  await second.press("Light from Rust command");
  await second.waitFor("text:B:light:live");
});
