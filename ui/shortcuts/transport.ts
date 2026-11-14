/** Validates NSH bridge envelopes and keeps native types outside the shortcut controller. */
import { ShortcutState } from "usher/nsh";
import { array, boolean, decimal, envelope, json, object, oneOf, string, subscribeDecoded } from "../state-runtime/decode";
import type { Health, Phase, Transport, NativeEffect, Command } from "./contracts";
const phases: readonly Phase[] = ["validating", "loading", "installing", "ready", "failed", "notRequested", "notLoaded", "inactive", "unsupported", "recovering"];
export function decodeHealth(value: unknown): Health {
  const data = object(value);
  return {
    startup: oneOf(data.startup, phases), page: oneOf(data.page, phases), global: oneOf(data.global, phases), entry: data.entry == null ? null : string(data.entry),
    modules: array(data.modules).map(raw => { const m = object(raw); return { id: string(m.id), kind: oneOf(m.kind, ["global", "application", "page"]), page: m.page == null ? null : string(m.page), phase: oneOf(m.phase, phases), error: m.error == null ? null : string(m.error) }; }),
    bindings: array(data.bindings).map(raw => { const b = object(raw); return { id: string(b.id), module: string(b.module), keystroke: string(b.keystroke), state: oneOf(b.state, ["installed", "registered", "installing", "failed", "releasing", "inactive"]) }; }),
  };
}
export const decode = (wire: unknown) => envelope(wire, decodeHealth, true);
export function decodeEffect(raw: unknown): NativeEffect {
  const e = object(raw);
  return { kind: oneOf(e.kind, ["install", "release"]), ticket: decimal(e.ticket), binding: string(e.binding), keystroke: string(e.keystroke), editing: oneOf(e.editing, ["auto", "always", "notEditing"]), repeat: boolean(e.repeat), passThrough: boolean(e.passThrough) };
}
export function shortcutTransport(windowScope: string, os: string, attachment: unknown = ShortcutState.attach(windowScope, os)): Transport {
  const initial = decode(attachment);
  const scope = initial.storeId;
  return {
    initial,
    read: async () => decode(ShortcutState.read(scope)),
    subscribe: observer => subscribeDecoded(receive => ShortcutState.subscribe(scope, receive), observer, decode),
    execute(request) {
      const reply = ShortcutState.execute(scope, JSON.stringify(request));
      return { snapshot: decode(reply.snapshot), effects: array(json(reply.effects)).map(decodeEffect), ticket: reply.ticket == null ? null : decimal(reply.ticket) };
    },
    trigger(ticket): Command | null {
      const reply = json(ShortcutState.trigger(scope, ticket));
      if (reply === null) return null;
      const c = object(reply); return { module: oneOf(c.module, ["navigation"]), action: oneOf(c.action, ["back", "forward"]) };
    },
    close: () => ShortcutState.close(scope),
  };
}
