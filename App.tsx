/** Composes application/window state, the router and the shell without page-ID branches. */
import { ErrorBoundary, locale, mount, Text, View } from "nsh";
import { Button } from "nsh/base";
import { Frame } from "./ui/layout/Frame";
import { createRouter, RouterProvider } from "./ui/router";
import { bindNativeRouting } from "./ui/router/native";
import { RouterView } from "./ui/router/RouterView";
import { pages } from "./ui/pages";
import { SettingsProvider } from "./ui/state/app";
import { createAppearanceStore } from "./ui/state/appearance";
import { createWindowState } from "./ui/state/window";
import { translate } from "./ui/i18n";

function App() {
  const window = createWindowState();
  const appearance = createAppearanceStore(window.session);
  const navigation = createRouter(window.routing);
  bindNativeRouting(navigation);
  return <SettingsProvider value={appearance.state}>
    <RouterProvider value={navigation}>
      <Frame appearance={appearance} navigation={navigation}>
        <RouterView pages={pages} appearance={appearance} navigation={navigation} session={window.session} />
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
