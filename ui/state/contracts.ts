/** Framework-independent read, transport, lifecycle and reactive adapter contracts. */
export type Immutable<T> = T extends (...args: never[]) => unknown ? T
  : T extends object ? { readonly [K in keyof T]: Immutable<T[K]> } : T;
export interface Readable<T> { (): T }
export interface Cell<T> { read: Readable<T>; write(value: T): void }
export interface ReactiveAdapter {
  cell<T>(initial: T): Cell<T>;
  select<T>(read: () => T): Readable<T>;
  batch<T>(run: () => T): T;
}
/** Identity and sequence use canonical unsigned decimal strings, never JS numbers. */
export interface Envelope<S> { readonly storeId: string; readonly sequence: string; readonly value: S }
export type End = "closed" | "cancelled" | "error";
export interface Observer<S> { snapshot(value: Envelope<S>): void; end(reason: End): void }
export type Dispose = () => void;
/** Registration atomically captures the initial snapshot and cursor; delivery may be
 * asynchronous. Disposers are idempotent and release only this observer. Domain command
 * rejection does not terminate synchronization; terminal events close the binding.
 * Reads and command receipts return consistent envelopes, never unversioned cached data. */
export interface StateTransport<S, C> {
  subscribe(observer: Observer<S>): Dispose | Promise<Dispose>;
  read(): Promise<Envelope<S>>;
  dispatch(command: C): Promise<Envelope<S>>;
}
export type SyncStatus = "connecting" | "live" | End | "disposed";
/** Borrowed by consumers. Only the application owner receives dispose/connect. */
export interface StateView<S, C> {
  readonly value: Readable<Immutable<S>>;
  readonly status: Readable<SyncStatus>;
  select<T>(select: (value: Immutable<S>) => T): Readable<T>;
  dispatch(command: C): Promise<Immutable<S>>;
  refresh(): Promise<Immutable<S>>;
}
