/** Structural wire decoding primitives; business schemas remain in their owning modules. */
import type { Envelope, Observer } from "./contracts";
import { isDecimal } from "./projection";

import { SnapshotDecodeError } from "./errors";
export { SnapshotDecodeError } from "./errors";
export function requireValue(condition: unknown): asserts condition {
  if (!condition) throw new SnapshotDecodeError();
}
export function object(value: unknown): Record<string, unknown> {
  requireValue(typeof value === "object" && value !== null && !Array.isArray(value));
  return value as Record<string, unknown>;
}
export function string(value: unknown): string { requireValue(typeof value === "string"); return value; }
export function boolean(value: unknown): boolean { requireValue(typeof value === "boolean"); return value; }
export function number(value: unknown): number { requireValue(typeof value === "number" && Number.isFinite(value)); return value; }
export function integer(value: unknown): number { const n = number(value); requireValue(Number.isSafeInteger(n) && n >= 0); return n; }
export function decimal(value: unknown): string { requireValue(isDecimal(value)); return value as string; }
export function array(value: unknown): unknown[] { requireValue(Array.isArray(value)); return value; }
export function optional(value: unknown, decode: (value: unknown) => unknown) { if (value !== undefined && value !== null) decode(value); }
export function oneOf<T extends string>(value: unknown, choices: readonly T[]): T {
  requireValue(typeof value === "string" && choices.includes(value as T)); return value as T;
}
export function json(value: unknown): unknown {
  try { return JSON.parse(string(value)); } catch { throw new SnapshotDecodeError(); }
}
export function envelope<T>(wire: unknown, decode: (value: unknown) => T, serialized = false): Envelope<T> {
  const source = object(wire);
  return { storeId: decimal(source.storeId), sequence: decimal(source.sequence), value: decode(serialized ? json(source.value) : source.value) };
}
/** Ends malformed streams before any consumer can observe invalid business data. */
export function subscribeDecoded<T>(
  subscribe: (receive: (event: unknown) => void) => () => void,
  observer: Observer<T>,
  decode: (wire: unknown) => Envelope<T>,
) {
  let active = true;
  const stop = subscribe(event => {
    if (!active) return;
    try {
      const source = object(event);
      if (source.snapshot != null && source.end == null) observer.snapshot(decode(source.snapshot));
      else {
        requireValue(source.snapshot == null);
        const end = oneOf(source.end, ["closed", "cancelled", "error"]);
        active = false; observer.end(end);
      }
    } catch { active = false; observer.end("error"); }
  });
  return () => { active = false; stop(); };
}
