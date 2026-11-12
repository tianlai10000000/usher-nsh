import { Text } from "nsh";
import { Button, Input } from "nsh/base";
import type { AppearanceStore } from "../state/appearance";
import { buttonLook, fieldLook } from "../theme/look";
import { AppKitButton } from "./AppKitControls";

export type PaletteAccessor = AppearanceStore["palette"];

export function ThemeButton(props: {
  palette: PaletteAccessor;
  label: string;
  testID?: string;
  selected?: boolean;
  prominent?: boolean;
  native?: boolean;
  disabled?: boolean;
  onPress: () => void;
}) {
  // The look is recomputed where it is read, so it follows the theme, the scheme and the accent.
  const look = () =>
    buttonLook(props.palette, { kind: props.prominent ? "primary" : "secondary", selected: props.selected });
  return (
    <If
      when={props.native !== false && props.palette().nativeAppKit}
      else={
        <Button
          testID={props.testID}
          style={look().style}
          hoverStyle={look().hoverStyle}
          activeStyle={look().activeStyle}
          focusStyle={look().focusStyle}
          disabledStyle={look().disabledStyle}
          selected={props.selected}
          disabled={props.disabled}
          onPress={props.onPress}
        >
          <Text style={look().label}>{props.label}</Text>
        </Button>
      }
    >
      <AppKitButton
        testID={props.testID}
        style={{ height: props.palette().metric.buttonHeight }}
        label={props.label}
        enabled={!props.disabled}
        onActivate={props.onPress}
      />
    </If>
  );
}

export function ThemeInput(props: {
  palette: PaletteAccessor;
  label: string;
  value: string;
  testID: string;
  placeholder?: string;
  onChange: (value: string) => void;
}) {
  const look = () => fieldLook(props.palette);
  return (
    <Input
      accessibilityLabel={props.label}
      testID={props.testID}
      style={look().style}
      hoverStyle={look().hoverStyle}
      focusStyle={look().focusStyle}
      disabledStyle={look().disabledStyle}
      autoCapitalize="none"
      autoCorrect={false}
      placeholder={props.placeholder}
      value={props.value}
      onChange={({ value }) => props.onChange(value)}
    />
  );
}
