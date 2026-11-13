/** Encapsulates the native window wire protocol and validates its serialized envelope. */
import { currentWindow } from "nsh/desktop";
import { WindowState } from "usher/nsh";
import type { Envelope, Observer } from "./contracts";
import type { SessionCommand, WindowData } from "./window-contracts";
function decode(wire: { storeId: string; sequence: string; value: string }): Envelope<WindowData> {
  const value = JSON.parse(wire.value) as WindowData;
  if (!value?.history || !Array.isArray(value.history.entries) || !value.history.entries[value.history.cursor] || !value.session) throw new Error("window_invalid_snapshot");
  return { storeId: wire.storeId, sequence: wire.sequence, value };
}
export function windowTransport(catalog: unknown) {
  // Capture the rendering window once. Async callbacks must never resolve the focused window.
  const hostWindow = currentWindow();
  const attachment = WindowState.attach(JSON.stringify(catalog), hostWindow.id);
  const initial = decode(attachment.snapshot);
  const scope = attachment.scope;
  const lifetime = initial.storeId;
  // Cover a close in this same turn, before the owner-independent listener can install.
  const stopInitial = hostWindow.on("close", () => WindowState.close(lifetime));
  void Promise.resolve().then(() => {
    // NSH callbacks outside rendering have no reactive owner. This single listener belongs
    // to the host window and is removed by WindowRef when that opening closes, not by HMR.
    if (WindowState.claimCloseListener(lifetime)) {
      hostWindow.on("close", () => WindowState.close(lifetime));
    }
    stopInitial();
  });
  return {
    initial,
    read: async () => decode(WindowState.snapshot(scope)),
    subscribe(observer: Observer<WindowData>) {
      let active = true;
      const stop = WindowState.subscribe(scope, event => {
        if (!active) return;
        try {
          if (event.snapshot && !event.end) {
            const snapshot = decode(event.snapshot);
            if (snapshot.value.history.catalogIdentity !== initial.value.history.catalogIdentity) {
              active = false; observer.end("cancelled");
            } else observer.snapshot(snapshot);
          }
          else if (event.end === "closed" || event.end === "cancelled" || event.end === "error") observer.end(event.end);
          else observer.end("error");
        } catch { observer.end("error"); }
      });
      return () => { active = false; stop(); };
    },
    execute(command: SessionCommand) { const reply = WindowState.session(scope, JSON.stringify(command)); return { receipt: reply.receipt, snapshot: decode(reply.snapshot) }; },
    navigate: (command: unknown) => JSON.parse(WindowState.navigate(scope, JSON.stringify(command)).receipt),
    resolve: (target: unknown) => JSON.parse(WindowState.resolve(scope, JSON.stringify(target))),
    begin: (command: unknown) => WindowState.begin(scope, JSON.stringify(command)),
    complete: (ticket: string, decision: string, target?: unknown) => JSON.parse(WindowState.complete(scope, ticket, decision, target === undefined ? null : JSON.stringify(target))),
    exportDiagnostics: (from: string, to: string) => WindowState.exportDiagnostics(scope, from, to),
  };
}
