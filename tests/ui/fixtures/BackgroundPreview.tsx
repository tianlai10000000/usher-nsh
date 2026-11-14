/** Loads a synthetic image through the real bridge to inspect background layering on native hosts. */
import { mount, View } from "nsh";
import { Settings } from "usher/nsh";
import { ThemeButton } from "../../../ui/components/ThemeControls";
import { SettingsPage } from "../../../ui/features/settings/SettingsPage";
import { Frame } from "../../../ui/layout/Frame";
import { createRouter } from "../../../ui/router/index";
import { createAppState } from "../../../ui/state/app";
import { spacing } from "../../../ui/theme/look";
import fixture from "../../fixtures/background.json";

function BackgroundPreview() {
  const appearance = createAppState();
  const navigation = createRouter();
  navigation.navigate("settings");
  async function load() {
    const bytes = await (await fetch(fixture.dataUrl)).bytes();
    await Settings.importBackgroundImage("preview.png", bytes);
    await appearance.appearance.choose("light");
  }
  return (
    <Frame appearance={appearance} navigation={navigation}>
      <View style={spacing(appearance.appearance.palette).stack}>
        <ThemeButton palette={appearance.appearance.palette} label="Load preview image" testID="load-preview-image"
          disabled={appearance.settings.busy()} onPress={() => void load()} />
        <SettingsPage appearance={appearance} />
      </View>
    </Frame>
  );
}

mount(BackgroundPreview);
