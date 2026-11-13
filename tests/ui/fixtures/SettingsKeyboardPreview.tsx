/** Mounts the real settings page with its section list first in the keyboard focus order. */
import { mount, View } from "nsh";
import { SettingsPage } from "../../../ui/features/settings/SettingsPage";
import { createAppearanceStore } from "../../../ui/state/appearance";

import { createUpdates } from "../../../ui/features/updates";

function SettingsKeyboardPreview() {
  const appearance = createAppearanceStore();
  const updates = createUpdates(appearance);
  return (
    <View class="flex-1" style={{ backgroundColor: appearance.palette().panel }}>
      <SettingsPage appearance={appearance} updates={updates} />
    </View>
  );
}

mount(SettingsKeyboardPreview);
