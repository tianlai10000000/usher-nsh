/** Owns native animation lifetimes and shared Gallery popup motion, independent of business state. */
import { cleanup, createAnimatedValue, createMotion, Easing, effect, platform, reduceMotion } from "nsh";
import type { SpringConfig, ViewStyle } from "nsh";
import type { ThemeAccessor } from "./index";

type Palette = ThemeAccessor;
export type PopupSide = "top" | "bottom" | "left" | "right";
const origins: Record<PopupSide, string> = { top: "bottom", bottom: "top", left: "right", right: "left" };

/** Native-only motion is disabled in terminals as well as by the system accessibility setting. */
export const motionReduced = () => reduceMotion() || platform.idiom === "terminal";

/** Call once per mounted owner; retargeting preserves the running spring's position and velocity. */
export function followSpring(target: () => number, spring: () => Omit<SpringConfig, "toValue">) {
  let shown = target();
  const value = createAnimatedValue(shown);
  effect(() => {
    const next = target();
    const reduced = motionReduced();
    const config = spring();
    if (reduced) value.set(next);
    else if (shown !== next) void value.spring(next, config);
    shown = next;
  });
  cleanup(() => { void value.stop(); });
  return value;
}

/** Converts the theme's CSS curve for native keyframes without maintaining a second set of values. */
export function nativeCurve(curve: string) {
  const points = curve.slice(curve.indexOf("(") + 1, curve.lastIndexOf(")")).split(",").map(Number);
  return points.length === 4 && points.every(Number.isFinite)
    ? Easing.bezier(points[0], points[1], points[2], points[3]) : Easing.ease;
}

/** CSS starting/ending states let NSH own exit retention, hit testing and interrupted reopen. */
export function popupMotion(palette: Palette, side: PopupSide = "bottom", tooltip = false) {
  const p = palette().motion;
  if (motionReduced()) return { style: { transitionDuration: 0 } satisfies ViewStyle, starting: undefined, ending: undefined };
  const distance = tooltip ? p.tooltipTravel : p.popupTravel;
  const away = side === "top" || side === "left" ? distance : -distance;
  const offset = side === "top" || side === "bottom" ? { translateY: away } : { translateX: away };
  const hidden: ViewStyle = { opacity: 0, transform: [{ scale: p.popupScale }, offset] };
  return {
    style: { transformOrigin: origins[side], transitionProperty: "opacity, transform",
      transitionDuration: p.popupMs, transitionTimingFunction: p.popupCurve } satisfies ViewStyle,
    starting: hidden,
    ending: hidden,
  };
}

/** NSH 0.5.0 lacks Select exit styles; upstream #1857 adds them after this release. Mobile keeps native sheet motion. */
export function createSelectMotion(palette: Palette, side: PopupSide = "bottom") {
  const motion = createMotion({ initial: { opacity: 0 } });
  let visible = false;
  const reset = () => { motion.reset(); motion.opacity.set(0); };
  effect(() => {
    if (motionReduced()) {
      motion.reset();
      motion.opacity.set(visible ? 1 : 0);
    }
  });
  cleanup(() => motion.stop());
  return {
    get style(): ViewStyle { return platform.desktop ? { transformOrigin: origins[side], ...motion.style } : {}; },
    show() {
      visible = true;
      const p = palette().motion;
      if (!platform.desktop || motionReduced()) { motion.reset(); motion.opacity.set(1); return; }
      const away = side === "top" || side === "left" ? p.popupTravel : -p.popupTravel;
      const axis = side === "top" || side === "bottom" ? "translateY" : "translateX";
      void motion.play({ duration: p.popupMs, easing: nativeCurve(p.popupCurve),
        keyframes: { opacity: [0, 1], scale: [p.popupScale, 1], [axis]: [away, 0] } });
    },
    hide() { visible = false; reset(); },
  };
}
