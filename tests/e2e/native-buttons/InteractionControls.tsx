/** Hosts real AppKit controls and a public-action driver for temporary submission protection. */
import { mount, signal, Text, View } from "nsh";
import { Button } from "nsh/base";
import * as native from "usher/nsh";
import { AppKitButton, AppKitPopup, AppKitSegments } from "../../../ui/components/AppKitControls";
import { createAppState } from "../../../ui/state/app";
import { spacing } from "../../../ui/theme/look";

const driver = (native as typeof native & { NativeInteraction: { inspect(activate: boolean): string } }).NativeInteraction;
function Preview() {
  const { palette } = createAppState().appearance;
  const blocked = signal(false);
  const enabled = signal(true);
  const calls = signal(0);
  const selection = signal("first");
  const state = signal("");
  const height = signal(0);
  const items = [{ id: "first", label: "First" }, { id: "second", label: "Second" }];
  const change = ({ id }: { id: string }) => { selection.set(id); calls.update(value => value + 1); };
  return <View style={{ backgroundColor: palette().panel }}>
    <View style={{ ...spacing(palette).stack, padding: palette().metric.cardPad }} onLayout={({ bounds }) => height.set(bounds.size.height)}>
      <AppKitSegments label="Contract segments" items={items} selectedId={selection()} enabled={enabled()} interactionBlocked={blocked()} onChange={change} />
      <AppKitPopup label="Contract popup" items={items} selectedId={selection()} enabled={enabled()} interactionBlocked={blocked()} onChange={change} />
      <AppKitButton label="Contract button" enabled={enabled()} interactionBlocked={blocked()} onActivate={() => calls.update(value => value + 1)} />
    </View>
    <Text testID="native-neutral">Surface height:{height()}</Text>
    <Text>Calls:{calls()}</Text><Text>Selection:{selection()}</Text><Text>Native state:{state()}</Text>
    <Button testID="native-lock" onPress={() => blocked.set(true)}><Text>Lock</Text></Button>
    <Button testID="native-unlock" onPress={() => blocked.set(false)}><Text>Unlock</Text></Button>
    <Button testID="native-disable" onPress={() => enabled.set(false)}><Text>Disable</Text></Button>
    <Button testID="native-clear-selection" onPress={() => selection.set("missing")}><Text>Clear selection</Text></Button>
    <Button testID="native-enable" onPress={() => enabled.set(true)}><Text>Enable</Text></Button>
    <Button testID="native-inspect" onPress={() => state.set(driver.inspect(false))}><Text>Inspect</Text></Button>
    <Button testID="native-activate" onPress={() => state.set(driver.inspect(true))}><Text>Activate native targets</Text></Button>
  </View>;
}
mount(Preview);
