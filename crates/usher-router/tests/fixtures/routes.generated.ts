/** Generated route contracts. Edit the catalog, then regenerate this file. */
export type RouteId = "home" | "about" | "about-slash" | "user-new" | "user" | "files";
export type PageKey = "home" | "about" | "user" | "files" | "not-found";
export const routeMetadata = {
  ["home"]: {"id":"home","menu":null,"page":"home","parent":null,"path":"/","titleKey":"test.home"},
  ["about"]: {"id":"about","menu":null,"page":"about","parent":"home","path":"/about","titleKey":"test.about"},
  ["about-slash"]: {"id":"about-slash","menu":null,"page":"about","parent":"home","path":"/about/","titleKey":"test.about-slash"},
  ["user-new"]: {"id":"user-new","menu":null,"page":"user","parent":"home","path":"/users/new","titleKey":"test.user-new"},
  ["user"]: {"id":"user","menu":null,"page":"user","parent":"home","path":"/users/{id}","titleKey":"test.user"},
  ["files"]: {"id":"files","menu":null,"page":"files","parent":"home","path":"/files/{*rest}","titleKey":"test.files"},
} as const;
export interface RouteParams {
  "home": Record<string, never>;
  "about": Record<string, never>;
  "about-slash": Record<string, never>;
  "user-new": Record<string, never>;
  "user": { "id": string };
  "files": { "rest": string };
}
export interface RouteQuery {
  "home": { [key: string]: never; };
  "about": { [key: string]: never; };
  "about-slash": { [key: string]: never; };
  "user-new": { [key: string]: never; };
  "user": { "page"?: readonly (`${bigint}`)[]; "tag"?: readonly (string)[]; };
  "files": { [key: string]: readonly string[] | undefined; };
}
