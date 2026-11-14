/** Verifies asynchronous page leases and input lifetime using explicit transport/backend fakes. */
import { expect, test } from "nsh/test";
import { createController } from "../controller";
import type { Request, Transport, NativeEffect } from "../contracts";

function fixture() {
  const requests: Request[] = [];
  let serial = 0;
  let closed = 0;
  const wire = {
    execute(request: Request) { requests.push(request); return { effects: [], ticket: request.kind === "beginPage" ? String(++serial) : null }; },
    close() { closed++; },
  } as unknown as Transport;
  const controller = createController(wire, { bind() { return () => {}; } }, () => {}, async () => {}, error => { throw error; });
  return { controller, requests, closed: () => closed };
}
test("shortcut controller retry preserves the current page cleanup lease", async () => {
  const f = fixture();
  const leave = f.controller.mountPage("entry", "page");
  await f.controller.settled();
  await f.controller.retry();
  leave(); await f.controller.settled();
  expect(f.requests.filter(r => r.kind === "leavePage")).toEqual([{ kind: "leavePage", ticket: "2" }]);
  f.controller.dispose(); f.controller.dispose(); expect(f.closed()).toBe(1);
});
test("shortcut controller retires a page before its factory starts", async () => {
  const f = fixture();
  const leave = f.controller.mountPage("old", "page"); leave();
  f.controller.mountPage("new", "page"); await f.controller.settled();
  expect(f.requests.filter(r => r.kind === "beginPage")).toEqual([{ kind: "beginPage", entry: "new", page: "page" }]);
  f.controller.dispose();
});
test("shortcut controller blocks callbacks synchronously and releases registrations on disposal", async () => {
  let fire = () => {}; let count = 0; let stops = 0;
  const item: NativeEffect = { kind: "install", ticket: "binding", binding: "test", keystroke: "mod+k", editing: "notEditing", repeat: false, passThrough: false };
  const wire = {
    execute(request: Request) { return { effects: request.kind === "initialize" ? [item] : [], ticket: null }; },
    trigger() { return { module: "navigation", action: "back" }; }, close() {},
  } as unknown as Transport;
  const controller = createController(wire, { bind(_, run) { fire = run; return () => { stops++; }; } }, () => { count++; }, async () => {}, error => { throw error; });
  await controller.start(); await controller.context(true, false);
  fire(); expect(count).toBe(1);
  void controller.context(true, true); fire(); expect(count).toBe(1);
  await controller.settled(); controller.dispose(); fire();
  expect(stops).toBe(1); expect(count).toBe(1);
});
test("shortcut controller reports installation failure and retries through authoritative effects", async () => {
  const receipts: Request[] = [];
  let attempts = 0; let stops = 0;
  const wire = {
    execute(request: Request) {
      if (request.kind === "acknowledge") receipts.push(request);
      return { effects: request.kind === "initialize" ? [{
        kind: "install", ticket: String(attempts), binding: "fixture", keystroke: "mod+k",
        editing: "notEditing", repeat: false, passThrough: false,
      }] : [], ticket: null };
    },
    close() {},
  } as unknown as Transport;
  const controller = createController(wire, {
    bind() { if (++attempts === 1) throw new Error("unavailable"); return () => { stops++; }; },
  }, () => {}, async () => {}, error => { throw error; });
  await controller.start();
  expect(receipts).toEqual([{ kind: "acknowledge", ticket: "0", success: false, release: false }]);
  await controller.retry();
  expect(receipts[1]).toEqual({ kind: "acknowledge", ticket: "1", success: true, release: false });
  controller.dispose(); expect(stops).toBe(1);
});
