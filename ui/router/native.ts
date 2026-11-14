/** Owns NSH host event listeners without introducing a second navigation history. */
import { UrlScheme } from "nsh";
import { currentWindow, focusedWindow } from "nsh/desktop";
import { BackHandler } from "nsh/mobile";
import manifest from "../../nsh.json";
import policy from "../config/routing.json";
import { observe, onDispose } from "../state-runtime/nsh";
import type { Navigation } from "./index";
import { bindHostRouting } from "./host";
/** Application links belong to the main window; back belongs to the active window.
 * Mobile and headless hosts have no desktop state and use their rendering owner. */
export function bindNativeRouting(router: Navigation) {
  const window = currentWindow();
  const dispose = bindHostRouting(router, {
    onOpen: listener => window.main ? UrlScheme.on("open", event => listener(event.url)) : () => {},
    onBack: listener => BackHandler.on("press", listener),
    observeReady: listener => { observe(listener); return () => {}; },
    ownsBack: () => { const focused = focusedWindow(); return focused ? focused.id === window.id : window.main; },
  }, { schemes: manifest.schemes, hosts: policy.hosts });
  onDispose(dispose);
  return dispose;
}
