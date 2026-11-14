/** Owns NSH keymap effects and captured window focus; business decisions stay in Rust. */
import { cleanup, effect, keymap, platform, signal } from "nsh";
import { currentWindow, focusedWindow } from "nsh/desktop";
import type { Navigation } from "../../router/index";
import { createProjection } from "../../state-runtime/projection";
import { nshRuntime } from "../../state-runtime/nsh";
import { createGlobalShortcuts, type GlobalShortcuts } from "./global";
import { combineHealth } from "../combined-health";
import type { Health, Transport } from "../contracts";
import { createController } from "../controller";
import { shortcutTransport } from "../transport";
import { navigationCommands } from "../modules/navigation";

export function createShortcuts(windowScope: string, router: Navigation, transport?: Transport, global: GlobalShortcuts = createGlobalShortcuts()) {
  const host = currentWindow();
  const wire = transport ?? shortcutTransport(windowScope, platform.os);
  const error = signal(false);
  const blocks = new Set<object>();
  const blocked = signal(false);
  const projection = createProjection<Health, never>(wire.initial.value, {
    read: wire.read, subscribe: wire.subscribe,
    dispatch: async () => { throw new Error("shortcuts_read_only"); },
  }, nshRuntime);
  const controller = createController(wire, {
    bind(item, run) { return keymap.bind(item.keystroke, run, { when: item.editing === "auto" ? undefined : item.editing, repeat: item.repeat, passThrough: item.passThrough }); },
  }, navigationCommands(router), () => projection.view.refresh(), () => error.set(true));
  void projection.connect();
  void controller.start();
  const ownsInput = () => { const focused = focusedWindow(); return focused ? focused.id === host.id : (host.state()?.focused ?? host.main); };
  const synchronize = () => controller.context(ownsInput(), blocked());
  effect(() => {
    router.canGoBack(); router.canGoForward(); router.current().key; host.state()?.focused;
    void synchronize();
  });
  effect(() => {
    const status = projection.view.status();
    if (status === "closed" || status === "cancelled") controller.dispose();
    else if (status === "error") { error.set(true); void controller.context(false, true); }
  });
  const stopFocus = host.on("focusChange", () => { void synchronize(); });
  const stopClose = host.on("close", () => controller.dispose());
  cleanup(() => { stopFocus(); stopClose(); controller.dispose(); projection.dispose(); });
  return {
    value: () => combineHealth(projection.view.value(), global.value()),
    connection: () => error() ? "error" : global.connection() !== "live" ? global.connection() : projection.view.status(),
    mountPage: controller.mountPage,
    retry: async () => { error.set(false); await projection.connect(); await synchronize(); await controller.retry(); await global.retry(); },
    /** Each overlay owns its blocker; closing one cannot unblock another. */
    block() {
      const token = {}; blocks.add(token); blocked.set(true); void controller.context(ownsInput(), true);
      let released = false;
      return () => { if (released) return; released = true; blocks.delete(token); blocked.set(blocks.size > 0); void controller.context(ownsInput(), blocks.size > 0); };
    },
  };
}
export type Shortcuts = ReturnType<typeof createShortcuts>;
