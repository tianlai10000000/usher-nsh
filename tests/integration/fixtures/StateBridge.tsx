/** Isolated headless probe for real native subscriptions, reactive owners and borrowed context. */
import { mount, signal, Text, View } from "nsh";
import { Button } from "nsh/base";
import { Settings, SettingsState } from "usher/nsh";
import { createSettingsState, SettingsProvider, useSettingsStore } from "../../../ui/state/settings/store";
import { observe, onDispose } from "../../../ui/state-runtime/nsh";
import { decodeSettings } from "../../../ui/state/settings/decode";
import initial from "./state-initial.json";

function Borrower() {
  const view = useSettingsStore();
  const mode = view.select(state => state.colorMode);
  return <Text testID="borrowed">Borrowed:{mode()}</Text>;
}
function Consumer(props: { name: string; observed?: (mode: string) => void; disposed?: () => void }) {
  const view = createSettingsState(decodeSettings(initial));
  observe(() => { if (view.status() === "live") props.observed?.(view.value().colorMode); });
  onDispose(() => props.disposed?.());
  return <SettingsProvider value={view}>
    <Text>{props.name}:{view.value().colorMode}:{view.status()}</Text>
    <Borrower />
  </SettingsProvider>;
}
function Probe() {
  const mounted = signal(true);
  const lastB = signal("none");
  const releases = signal(0);
  const cancelledCalls = signal(0);
  const phase = signal("idle");
  return <View>
    <Consumer name="A" />
    <If when={mounted()}><Consumer name="B" observed={lastB.set} disposed={() => releases.update(n => n + 1)} /></If>
    <Text>LastB:{lastB()}</Text>
    <Text>Released:{releases()}</Text>
    <Text>Cancelled:{cancelledCalls()}</Text>
    <Text>Phase:{phase()}</Text>
    <Button onPress={async () => { await Settings.setColorMode("dark"); phase.set("dark"); }} ><Text>Dark from Rust command</Text></Button>
    <Button onPress={async () => { await Settings.setColorMode("light"); phase.set("light"); }} ><Text>Light from Rust command</Text></Button>
    <Button onPress={() => mounted.update(value => !value)} ><Text>Toggle B</Text></Button>
    <Button onPress={() => {
      const stop = SettingsState.subscribe(() => cancelledCalls.update(n => n + 1));
      stop(); stop();
    }} ><Text>Cancel registration</Text></Button>
  </View>;
}
mount(Probe);
