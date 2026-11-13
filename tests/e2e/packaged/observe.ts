/** Observes the real product router only in the isolated packaged acceptance source copy. */
import { currentWindow } from "nsh/desktop";
import type { Navigation } from "../../../ui/router";
import { observe } from "../../../ui/state/runtime/nsh";
import config from "./config.json";
export function observePackagedRouter(router: Navigation) {
  const window = currentWindow();
  observe(() => {
    if (router.status() === "live") console.log(config.marker + JSON.stringify({
      path: router.current().location.path,
      route: router.route(),
      entry: router.current().key,
      pending: router.pending(),
      focused: window.state()?.focused ?? null,
    }));
  });
}
