/** Controlled transport replies and a runtime-free reactive adapter for projection contracts. */
import type { Dispose, Envelope, Observer, ReactiveAdapter, StateTransport } from "../../../state-runtime/contracts";

export interface Data { count: number; revision: number; saved: boolean; nested: { label: string } }
export const initial = (): Data => ({ count: 0, revision: 0, saved: true, nested: { label: "initial" } });
export const envelope = (sequence: string, count: number, storeId = "1"): Envelope<Data> => ({
  storeId, sequence, value: { count, revision: 1, saved: true, nested: { label: sequence } },
});
export function deferred<T>() {
  let resolve!: (value: T) => void;
  let reject!: (error: Error) => void;
  const promise = new Promise<T>((yes, no) => { resolve = yes; reject = no; });
  return { promise, resolve, reject };
}
export const plainRuntime: ReactiveAdapter = {
  cell<T>(initial: T) { let value = initial; return { read: () => value, write: (next: T) => { value = next; } }; },
  select: read => read,
  batch: run => run(),
};
export function controlled() {
  const observers: Observer<Data>[] = [];
  const registrations: ReturnType<typeof deferred<Dispose>>[] = [];
  const replies: ReturnType<typeof deferred<Envelope<Data>>>[] = [];
  let stops = 0;
  const transport: StateTransport<Data, number> = {
    subscribe(observer) { observers.push(observer); const next = deferred<Dispose>(); registrations.push(next); return next.promise; },
    read() { const next = deferred<Envelope<Data>>(); replies.push(next); return next.promise; },
    dispatch() { return this.read(); },
  };
  return { transport, observers, registrations, replies, stops: () => stops,
    finish(index: number) { registrations[index].resolve(() => { stops += 1; }); },
  };
}
