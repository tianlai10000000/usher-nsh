/**
 * The rounded content panel to the right of the menus.
 *
 * It owns the panel's fill, clipped corners, hairline edge and the scrolling page area. The
 * panel fills whatever width the menus leave, so it grows to the left when the secondary menu
 * hides.
 */
import { platform, Scroll, View } from "nsh";
import type { ScrollbarTheme } from "nsh/base";
import type { JSX } from "nsh/jsx-runtime";
import type { AppearanceStore } from "../../state/appearance";
import { contentBackgroundColor } from "../../theme/look";

// The thumb is laid out with the scroll viewport so its right edge tracks window resizes. Its
// color is a theme token, read when drawn, so the bar follows the theme and color scheme.
const macScrollBar = (thumbColor: () => string) => ({
  mode: "hover",
  motion: { idle: 700, exit: 300 },
  minThumbLength: 32,
  vertical: {
    style: { position: "absolute", top: 2, right: 0, bottom: 2, width: 12, paddingLeft: 4, paddingRight: 2 },
    thumbClass: "rounded-full",
    thumbStyle: { get backgroundColor() { return thumbColor(); } },
  },
}) satisfies ScrollbarTheme;

export function ContentPanel(props: { appearance: AppearanceStore; children: JSX.Element }) {
  const { palette } = props.appearance;
  const desktop = platform.desktop;
  // Built once so a theme change never replaces the bar.
  const scrollBar = platform.os === "macos" ? macScrollBar(() => palette().scrollbarThumb) : true;
  const radius = () => (desktop ? palette().panelRadius : 0);
  const inset = () => (desktop ? palette().metric.panelInset : 0);

  // Outer view: fill and shadow. Inner view: clips the content. The border is an overlay drawn
  // on top of the clip, so the rounded edge is antialiased once.
  return (
    <View testID="main-panel" class="min-h-0 min-w-0 flex-1"
      style={{ marginRight: inset(), marginBottom: inset(), borderRadius: radius() }}>
      <View class="min-h-0 min-w-0 flex-1 overflow-hidden"
        style={{ borderRadius: radius(),
          backgroundColor: contentBackgroundColor(palette().panel, props.appearance.contentBackgroundTransparency()) }}>
        <Scroll testID="main-content" class="min-h-0 flex-1" contentClass="grow p-4" scrollbar={scrollBar}>
          {props.children}
        </Scroll>
      </View>
      <If when={desktop}>
        <View style={{ position: "absolute", left: 0, top: 0, right: 0, bottom: 0, pointerEvents: "none",
          borderRadius: radius(), borderWidth: 1, borderColor: palette().shellBorder }} />
      </If>
    </View>
  );
}
