/** Checks full VM replacement using production stores and an isolated durable checkpoint. */
import { mount, signal, Storage, Text, View } from "nsh";
import { Button } from "nsh/base";
import { currentWindow } from "nsh/desktop";
import { WindowState } from "usher/nsh";
import { createWindowState } from "../../../ui/state/window";
import { createAppearanceStore } from "../../../ui/state/appearance";
import { createRouter } from "../../../ui/router";
import catalog from "../../../ui/pages/routes.json";
import config from "../native/config.json";
function Probe() {
  const window = createWindowState();
  const appearance = createAppearanceStore(window.session);
  const router = createRouter(window.routing);
  const scope = WindowState.register(JSON.stringify(catalog), currentWindow().id).storeId;
  const checkpoint = signal("none");
  const oldCallback = signal("none");
  void Storage.getItem(config.reloadCheckpointKey).then(saved => {
    if (!saved) return;
    const previous = JSON.parse(saved) as { scope: string; ticket: string };
    if (previous.scope === scope) return;
    try { WindowState.complete(previous.scope, previous.ticket, "allow", null); oldCallback.set("accepted"); }
    catch { oldCallback.set("rejected"); }
  });
  async function prepare() {
    await window.session.change({ kind: "increment" });
    await window.session.preview(true, config.preview);
    await window.session.change({ kind: "section", value: config.section });
    await appearance.state.dispatch({ type: "colorMode", value: "dark" });
    await router.navigate("settings");
  }
  return <View>
    <Text>{`Store:${scope}`}</Text>
    <Text>{`Sync:${window.routing.status()}/${appearance.state.status()}`}</Text>
    <Text>{`Color:${appearance.state.value().colorMode}`}</Text>
    <Text>{`Saved:${appearance.state.value().saved}`}</Text>
    <Text>{`Count:${window.session.value().clicks}`}</Text>
    <Text>{`Section:${window.session.value().section}`}</Text>
    <Text>{`Preview:${window.session.value().opacityPreview ?? "none"}`}</Text>
    <Text>{`Path:${router.current().location.path}`}</Text>
    <Text>{`Pending:${router.pending() !== null}`}</Text>
    <Text>{`Checkpoint:${checkpoint()}`}</Text>
    <Text>{`Old callback:${oldCallback()}`}</Text>
    <Button onPress={() => void prepare()}><Text>Prepare retained and transient data</Text></Button>
    <Button onPress={() => { router.beforeEach(() => new Promise<boolean>(() => {})); void router.navigate("home"); }}><Text>Begin pending guard</Text></Button>
    <Button onPress={() => void Storage.setItem(config.reloadCheckpointKey, JSON.stringify({ scope, ticket: window.routing.value().pending })).then(() => checkpoint.set("saved"))}><Text>Save callback checkpoint</Text></Button>
  </View>;
}
mount(Probe);
