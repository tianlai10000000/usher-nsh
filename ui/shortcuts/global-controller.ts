/** Serializes asynchronous system registrations; pending grants never yield their resource to a retry. */
import type { Command, NativeEffect } from "./contracts";
export type GlobalWire = {
  execute(request: { kind: "initialize" | "retry" } | { kind: "acknowledge"; ticket: string; success: boolean; release: boolean }): { effects: NativeEffect[] };
  trigger(ticket: string): Command | null;
};
export type GlobalBackend = { register(key: string, run: () => void): Promise<boolean>; unregister(key: string): void };
/** Owned by the VM, not a component. Disposal must drain late grants before releasing its lease. */
export function createGlobalController(wire: GlobalWire, backend: GlobalBackend, dispatch: (command: Command) => unknown, changed: () => Promise<unknown>, failed: (error: unknown) => void) {
  let pending = Promise.resolve();
  let disposed = false;
  const owned = new Map<string, string>();
  const queue: NativeEffect[] = [];
  const receipts: Array<{ kind: "acknowledge"; ticket: string; success: boolean; release: boolean }> = [];
  function flushReceipts(): NativeEffect[] {
    const effects: NativeEffect[] = [];
    while (receipts.length) {
      effects.push(...wire.execute(receipts[0]).effects);
      receipts.shift();
    }
    return effects;
  }
  async function effects(items: NativeEffect[]) {
    queue.push(...items);
    while (queue.length) {
      const item = queue.shift()!;
      let success = true;
      if (item.kind === "install") {
        if (disposed) {
          queue.push(...wire.execute({ kind: "acknowledge", ticket: item.ticket, success: false, release: false }).effects);
          continue;
        }
        try {
          success = await backend.register(item.keystroke, () => {
            if (disposed || !owned.has(item.ticket)) return;
            try { const command = wire.trigger(item.ticket); if (command) void Promise.resolve(dispatch(command)).catch(failed); } catch (error) { failed(error); }
          });
          if (success) owned.set(item.ticket, item.keystroke);
        } catch (error) { success = false; failed(error); }
      } else {
        try { backend.unregister(item.keystroke); owned.delete(item.ticket); } catch (error) { success = false; failed(error); }
      }
      receipts.push({ kind: "acknowledge", ticket: item.ticket, success, release: item.kind === "release" });
      queue.push(...flushReceipts());
      await changed();
    }
  }
  function enqueue(run: () => Promise<void>) {
    pending = pending.then(run).catch(failed);
    return pending;
  }
  return {
    start: () => enqueue(async () => { if (!disposed) { await effects(wire.execute({ kind: "initialize" }).effects); await changed(); } }),
    retry: () => enqueue(async () => {
      if (!disposed) {
        await effects(flushReceipts());
        await effects(wire.execute({ kind: "initialize" }).effects);
        await effects(wire.execute({ kind: "retry" }).effects);
        await changed();
      }
    }),
    settled: () => pending,
    /** Waits for pending registration results and cleans only this executor's keys. */
    dispose() {
      disposed = true;
      return enqueue(async () => {
        for (const [ticket, key] of owned) {
          try { backend.unregister(key); owned.delete(ticket); } catch (error) { failed(error); }
        }
      });
    },
  };
}
