/** Serializable window data; navigation mutations are exposed only by ui/router. */
export type Stage = "idle" | "loading" | "saving" | "scanning" | "saved" | "load-error" | "save-error" | "scan-error" | "invalid" | "recovered" | "read-only" | "running" | "done" | "error";
export type OperationKind = "settings" | "directory" | "picker" | "export";
export type Operation = { ticket: string; stage: Stage; active: boolean; failed: boolean; error?: string | null; value?: string | null };
export type SessionData = { section: string; clicks: number; opacityPreview?: number | null; transparencyPreview?: number | null; backgroundProblem?: string | null; operations: Partial<Record<OperationKind, Operation>> };
export type RouteLocation = { id: string | null; page: string; path: string; params: Record<string, string>; query: Record<string, string[]>; ancestors: string[] };
export type RouteEntry = { key: string; location: RouteLocation };
export type NavigationResult = "committed" | "duplicated" | "cancelled" | "superseded" | "not_found" | "invalid_target" | "failed";
export type WindowData = { history: { entries: RouteEntry[]; cursor: number; catalogIdentity: string }; session: SessionData; pending: string | null; navigationResult: NavigationResult };
export type SessionCommand =
  | { kind: "section"; value: string }
  | { kind: "increment" }
  | { kind: "preview"; opacity: boolean; value: number | null; expected?: string }
  | { kind: "background_problem"; value: string | null }
  | { kind: "begin"; operation: OperationKind; stage: Stage }
  | { kind: "finish"; operation: OperationKind; ticket: string; stage: Stage; error: string | null; value: string | null; failed: boolean }
  | { kind: "observe_settings"; stage: Stage };
