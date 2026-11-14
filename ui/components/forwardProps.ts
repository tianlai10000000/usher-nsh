/** Forwards component properties without evaluating and freezing reactive JSX accessors. */

/** Omit adapter-only fields while preserving getters for native NSH property bindings. */
export function forwardProps<T extends object, K extends keyof T>(props: T, excluded: readonly K[]): Omit<T, K> {
  const descriptors = Object.getOwnPropertyDescriptors(props);
  for (const key of excluded) delete descriptors[key];
  return Object.defineProperties({}, descriptors) as Omit<T, K>;
}
