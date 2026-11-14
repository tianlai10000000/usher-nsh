/** Adapts public NSH value controls to the shared availability and temporary-lock contract. */
import { Switch, Slider, Select, type SwitchProps, type SliderProps, type SelectProps } from "nsh/base";
import { forwardProps } from "./forwardProps";
import type { PaletteAccessor } from "./ThemeControls";
import { controlInteraction, type InteractionProps } from "./interaction";
import { createSwitchLook, switchLook, sliderLook } from "../theme/look";

type Themed = InteractionProps & { palette: PaletteAccessor };

/** Uses the theme switch parts while preserving value ownership in the caller. */
export function ThemeSwitch(props: SwitchProps & Themed) {
  const interaction = controlInteraction(props);
  const look = createSwitchLook(props.palette, () => props.checked === true);
  const forwarded = forwardProps(props, ["palette", "interactionBlocked"]);
  return <Switch {...forwarded} disabled={interaction.disabled()}
    disabledStyle={interaction.dimmed() ? switchLook(props.palette, props.checked === true).disabledStyle : undefined}
    focusStyle={switchLook(props.palette, props.checked === true).focusStyle}
    trackClass={look.trackClass} trackStyle={look.trackStyle} thumbStyle={look.thumbStyle}
    onChange={event => { if (!interaction.disabled()) props.onChange?.(event); }} />;
}

/** Applies availability to both the slider host and its independently styled parts. */
export function ThemeSlider(props: SliderProps & Themed) {
  const interaction = controlInteraction(props);
  const look = () => sliderLook(props.palette, interaction.dimmed());
  const forwarded = forwardProps(props, ["palette", "interactionBlocked"]);
  return <Slider {...forwarded} disabled={interaction.disabled()}
    disabledStyle={interaction.dimmed() ? props.disabledStyle : undefined}
    trackClass={look().trackClass} trackStyle={look().trackStyle}
    indicatorStyle={look().indicatorStyle} rangeStyle={look().rangeStyle}
    thumbClass={look().thumbClass} thumbStyle={look().thumbStyle}
    onChange={event => { if (!interaction.disabled()) props.onChange?.(event); }}
    // Completion belongs to an accepted gesture, even if onChange acquired a lock.
    onChangeEnd={props.onChangeEnd} />;
}

/** Preserves caller-owned trigger/popup composition and blocks every selection event. */
export function InteractionSelect(props: SelectProps & InteractionProps) {
  const interaction = controlInteraction(props);
  const forwarded = forwardProps(props, ["interactionBlocked"]);
  return <Select {...forwarded} disabled={interaction.disabled()}
    disabledStyle={interaction.dimmed() ? props.disabledStyle : undefined}
    onChange={event => { if (!interaction.disabled()) props.onChange?.(event); }} />;
}
