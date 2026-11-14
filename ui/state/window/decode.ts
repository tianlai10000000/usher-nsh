/** Validates complete window snapshots and route locations before reactive publication. */
import * as d from "../../state-runtime/decode";
import type { WindowData, RouteLocation, SessionReceipt, NavigationResult } from "./contracts";
import background from "../../config/background.json";
import config from "../../config/session.json";
const results = ["committed", "duplicated", "cancelled", "superseded", "not_found", "invalid_target", "failed"] as const;
export function decodeLocation(value: unknown): RouteLocation {
  const route = d.object(value);
  d.optional(route.id, d.string); d.string(route.page); d.string(route.path);
  for (const param of Object.values(d.object(route.params))) d.string(param);
  for (const query of Object.values(d.object(route.query))) d.array(query).forEach(d.string);
  d.array(route.ancestors).forEach(d.string);
  return value as RouteLocation;
}
export function decodeWindow(value: unknown): WindowData {
  const state = d.object(value); const history = d.object(state.history);
  const entries = d.array(history.entries);
  const keys = new Set<string>();
  for (const item of entries) {
    const entry = d.object(item); const key = d.string(entry.key);
    d.requireValue(!keys.has(key)); keys.add(key); decodeLocation(entry.location);
  }
  d.requireValue(d.integer(history.cursor) < entries.length); d.string(history.catalogIdentity);
  d.optional(state.pending, d.decimal); d.oneOf(state.navigationResult, results);
  const session = d.object(state.session);
  d.oneOf(session.section, config.sections); d.integer(session.clicks);
  d.optional(session.opacityPreview, value => {
    const number = d.number(value);
    d.requireValue(number >= background.minOpacity && number <= background.maxOpacity);
  });
  d.optional(session.transparencyPreview, value => {
    const number = d.number(value);
    d.requireValue(number >= background.contentTransparency.min && number <= background.contentTransparency.max);
  });
  d.optional(session.backgroundProblem, d.string);
  const operations = d.object(session.operations);
  for (const [key, item] of Object.entries(operations)) {
    d.oneOf(key, ["settings", "directory", "picker", "export"]);
    const operation = d.object(item);
    d.string(operation.ticket); d.boolean(operation.active); d.boolean(operation.failed);
    if (operation.ticket !== "" || operation.active) {
      d.decimal(operation.ticket);
      d.requireValue(operation.ticket !== "0");
    }
    d.oneOf(operation.stage, ["idle", "loading", "saving", "scanning", "saved", "load-error", "save-error", "scan-error", "invalid", "recovered", "read-only", "running", "done", "error"]);
    d.optional(operation.error, d.string); d.optional(operation.value, d.string);
  }
  return value as WindowData;
}
export function decodeWindowEnvelope(wire: unknown, catalogIdentity?: string) {
  const snapshot = d.envelope(wire, decodeWindow, true);
  if (catalogIdentity !== undefined) d.requireValue(snapshot.value.history.catalogIdentity === catalogIdentity);
  return snapshot;
}

/** Receipts are protocol data and must be rejected before starting or completing an operation. */
export function decodeReceipt(wire: unknown): SessionReceipt {
  const value = d.object(d.json(wire));
  const kind = d.oneOf(value.kind, ["applied", "superseded", "ticket"]);
  if (kind === "ticket") {
    const ticket = d.decimal(value.ticket);
    d.requireValue(ticket !== "0");
    return { kind, ticket };
  }
  return { kind };
}
export function decodeNavigation(wire: unknown): NavigationResult | null {
  const value = d.json(wire);
  return value === null ? null : d.oneOf(value, results);
}
