/** Public router facade: all navigation reaches the same window-owned Rust history. */
import { routeMetadata } from "../pages/routes.generated";
import type { PageKey, RouteId, RouteParams, RouteQuery } from "../pages/routes.generated";
import type { MessageKey } from "../i18n/index";
import { createScope, onDispose } from "../state-runtime/nsh";
import { createWindowState, type RoutingBinding } from "../state/window/index";
import type { NavigationCommand, NavigationResult, RouteLocation, RouteTarget } from "../state/window/contracts";
export type { RouteId } from "../pages/routes.generated";
export type Target = RouteTarget;
export type Command = NavigationCommand;
export type Guard = (target: RouteLocation) => boolean | Target | Promise<boolean | Target>;
export const routes = routeMetadata satisfies Record<RouteId, {
  id: RouteId;
  path: string;
  page: PageKey;
  titleKey: MessageKey;
  parent: RouteId | null;
  menu: string | null;
}>;
export const routeList = Object.values(routes);
export function isRouteId(id: string): id is RouteId {
  return Object.hasOwn(routes, id);
}

function navigationFailure(error: unknown): NavigationResult {
  const code = error instanceof Error ? error.message : String(error);
  return code === "router_invalid_target" ? "invalid_target" : "failed";
}

export function createRouter(
  binding: RoutingBinding = createWindowState().routing,
  release: (dispose: () => void) => void = onDispose,
) {
  const guards = new Set<Guard>();
  const pending = new Set<string>();
  let disposed = false;

  async function closeTicket(ticket: string, decision: "cancel" | "fail") {
    try {
      await binding.complete(ticket, decision);
    } catch {
      // A closed transport cannot accept a terminal reply.
    }
  }
  release(() => {
    disposed = true;
    guards.clear();
    for (const ticket of pending) void closeTicket(ticket, "cancel");
    pending.clear();
  });
  const current = () => {
    const { history } = binding.value();
    return history.entries[history.cursor];
  };
  function targetFor(command: Command): RouteLocation {
    if (command.kind !== "go") return binding.resolve(command.target);
    const { history } = binding.value();
    return history.entries[history.cursor + command.target]?.location ?? current().location;
  }
  async function evaluateGuards(target: RouteLocation): Promise<boolean | Target> {
    for (const guard of guards) {
      const decision = await guard(target);
      if (disposed) return false;
      if (decision !== true) return decision;
    }
    return true;
  }
  async function runGuards(ticket: string, target: RouteLocation): Promise<NavigationResult> {
    // Rust owns redirect limits, loop detection and supersession.
    for (;;) {
      const decision = await evaluateGuards(target);
      if (disposed) return "cancelled";
      if (typeof decision === "boolean") {
        return await binding.complete(ticket, decision ? "allow" : "cancel") ?? "failed";
      }
      const terminal = await binding.complete(ticket, "redirect", decision);
      if (terminal !== null) return terminal;
      target = binding.resolve(decision);
    }
  }
  async function run(command: Command): Promise<NavigationResult> {
    if (disposed) return "cancelled";
    let ticket: string | undefined;
    try {
      if (!guards.size) return await binding.navigate(command);
      ticket = binding.begin(command);
      pending.add(ticket);
      return await runGuards(ticket, targetFor(command));
    } catch (error) {
      if (ticket && !disposed) await closeTicket(ticket, "fail");
      return ticket ? "failed" : navigationFailure(error);
    } finally {
      if (ticket) pending.delete(ticket);
    }
  }
  const navigate = (id: RouteId) => run({ kind: "push", target: { kind: "named", id } });
  return {
    current,
    status: binding.status,
    result: () => binding.value().navigationResult,
    pending: () => binding.value().pending,
    route: () => current().location.id,
    breadcrumbs: () => {
      const location = current().location;
      return location.ancestors.filter(isRouteId).map(id => ({
        id, titleKey: routes[id].titleKey, current: id === location.id,
      }));
    },
    canGoBack: () => binding.value().history.cursor > 0,
    canGoForward: () => binding.value().history.cursor + 1 < binding.value().history.entries.length,
    navigate,
    open: (target: string) => isRouteId(target) ? navigate(target) : Promise.resolve<NavigationResult>("invalid_target"),
    push: (target: Target) => run({ kind: "push", target }),
    replace: (target: Target) => run({ kind: "replace", target }),
    named: <I extends RouteId>(id: I, params: RouteParams[I], query?: RouteQuery[I]) =>
      run({ kind: "push", target: { kind: "named", id, params, query } }),
    go: (delta: number) => run({ kind: "go", target: delta }),
    goBack: () => run({ kind: "go", target: -1 }),
    goForward: () => run({ kind: "go", target: 1 }),
    resolve: (target: Target) => binding.resolve(target),
    beforeEach(guard: Guard) {
      guards.add(guard);
      return () => { guards.delete(guard); };
    },
  };
}
export type Navigation = ReturnType<typeof createRouter>;
const scope = createScope<Navigation>();
export const RouterProvider = scope.Provider;
export const useRouter = () => scope.use();
export const useRoute = () => {
  const router = scope.use();
  return () => {
    const entry = router.current();
    return { ...entry.location, entryKey: entry.key, pending: router.pending() };
  };
};
