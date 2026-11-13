/** Public router facade: all navigation reaches the same window-owned Rust history. */
import catalog from "../pages/routes.json";
import type { RouteId, RouteParams, RouteQuery } from "../pages/routes.generated";
import type { MessageKey } from "../i18n";
import { createScope, onDispose } from "../state/runtime/nsh";
import { createWindowState, type RoutingBinding } from "../state/window";
import type { NavigationResult, RouteLocation } from "../state/window-contracts";
export type { RouteId } from "../pages/routes.generated";
export type Target = { kind: "path"; location: string } | { kind: "named"; id: string; params?: Record<string, string>; query?: Record<string, readonly string[]> };
export type Command = { kind: "push" | "replace"; target: Target } | { kind: "go"; target: number };
export type Guard = (target: RouteLocation) => boolean | Target | Promise<boolean | Target>;
export const routes = Object.fromEntries(catalog.routes.map(route => [route.id, route])) as unknown as Record<RouteId, { id: RouteId; path: string; page: string; titleKey: MessageKey; parent: RouteId | null; menu: string | null }>;
export const routeList = Object.values(routes);
export function isRouteId(id: string): id is RouteId { return Object.hasOwn(routes, id); }
export function createRouter(binding: RoutingBinding = createWindowState().routing, release: (dispose: () => void) => void = onDispose) {
  const guards = new Set<Guard>();
  let disposed = false;
  const pending = new Set<string>();
  release(() => { disposed = true; guards.clear(); for (const ticket of pending) void binding.complete(ticket, "cancel").catch(() => {}); pending.clear(); });
  const current = () => binding.value().history.entries[binding.value().history.cursor];
  async function run(command: Command): Promise<NavigationResult> {
    if (disposed) return "cancelled";
    let ticket: string | undefined;
    try {
      if (!guards.size) return await binding.navigate(command) as NavigationResult;
      ticket = binding.begin(command); pending.add(ticket);
      let target = command.kind === "go"
        ? binding.value().history.entries[binding.value().history.cursor + command.target]?.location ?? current().location
        : binding.resolve(command.target) as RouteLocation;
      // Redirect accounting and loop detection are authoritative in Rust.
      for (;;) {
        let redirected = false;
        for (const guard of guards) {
          const result = await guard(target as RouteLocation);
          if (disposed) return "cancelled";
          if (result === false) return await binding.complete(ticket, "cancel");
          if (result !== true) {
            const terminal = await binding.complete(ticket, "redirect", result);
            if (terminal !== null) return terminal;
            target = binding.resolve(result) as RouteLocation;
            redirected = true; break;
          }
        }
        if (!redirected) return await binding.complete(ticket, "allow");
      }
    } catch {
      if (ticket && !disposed) {
        try { await binding.complete(ticket, "fail"); }
        catch { /* A closed transport cannot accept a terminal reply. */ }
      }
      return ticket ? "failed" : "invalid_target";
    } finally { if (ticket) pending.delete(ticket); }
  }
  const navigate = (id: RouteId) => run({ kind: "push", target: { kind: "named", id } });
  return {
    current, status: binding.status,
    result: () => binding.value().navigationResult,
    pending: () => binding.value().pending,
    route: () => current().location.id,
    breadcrumbs: () => current().location.ancestors.filter(isRouteId).map(id => ({ id, titleKey: routes[id].titleKey, current: id === current().location.id })),
    canGoBack: () => binding.value().history.cursor > 0,
    canGoForward: () => binding.value().history.cursor + 1 < binding.value().history.entries.length,
    navigate,
    open: (target: string) => isRouteId(target) ? navigate(target) : Promise.resolve<NavigationResult>("invalid_target"),
    push: (target: Target) => run({ kind: "push", target }),
    replace: (target: Target) => run({ kind: "replace", target }),
    named: <I extends RouteId>(id: I, params: RouteParams[I], query?: RouteQuery[I]) => run({ kind: "push", target: { kind: "named", id, params, query } }),
    go: (delta: number) => run({ kind: "go", target: delta }),
    goBack: () => run({ kind: "go", target: -1 }),
    goForward: () => run({ kind: "go", target: 1 }),
    resolve: (target: Target) => binding.resolve(target) as RouteLocation,
    beforeEach(guard: Guard) { guards.add(guard); return () => { guards.delete(guard); }; },
  };
}
export type Navigation = ReturnType<typeof createRouter>;
const scope = createScope<Navigation>();
export const RouterProvider = scope.Provider;
export const useRouter = () => scope.use();
export const useRoute = () => { const router = scope.use(); return () => ({ ...router.current().location, entryKey: router.current().key, pending: router.pending() }); };
