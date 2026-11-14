/** Public application shortcut facade; pages borrow leases and layout receives read-only status. */
export { createShortcuts, type Shortcuts } from "./runtime/nsh";
export { shortcutIndicator } from "./status";
export { createGlobalShortcuts } from "./runtime/global";
