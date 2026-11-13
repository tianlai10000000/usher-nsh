/** Assembles the window projection and separates session and router write capabilities. */
import catalog from "../pages/routes.json";
import { createProjection } from "./projection";
import { nshRuntime, onDispose } from "./runtime/nsh";
import { windowTransport } from "./window-transport";
import type {
  GuardDecision, NavigationCommand, OperationKind, RouteTarget, SessionCommand, Stage, WindowData,
} from "./window-contracts";

export function createWindowState(routes: unknown = catalog) {
  const wire = windowTransport(routes);
  const owner = createProjection<WindowData, SessionCommand>(wire.initial.value, {
    subscribe: wire.subscribe,
    read: wire.read,
    dispatch: async command => wire.execute(command).snapshot,
  }, nshRuntime);
  let disposed = false;
  const operations = new Map<OperationKind, string>();
  onDispose(() => {
    for (const [operation, ticket] of operations) {
      try {
        wire.execute({
          kind: "finish", operation, ticket,
          stage: operation === "settings" ? "load-error" : "error",
          error: "operation_cancelled", value: null, failed: true,
        });
      } catch {
        // The VM may already have closed its local service.
      }
    }
    operations.clear();
    disposed = true;
    owner.dispose();
  });
  void owner.connect();
  const ensureOpen = () => {
    if (disposed) throw new Error("state_binding_closed");
  };
  const session = {
    value: owner.view.select(value => value.session),
    change(command: SessionCommand) {
      ensureOpen();
      return owner.view.dispatch(command);
    },
    async preview(opacity: boolean, value: number | null, expected?: string) {
      ensureOpen();
      const reply = wire.execute({ kind: "preview", opacity, value, expected });
      await owner.view.refresh();
      return reply.receipt;
    },
    async begin(operation: OperationKind, stage: Stage) {
      ensureOpen();
      const receipt = wire.execute({ kind: "begin", operation, stage }).receipt;
      if (receipt.kind !== "ticket") throw new Error("window_invalid_receipt");
      operations.set(operation, receipt.ticket);
      await owner.view.refresh();
      return receipt.ticket;
    },
    async finish(
      operation: OperationKind, ticket: string, stage: Stage,
      error: string | null = null, value: string | null = null, failed = false,
    ) {
      ensureOpen();
      await owner.view.dispatch({ kind: "finish", operation, ticket, stage, error, value, failed });
      if (operations.get(operation) === ticket) operations.delete(operation);
    },
    async exportDiagnostics(from: string, to: string) {
      ensureOpen();
      await wire.exportDiagnostics(from, to);
      await owner.view.refresh();
    },
  };
  // Only the router adapter receives these methods. State consumers borrow session.
  const routing = {
    value: owner.view.value,
    status: owner.view.status,
    async navigate(command: NavigationCommand) {
      ensureOpen();
      const result = wire.navigate(command);
      await owner.view.refresh();
      return result;
    },
    resolve: wire.resolve,
    begin: wire.begin,
    async complete(ticket: string, decision: GuardDecision, target?: RouteTarget) {
      ensureOpen();
      const result = wire.complete(ticket, decision, target);
      await owner.view.refresh();
      return result;
    },
  };
  return { session, routing };
}
export type WindowSession = ReturnType<typeof createWindowState>["session"];
export type RoutingBinding = ReturnType<typeof createWindowState>["routing"];
