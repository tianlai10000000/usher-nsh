/**
 * The rounded content panel to the right of the menus.
 *
 * It owns the panel's fill, clipped corners, hairline edge and the scrolling page area. The
 * panel fills whatever width the menus leave, so it grows to the left when the secondary menu
 * hides.
 */
import { platform, Scroll, View, type ViewStyle } from "nsh";
import type { ScrollbarTheme } from "nsh/base";
import type { JSX } from "nsh/jsx-runtime";
import type { AppearanceStore } from "../../state/appearance";
import { contentBackgroundColor } from "../../theme/look";

/** Keeps one scrollbar object while its part getters follow theme changes. */
const themedScrollBar = (palette: AppearanceStore["palette"]): ScrollbarTheme => ({
  mode: "hover",
  get motion() { return { idle: palette().metric.scrollbarIdleMs,
    enter: palette().metric.scrollbarEnterMs, exit: palette().metric.scrollbarExitMs }; },
  get minThumbLength() { return palette().metric.scrollbarMinThumb; },
  vertical: {
    get style(): ViewStyle { return { position: "absolute", top: 0, right: 0, bottom: 0,
      width: palette().metric.scrollbar, padding: palette().metric.scrollbarInset, alignItems: "flex-end" }; },
    get thumbClass() {
      const p = palette();
      const color = p.scrollbarThumbHover.replace(/\s+/g, "_");
      return `rounded-full w-[${p.metric.scrollbarThumb}px] bg-[${p.scrollbarThumb.replace(/\s+/g, "_")}]
        hover:w-[${p.metric.scrollbarThumbHover}px] active:w-[${p.metric.scrollbarThumbHover}px]
        hover:bg-[${color}] active:bg-[${color}]
        transition-all duration-[${p.metric.scrollbarEnterMs}ms] motion-reduce:transition-none`;
    },
  },
});

export function ContentPanel(props: { appearance: AppearanceStore; children: JSX.Element }) {
  const { palette } = props.appearance;
  const desktop = platform.desktop;
  // Built once so a theme change never replaces the bar.
  const scrollBar = platform.os === "macos" ? themedScrollBar(palette) : true;
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
