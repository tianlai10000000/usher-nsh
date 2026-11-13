/** Generated route contracts. Edit the catalog, then regenerate this file. */
export type RouteId = "home" | "about" | "about-slash" | "user-new" | "user" | "files";
export type PageKey = "home" | "about" | "user" | "files" | "not-found";
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
