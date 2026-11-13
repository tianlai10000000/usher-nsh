/** Layout-owned update projection; feature code supplies content without changing the rail's dependencies. */
import type { JSX } from "nsh/jsx-runtime";
export type UpdateIndicator = {
  state(): {
    visible: boolean;
    shape: "quiet" | "emphasis" | "progress" | "checking";
    label: string;
    disabled: boolean;
    progress: number | null;
    ready: boolean;
    warning: boolean;
  };
  /** Wraps the layout-owned trigger in the feature's library popover. */
  present(trigger: JSX.Element): JSX.Element;
};
