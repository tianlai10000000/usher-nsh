/** Tests business command routing and appearance semantics separately from registration mechanics. */
import { expect, test } from "nsh/test";
import { createApplicationCommands } from "../modules/application";
import { toggleScheme } from "../modules/appearance";
test("shortcut application queues startup intents until a live main-window handler exists", async () => {
  const bus = createApplicationCommands(() => {});
  const calls: string[] = [];
  await bus.dispatch({ module: "window", action: "openSettings" });
  const binding = bus.bind(async command => { calls.push(command.action); });
  await binding.ready;
  expect(calls).toEqual(["openSettings"]);
  binding.release();
});
test("shortcut application old cleanup cannot remove the replacement main-window handler", async () => {
  const bus = createApplicationCommands(() => {}); const calls: string[] = [];
  const old = bus.bind(async () => { calls.push("old"); }); await old.ready;
  const next = bus.bind(async () => { calls.push("new"); }); await next.ready;
  old.release();
  await bus.dispatch({ module: "appearance", action: "toggleScheme" });
  expect(calls).toEqual(["new"]); next.release();
});
test("shortcut application coalesces unavailable-owner repeats and reports business failures", async () => {
  const bus = createApplicationCommands(() => {}); let calls = 0; let errors = 0;
  bus.onError(() => { errors++; });
  await bus.dispatch({ module: "window", action: "openSettings" });
  await bus.dispatch({ module: "window", action: "openSettings" });
  const binding = bus.bind(async () => { calls++; throw new Error("business_failure"); });
  await binding.ready; expect(calls).toBe(1); expect(errors).toBe(1); binding.release();
});
test("shortcut appearance toggles saved light and dark and resolves system mode from its effective scheme", async () => {
  for (const [saved, effective, expected] of [
    ["light", "light", "dark"], ["dark", "dark", "light"], ["system", "light", "dark"], ["system", "dark", "light"],
  ] as const) {
    const choices: string[] = [];
    await toggleScheme({ mode: () => saved, effectiveScheme: () => effective, choose: async value => { choices.push(value); return true; } });
    expect(choices).toEqual([expected]);
  }
});
