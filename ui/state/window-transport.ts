/** Encapsulates the native window wire protocol and validates serialized envelopes and receipts. */
import { currentWindow } from "nsh/desktop";
import { WindowState } from "usher/nsh";
import type { Envelope, Observer } from "./contracts";
import type {
  GuardDecision, NavigationCommand, NavigationResult, RouteLocation,
  RouteTarget, SessionCommand, SessionReceipt, WindowData,
} from "./window-contracts";

function decode(wire: { storeId: string; sequence: string; value: string }): Envelope<WindowData> {
  const value: WindowData = JSON.parse(wire.value);
  if (!value?.history || !Array.isArray(value.history.entries)
    || !value.history.entries[value.history.cursor] || !value.session) {
    throw new Error("window_invalid_snapshot");
  }
  return { storeId: wire.storeId, sequence: wire.sequence, value };
}
function decodeReceipt(wire: string): SessionReceipt {
  const value: unknown = JSON.parse(wire);
  if (typeof value === "object" && value !== null && "kind" in value) {
    if (value.kind === "applied" || value.kind === "superseded") return { kind: value.kind };
    if (value.kind === "ticket" && "ticket" in value && typeof value.ticket === "string") {
      return { kind: "ticket", ticket: value.ticket };
    }
  }
  throw new Error("window_invalid_receipt");
}
function decodeNavigation(wire: string): NavigationResult | null {
  const value: unknown = JSON.parse(wire);
  switch (value) {
    case null:
    case "committed": case "duplicated": case "cancelled": case "superseded":
    case "not_found": case "invalid_target": case "failed":
      return value;
    default:
      throw new Error("window_invalid_receipt");
  }
}

export function windowTransport(catalog: unknown) {
  // Capture the rendering window once. Async callbacks must never resolve the focused window.
  const hostWindow = currentWindow();
  const attachment = WindowState.attach(JSON.stringify(catalog), hostWindow.id);
  const initial = decode(attachment.snapshot);
  const scope = attachment.scope;
  const lifetime = initial.storeId;
  // Cover a close in this turn before the owner-independent listener installs.
  const stopInitial = hostWindow.on("close", () => WindowState.close(lifetime));
  void Promise.resolve().then(() => {
    // This listener belongs to the host window, not to an HMR projection owner.
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
              active = false;
              observer.end("cancelled");
            } else {
              observer.snapshot(snapshot);
            }
          } else if (event.end === "closed" || event.end === "cancelled" || event.end === "error") {
            observer.end(event.end);
          } else {
            observer.end("error");
          }
        } catch {
          observer.end("error");
        }
      });
      return () => { active = false; stop(); };
    },
    execute(command: SessionCommand) {
      const reply = WindowState.sessionResult(scope, JSON.stringify(command));
      return { receipt: decodeReceipt(reply.receipt), snapshot: decode(reply.snapshot) };
    },
    navigate(command: NavigationCommand): NavigationResult {
      const result = decodeNavigation(WindowState.navigate(scope, JSON.stringify(command)).receipt);
      if (result === null) throw new Error("window_invalid_receipt");
      return result;
    },
    resolve(target: RouteTarget): RouteLocation {
      return JSON.parse(WindowState.resolve(scope, JSON.stringify(target)));
    },
    begin: (command: NavigationCommand) => WindowState.begin(scope, JSON.stringify(command)),
    complete(ticket: string, decision: GuardDecision, target?: RouteTarget): NavigationResult | null {
      const redirect = target === undefined ? null : JSON.stringify(target);
      return decodeNavigation(WindowState.complete(scope, ticket, decision, redirect));
    },
    exportDiagnostics: (from: string, to: string) => WindowState.exportDiagnostics(scope, from, to),
  };
}
