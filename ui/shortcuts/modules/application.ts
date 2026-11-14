/** Routes process shortcut intents to the live main-window business adapter without retaining stale pages. */
import type { Command } from "../contracts";
import { windowCommand } from "./window";
type Handler = (command: Command) => Promise<unknown>;
/** Queues at most one pending intent of each kind until a main-window adapter is available. */
export function createApplicationCommands(focus: (command: Command) => void = windowCommand) {
  let handler: Handler | undefined;
  let failure: (error: unknown) => void = () => {};
  let generation = 0;
  let draining = false;
  const pending = new Map<string, Command>();
  async function drain() {
    if (draining) return;
    draining = true;
    try {
      while (handler && pending.size) {
        const [key, command] = pending.entries().next().value!;
        pending.delete(key);
        try { await handler(command); } catch (error) { failure(error); }
      }
    } finally { draining = false; }
  }
  return {
    onError(report: (error: unknown) => void) { failure = report; },
    dispatch(command: Command) {
      if (command.module === "window" && command.action === "focusMain") { focus(command); return Promise.resolve(); }
      pending.set(`${command.module}.${command.action}`, command);
      return drain();
    },
    bind(next: Handler) {
      handler = next; const token = ++generation;
      const ready = drain();
      return { ready, release() { if (generation === token) { handler = undefined; ++generation; } } };
    },
  };
}
export const applicationCommands = createApplicationCommands();
