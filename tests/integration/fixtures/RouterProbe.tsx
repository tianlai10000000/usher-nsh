/** Exercises router edge paths and shared window state through production adapters. */
import { mount, Text, View } from "nsh";
import { Button } from "nsh/base";
import { createWindowState } from "../../../ui/state/window/index";
import { createAppState } from "../../../ui/state/app";
import { createRouter } from "../../../ui/router/index";
import { bindNativeRouting } from "../../../ui/router/native";
import { RouterView } from "../../../ui/router/RouterView";
import { pages } from "../../../ui/pages/index";
function Probe() {
  const window = createWindowState();
  const appearance = createAppState(window);
  const router = createRouter(window.routing);
  bindNativeRouting(router);
  let allow: (() => void) | undefined;
  let cancel: (() => void) | undefined;
  const stop = router.beforeEach(target => target.id === "settings" ? new Promise<boolean>(resolve => { allow = () => resolve(true); cancel = () => resolve(false); }) : true);
  return <View>
    <Text>{`Path:${router.current().location.path}`}</Text>
    <Text>{`Key:${router.current().key}`}</Text>
    <Text>{`Preview:${appearance.background.backgroundImageOpacity()}`}</Text>
    <Text>{`Section:${appearance.settings.settingsSection()}`}</Text>
    <Button onPress={() => void router.navigate("settings")}><Text>Guarded settings</Text></Button>
    <Button onPress={() => cancel?.()}><Text>Cancel old guard</Text></Button>
    <Button onPress={() => allow?.()}><Text>Allow old guard</Text></Button>
    <Button onPress={() => void router.push({ kind: "path", location: "/unknown" })}><Text>Unknown page</Text></Button>
    <Button onPress={() => { stop(); void router.replace({ kind: "named", id: "settings" }); }}><Text>Replace settings</Text></Button>
    <Button onPress={() => void router.goBack()}><Text>Back from probe</Text></Button>
    <Button onPress={() => appearance.background.previewBackgroundImageOpacity(45)}><Text>Preview from another consumer</Text></Button>
    <Button onPress={() => appearance.settings.setSettingsSection("theme")}><Text>Section from another consumer</Text></Button>
    <RouterView pages={pages} appearance={appearance} navigation={router} session={window.session} />
  </View>;
}
mount(Probe);
