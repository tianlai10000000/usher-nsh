/** Orders snapshots independently of their transport and reactive implementation. */
import type { Dispose, Envelope, Immutable, ReactiveAdapter, StateTransport, StateView, SyncStatus } from "./contracts";

export function isDecimal(value: unknown): value is string {
  return typeof value === "string" && /^(0|[1-9][0-9]*)$/.test(value);
}
export function compareSequence(left: string, right: string): number {
  if (!isDecimal(left) || !isDecimal(right)) throw new Error("state_invalid_sequence");
  return left.length === right.length ? (left === right ? 0 : left < right ? -1 : 1) : left.length < right.length ? -1 : 1;
}
function immutable<T>(value: T): Immutable<T> {
  if (value !== null && typeof value === "object") {
    for (const child of Object.values(value)) immutable(child);
    Object.freeze(value);
  }
  return value as Immutable<T>;
}

/** The owner reconnects or disposes; consumers receive only view. Old callbacks and
 * replies retain their original binding identity, even across asynchronous registration. */
export function createProjection<S, C>(initial: S, transport: StateTransport<S, C>, runtime: ReactiveAdapter) {
  const value = runtime.cell<Immutable<S>>(immutable(initial));
  const status = runtime.cell<SyncStatus>("connecting");
  let current: { active: boolean; stop?: Dispose; storeId?: string; sequence?: string } | undefined;
  let disposed = false;
  const active = (binding: typeof current) => !!binding && binding === current && binding.active && !disposed;
  function stop(binding: typeof current) {
    if (!binding) return;
    binding.active = false;
    const release = binding.stop;
    binding.stop = undefined;
    release?.();
  }
  function accept(binding: NonNullable<typeof current>, incoming: Envelope<S>) {
    if (!active(binding)) return;
    if (!isDecimal(incoming.storeId) || !isDecimal(incoming.sequence)) throw new Error("state_invalid_envelope");
    if (binding.storeId !== undefined && incoming.storeId !== binding.storeId) return;
    if (binding.sequence !== undefined && compareSequence(incoming.sequence, binding.sequence) <= 0) return;
    binding.storeId = incoming.storeId;
    binding.sequence = incoming.sequence;
    runtime.batch(() => { value.write(immutable(incoming.value)); status.write("live"); });
  }
  async function connect() {
    if (disposed) return;
    stop(current);
    const binding: NonNullable<typeof current> = { active: true };
    current = binding;
    status.write("connecting");
    try {
      const release = await transport.subscribe({
        snapshot(incoming) {
          try { accept(binding, incoming); }
          catch { if (active(binding)) { status.write("error"); stop(binding); } }
        },
        end(reason) {
          if (active(binding)) { status.write(reason); stop(binding); }
        },
      });
      if (active(binding)) binding.stop = release;
      else release();
    } catch {
      if (active(binding)) { status.write("error"); stop(binding); }
    }
  }
  async function request(run: () => Promise<Envelope<S>>) {
    const binding = current;
    if (!active(binding)) throw new Error("state_binding_closed");
    const incoming = await run();
    if (!active(binding)) throw new Error("state_binding_closed");
    try { accept(binding!, incoming); }
    catch (error) { status.write("error"); stop(binding); throw error; }
    return value.read();
  }
  const view: StateView<S, C> = Object.freeze({
    value: value.read,
    status: status.read,
    select: <T>(select: (state: Immutable<S>) => T) => runtime.select(() => select(value.read())),
    dispatch: (command: C) => request(() => transport.dispatch(command)),
    refresh: () => request(() => transport.read()),
  });
  return {
    view, connect,
    dispose() { if (!disposed) { disposed = true; stop(current); status.write("disposed"); } },
  };
}
