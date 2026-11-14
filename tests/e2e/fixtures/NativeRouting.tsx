/** Runs native back and modal arbitration against the production routing adapter. */
import { mount, signal, Text, View } from "nsh";
import { Button, Dialog } from "nsh/base";
import { createWindowState } from "../../../ui/state/window/index";
import { createRouter } from "../../../ui/router/index";
import { bindNativeRouting } from "../../../ui/router/native";
function Probe() {
  const window = createWindowState();
  const router = createRouter(window.routing);
  bindNativeRouting(router);
  const modal = signal(false);
  let release: ((allowed: boolean) => void) | undefined;
  router.beforeEach(target => target.id === "home" ? new Promise<boolean>(resolve => { release = resolve; }) : true);
  return <View>
    <Text>{`Path:${router.current().location.path}`}</Text>
    <Text>{`Pending:${router.pending() !== null}`}</Text>
    <Button onPress={() => void router.navigate("settings")}><Text>Open settings</Text></Button>
    <Button onPress={() => release?.(false)}><Text>Cancel return</Text></Button>
    <Button onPress={() => release?.(true)}><Text>Allow return</Text></Button>
    <Button onPress={() => modal.set(true)}><Text>Open modal</Text></Button>
    <Dialog open={modal()} onOpenChange={({ open }) => modal.set(open)}><Text>Modal content</Text></Dialog>
  </View>;
}
mount(Probe);
