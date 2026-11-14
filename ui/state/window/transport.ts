/** Encapsulates the native window wire protocol and validates serialized envelopes and receipts. */
import { decodeWindowEnvelope as decode, decodeLocation, decodeReceipt, decodeNavigation } from "./decode";
import { subscribeWindow } from "./stream";
import { json } from "../../state-runtime/decode";
import { currentWindow } from "nsh/desktop";
import { WindowState } from "usher/nsh";
import type { Observer } from "../../state-runtime/contracts";
import type {
  GuardDecision, NavigationCommand, NavigationResult, RouteLocation,
  RouteTarget, SessionCommand, WindowData,
} from "./contracts";

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
    read: async () => decode(WindowState.snapshot(scope), initial.value.history.catalogIdentity),
    subscribe: (observer: Observer<WindowData>) => subscribeWindow(
      receive => WindowState.subscribe(scope, receive), observer, initial.value.history.catalogIdentity,
    ),
    execute(command: SessionCommand) {
      const reply = WindowState.sessionResult(scope, JSON.stringify(command));
      return { receipt: decodeReceipt(reply.receipt), snapshot: decode(reply.snapshot, initial.value.history.catalogIdentity) };
    },
    navigate(command: NavigationCommand): NavigationResult {
      const result = decodeNavigation(WindowState.navigate(scope, JSON.stringify(command)).receipt);
      if (result === null) throw new Error("window_invalid_receipt");
      return result;
    },
    resolve(target: RouteTarget): RouteLocation {
      return decodeLocation(json(WindowState.resolve(scope, JSON.stringify(target))));
    },
    begin: (command: NavigationCommand) => WindowState.begin(scope, JSON.stringify(command)),
    complete(ticket: string, decision: GuardDecision, target?: RouteTarget): NavigationResult | null {
      const redirect = target === undefined ? null : JSON.stringify(target);
      return decodeNavigation(WindowState.complete(scope, ticket, decision, redirect));
    },
    exportDiagnostics: (from: string, to: string) => WindowState.exportDiagnostics(scope, from, to),
  };
}
