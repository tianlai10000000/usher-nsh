/** Public read-only state contracts and application-scoped settings access. */
export type { Immutable, Readable, StateView, SyncStatus } from "./contracts";
export type { SettingsCommand, SettingsData } from "./settings";
export { useSettingsStore } from "./app";
