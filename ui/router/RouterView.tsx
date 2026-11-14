/** Mounts exactly one registered page per history entry with a recoverable error boundary. */
import { cleanup, ErrorBoundary, Text, View } from "nsh";
import { ThemeButton } from "../components/ThemeControls";
import type { JSX } from "nsh/jsx-runtime";
import type { PageKey } from "../pages/routes.generated";
import type { PageProps } from "../pages/contracts";
export type PageRegistry = Record<PageKey, (props: PageProps) => JSX.Element>;
/** The composition root supplies page components; the renderer never owns the catalog. */
export function RouterView(props: PageProps & { pages: PageRegistry }) {
  const palette = props.appearance.appearance.palette;
  const t = props.appearance.preferences.t;
  return <Each of={[props.navigation.current()]} key={entry => JSON.stringify([entry.key, entry.location.page])}>{entry => {
    const Page = props.pages[entry().location.page as PageKey];
    return <ErrorBoundary fallback={(_error, reset) => <View testID="route-error">
      <Text style={{ color: palette().text }}>{t("router.failed")}</Text>
      <ThemeButton palette={palette} label={t("settings.retry")} onPress={reset} />
      <ThemeButton palette={palette} label={t("nav.back")} onPress={() => props.navigation.goBack()} disabled={!props.navigation.canGoBack()} />
    </View>}><MountedPage Page={Page} pageProps={props} entry={entry().key} page={entry().location.page} /></ErrorBoundary>;
  }}</Each>;
}

/** Acquires a page lease only inside the successfully mounted page error boundary. */
function MountedPage(props: { Page: (props: PageProps) => JSX.Element; pageProps: PageProps; entry: string; page: string }) {
  const content = <props.Page {...props.pageProps} />;
  const release = props.pageProps.shortcuts?.mountPage(props.entry, props.page);
  if (release) cleanup(release);
  return content;
}
