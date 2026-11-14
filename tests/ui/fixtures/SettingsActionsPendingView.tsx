/** Delays selected production settings actions before invoking their real, isolated services. */
import { signal, Text, View } from "nsh";
import { Button } from "nsh/base";
import { SettingsPage } from "../../../ui/features/settings/SettingsPage";
import { createAppState } from "../../../ui/state/app";
import { spacing } from "../../../ui/theme/look";

export function SettingsActionsPending(props: { controls?: import("nsh/jsx-runtime").JSX.Element }) {
  const appearance = createAppState();
  const pending = signal("");
  const calls = signal(0);
  const pageTop = signal(0);
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
    settings: { ...appearance.settings, busy: () => pending() !== "" || appearance.settings.busy() },
    preferences: { ...appearance.preferences,
    setShowStatusBar: value => hold("statusbar", () => appearance.preferences.setShowStatusBar(value)),
    chooseLanguage: value => hold("language", () => appearance.preferences.chooseLanguage(value)),
    },
    appearance: { ...appearance.appearance,
    chooseTheme: id => hold("theme", () => appearance.appearance.chooseTheme(id)),
    choose: mode => hold("appearance", () => appearance.appearance.choose(mode)),
    setAccent: value => hold("accent", () => appearance.appearance.setAccent(value)),
    rescanThemes: () => hold("scan", () => appearance.appearance.rescanThemes()),
    },
    background: { ...appearance.background,
    chooseBackgroundImage: () => hold("picker", () => appearance.background.chooseBackgroundImage()),
    pickingBackground: () => pending() === "picker" || appearance.background.pickingBackground(),
    },
    diagnostics: { ...appearance.diagnostics,
    exportDiagnostics: days => hold("export", () => appearance.diagnostics.exportDiagnostics(days)),
    },
  };
  return <View class="flex-1" style={{ backgroundColor: appearance.appearance.palette().panel, ...spacing(appearance.appearance.palette).stack }}>
    <View class="flex-row">
      <Text>Pending:{pending() || "none"}</Text>
      <Text>Calls:{calls()}</Text>
      <Button testID="release-settings" onPress={() => release?.()}><Text>Release settings</Text></Button>
    </View>
    {props.controls}
    <Text>Picker:{appearance.background.pickingBackground() ? "running" : "idle"}</Text>
    <Text>Background:{appearance.background.backgroundImageId() ?? "none"}</Text>
    <Text>Picker error:{appearance.background.backgroundProblem() ?? "none"}</Text>
    <Text testID="settings-neutral">Page top:{pageTop()}</Text>
    <View class="flex-1" onLayout={({ bounds }) => pageTop.set(bounds.origin.y)}><SettingsPage appearance={delayed} /></View>
  </View>;
}
