/** Serializes host effects and page load requests without owning business state or reactivity. */
import type { Command, InputBackend, NativeEffect, Request, Transport } from "./contracts";

export function createController(wire: Transport, backend: InputBackend, dispatch: (command: Command) => unknown, changed: () => Promise<unknown>, failed: (error: unknown) => void) {
  const handles = new Map<string, () => void>();
  let disposed = false;
  let focused = false;
  let blocked = false;
  let pageGeneration = 0;
  let pageTicket: string | null = null;
  let currentPage: { entry: string; page: string } | null = null;
  let pending = Promise.resolve();
  function clearHandles() { for (const stop of handles.values()) stop(); handles.clear(); }
  function effects(items: NativeEffect[]) {
    const queue = [...items];
    while (queue.length && !disposed) {
      const item = queue.shift()!;
      let success = true;
      try {
        if (item.kind === "release") { handles.get(item.ticket)?.(); handles.delete(item.ticket); }
        else if (!handles.has(item.ticket)) {
          handles.set(item.ticket, backend.bind(item, () => {
            if (disposed || !focused || blocked) return;
            try { const command = wire.trigger(item.ticket); if (command) void Promise.resolve(dispatch(command)).catch(failed); }
            catch (error) { failed(error); }
          }));
        }
      } catch { success = false; }
      const reply = wire.execute({ kind: "acknowledge", ticket: item.ticket, success, release: item.kind === "release" });
      queue.push(...reply.effects);
    }
  }
  function execute(request: Request) { const reply = wire.execute(request); effects(reply.effects); return reply; }
  function enqueue(run: () => void | Promise<void>) {
    pending = pending.then(async () => { if (!disposed) { await run(); if (!disposed) await changed(); } }).catch(error => {
      if (!disposed && !(error instanceof Error && error.message.includes("shortcuts_stale_page"))) { clearHandles(); failed(error); }
    });
    return pending;
  }
  async function loadPage(generation: number, entry: string, page: string) {
    if (disposed || generation !== pageGeneration) return;
    const reply = execute({ kind: "beginPage", entry, page });
    pageTicket = reply.ticket;
    // An entry may leave before its module is needed. Never load or install a retired request.
    await changed();
    await Promise.resolve();
    if (!disposed && generation === pageGeneration && pageTicket) execute({ kind: "finishPage", ticket: pageTicket });
  }
  return {
    start: () => enqueue(() => { execute({ kind: "initialize" }); execute({ kind: "context", focused, blocked }); }),
    context(nextFocused: boolean, nextBlocked: boolean) {
      focused = nextFocused; blocked = nextBlocked;
      return enqueue(() => { execute({ kind: "context", focused, blocked }); });
    },
    mountPage(entry: string, page: string) {
      const generation = ++pageGeneration;
      currentPage = { entry, page };
      void enqueue(() => loadPage(generation, entry, page));
      return () => {
        if (generation !== pageGeneration) return;
        ++pageGeneration; currentPage = null;
        const ticket = pageTicket; pageTicket = null;
        if (ticket) void enqueue(() => { execute({ kind: "leavePage", ticket }); });
      };
    },
    retry: () => enqueue(async () => {
      execute({ kind: "initialize" }); execute({ kind: "retry" }); execute({ kind: "context", focused, blocked });
      if (currentPage) await loadPage(pageGeneration, currentPage.entry, currentPage.page);
    }),
    settled: () => pending,
    dispose() {
      if (disposed) return;
      disposed = true; ++pageGeneration; clearHandles(); wire.close();
    },
  };
}
