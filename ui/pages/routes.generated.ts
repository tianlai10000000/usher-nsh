/** Generated route contracts. Edit the catalog, then regenerate this file. */
export type RouteId = "home" | "settings";
export type PageKey = "home" | "settings" | "not-found";
export const routeMetadata = {
  ["home"]: {"id":"home","menu":"primary","page":"home","parent":null,"path":"/","titleKey":"nav.home"},
  ["settings"]: {"id":"settings","menu":"settings","page":"settings","parent":"home","path":"/settings","titleKey":"nav.settings"},
} as const;
export interface RouteParams {
  "home": Record<string, never>;
  "settings": Record<string, never>;
}
export interface RouteQuery {
  "home": { [key: string]: never; };
  "settings": { [key: string]: never; };
}
