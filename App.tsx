/** Composes application/window state, the router and the shell without page-ID branches. */
import { ErrorBoundary, locale, mount, Text, View } from "nsh";
import { Button } from "nsh/base";
import { createUpdates } from "./ui/features/updates";
import { Frame } from "./ui/layout/Frame";
import { createRouter, RouterProvider } from "./ui/router";
import { bindNativeRouting } from "./ui/router/native";
import { RouterView } from "./ui/router/RouterView";
import { pages } from "./ui/pages";
import { SettingsProvider } from "./ui/state/settings/store";
import { createAppState } from "./ui/state/app";
import { createGlobalShortcuts, createShortcuts, shortcutIndicator } from "./ui/shortcuts";
import { bindMainWindowShortcuts } from "./ui/shortcuts/modules/main-window";
import { translate } from "./ui/i18n";

function App() {
  createGlobalShortcuts();
  const appearance = createAppState();
  const window = appearance.window;
  const updates = createUpdates(appearance);
  const navigation = createRouter(window.routing);
  bindNativeRouting(navigation);
  bindMainWindowShortcuts(navigation, appearance);
  const shortcuts = createShortcuts(window.scope, navigation);
  const shortcutStatus = shortcutIndicator(shortcuts, appearance.preferences.t);
  return <SettingsProvider value={appearance.settings.view}>
    <RouterProvider value={navigation}>
      <Frame appearance={appearance} navigation={navigation} updates={updates.indicator} shortcuts={shortcutStatus} blockShortcuts={shortcuts.block}>
        <RouterView pages={pages} appearance={appearance} navigation={navigation} session={window.session} shortcuts={shortcuts} updates={updates} />
      </Frame>
    </RouterProvider>
  </SettingsProvider>;
}
function Root() {
  return <ErrorBoundary fallback={(_error, reset) => <View testID="router-initialization-error">
    <Text>{translate("system", locale(), "router.initializationFailed")}</Text>
    <Button onPress={reset}><Text>{translate("system", locale(), "settings.retry")}</Text></Button>
  </View>}><App /></ErrorBoundary>;
}
mount(Root);
