/** Injects failing page components while retaining the actual RouterView and history owner. */
import { mount, Text, View } from "nsh";
import { Button } from "nsh/base";
import { createWindowState } from "../../../state/window/index";
import { createAppState } from "../../../state/app";
import { createRouter } from "../../index";
import { RouterView, type PageRegistry } from "../../RouterView";

function Probe() {
  const window = createWindowState();
  const appearance = createAppState(window);
  const navigation = createRouter(window.routing);
  let failHome = true;
  const pages: PageRegistry = {
    home: () => {
      if (failHome) {
        failHome = false;
        throw new Error("injected_initial_page_failure");
      }
      return <Text>Recovered home page</Text>;
    },
    settings: () => { throw new Error("injected_persistent_page_failure"); },
    "not-found": () => <Text>Unmatched fixture page</Text>,
  };
  return <View>
    <Text>{`Entry:${navigation.current().key}`}</Text>
    <Text>{`Path:${navigation.current().location.path}`}</Text>
    <Button onPress={() => void navigation.navigate("settings")}><Text>Open failing page</Text></Button>
    <RouterView pages={pages} appearance={appearance} navigation={navigation} session={window.session} />
  </View>;
}
mount(Probe);
