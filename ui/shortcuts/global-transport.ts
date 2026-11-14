/** Adapts the process global bridge; observers never gain registration authority from a snapshot. */
import { GlobalShortcutState } from "usher/nsh";
import { array, boolean, json, object, oneOf, subscribeDecoded } from "../state-runtime/decode";
import { decode, decodeEffect } from "./transport";
import type { Observer } from "../state-runtime/contracts";
import type { Health, Command } from "./contracts";
import type { GlobalWire } from "./global-controller";
export function globalTransport(attachment: unknown = GlobalShortcutState.attach()) {
  const data = object(attachment);
  return {
    initial: decode(data.snapshot),
    executor: boolean(data.executor),
    claim: () => GlobalShortcutState.claim(),
    read: async () => decode(GlobalShortcutState.read()),
    subscribe: (observer: Observer<Health>) =>
      subscribeDecoded(receive => GlobalShortcutState.subscribe(receive), observer, decode),
    execute(request: Parameters<GlobalWire["execute"]>[0]) {
      const reply = GlobalShortcutState.execute(JSON.stringify(request));
      return { effects: array(json(reply.effects)).map(decodeEffect) };
    },
    trigger(ticket: string): Command | null {
      const raw = json(GlobalShortcutState.trigger(ticket));
      if (raw === null) return null;
      const c = object(raw);
      const module = oneOf(c.module, ["window", "appearance"]);
      return module === "window"
        ? { module, action: oneOf(c.action, ["focusMain", "openSettings"]) }
        : { module, action: oneOf(c.action, ["toggleScheme"]) };
    },
  };
}
export type GlobalTransport = ReturnType<typeof globalTransport>;
