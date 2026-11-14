/** Composes themed controls with native adapters and optional inline busy feedback. */
import { Text, View } from "nsh";
import { Button, Input } from "nsh/base";
import type { ThemeAccessor } from "../theme/index";
import { buttonLook, fieldLook } from "../theme/look";
import { AppKitButton } from "./AppKitControls";
import { ProgressRing } from "./ProgressRing";
import type { InteractionProps } from "./interaction";
import { useButtonAction } from "./AsyncButton";

export type PaletteAccessor = ThemeAccessor;

export function ThemeButton(props: InteractionProps & {
  palette: PaletteAccessor;
  label: string;
  testID?: string;
  selected?: boolean;
  prominent?: boolean;
  native?: boolean;
  loadingFeedback?: boolean;
  busy?: boolean;
  onPress: () => void | Promise<unknown>;
}) {
  const action = useButtonAction(props);
  const supportsLoading = () => props.loadingFeedback === true || props.busy !== undefined;
  // The look is recomputed where it is read, so it follows the theme, the scheme and the accent.
  const look = () =>
    buttonLook(props.palette, { kind: props.prominent ? "primary" : "secondary", selected: props.selected, busy: action.busy() });
  return (
    <If
      when={props.native !== false && props.palette().nativeAppKit && !supportsLoading()}
      else={
        <Button
          testID={props.testID}
          style={look().style}
          hoverStyle={look().hoverStyle}
          activeStyle={look().activeStyle}
          focusStyle={look().focusStyle}
          disabledStyle={action.dimmed() ? look().disabledStyle : undefined}
          selected={props.selected}
          disabled={action.disabled()}
          onPress={action.run}
        >
          <If when={supportsLoading()}>
            <View style={{ width: props.palette().metric.controlIcon, height: props.palette().metric.controlIcon, flexShrink: 0 }}>
              <If when={action.busy()}>
                <ProgressRing
                  palette={props.palette}
                  size={props.palette().metric.controlIcon}
                  progress={null}
                  label={props.label}
                  testID={props.testID ? `${props.testID}-loading` : undefined}
                />
              </If>
            </View>
          </If>
          <Text style={look().label}>{props.label}</Text>
        </Button>
      }
    >
      <AppKitButton
        testID={props.testID}
        style={{ height: props.palette().metric.buttonHeight }}
        label={props.label}
        enabled={!props.disabled && !action.busy()}
        interactionBlocked={props.interactionBlocked}
        onActivate={action.run}
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
