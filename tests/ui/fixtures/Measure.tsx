import { mount, signal, Text, View } from "nsh";
import { ThemeButton, ThemeInput } from "../../../ui/components/ThemeControls";
import { createAppearanceStore } from "../../../ui/state/appearance";

// Reports the laid-out size of a themed button and field as text, so a test can read what a
// theme's metrics became on screen.
function Measure() {
  const appearance = createAppearanceStore();
  const { palette } = appearance;
  const button = signal({ width: 0, height: 0 });
  const field = signal({ width: 0, height: 0 });
  return (
    <View class="gap-4 p-4">
      <View onLayout={({ bounds }) => button.set(bounds.size)} class="self-start">
        <ThemeButton palette={palette} label="Measured" testID="measured-button" native={false} onPress={() => {}} />
      </View>
      <View onLayout={({ bounds }) => field.set(bounds.size)}>
        <ThemeInput palette={palette} label="Field" value="" testID="measured-field" onChange={() => {}} />
      </View>
      <Text testID="button-height">{`button ${Math.round(button().height)}`}</Text>
      <Text testID="field-height">{`field ${Math.round(field().height)}`}</Text>
      <Text testID="theme-id">{`theme ${appearance.themeId()}`}</Text>
      <ThemeButton palette={palette} testID="pick-primer" label="Primer" native={false}
        onPress={() => void appearance.chooseTheme("builtin.primer")} />
      <ThemeButton palette={palette} testID="pick-slate" label="Slate" native={false}
        onPress={() => void appearance.chooseTheme("builtin.slate")} />
      <ThemeButton palette={palette} testID="pick-gallery" label="Gallery" native={false}
        onPress={() => void appearance.chooseTheme("builtin.nsh-gallery")} />
    </View>
  );
}

mount(Measure);
