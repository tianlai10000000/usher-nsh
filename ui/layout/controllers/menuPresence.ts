/**
 * Show, hide and animate the secondary menu.
 *
 * Wide desktop windows dock the menu beside the content; the content grows into the freed width
 * while a layout animation runs. Narrow windows and mobile show it as a drawer instead. The
 * docked preference is saved through the appearance store; the toggle reacts at once and the
 * save happens behind it.
 */
import { cleanup, effect, LayoutAnimation, platform, reduceMotion, signal, windowSize } from "nsh";
import type { AppState } from "../../state/app";

export function createMenuPresence(appearance: AppState) {
  const palette = appearance.appearance.palette;
  const setSidebarDocked = appearance.preferences.setSidebarDocked;
  const sidebarDocked = appearance.preferences.sidebarDocked;
  const wide = () => platform.desktop && windowSize().width >= palette().metric.wideBreakpoint;
  const slideMs = () => (reduceMotion() ? 0 : palette().metric.layoutMs);

  const drawerOpen = signal(false);
  const dockedLocal = signal(sidebarDocked());
  effect(() => dockedLocal.set(sidebarDocked()));
  const docked = () => wide() && dockedLocal();

  // The menu stays mounted until its slide-out has finished.
  const mounted = signal(docked());
  effect(() => {
    if (docked()) {
      mounted.set(true);
      return;
    }
    const timer = setTimeout(() => mounted.set(false), slideMs());
    cleanup(() => clearTimeout(timer));
  });

  effect(() => {
    if (wide()) drawerOpen.set(false);
  });

  // Presses during a slide only change the wanted state; the slide in flight always finishes,
  // then the latest wish is applied, so the menu never stops half way.
  let wanted = dockedLocal();
  let sliding = false;
  let settleTimer: ReturnType<typeof setTimeout> | undefined;
  cleanup(() => clearTimeout(settleTimer));

  function settle() {
    if (sliding || wanted === dockedLocal()) return;
    sliding = true;
    if (!reduceMotion()) {
      LayoutAnimation.configureNext(LayoutAnimation.create(palette().metric.layoutMs, "easeInEaseOut", "opacity"));
    }
    dockedLocal.set(wanted);
    const saving = setSidebarDocked(wanted);
    settleTimer = setTimeout(() => {
      sliding = false;
      settle();
    }, slideMs());
    return saving;
  }

  function toggle() {
    if (!wide()) {
      drawerOpen.update((open) => !open);
      return;
    }
    wanted = !wanted;
    return settle();
  }

  return { wide, docked, mounted, drawerOpen, toggle };
}

export type MenuPresence = ReturnType<typeof createMenuPresence>;
