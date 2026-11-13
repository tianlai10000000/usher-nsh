/** Drives update presentation phases independently of native installation or process restarts. */
import { mount, signal, Text, View } from "nsh";
import { Button } from "nsh/base";
import { UpdatesSettings } from "../../../ui/features/updates/UpdatesSettings";
import { presentation } from "../../../ui/features/updates/indicator";
import { initial, type Phase, type UpdateData } from "../../../ui/features/updates/contracts";
import type { UpdateActions } from "../../../ui/features/updates";
import { createAppearanceStore } from "../../../ui/state/appearance";

function Preview() {
  const appearance = createAppearanceStore();
  const candidate = { version: "1.1.0", kind: "shell" } as const;
  const state = signal<UpdateData>({
    ...initial,
    domain: { shell: "1.0.0", running: "1.0.0", capability: "in_app", auto_check: true,
      phase: { type: "ready_to_install", candidate } },
  });
  const setPhase = (phase: Phase) => state.set({ ...state(), domain: { ...state().domain!, phase } });
  const updates: UpdateActions = {
    data: state,
    view: () => presentation(state()),
    error: () => null,
    check: async () => setPhase({ type: "checking" }),
    download: async () => setPhase({ type: "downloading", candidate, downloaded: "0", total: "100" }),
    install: async () => setPhase({ type: "installing", candidate }),
    autoCheck: async () => {},
    manual: async () => {},
  };
  return <View>
    <Button testID="await-launch" onPress={() => setPhase({ type: "awaiting_first_launch", candidate })}><Text>Await launch</Text></Button>
    <Button testID="fail-install" onPress={() => setPhase({ type: "install_failed", candidate, failure: "disk" })}><Text>Fail install</Text></Button>
    <UpdatesSettings appearance={appearance} updates={updates} />
  </View>;
}
mount(Preview);
