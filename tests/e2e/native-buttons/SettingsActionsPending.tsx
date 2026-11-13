/** Cancels the real AppKit picker through a test-only host extension, retaining production settings. */
import { cleanup, mount, signal, Text, View } from "nsh";
import { Button } from "nsh/base";
import * as native from "usher/nsh";
import { SettingsActionsPending } from "../../ui/fixtures/SettingsActionsPendingView";
import config from "../native/config.json";

// The isolated runner adds this module; it is absent from the production registry.
const picker = (native as typeof native & {
  NativePicker: { cancelVisible(): boolean; visible(): boolean };
}).NativePicker;

function PickerControl() {
  const state = signal("idle");
  const cancellations = signal(0);
  let disposed = false;
  cleanup(() => { disposed = true; });
  const frame = () => new Promise<void>(resolve => requestAnimationFrame(() => resolve()));
  async function cancelNext() {
    if (state() === "armed") return;
    state.set("armed");
    const deadline = Date.now() + config.conditionTimeoutMs;
    try {
      while (!disposed && Date.now() < deadline) {
        if (picker.cancelVisible()) {
          cancellations.update(value => value + 1);
          while (!disposed && picker.visible() && Date.now() < deadline) await frame();
          if (!disposed) state.set(picker.visible() ? "close-timeout" : "closed");
          return;
        }
        await frame();
      }
      if (!disposed) state.set("open-timeout");
    } catch (error) {
      if (!disposed) state.set(`error:${String(error)}`);
    }
  }
  return <View class="flex-row">
    <Button testID="native-picker-arm" onPress={() => { void cancelNext(); }}><Text>Cancel next native picker</Text></Button>
    <Text>Native picker:{state()}</Text>
    <Text>Native cancellations:{cancellations()}</Text>
  </View>;
}

mount(() => <SettingsActionsPending controls={<PickerControl />} />);
