/** A VM-lifetime executor and shared health projection, independent of component and page mounts. */
import { signal } from "nsh";
import { GlobalShortcut } from "nsh/desktop";
import { createProjection } from "../../state-runtime/projection";
import { nshRuntime } from "../../state-runtime/nsh";
import type { Health } from "../contracts";
import { createGlobalController } from "../global-controller";
import { globalTransport, type GlobalTransport } from "../global-transport";
import { applicationCommands } from "../modules/application";
let singleton: ReturnType<typeof build> | undefined;
function build(wire: GlobalTransport) {
  const error = signal(false);
  applicationCommands.onError(() => error.set(true));
  const projection = createProjection<Health, never>(wire.initial.value, {
    read: wire.read, subscribe: wire.subscribe, dispatch: async () => { throw new Error("shortcuts_read_only"); },
  }, nshRuntime);
  const controller = createGlobalController(wire, GlobalShortcut, command => applicationCommands.dispatch(command), () => projection.view.refresh(), () => error.set(true));
  // Outside a rendering owner: component cleanup must not cancel the process subscriber.
  void Promise.resolve().then(async () => { await projection.connect(); if (wire.executor) await controller.start(); }).catch(() => error.set(true));
  return {
    value: projection.view.value,
    connection: () => error() ? "error" : projection.view.status(),
    retry: async () => {
      error.set(false); await projection.connect();
      if (wire.claim()) await controller.retry();
    },
  };
}
/** Called during startup, before window or page initialization; subsequent windows borrow it. */
export function createGlobalShortcuts(transport?: GlobalTransport) {
  return singleton ??= build(transport ?? globalTransport());
}
export type GlobalShortcuts = ReturnType<typeof createGlobalShortcuts>;
