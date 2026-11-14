/** Maps navigation shortcut commands to the existing guarded router facade. */
import type { Navigation } from "../../router/index";
import type { Command } from "../contracts";
export function navigationCommands(router: Navigation) {
  return (command: Command) => {
    if (command.module !== "navigation") throw new Error("shortcuts_unknown_module");
    switch (command.action) {
      case "back": return router.canGoBack() ? router.goBack() : undefined;
      case "forward": return router.canGoForward() ? router.goForward() : undefined;
    }
  };
}
