/** Uses controllable native promises to verify global ownership, late results and transport recovery. */
import { expect, test } from "nsh/test";
import { createGlobalController, type GlobalWire } from "../global-controller";
import type { NativeEffect } from "../contracts";
const install: NativeEffect = { kind: "install", ticket: "one", binding: "fixture", keystroke: "ctrl+alt+shift+u", editing: "always", repeat: true, passThrough: false };
function deferred<T>() { let resolve!: (value: T) => void; const promise = new Promise<T>(done => { resolve = done; }); return { promise, resolve }; }
function fixture() {
  let issued = false; let confirmed = false; let calls = 0;
  const receipts: unknown[] = [];
  const wire: GlobalWire = {
    execute(request) {
      if (request.kind === "acknowledge") { receipts.push(request); confirmed = request.success; }
      if (request.kind === "initialize" && !issued) { issued = true; return { effects: [install] }; }
      return { effects: [] };
    },
    trigger: () => confirmed ? { module: "window", action: "focusMain" } : null,
  };
  return { wire, receipts, dispatch: () => { calls++; }, calls: () => calls };
}
test("shortcut global pending registration blocks retry and early callbacks until confirmed", async () => {
  const f = fixture(); const grant = deferred<boolean>(); let registrations = 0; let fire = () => {};
  const c = createGlobalController(f.wire, { register: async (_, run) => { registrations++; fire = run; return grant.promise; }, unregister() {} }, f.dispatch, async () => {}, () => {});
  const start = c.start(); await Promise.resolve(); fire(); expect(f.calls()).toBe(0);
  const retry = c.retry(); await Promise.resolve(); expect(registrations).toBe(1);
  grant.resolve(true); await start; await retry; fire();
  expect(f.calls()).toBe(1); expect(registrations).toBe(1);
  await c.dispose();
});
test("shortcut global disposal drains a late grant and never dispatches its callback", async () => {
  const f = fixture(); const grant = deferred<boolean>(); let releases = 0; let fire = () => {};
  const c = createGlobalController(f.wire, { register: async (_, run) => { fire = run; return grant.promise; }, unregister() { releases++; } }, f.dispatch, async () => {}, () => {});
  void c.start(); await Promise.resolve(); const stopped = c.dispose();
  grant.resolve(true); await stopped; fire();
  expect(releases).toBe(1); expect(f.calls()).toBe(0);
  await c.dispose(); expect(releases).toBe(1);
});
test("shortcut global rejection and exceptions never report a successful registration", async () => {
  for (const throws of [false, true]) {
    const f = fixture(); const errors: unknown[] = [];
    const c = createGlobalController(f.wire, { register: async () => { if (throws) throw new Error("unavailable"); return false; }, unregister() {} }, f.dispatch, async () => {}, error => errors.push(error));
    await c.start(); expect(f.receipts).toEqual([{ kind: "acknowledge", ticket: "one", success: false, release: false }]);
    expect(errors.length).toBe(throws ? 1 : 0); await c.dispose();
  }
});
test("shortcut global retries a lost acknowledgement without registering the key twice", async () => {
  const f = fixture(); const execute = f.wire.execute; let reject = true; let registrations = 0;
  f.wire.execute = request => {
    if (request.kind === "acknowledge" && reject) { reject = false; throw new Error("transport"); }
    return execute(request);
  };
  const c = createGlobalController(f.wire, { register: async () => { registrations++; return true; }, unregister() {} }, f.dispatch, async () => {}, () => {});
  await c.start(); await c.retry();
  expect(registrations).toBe(1); expect(f.receipts).toHaveLength(1); await c.dispose();
});
test("shortcut global release failure is acknowledged and does not delete ownership", async () => {
  const f = fixture(); const execute = f.wire.execute; let release = false; let stops = 0;
  f.wire.execute = request => request.kind === "retry" && !release
    ? (release = true, { effects: [{ ...install, kind: "release" }] }) : execute(request);
  const c = createGlobalController(f.wire, { register: async () => true, unregister() { if (++stops === 1) throw new Error("release failed"); } }, f.dispatch, async () => {}, () => {});
  await c.start(); await c.retry();
  expect(f.receipts).toContainEqual({ kind: "acknowledge", ticket: "one", success: false, release: true });
  await c.dispose(); expect(stops).toBe(2);
});
test("shortcut global preserves remaining installs when the first acknowledgement disconnects", async () => {
  const f = fixture(); const execute = f.wire.execute; let lost = true; let initial = true;
  const registered: string[] = [];
  f.wire.execute = request => {
    if (request.kind === "initialize" && initial) {
      initial = false; execute(request);
      return { effects: [install, { ...install, ticket: "two", keystroke: "ctrl+alt+shift+i" }] };
    }
    if (request.kind === "acknowledge" && lost) { lost = false; throw new Error("transport"); }
    return execute(request);
  };
  const c = createGlobalController(f.wire, { register: async key => { registered.push(key); return true; }, unregister() {} }, f.dispatch, async () => {}, () => {});
  await c.start(); expect(registered).toHaveLength(1);
  await c.retry(); expect(registered).toEqual([install.keystroke, "ctrl+alt+shift+i"]);
  await c.dispose();
});
