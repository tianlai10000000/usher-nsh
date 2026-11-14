/** Observes same-VM window shortcut ownership through shared test-only projections. */
import { initializeFixtureGlobals } from "./global";
import { cleanup, effect, mount, signal, Text, View } from "nsh";
import { Button } from "nsh/base";
import { currentWindow, Window } from "nsh/desktop";
import { createAppState } from "../../../ui/state/app";
import { createRouter } from "../../../ui/router";
import { createShortcuts } from "../../../ui/shortcuts";
import config from "./config.json";
const scopes = signal<Record<string, string>>({});
const paths = signal<Record<string, string>>({});
function Probe(props: { name: string }) {
  const state = createAppState(); const router = createRouter(state.window.routing);
  createShortcuts(state.window.scope, router);
  scopes.update(all => ({ ...all, [props.name]: state.window.scope }));
  effect(() => { const path = router.current().location.path; paths.update(all => ({ ...all, [props.name]: path })); });
  void router.navigate("settings");
  return <Text>{`Probe:${props.name}`}</Text>;
}
function Harness() {
  initializeFixtureGlobals();
  const main = currentWindow();
  let secondary: ReturnType<typeof Window.open> | undefined;
  cleanup(() => secondary?.close());
  return <View><Probe name="main" />
    <Text>{`SecondaryScope:${scopes().secondary}`}</Text>
    <Text>{`Main:${paths().main}`}</Text><Text>{`Secondary:${paths().secondary}`}</Text>
    <Button testID="open-secondary" onPress={() => { secondary = Window.open(() => <Probe name="secondary" />, { id: config.secondaryId, title: config.secondaryId, defaultSize: config.windowSize }); }}><Text>Open secondary</Text></Button>
    <Button testID="focus-main" onPress={() => main.focus()}><Text>Focus main</Text></Button>
    <Button testID="focus-secondary" onPress={() => secondary?.focus()}><Text>Focus secondary</Text></Button>
  </View>;
}
mount(Harness);
