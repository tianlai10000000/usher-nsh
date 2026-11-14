/** A hot-reloadable native window composition using production projections and routing. */
import { cleanup, effect, Text, View } from "nsh";
import { currentWindow } from "nsh/desktop";
import { WindowState } from "usher/nsh";
import { createWindowState } from "../../../ui/state/window/index";
import { createAppState } from "../../../ui/state/app";
import { createRouter } from "../../../ui/router/index";
import { RouterView } from "../../../ui/router/RouterView";
import { pages } from "../../../ui/pages/index";
import catalog from "../../../ui/pages/routes.json";
import { observe, removed } from "./controller";
export function Probe() {
  const generation = 1; // Replaced only in the isolated source copy to trigger real NSH HMR.
  const activeCatalog = { ...catalog, identity: `${catalog.identity}-native-${generation}` };
  const window = createWindowState(activeCatalog);
  const appearance = createAppState(window);
  const router = createRouter(window.routing);
  const host = currentWindow();
  const id = host.id;
  const scope = WindowState.register(JSON.stringify(activeCatalog), id).storeId;
  cleanup(removed);
  effect(() => {
    if (window.routing.status() === "live" && appearance.settings.view.status() === "live") {
      observe(id, { host, scope, window, appearance, router, generation });
    }
  });
  return <View>
    <Text>{`Native acceptance generation ${generation}: ${id}`}</Text>
    <RouterView pages={pages} appearance={appearance} navigation={router} session={window.session} />
  </View>;
}
