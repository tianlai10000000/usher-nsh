import { computed, signal } from "nsh";
import type { MessageKey } from "../i18n";

export type RouteId = "home" | "settings";

type RouteDefinition = {
  parent: RouteId | null;
  titleKey: MessageKey;
};

export const routes = {
  home: { parent: null, titleKey: "nav.home" },
  settings: { parent: "home", titleKey: "nav.settings" },
} as const satisfies Record<RouteId, RouteDefinition>;

type Breadcrumb = {
  id: RouteId;
  titleKey: MessageKey;
  current: boolean;
};

type History = {
  entries: readonly RouteId[];
  index: number;
};

function resolveRoute(target: string): RouteId {
  return Object.hasOwn(routes, target) ? target as RouteId : "home";
}

export function createNavigation(initialRoute = "home") {
  // Each shell owns its cursor and history, which advance together.
  const history = signal<History>({ entries: [resolveRoute(initialRoute)], index: 0 });
  const route = computed(() => {
    const { entries, index } = history();
    return entries[index];
  });
  const canGoBack = computed(() => history().index > 0);
  const canGoForward = computed(() => {
    const { entries, index } = history();
    return index + 1 < entries.length;
  });
  const breadcrumbs = computed<readonly Breadcrumb[]>(() => {
    const active = route();
    const trail: Breadcrumb[] = [];
    for (let id: RouteId | null = active; id !== null; id = routes[id].parent) {
      trail.push({ id, titleKey: routes[id].titleKey, current: id === active });
    }
    return trail.reverse();
  });

  function navigate(target: RouteId): boolean {
    const { entries, index } = history();
    if (entries[index] === target) return false;

    history.set({
      entries: [...entries.slice(0, index + 1), target],
      index: index + 1,
    });
    return true;
  }

  function move(delta: -1 | 1): boolean {
    const current = history();
    const index = current.index + delta;
    if (index < 0 || index >= current.entries.length) return false;

    history.set({ ...current, index });
    return true;
  }

  return {
    route,
    breadcrumbs,
    canGoBack,
    canGoForward,
    navigate,
    open: (target: string) => navigate(resolveRoute(target)),
    goBack: () => move(-1),
    goForward: () => move(1),
  };
}

export type Navigation = ReturnType<typeof createNavigation>;
