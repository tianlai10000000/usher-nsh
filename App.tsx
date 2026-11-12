import { mount, signal, Text, View } from "nsh";
import { ThemeButton } from "./ui/components/ThemeControls";
import { EffectiveAppearance } from "./ui/features/settings/EffectiveAppearance";
import { SettingsPage } from "./ui/features/settings/SettingsPage";
import { createNavigation } from "./ui/navigation";
import { Frame } from "./ui/layout/Frame";
import { createAppearanceStore } from "./ui/state/appearance";
import { headingLook, spacing } from "./ui/theme/look";

function App() {
  // State shared by pages is owned above their conditional mounts.
  const appearance = createAppearanceStore();
  const navigation = createNavigation();
  const clicks = signal(0);
  const { palette, t } = appearance;

  return (
    <Frame appearance={appearance} navigation={navigation}>
      <If when={navigation.route() === "home"} else={<SettingsPage appearance={appearance} />}>
        <View class="flex-1 items-center justify-center" style={spacing(palette).stack}>
          <Text style={headingLook(palette, "display")}>{t("home.greeting")}</Text>
          <ThemeButton palette={palette} label={t("home.click")} prominent native={false}
            onPress={() => clicks.update((count) => count + 1)} />
          <Text style={{ color: palette().text }}>{t("home.clicks", { count: clicks() })}</Text>
          <EffectiveAppearance appearance={appearance} />
        </View>
      </If>
    </Frame>
  );
}

mount(App);
