/** Binds authoritative appearance preferences to NSH without mirroring its effective scheme. */
import { colorScheme, effect, setColorScheme } from "nsh";
import type { Readable, SyncStatus } from "../../state-runtime/contracts";
import type { ColorMode } from "../settings/contracts";

/** Host operations are injectable for tests; production uses NSH's own reactive environment. */
export interface AppearanceHost {
  effectiveScheme: Readable<"light" | "dark">;
  applyMode(mode: ColorMode): void;
}
const nshHost: AppearanceHost = { effectiveScheme: colorScheme, applyMode: setColorScheme };

/** Bind once under the application owner. NSH disposes the effect with that owner. */
export function bindAppearanceHost(
  mode: Readable<ColorMode>,
  status: Readable<SyncStatus>,
  host: AppearanceHost = nshHost,
) {
  effect(() => {
    if (status() === "live") host.applyMode(mode());
  });
  return host.effectiveScheme;
}
