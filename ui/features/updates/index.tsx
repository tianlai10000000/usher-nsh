/** Renders update controls using the application-owned update module. */
import * as native from "nsh/updater";
import type { AppState } from "../../state/app";
import { createUpdateState } from "../../state/updates";
import { UpdatePanel } from "./UpdatePanel";
import type { JSX } from "nsh/jsx-runtime";
export function createUpdates(appearance: AppState, updater: typeof native = native) {
  const updates = createUpdateState(updater);
  const view = updates.view;
  return {
    ...updates,
    indicator: {
      state: () => ({
        ...view(),
        label: appearance.preferences.t(view().indicatorKey, {
          version: view().candidate?.version ?? "",
        }),
      }),
      present: (trigger: JSX.Element) => (
        <UpdatePanel
          appearance={appearance}
          updates={updates}
          trigger={trigger}
        />
      ),
    },
  };
}
export type Updates = ReturnType<typeof createUpdates>;
export type UpdateActions = Omit<Updates, "indicator">;
