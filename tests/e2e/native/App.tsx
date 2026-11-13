/** Opens two real windows in one VM; the test runner controls source updates and data roots. */
import { effect, mount, View } from "nsh";
import { Window } from "nsh/desktop";
import { Probe } from "./Probe";
import { opened, secondaryVisible } from "./controller";
import config from "./config.json";
function App() {
  const open = () => Window.open(() => <View><If when={secondaryVisible()}><Probe /></If></View>, {
    id: config.secondaryId, title: config.secondaryId, defaultSize: config.windowSize,
  });
  opened(open);
  effect(() => { open(); });
  return <View><Probe /></View>;
}
mount(App);
