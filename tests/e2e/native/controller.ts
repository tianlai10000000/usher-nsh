/** Coordinates native lifecycle assertions; stores test observations, never application state. */
import { signal } from "nsh";
import { Window, type WindowRef } from "nsh/desktop";
import { WindowState } from "usher/nsh";
import type { Navigation } from "../../../ui/router/index";
import type { AppState } from "../../../ui/state/app";
import type { createWindowState } from "../../../ui/state/window/index";
import config from "./config.json";
export type Probe = { host: WindowRef; scope: string; window: ReturnType<typeof createWindowState>; appearance: AppState; router: Navigation; generation: number };
export const secondaryVisible = signal(true);
export const probes = new Map<string, Probe>();
let before: Map<string, Probe> | undefined;
let oldGuard: Promise<unknown> | undefined;
let releaseGuard: (() => void) | undefined;
let running = false;
const completed = new Set<number>();
let openSecondary: (() => void) | undefined;
let cleanupCount = 0;
export function removed() { cleanupCount++; }
export function assert(value: unknown, message: string): asserts value {
  if (!value) throw new Error(message);
}
export function evidence(event: string, data: unknown = {}) {
  console.log(config.marker + JSON.stringify({ event, data }));
}
export async function until(read: () => boolean, message: string) {
  const deadline = Date.now() + config.conditionTimeoutMs;
  while (!read()) {
    assert(Date.now() < deadline, message);
    await new Promise<void>(resolve => requestAnimationFrame(() => resolve()));
  }
}
export function opened(open: () => void) { openSecondary = open; }
export function observe(id: string, probe: Probe) {
  probes.set(id, probe);
  if (running || completed.has(probe.generation) || !openSecondary) return;
  if (probes.size < 2 || [...probes.values()].some(value => value.generation !== probe.generation)) return;
  running = true;
  void verify(probe.generation).then(() => completed.add(probe.generation)).catch(error => evidence("failed", String(error))).finally(() => { running = false; });
}
async function verify(generation: number) {
  const main = probes.get("main")!;
  const second = probes.get(config.secondaryId)!;
  await until(() => [...probes.values()].every(value => value.window.routing.status() === "live" && value.appearance.settings.view.status() === "live"), "subscriptions did not become live");
  await until(() => main.host.state()?.visible === true && Window.get(config.secondaryId)?.state()?.visible === true, "two native windows did not become visible");
  assert(main.scope !== second.scope, "native windows share a window Store");
  if (generation === 1) {
    await main.window.session.change({ kind: "increment" });
    await main.router.navigate("settings");
    await main.window.session.change({ kind: "section", value: config.section });
    await main.window.session.preview(true, config.preview);
    await main.appearance.settings.view.dispatch({ type: "colorMode", value: "dark" });
    await until(() => second.appearance.settings.view.value().colorMode === config.colorMode, "shared preference did not reach second window");
    assert(second.window.session.value().clicks === 0, "counter leaked across windows");
    assert(second.router.route() === "home", "history leaked across windows");
    assert(second.window.session.value().section === "general", "section leaked across windows");
    assert(second.window.session.value().opacityPreview == null, "preview leaked across windows");
    assert(main.appearance.settings.view.value().saved, "preference did not persist");
    before = new Map(probes);
    main.router.beforeEach(() => new Promise<boolean>(resolve => { releaseGuard = () => resolve(true); }));
    oldGuard = main.router.navigate("home");
    evidence("before_hmr", { main: main.scope, second: second.scope, entry: main.router.current().key });
    return;
  }
  assert(before, "HMR replaced the controller or rebuilt the VM");
  assert(main.scope === before.get("main")!.scope && second.scope === before.get(config.secondaryId)!.scope, "HMR recreated a window Store");
  assert(main.window.routing.value().history.catalogIdentity !== before.get("main")!.window.routing.value().history.catalogIdentity, "HMR did not migrate the catalog");
  assert(main.router.current().key === before.get("main")!.router.current().key, "HMR replaced history entry");
  assert(main.window.session.value().clicks === 1 && main.window.session.value().section === config.section, "HMR lost session state");
  assert(main.window.session.value().opacityPreview === config.preview, "HMR lost preview");
  assert(cleanupCount >= 2, "HMR did not clean old owners");
  let rejected = false;
  try { await before.get("main")!.window.session.change({ kind: "increment" }); } catch { rejected = true; }
  assert(rejected, "old projection still accepts writes");
  releaseGuard!();
  assert(await oldGuard === "cancelled", "old guard did not cancel");
  assert(main.router.route() === "settings", "old HMR callback overwrote current route");
  await main.appearance.settings.view.dispatch({ type: "colorMode", value: "light" });
  await until(() => second.appearance.settings.view.value().colorMode === "light", "new subscriptions did not receive a post-HMR preference");
  assert(before.get("main")!.appearance.settings.view.status() === "disposed", "old settings owner did not dispose");
  assert(before.get("main")!.appearance.settings.view.value().colorMode === config.colorMode, "old settings subscription received a post-HMR callback");
  await main.appearance.settings.view.dispatch({ type: "colorMode", value: "dark" });
  await until(() => second.appearance.settings.view.value().colorMode === config.colorMode, "post-HMR setting restore did not synchronize");
  evidence("hmr_passed", { generation, cleanupCount, main: main.scope, second: second.scope });
  const oldScope = second.scope;
  secondaryVisible.set(false);
  await until(() => second.window.routing.status() === "disposed", "secondary content did not unmount");
  Window.get(config.secondaryId)!.close();
  await until(() => Window.get(config.secondaryId) === null, "secondary did not close");
  let closed = false;
  try { WindowState.snapshot(oldScope); } catch { closed = true; }
  assert(closed, "native close did not close the window Store");
  probes.delete(config.secondaryId);
  secondaryVisible.set(true);
  openSecondary!();
  await until(() => probes.has(config.secondaryId) && probes.get(config.secondaryId)!.window.routing.status() === "live", "secondary did not reopen");
  const reopened = probes.get(config.secondaryId)!;
  await until(() => reopened.appearance.settings.view.value().colorMode === config.colorMode, "reopened window lost shared preference");
  assert(reopened.scope !== oldScope && reopened.window.session.value().clicks === 0 && reopened.router.route() === "home", "reopening reused old session");
  assert(main.window.session.value().clicks === 1, "closing secondary changed main");
  evidence("complete", { main: main.scope, closed: oldScope, reopened: reopened.scope, colorMode: reopened.appearance.settings.view.value().colorMode });
}
