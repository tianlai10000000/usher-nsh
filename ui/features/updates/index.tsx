/** Composes borrowed update projections and retains one native executor per Rust scope in this VM. */
import * as native from "nsh/updater";
import { appInfo, openUrl } from "nsh";
import type { AppearanceStore } from "../../state/appearance";
import { createProjection } from "../../state/projection";
import { nshRuntime, onDispose } from "../../state/runtime/nsh";
import { initial } from "./contracts";
import config from "../../config/updates.json";
import { bridge } from "./transport";
import { createController } from "./controller";
import { presentation } from "./indicator";
import { UpdatePanel } from "./UpdatePanel";
import type { JSX } from "nsh/jsx-runtime";
import type { MessageKey } from "../../i18n";

type Shared = {
  controller: ReturnType<typeof createController>;
  users: number;
  error: ReturnType<typeof nshRuntime.cell<MessageKey | null>>;
};
// A surviving VM keeps the same executor object through view/module reloads; this is not lease handover.
const host = globalThis as typeof globalThis & {
  __usherUpdates?: Map<string, Shared>;
};
const controllers = host.__usherUpdates ??= new Map();
/** Call inside a component owner; disposal releases its projection and shared executor demand. */
export function createUpdates(
  appearance: AppearanceStore,
  updater: typeof native = native,
) {
  const owner = createProjection(initial, bridge, nshRuntime);
  const error = nshRuntime.cell<MessageKey | null>(null);
  let shared: Shared | undefined;
  let disposed = false;
  void owner.connect();
  void bridge.read().then((snapshot) => {
    if (disposed) return;
    shared = controllers.get(snapshot.storeId);
    if (!shared) {
      const problem = nshRuntime.cell<MessageKey | null>(null);
      shared = {
        users: 0,
        error: problem,
        controller: createController(
          bridge,
          updater,
          config,
          (value) => problem.write(value as MessageKey | null),
        ),
      };
      controllers.set(snapshot.storeId, shared);
    }
    shared.users++;
    return shared.controller.start();
  }).catch(() => error.write("update.connectionFailed"));
  onDispose(() => {
    disposed = true;
    owner.dispose();
    if (shared && --shared.users === 0) shared.controller.detach();
  });
  const view = () => presentation(owner.view.value(), appInfo().development);
  async function dispatch(command: Parameters<typeof owner.view.dispatch>[0]) {
    error.write(null);
    try {
      await owner.view.dispatch(command);
    } catch (failure) {
      error.write(
        String(failure).includes("updates_settings_unsaved")
          ? "update.saveFailed"
          : "update.connectionFailed",
      );
    }
  }
  const updates = {
    data: owner.view.value,
    view,
    error: () =>
      error.read() ?? shared?.error.read() ??
        (owner.view.status() === "error" ? "update.connectionFailed" : null),
    check: () => dispatch({ type: "check", background: false }),
    download: () => dispatch({ type: "download" }),
    install: () => dispatch({ type: "install" }),
    autoCheck: (enabled: boolean) => dispatch({ type: "auto_check", enabled }),
    async manual() {
      const url = owner.view.value().info.manual_url;
      if (url?.startsWith("https://")) {
        try {
          await openUrl(url);
        } catch {
          error.write("update.openFailed");
        }
      }
    },
  };
  return {
    ...updates,
    indicator: {
      state: () => ({
        ...view(),
        label: appearance.t(view().indicatorKey, {
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
