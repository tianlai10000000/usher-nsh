/** Keeps NSH reactivity, owner cleanup and context behind application interfaces. */
import { batch, cleanup, computed, context, effect, signal } from "nsh";
import type { ReactiveAdapter } from "./contracts";

export const nshRuntime: ReactiveAdapter = {
  cell<T>(initial: T) {
    const value = signal(initial);
    return { read: () => value(), write: (next: T) => value.set(next) };
  },
  select: read => computed(read),
  batch,
};
export const onDispose = (dispose: () => void) => cleanup(dispose);
export const observe = (run: () => void) => effect(run);
/** State scopes expose a borrowed view; disposal remains with their creator. */
export function createScope<T>() {
  const scope = context<T | undefined>(undefined);
  return {
    Provider: scope.Provider,
    use(): T {
      const value = scope();
      if (!value) throw new Error("state_scope_missing");
      return value;
    },
  };
}
