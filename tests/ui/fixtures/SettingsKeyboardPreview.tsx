/** Mounts the real settings page with its section list first in the keyboard focus order. */
import { mount, View } from "nsh";
import { SettingsPage } from "../../../ui/features/settings/SettingsPage";
import { createAppearanceStore } from "../../../ui/state/appearance";

function SettingsKeyboardPreview() {
  const appearance = createAppearanceStore();
  return (
    <View class="flex-1" style={{ backgroundColor: appearance.palette().panel }}>
      <SettingsPage appearance={appearance} />
    </View>
  );
}

mount(SettingsKeyboardPreview);
