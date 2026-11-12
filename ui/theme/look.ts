/**
 * What each control looks like, as styles computed from the selected theme's tokens.
 *
 * Views spread these objects onto `nsh/base` components and never write a size, color or
 * duration themselves, so a theme changes controls by changing token values. State feedback is
 * the same kind of data: how far a press sinks, how strong a hover wash is, how wide the
 * keyboard focus halo is and how long a state change eases all come from the theme.
 */
import type { InputStyle, TextStyle, ViewStyle } from "nsh";
import { reduceMotion } from "nsh";
import type { AppearanceStore } from "../state/appearance";
import backgroundConfig from "../config/background.json";

export type ButtonKind = "primary" | "secondary" | "ghost";
export type ButtonSize = "small" | "regular" | "large";

type Palette = AppearanceStore["palette"];

/** A `rgba`-style wash of `color` over its surface, as NSH resolves `color-mix`. */
const mix = (color: string, percent: number) =>
  `color-mix(in srgb, ${color} ${Math.round(percent * 100)}%, transparent)`;

/** Reduces only a content surface's fill alpha, preserving theme alpha and child visibility. */
export function contentBackgroundColor(color: string, transparency: number): string {
  const { min, max } = backgroundConfig.contentTransparency;
  const value = Math.min(max, Math.max(min, transparency));
  return value === min ? color : mix(color, (max - value) / (max - min));
}

/** The parts of a finish that are drawn. A part the theme leaves at `none` or `transparent` is left out, so the control keeps its own fill and edge. */
function finishStyle(f: { fill?: string; image?: string; shadow?: string; edge?: string }): ViewStyle {
  const style: ViewStyle = {};
  // Spreading an `undefined` property would erase the control's own fill or edge, so a part is
  // only written when the theme gives it a real value.
  const drawn = (value: string | undefined, nothing: string) =>
    value !== undefined && value !== nothing ? value : undefined;
  const fill = drawn(f.fill, "transparent");
  const image = drawn(f.image, "none");
  const shadow = drawn(f.shadow, "none");
  const edge = drawn(f.edge, "transparent");
  if (fill) style.backgroundColor = fill;
  if (image) style.backgroundImage = image;
  if (shadow) style.boxShadow = shadow;
  if (edge) style.borderColor = edge;
  return style;
}

/** The easing every state change shares: a press sinks and everything else follows over it. */
function transition(palette: Palette, properties: string): ViewStyle {
  const { metric } = palette();
  return {
    transitionProperty: `${properties}, transform`,
    transitionDuration: reduceMotion() ? 0 : Math.max(metric.pressMs, metric.stateMs),
    transitionTimingFunction: "ease-out",
  };
}

/**
 * The keyboard focus: the edge takes the theme's ring color and a halo of `focus-width` surrounds
 * it. The halo is the part that shows where the focus is, so it is drawn in `focus-ring`, an
 * opaque color the registry keeps at 3:1 or better against every surface (WCAG 2.1 SC 1.4.11).
 */
export function focusRing(palette: Palette, offset?: number): ViewStyle {
  const { metric, ring, focusRing: halo } = palette();
  return {
    borderColor: ring,
    outlineStyle: "solid",
    outlineWidth: metric.focusWidth,
    outlineColor: halo,
    outlineOffset: offset ?? metric.focusOffset,
  };
}

/** A press that sinks the control to `scale`. None with reduced motion. */
function sink(scale: number): ViewStyle {
  return reduceMotion() ? {} : { transform: [{ scale }] };
}

const dimensions = (palette: Palette, size: ButtonSize) => {
  const { metric } = palette();
  switch (size) {
    case "small":
      return { height: metric.buttonHeightSmall, pad: metric.buttonPadXSmall, font: metric.fontButtonSmall };
    case "large":
      return { height: metric.buttonHeightLarge, pad: metric.buttonPadXLarge, font: metric.fontButtonLarge };
    default:
      return { height: metric.buttonHeight, pad: metric.buttonPadX, font: metric.fontButton };
  }
};

/**
 * The props of a button: its box, and its hover, pressed, focus and disabled states.
 * `selected` is the chosen one of a few (a segment, a mode); it keeps the filled look.
 */
export function buttonLook(
  palette: Palette,
  options: { kind?: ButtonKind; size?: ButtonSize; selected?: boolean; busy?: boolean } = {},
) {
  const kind = options.kind ?? "secondary";
  const size = dimensions(palette, options.size ?? "regular");
  const p = palette();
  const filled = kind === "primary" || options.selected === true;
  const idle = options.busy === true;
  // A selected neutral button takes the "on" finish; a primary one the accent finish. When the
  // theme draws no finish the plain fill and edge below apply, so a flat theme still reads.
  const chosen = kind === "primary" ? p.finish.accent : options.selected ? p.finish.on : p.finish.raised;
  const base: ViewStyle =
    kind === "primary"
      ? { backgroundColor: p.accent, borderColor: p.accent, ...finishStyle(chosen) }
      : options.selected
        ? { backgroundColor: p.accent, borderColor: p.accent, ...finishStyle(p.finish.accent) }
        : kind === "ghost"
          ? { backgroundColor: "transparent", borderColor: "transparent" }
          : { backgroundColor: "transparent", borderColor: p.border, ...finishStyle(chosen) };

  return {
    style: {
      height: size.height,
      paddingHorizontal: size.pad,
      borderRadius: p.metric.radiusControl,
      borderCurve: "continuous",
      borderWidth: p.metric.borderWidth,
      flexDirection: "row",
      alignItems: "center",
      justifyContent: "center",
      gap: p.metric.buttonGap,
      ...base,
      ...transition(palette, "background-color, border-color, box-shadow, opacity"),
    } satisfies ViewStyle,
    // A filled button dims a touch under the pointer; the others take a wash of the text color.
    hoverStyle: idle
      ? undefined
      : filled
        ? ({ opacity: 0.94 } satisfies ViewStyle)
        : ({
            backgroundColor: mix(p.text, p.metric.hoverWash),
            ...(kind === "ghost" ? {} : finishStyle(p.finish.hover)),
          } satisfies ViewStyle),
    activeStyle: idle
      ? undefined
      : ({
          ...(filled ? { opacity: 0.88 } : { backgroundColor: p.itemPressed }),
          ...(filled || kind === "ghost" ? {} : finishStyle(p.finish.pressed)),
          ...sink(p.metric.pressScale),
        } satisfies ViewStyle),
    focusStyle: focusRing(palette),
    disabledStyle: { opacity: p.metric.disabledOpacity } satisfies ViewStyle,
    label: {
      color: filled ? p.accentText : p.text,
      fontSize: size.font,
      fontWeight: "500",
    } satisfies TextStyle,
  };
}

/**
 * A square icon-only button of the title bar and the rail, sized by `size-toolbar-button` so it
 * fits the larger toolbar icons. `compact` uses `size-titlebar-button` instead, for the small
 * history and color mode buttons in the title bar. `selected` marks the current page with the
 * same quiet fill the secondary menu uses.
 */
export function iconButtonLook(palette: Palette, selected = false, compact = false) {
  const p = palette();
  const side = compact ? p.metric.titlebarButton : p.metric.toolbarButton;
  return {
    style: {
      width: side,
      height: side,
      alignItems: "center",
      justifyContent: "center",
      borderRadius: p.metric.radiusControl,
      borderCurve: "continuous",
      backgroundColor: selected ? p.menuSelected : "transparent",
      ...transition(palette, "background-color, opacity"),
    } satisfies ViewStyle,
    hoverStyle: { backgroundColor: selected ? p.menuSelected : p.itemHover } satisfies ViewStyle,
    activeStyle: { backgroundColor: p.itemPressed, ...sink(p.metric.pressScaleIcon) } satisfies ViewStyle,
    focusStyle: focusRing(palette),
    disabledStyle: { opacity: p.metric.disabledOpacity } satisfies ViewStyle,
  };
}

/** A single-line text field: a recessed well with a stronger edge under the pointer. */
export function fieldLook(palette: Palette) {
  const p = palette();
  return {
    style: {
      minHeight: p.metric.fieldHeight,
      paddingHorizontal: p.metric.fieldPadX,
      borderRadius: p.metric.radiusControl,
      borderCurve: "continuous",
      borderWidth: p.metric.borderWidth,
      borderColor: p.border,
      backgroundColor: p.background,
      color: p.text,
      fontSize: p.metric.fontField,
      placeholderColor: p.muted,
      caretColor: p.accent,
      selectionColor: p.accent,
      ...(p.finish.field.boxShadow !== "none" ? { boxShadow: p.finish.field.boxShadow } : {}),
      ...transition(palette, "border-color, box-shadow"),
    } satisfies InputStyle,
    hoverStyle: { borderColor: p.borderStrong } satisfies ViewStyle,
    focusStyle: focusRing(palette),
    disabledStyle: { opacity: p.metric.disabledOpacity } satisfies ViewStyle,
  };
}

/** Styles Select parts; the framework's focus state follows both pointer and keyboard highlights. */
export function selectLook(palette: Palette) {
  const p = palette();
  // Tailwind arbitrary values use underscores for spaces inside CSS color functions.
  const highlightColor = p.itemHover.replace(/\s+/g, "_");
  return {
    popupStyle: {
      backgroundColor: p.surface,
      borderColor: p.border,
      padding: p.metric.textGap,
      borderRadius: p.metric.radiusCard,
      boxShadow: p.shellShadow,
    } satisfies ViewStyle,
    itemClass: `flex-row items-center focus:bg-[${highlightColor}]`,
    itemStyle: {
      height: p.metric.sidebarItem,
      paddingHorizontal: p.metric.buttonPadX,
      borderRadius: p.metric.radiusItem,
      color: p.text,
    } satisfies TextStyle,
  };
}

/** A card: the flat surface a group of settings sits on. */
export function cardLook(palette: Palette): ViewStyle {
  const p = palette();
  return {
    padding: p.metric.cardPad,
    gap: p.metric.stackGap,
    backgroundColor: p.surface,
    borderColor: p.border,
    borderWidth: p.metric.borderWidth,
    borderRadius: p.metric.radiusCard,
    borderCurve: "continuous",
  };
}

/** A selectable list item, used as the basis for sidebar routes. */
export function itemLook(palette: Palette, selected: boolean) {
  const p = palette();
  return {
    style: {
      height: p.metric.sidebarItem,
      flexDirection: "row",
      alignItems: "center",
      borderRadius: p.metric.radiusItem,
      borderCurve: "continuous",
      backgroundColor: selected ? p.itemSelected : "transparent",
      boxShadow: selected ? p.itemShadow : undefined,
      ...transition(palette, "background-color, box-shadow"),
    } satisfies ViewStyle,
    hoverStyle: { backgroundColor: selected ? p.itemSelected : p.itemHover } satisfies ViewStyle,
    activeStyle: { backgroundColor: p.itemPressed, ...sink(p.metric.pressScale) } satisfies ViewStyle,
    focusStyle: focusRing(palette, -p.metric.focusWidth),
    label: {
      color: selected ? p.text : p.muted,
      fontSize: p.metric.fontBody,
      fontWeight: selected ? "500" : "400",
    } satisfies TextStyle,
  };
}

/** Styles the framework's tabs; one native indicator owns the selected surface and edge. */
export function tabLook(palette: Palette, selected = false) {
  const p = palette();
  return {
    indicatorStyle: {
      left: 0,
      right: 0,
      borderRadius: p.metric.radiusItem,
      borderCurve: "continuous",
      borderWidth: p.metric.borderWidth,
      borderColor: p.border,
      backgroundColor: p.itemSelected,
      boxShadow: p.itemShadow,
    } satisfies ViewStyle,
    style: {
      height: p.metric.sidebarItem,
      paddingHorizontal: p.metric.buttonPadX,
      flexDirection: "row",
      alignItems: "center",
      borderRadius: p.metric.radiusItem,
      borderCurve: "continuous",
    } satisfies ViewStyle,
    hoverStyle: { backgroundColor: selected ? "transparent" : p.itemHover } satisfies ViewStyle,
    activeStyle: { backgroundColor: p.itemPressed } satisfies ViewStyle,
    focusVisibleStyle: focusRing(palette, -p.metric.focusWidth),
    label: {
      color: selected ? p.text : p.muted,
      fontSize: p.metric.fontBody,
      fontWeight: selected ? "500" : "400",
    } satisfies TextStyle,
  };
}

/**
 * An item of the secondary menu. The current page takes a quiet fill of its own, without the
 * lift `itemLook` gives it, because the menu already sits on a raised surface.
 */
export function menuItemLook(palette: Palette, selected: boolean) {
  const p = palette();
  const base = itemLook(palette, selected);
  return {
    ...base,
    style: { ...base.style, backgroundColor: selected ? p.menuSelected : "transparent", boxShadow: undefined },
    hoverStyle: { backgroundColor: selected ? p.menuSelected : p.itemHover } satisfies ViewStyle,
    label: { ...base.label, color: p.text },
  };
}

/** The on/off switch: track, knob and the travel between them, all from the switch metrics. */
export function switchLook(palette: Palette, checked: boolean) {
  const p = palette();
  const inset = (p.metric.switchHeight - p.metric.switchKnob) / 2;
  return {
    trackStyle: {
      width: p.metric.switchWidth,
      height: p.metric.switchHeight,
      padding: inset,
      justifyContent: "center",
      borderRadius: p.metric.switchHeight / 2,
      backgroundColor: checked ? p.accent : p.border,
      ...transition(palette, "background-color"),
    } satisfies ViewStyle,
    thumbStyle: {
      width: p.metric.switchKnob,
      height: p.metric.switchKnob,
      borderRadius: p.metric.switchKnob / 2,
      backgroundColor: checked ? p.accentText : p.surface,
      transform: [{ translateX: checked ? p.metric.switchWidth - p.metric.switchKnob - 2 * inset : 0 }],
      ...transition(palette, "background-color"),
    } satisfies ViewStyle,
    focusStyle: focusRing(palette, 2),
    disabledStyle: { opacity: p.metric.disabledOpacity } satisfies ViewStyle,
  };
}

/** The framework slider's track, selected range and thumb, sized entirely by theme metrics. */
export function sliderLook(palette: Palette) {
  const p = palette();
  return {
    trackStyle: { height: p.metric.fieldHeight, justifyContent: "center",
      paddingHorizontal: p.metric.sliderThumb / 2 } satisfies ViewStyle,
    indicatorStyle: { height: p.metric.sliderTrack, borderRadius: p.metric.radiusControl,
      backgroundColor: p.border } satisfies ViewStyle,
    rangeStyle: { borderRadius: p.metric.radiusControl, backgroundColor: p.accent } satisfies ViewStyle,
    thumbStyle: { width: p.metric.sliderThumb, height: p.metric.sliderThumb,
      backgroundColor: p.surface, borderWidth: p.metric.borderWidth,
      borderColor: p.borderStrong, boxShadow: p.itemShadow } satisfies ViewStyle,
  };
}

/** A settings group with adjustable fill transparency; row content and edges retain their alpha. */
export function groupLook(palette: Palette, transparency: number): ViewStyle {
  const p = palette();
  return {
    backgroundColor: contentBackgroundColor(p.surface, transparency),
    borderColor: p.border,
    borderWidth: p.metric.borderWidth,
    borderRadius: p.metric.radiusCard,
    borderCurve: "continuous",
    overflow: "hidden",
    ...(p.finish.card.boxShadow !== "none" ? { boxShadow: p.finish.card.boxShadow } : {}),
  };
}

/** One row of a group: its label on the left and its control on the right, a hairline above all but the first. */
export function rowLook(palette: Palette, options: { divider?: boolean } = {}): ViewStyle {
  const p = palette();
  return {
    paddingHorizontal: p.metric.rowPadX,
    paddingVertical: p.metric.rowPadY,
    gap: p.metric.rowGap,
    borderTopWidth: options.divider ? p.metric.borderWidth : 0,
    borderTopColor: p.border,
  };
}

/** The title above a group. It lines up with the rows' text, so it takes their horizontal padding. */
export function sectionTitleLook(palette: Palette): TextStyle {
  const p = palette();
  return { paddingHorizontal: p.metric.titlePadX, color: p.text, fontSize: p.metric.fontTitle, fontWeight: "600" };
}

/** Quieter text under a row's label: a description, a status or a hint. */
export function noteLook(palette: Palette, tone: "muted" | "text" = "muted"): TextStyle {
  const p = palette();
  return { color: tone === "text" ? p.text : p.muted, fontSize: p.metric.fontNote };
}

/** A page's main heading. */
export function headingLook(palette: Palette, size: "heading" | "display" = "heading"): TextStyle {
  const p = palette();
  return { color: p.text, fontSize: size === "display" ? p.metric.fontDisplay : p.metric.fontHeading, fontWeight: "600" };
}

/** The spacing shared by the views: gaps between a title and its group, between groups and inside a row's text. */
export function spacing(palette: Palette) {
  const { metric } = palette();
  return {
    text: { gap: metric.textGap } satisfies ViewStyle,
    group: { gap: metric.groupGap } satisfies ViewStyle,
    page: { gap: metric.pageGap } satisfies ViewStyle,
    wrap: { gap: metric.wrapGap } satisfies ViewStyle,
    stack: { gap: metric.stackGap } satisfies ViewStyle,
  };
}
