/** Keeps local acceptance independent of production system key assignments. */
import * as native from "usher/nsh";
import { createGlobalShortcuts } from "../../../ui/shortcuts";
import { globalTransport } from "../../../ui/shortcuts/global-transport";
export function initializeFixtureGlobals() {
  const fixture = (native as typeof native & { ShortcutFixture: { global(): unknown } }).ShortcutFixture;
  createGlobalShortcuts(globalTransport(fixture.global()));
}
