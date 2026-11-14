/** Exercises real subscription consumers, owner cleanup and remount in one isolated test VM. */
import { mount, signal, Text, View } from "nsh";
import { Button } from "nsh/base";
import { Settings, SettingsState } from "usher/nsh";
import { createAppState } from "../../../ui/state/app";
import { SettingsProvider, useSettingsStore } from "../../../ui/state/settings/store";

function Borrower() {
  const settings = useSettingsStore();
  const selected = settings.select(value => value.colorMode);
  return <Text>{`Borrowed: ${selected()}`}</Text>;
}

function Preview() {
  const appearance = createAppState();
  const show = signal(true);
  const deliveries = signal(0);
  const observed = signal("waiting");
  const foreignDone = signal("none");
  const instance = signal("");
  function Probe() {
    // Deliberately no TS cleanup: exercise the native Stop's owner cleanup itself.
    SettingsState.subscribe(event => {
      if (event.snapshot) {
        deliveries.update(value => value + 1);
        observed.set(event.snapshot.value.colorMode);
        instance.set(event.snapshot.storeId);
      }
    });
    return <Text>Probe mounted</Text>;
  }
  async function foreign(mode: string) {
    // Ignore the command reply: only the subscription can update either consumer.
    await Settings.setColorMode(mode);
    foreignDone.set(mode);
  }
  return <SettingsProvider value={appearance.settings.view}>
    <View>
      <Text>{`Primary: ${appearance.appearance.mode()}`}</Text>
      <Text>{`Sync: ${appearance.settings.view.status()}`}</Text>
      <Borrower />
      <Text>{`Observed: ${observed()}`}</Text>
      <Text>{`Deliveries: ${deliveries()}`}</Text>
      <Text>{`Instance: ${instance()}`}</Text>
      <Text>{`Foreign: ${foreignDone()}`}</Text>
      <Button testID="foreign-dark" onPress={() => void foreign("dark")}><Text>Foreign dark</Text></Button>
      <Button testID="foreign-light" onPress={() => void foreign("light")}><Text>Foreign light</Text></Button>
      <Button testID="unmount" onPress={() => show.set(false)}><Text>Unmount probe</Text></Button>
      <Button testID="remount" onPress={() => show.set(true)}><Text>Remount probe</Text></Button>
      <If when={show()}><Probe /></If>
    </View>
  </SettingsProvider>;
}
mount(Preview);
