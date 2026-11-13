/** Layout-owned update projection; feature code supplies content without changing the rail's dependencies. */
import type { JSX } from "nsh/jsx-runtime";
export type UpdateIndicator = {
  state(): {
    shape: "quiet" | "emphasis" | "progress";
    label: string;
    disabled: boolean;
    progress: number | null;
    ready: boolean;
    warning: boolean;
    spinning: boolean;
  };
  /** Wraps the layout-owned trigger in the feature's library popover. */
  present(trigger: JSX.Element): JSX.Element;
};
