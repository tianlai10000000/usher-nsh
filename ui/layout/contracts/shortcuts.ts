/** Layout-owned read-only shortcut health contract; initialization never depends on this view. */
export type ShortcutIndicator = {
  summary(): string;
  pageSummary(): string;
  warning(): boolean;
  details(): ReadonlyArray<{ id: string; label: string; value: string }>;
  retryable(): boolean;
  retry(): Promise<void>;
};
