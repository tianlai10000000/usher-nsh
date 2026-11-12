/**
 * Window chrome measurements for the frame layout.
 *
 * On desktop the system title bar is hidden and the app draws its own, so the frame needs the
 * height of that bar and the room the system window controls take at either end. This file owns
 * that single source and the title bar mode switch with its cleanup; the views only read it.
 */
import { cleanup, effect, signal } from "nsh";
import { currentWindow, isAvailable as hasWindowChrome, titleBar } from "nsh/desktop";
import type { AppearanceStore } from "../../state/appearance";

export function createWindowChrome(appearance: AppearanceStore) {
  const { palette } = appearance;

  // titleBar() is all zeros in full screen. Keep the last measured height so the bar does not
  // jump when entering or leaving it.
  const measuredHeight = signal<number | undefined>(undefined);
  effect(() => {
    const height = hasWindowChrome ? titleBar().height : 0;
    if (height > 0) measuredHeight.set(height);
  });

  if (hasWindowChrome) {
    const window = currentWindow();
    effect(() => {
      window.set({ titleBar: { style: "hidden", height: palette().metric.titleBar } });
      cleanup(() => window.set({ titleBar: { style: "default" } }));
    });
  }

  return {
    /** The drawn bar's height: AppKit's measured value, else the theme's `layout-title-bar`. */
    barHeight: () => measuredHeight() ?? palette().metric.titleBar,
    /**
     * Where the first control starts. `titleBar().left` already ends after the traffic lights;
     * in full screen they are gone and the controls move to the edge.
     */
    leftInset: () => {
      const left = hasWindowChrome ? titleBar().left : 0;
      return left > 0 ? left : palette().metric.panelInset;
    },
    /** Room for caption buttons at the trailing end (Windows, GTK), plus the edge inset. */
    rightInset: () => (hasWindowChrome ? titleBar().right : 0) + palette().metric.panelInset,
  };
}

export type WindowChrome = ReturnType<typeof createWindowChrome>;
