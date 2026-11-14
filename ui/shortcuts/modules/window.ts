/** Executes window business intents without borrowing a page or the focused Router. */
import { windows } from "nsh/desktop";
import type { Command } from "../contracts";
/** Resolves the main host window at dispatch time; no component-owned ref survives reload. */
export function windowCommand(command: Command) {
  if (command.module === "window" && command.action === "focusMain") windows().find(window => window.main)?.focus();
}
