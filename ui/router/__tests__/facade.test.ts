/** Router facade contracts with controlled guard replies and an injected window binding. */
import { expect, test } from "nsh/test";
import { createRouter } from "../index";
import type { RoutingBinding } from "../../state/window";
import type { WindowData } from "../../state/window-contracts";
function setup() {
  const data: WindowData = { history: { entries: [{ key: "1", location: { id: "home", page: "home", path: "/", params: {}, query: {}, ancestors: ["home"] } }], cursor: 0, catalogIdentity: "fixture" }, session: { section: "general", clicks: 0, operations: {} }, pending: null, navigationResult: "committed" };
  const calls: unknown[] = [];
  let ticket = 0;
  let dispose = () => {};
  const binding: RoutingBinding = {
    value: () => data, status: () => "live",
    navigate: async command => { calls.push(command); return "committed"; },
    resolve: () => data.history.entries[0].location,
    begin: command => { calls.push(command); return String(++ticket); },
    complete: async (id, decision, target) => { calls.push([id, decision, target]); return decision === "redirect" ? null : decision === "allow" ? "committed" : decision === "cancel" ? "cancelled" : "failed"; },
  };
  const router = createRouter(binding, callback => { dispose = callback; });
  return { router, binding, calls, dispose: () => dispose() };
}
test("router facade delegates typed actions and rejects unknown legacy names", async () => {
  const { router, calls, dispose } = setup();
  expect(await router.open("missing")).toBe("invalid_target"); expect(calls.length).toBe(0);
  expect(await router.navigate("settings")).toBe("committed");
  await router.replace({ kind: "path", location: "/unknown" }); await router.goBack();
  expect(calls).toEqual([{ kind: "push", target: { kind: "named", id: "settings" } }, { kind: "replace", target: { kind: "path", location: "/unknown" } }, { kind: "go", target: -1 }]);
  expect(router.breadcrumbs()[0].id).toBe("home");
  dispose(); expect(await router.navigate("home")).toBe("cancelled"); expect(calls.length).toBe(3);
});
test("router guards cancel fail redirect and unregister through the same ticket boundary", async () => {
  const { router, calls, dispose } = setup();
  let remove = router.beforeEach(() => false);
  expect(await router.navigate("settings")).toBe("cancelled"); remove();
  remove = router.beforeEach(() => { throw new Error("injected"); });
  expect(await router.navigate("settings")).toBe("failed"); remove();
  let redirects = 0;
  remove = router.beforeEach(() => redirects++ === 0 ? { kind: "named", id: "home" } : true);
  expect(await router.navigate("settings")).toBe("committed"); remove();
  expect(calls).toContainEqual(["1", "cancel", undefined]);
  expect(calls).toContainEqual(["2", "fail", undefined]);
  expect(calls).toContainEqual(["3", "redirect", { kind: "named", id: "home" }]);
  expect(calls).toContainEqual(["3", "allow", undefined]);
  dispose();
});
test("router disposal cancels pending guards and ignores their late replies", async () => {
  const { router, calls, dispose } = setup();
  let release!: (allow: boolean) => void;
  router.beforeEach(() => new Promise<boolean>(resolve => { release = resolve; }));
  const pending = router.navigate("settings"); dispose(); release(true);
  expect(await pending).toBe("cancelled");
  expect(calls).toContainEqual(["1", "cancel", undefined]);
  expect(calls.some(call => Array.isArray(call) && call[1] === "allow")).toBe(false);
});

test("router guards return a terminal failure when their transport closes during completion", async () => {
  const { router, binding, dispose } = setup();
  router.beforeEach(() => true);
  binding.complete = async () => { throw new Error("state_binding_closed"); };
  expect(await router.navigate("settings")).toBe("failed");
  dispose();
});

test("router facade distinguishes invalid targets from unavailable unguarded transports", async () => {
  const { router, binding, dispose } = setup();
  for (const code of ["router_closed", "router_unavailable", "state_binding_closed", "router_capacity_exhausted"]) {
    binding.navigate = async () => { throw new Error(code); };
    expect(await router.navigate("settings")).toBe("failed");
  }
  binding.navigate = async () => { throw new Error("router_invalid_target"); };
  expect(await router.navigate("settings")).toBe("invalid_target");
  dispose();
});

test("router guards distinguish begin failures and never release an unissued ticket", async () => {
  const { router, binding, calls, dispose } = setup();
  router.beforeEach(() => true);
  binding.begin = () => { throw new Error("router_unavailable"); };
  expect(await router.navigate("settings")).toBe("failed");
  binding.begin = () => { throw new Error("router_invalid_target"); };
  expect(await router.navigate("settings")).toBe("invalid_target");
  expect(calls.length).toBe(0);
  dispose();
});
