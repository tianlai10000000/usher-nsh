/** Exercises shared control adapters independently of any settings page or persistence service. */
import { mount, signal, Text, View } from "nsh";
import { Button } from "nsh/base";
import { AsyncButton } from "../../../ui/components/AsyncButton";
import { ThemeButton } from "../../../ui/components/ThemeControls";
import { ThemeSwitch, ThemeSlider, InteractionSelect } from "../../../ui/components/ValueControls";
import { createAppState } from "../../../ui/state/app";
import { buttonLook, fieldLook, spacing } from "../../../ui/theme/look";

function Preview() {
  const { palette } = createAppState().appearance;
  const blocked = signal(false);
  const unavailable = signal(false);
  const calls = signal(0);
  const ends = signal(0);
  let lockOnChange = false;
  const height = signal(0);
  const checked = signal(true);
  const value = signal(50);
  const choice = signal("first");
  const iconLabel = signal("Protected icon");
  let commitValues = false;
  const change = () => calls.update(value => value + 1);
  return <View style={{ backgroundColor: palette().panel }}>
    <View testID="interaction-surface" style={{ ...spacing(palette).stack, padding: palette().metric.cardPad }} onLayout={({ bounds }) => height.set(bounds.size.height)}>
      <ThemeButton palette={palette} native={false} testID="protected-button" label="Protected button" interactionBlocked={blocked()} disabled={unavailable()} onPress={change} />
      <AsyncButton palette={palette} testID="protected-icon" accessibilityLabel={iconLabel()} iconOnly interactionBlocked={blocked()} disabled={unavailable()}
        style={buttonLook(palette).style} disabledStyle={buttonLook(palette).disabledStyle} onPress={change}><Text>+</Text></AsyncButton>
      <ThemeSwitch palette={palette} testID="protected-switch" checked={checked()} interactionBlocked={blocked()} disabled={unavailable()} onChange={event => { change(); if (commitValues) checked.set(event.checked); }} />
      <ThemeSlider palette={palette} testID="protected-slider" value={value()} min={0} max={100} interactionBlocked={blocked()} disabled={unavailable()} onChange={event => { change(); if (commitValues && typeof event.value === "number") value.set(event.value); if (lockOnChange) blocked.set(true); }} onChangeEnd={() => ends.update(value => value + 1)} />
      <InteractionSelect interactionBlocked={blocked()} disabled={unavailable()} disabledStyle={fieldLook(palette).disabledStyle}
        items={[{ value: "first", title: "First" }, { value: "second", title: "Second" }]} value={choice()} onChange={event => { change(); if (commitValues && event.value) choice.set(event.value); }}
        trigger={<View testID="protected-select" style={fieldLook(palette).style}><Text>{choice() === "first" ? "First" : "Second"}</Text></View>} />
      <ThemeButton palette={palette} native={false} testID="unavailable-button" label="Unavailable" disabled onPress={change} />
      <ThemeButton palette={palette} native={false} testID="independent-button" label="Independent action" onPress={change} />
    </View>
    <Text testID="interaction-neutral">Surface height:{height()}</Text>
    <Text>Checked:{String(checked())}</Text><Text>Value:{value()}</Text><Text>Choice:{choice()}</Text>
    <Text>Ends:{ends()}</Text><Text>Calls:{calls()}</Text><Text>Blocked:{String(blocked())}</Text>
    <Button testID="commit-values" onPress={() => { commitValues = true; }}><Text>Commit values</Text></Button>
    <Button testID="change-icon-label" onPress={() => iconLabel.set("Changed icon")}><Text>Change label</Text></Button>
    <Button testID="arm-slider-lock" onPress={() => { lockOnChange = true; }}><Text>Lock on slider change</Text></Button>
    <Button testID="lock-controls" onPress={() => blocked.set(true)}><Text>Lock</Text></Button>
    <Button testID="unlock-controls" onPress={() => blocked.set(false)}><Text>Unlock</Text></Button>
    <Button testID="disable-controls" onPress={() => unavailable.set(true)}><Text>Disable</Text></Button>
  </View>;
}
mount(Preview);
