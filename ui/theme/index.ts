/** Resolves theme files into semantic surface, control and layout tokens. */
import { platform } from "nsh";
import type { StyleThemeValue } from "usher/nsh";
import rootThemeFile from "./builtin/default/theme.json";

export type Scheme = "light" | "dark";
export type ComponentFamily = "nsh" | "appkit";

/** The four colors the in-app palette editor edits. */
export type CustomPalette = {
  background: string;
  surface: string;
  text: string;
  accent: string;
};

export type CustomTheme = {
  id: string;
  name: string;
  light: CustomPalette;
  dark: CustomPalette;
};

/** A theme with every token resolved by the Rust registry: inheritance and `var()` are applied. */
export type StyleTheme = StyleThemeValue;

/**
 * The root every theme chain ends in. It is read from the file the registry bundles, so the ID
 * and the token set have a single source.
 */
export const defaultThemeId = rootThemeFile.id;

type RawTokens = Record<string, string | number>;

/** Whether metrics come from the touch table: a device without a pointer-first desktop idiom. */
export const touchDevice = !platform.desktop;

/** Merges the shared table into a scheme's, as the registry does for the themes it sends. */
function tokenTable(shared: RawTokens, scheme: RawTokens): Record<string, string> {
  return Object.fromEntries(Object.entries({ ...shared, ...scheme }).map(([name, value]) => [name, String(value)]));
}

/**
 * Used until the first settings snapshot arrives and when the selected theme is unavailable.
 * It is the root theme file itself, so it matches what the registry returns for that theme.
 */
export const fallbackTheme: StyleTheme = {
  id: rootThemeFile.id,
  name: rootThemeFile.name,
  names: {},
  material: rootThemeFile.material,
  cssVars: {
    touch: tokenTable({}, rootThemeFile.cssVars.touch),
    light: tokenTable(rootThemeFile.cssVars.shared, rootThemeFile.cssVars.light),
    dark: tokenTable(rootThemeFile.cssVars.shared, rootThemeFile.cssVars.dark),
  },
};

/** Reads the palette editor's four colors from a resolved theme. */
export function editorPalette(theme: StyleTheme, scheme: Scheme): CustomPalette {
  const tokens = theme.cssVars[scheme];
  return {
    background: tokens.background,
    surface: tokens.card,
    text: tokens.foreground,
    accent: tokens.primary,
  };
}

function linearColor(channel: string): number {
  const srgb = Number.parseInt(channel, 16) / 255;
  return srgb <= 0.04045 ? srgb / 12.92 : ((srgb + 0.055) / 1.055) ** 2.4;
}

function accentForeground(accent: string): string {
  const red = linearColor(accent.slice(1, 3));
  const green = linearColor(accent.slice(3, 5));
  const blue = linearColor(accent.slice(5, 7));
  const luminance = 0.2126 * red + 0.7152 * green + 0.0722 * blue;
  // Pick whichever of black and white has the greater contrast ratio.
  return luminance >= 0.179 ? "#000000" : "#ffffff";
}

/**
 * Projects a theme's tokens onto the semantic roles the views use. This is the only place that
 * names tokens: a theme changes the look by changing token values, never by adding a branch here.
 * A token a theme omits falls back to the root theme, which defines all of them.
 */
export function resolveTheme(
  theme: StyleTheme | undefined,
  scheme: Scheme,
  options: {
    accent?: string | null;
    componentFamily?: ComponentFamily;
    appKitAvailable?: boolean;
    /** Overrides the device check. Tests use it to compare both tiers on one platform. */
    touch?: boolean;
  } = {},
) {
  const style = theme ?? fallbackTheme;
  // On a touch device a metric the theme states for touch wins over its pointer value.
  const token = (name: string): string =>
    (options.touch ?? touchDevice ? style.cssVars.touch[name] : undefined) ??
    style.cssVars[scheme][name] ??
    fallbackTheme.cssVars[scheme][name];
  const accent = options.accent ?? token("primary");
  // Metrics are whole points or ratios. The registry has validated them, and the root theme
  // defines every one, so a missing or malformed value can only come from a stale snapshot.
  const metric = (name: string): number => Number.parseFloat(token(name));
  // A finish is a fill, a gradient drawn over it, an edge color and shadow layers. `none` and
  // `transparent` mean the theme draws nothing for that part, and the control keeps its own.
  const finish = (state: string) => ({
    fill: token(`finish-${state}-fill`),
    image: token(`finish-${state}-image`),
    shadow: token(`finish-${state}-shadow`),
    edge: token(`finish-${state}-edge`),
  });

  return {
    background: token("background"),
    surface: token("card"),
    text: token("foreground"),
    accent,
    accentText: accentForeground(accent),
    border: token("border"),
    muted: token("muted-foreground"),
    sidebar: token("sidebar"),
    menuSurface: token("menu-surface"),
    menuSelected: token("menu-selected"),
    panel: token("panel"),
    panelRadius: metric("radius"),
    itemSelected: token("item-selected"),
    itemHover: token("item-hover"),
    itemShadow: token("item-shadow"),
    shellBorder: token("shell-border"),
    shellShadow: token("shell-shadow"),
    windowTint: token("window-tint"),
    scrim: token("scrim"),
    scrollbarThumb: token("scrollbar-thumb"),
    /** Sizes, spacing, type, radii and feedback, in points or ratios. */
    metric: {
      buttonHeight: metric("size-button"),
      buttonHeightSmall: metric("size-button-small"),
      buttonHeightLarge: metric("size-button-large"),
      iconButton: metric("size-icon-button"),
      fieldHeight: metric("size-field"),
      rowHeight: metric("size-row"),
      switchWidth: metric("size-switch-width"),
      switchHeight: metric("size-switch-height"),
      switchKnob: metric("size-switch-knob"),
      sliderTrack: metric("size-slider-track"),
      sliderThumb: metric("size-slider-thumb"),
      navButton: metric("size-nav-button"),
      sidebarItem: metric("size-sidebar-item"),
      statusBar: metric("size-status-bar"),
      controlIcon: metric("size-control-icon"),
      avatar: metric("size-avatar"),
      toolbarIcon: metric("size-toolbar-icon"),
      railIcon: metric("size-rail-icon"),
      railUtilityIcon: metric("size-rail-utility-icon"),
      toolbarButton: metric("size-toolbar-button"),
      titlebarIcon: metric("size-titlebar-icon"),
      titlebarButton: metric("size-titlebar-button"),
      badge: metric("size-badge"),
      railWidth: metric("layout-rail-width"),
      menuWidth: metric("layout-menu-width"),
      panelInset: metric("layout-panel-inset"),
      titleBar: metric("layout-title-bar"),
      wideBreakpoint: metric("layout-wide-breakpoint"),
      layoutMs: metric("motion-layout"),
      buttonPadX: metric("space-button-x"),
      buttonPadXSmall: metric("space-button-small-x"),
      buttonPadXLarge: metric("space-button-large-x"),
      buttonGap: metric("space-button-gap"),
      fieldPadX: metric("space-field-x"),
      cardPad: metric("space-card"),
      stackGap: metric("space-stack"),
      wrapGap: metric("space-wrap"),
      sectionGap: metric("space-section"),
      rowPadX: metric("space-row-x"),
      rowPadY: metric("space-row-y"),
      rowGap: metric("space-row-gap"),
      textGap: metric("space-text-gap"),
      groupGap: metric("space-group-gap"),
      pageGap: metric("space-page-gap"),
      titlePadX: metric("space-title-x"),
      sidebarPadX: metric("space-sidebar-x"),
      sidebarItemPadX: metric("space-sidebar-item-x"),
      statusPadX: metric("space-status-x"),
      fontBody: metric("font-body"),
      fontButton: metric("font-button"),
      fontButtonSmall: metric("font-button-small"),
      fontButtonLarge: metric("font-button-large"),
      fontField: metric("font-field"),
      fontTitle: metric("font-title"),
      fontHeading: metric("font-heading"),
      fontDisplay: metric("font-display"),
      fontNote: metric("font-note"),
      radiusControl: metric("radius-control"),
      radiusCard: metric("radius-card"),
      radiusItem: metric("radius-item"),
      borderWidth: metric("size-border"),
      focusWidth: metric("focus-width"),
      focusOffset: metric("focus-offset"),
      pressMs: metric("motion-press"),
      stateMs: metric("motion-state"),
      pressScale: metric("feedback-press-scale"),
      pressScaleIcon: metric("feedback-press-scale-icon"),
      hoverWash: metric("feedback-hover-wash"),
      disabledOpacity: metric("feedback-disabled-opacity"),
    },
    /** Surface finishes by control state: a fill, a gradient over it, an edge and shadow layers. */
    finish: {
      raised: finish("raised"),
      hover: finish("hover"),
      pressed: finish("pressed"),
      on: finish("on"),
      accent: finish("accent"),
      field: { boxShadow: token("finish-field-shadow") },
      card: { boxShadow: token("finish-card-shadow") },
    },
    ring: token("ring"),
    /** The keyboard focus halo: an opaque color that reads at 3:1 on every surface. */
    focusRing: token("focus-ring"),
    itemPressed: token("item-pressed"),
    borderStrong: token("border-strong"),
    /** Whether the theme lets the window material show through its surfaces. */
    glass: style.material === "glass",
    nativeAppKit: options.componentFamily === "appkit" && options.appKitAvailable === true,
  };
}

export function isModernMac(): boolean {
  return platform.os === "macos" && Number.parseInt(platform.version, 10) >= 26;
}
