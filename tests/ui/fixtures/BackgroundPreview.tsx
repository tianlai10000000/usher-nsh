/** Loads a synthetic image through the real bridge to inspect background layering on native hosts. */
import { mount, View } from "nsh";
import { Settings } from "usher/nsh";
import { ThemeButton } from "../../../ui/components/ThemeControls";
import { SettingsPage } from "../../../ui/features/settings/SettingsPage";
import { Frame } from "../../../ui/layout/Frame";
import { createNavigation } from "../../../ui/navigation";
import { createAppearanceStore } from "../../../ui/state/appearance";
import { spacing } from "../../../ui/theme/look";
import fixture from "../../fixtures/background.json";

function BackgroundPreview() {
  const appearance = createAppearanceStore();
  const navigation = createNavigation();
  navigation.navigate("settings");
  async function load() {
    const bytes = await (await fetch(fixture.dataUrl)).bytes();
    await Settings.importBackgroundImage("preview.png", bytes);
    await appearance.choose("light");
  }
  return (
    <Frame appearance={appearance} navigation={navigation}>
      <View style={spacing(appearance.palette).stack}>
        <ThemeButton palette={appearance.palette} label="Load preview image" testID="load-preview-image"
          disabled={appearance.busy()} onPress={() => void load()} />
        <SettingsPage appearance={appearance} />
      </View>
    </Frame>
  );
}

mount(BackgroundPreview);
