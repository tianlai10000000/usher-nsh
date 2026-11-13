/** Delays selected production settings actions before invoking their real, isolated services. */
import { signal, Text, View } from "nsh";
import { Button } from "nsh/base";
import { SettingsPage } from "../../../ui/features/settings/SettingsPage";
import { createAppearanceStore } from "../../../ui/state/appearance";
import { spacing } from "../../../ui/theme/look";

export function SettingsActionsPending(props: { controls?: import("nsh/jsx-runtime").JSX.Element }) {
  const appearance = createAppearanceStore();
  const pending = signal("");
  const calls = signal(0);
  let release: (() => void) | undefined;
  async function hold<T>(name: string, run: () => Promise<T>): Promise<T> {
    calls.update(value => value + 1);
    pending.set(name);
    try {
      await new Promise<void>(resolve => { release = resolve; });
      return await run();
    } finally { pending.set(""); release = undefined; }
  }
  const delayed: typeof appearance = {
    ...appearance,
    busy: () => pending() !== "" || appearance.busy(),
    chooseTheme: id => hold("theme", () => appearance.chooseTheme(id)),
    choose: mode => hold("appearance", () => appearance.choose(mode)),
    setAccent: value => hold("accent", () => appearance.setAccent(value)),
    rescanThemes: () => hold("scan", () => appearance.rescanThemes()),
    chooseBackgroundImage: () => hold("picker", () => appearance.chooseBackgroundImage()),
    pickingBackground: () => pending() === "picker" || appearance.pickingBackground(),
    exportDiagnostics: days => hold("export", () => appearance.exportDiagnostics(days)),
  };
  return <View class="flex-1" style={{ backgroundColor: appearance.palette().panel, ...spacing(appearance.palette).stack }}>
    <View class="flex-row">
      <Text>Pending:{pending() || "none"}</Text>
      <Text>Calls:{calls()}</Text>
      <Button testID="release-settings" onPress={() => release?.()}><Text>Release settings</Text></Button>
    </View>
    {props.controls}
    <Text>Picker:{appearance.pickingBackground() ? "running" : "idle"}</Text>
    <Text>Background:{appearance.backgroundImageId() ?? "none"}</Text>
    <Text>Picker error:{appearance.backgroundProblem() ?? "none"}</Text>
    <SettingsPage appearance={delayed} />
  </View>;
}
