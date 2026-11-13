/** Remounts the composition owner in one VM to distinguish projection cleanup from store lifetime. */
import { mount, signal, Text, View } from "nsh";
import { Button } from "nsh/base";
import { createWindowState } from "../../../ui/state/window";
import { createRouter } from "../../../ui/router";
function Owner() {
  const state = createWindowState();
  const router = createRouter(state.routing);
  return <View>
    <Text>{`Count:${state.session.value().clicks}`}</Text>
    <Text>{`Preview:${state.session.value().opacityPreview}`}</Text>
    <Text>{`Path:${router.current().location.path}`}</Text>
    <Text>{`Entry:${router.current().key}`}</Text>
    <Button onPress={() => void state.session.change({ kind: "increment" })}><Text>Increase window counter</Text></Button>
    <Button onPress={() => void state.session.preview(true, 45)}><Text>Set retained preview</Text></Button>
    <Button onPress={() => void router.navigate("settings")}><Text>Navigate retained history</Text></Button>
  </View>;
}
function Host() {
  const visible = signal(true);
  return <View>
    <Button onPress={() => visible.set(false)}><Text>Unmount composition</Text></Button>
    <Button onPress={() => visible.set(true)}><Text>Remount composition</Text></Button>
    <If when={visible()}><Owner /></If>
  </View>;
}
mount(Host);
