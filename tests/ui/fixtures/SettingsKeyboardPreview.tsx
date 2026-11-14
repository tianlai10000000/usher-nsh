/** Mounts the real settings page with its section list first in the keyboard focus order. */
import { mount, View } from "nsh";
import { SettingsPage } from "../../../ui/features/settings/SettingsPage";
import { createAppState } from "../../../ui/state/app";

import { createUpdates } from "../../../ui/features/updates/index";

function SettingsKeyboardPreview() {
  const appearance = createAppState();
  const updates = createUpdates(appearance);
  return (
    <View class="flex-1" style={{ backgroundColor: appearance.appearance.palette().panel }}>
      <SettingsPage appearance={appearance} updates={updates} />
    </View>
  );
}

mount(SettingsKeyboardPreview);
