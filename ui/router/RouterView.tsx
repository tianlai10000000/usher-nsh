/** Mounts exactly one registered page per history entry with a recoverable error boundary. */
import { ErrorBoundary, Text, View } from "nsh";
import { ThemeButton } from "../components/ThemeControls";
import type { JSX } from "nsh/jsx-runtime";
import type { PageKey } from "../pages/routes.generated";
import type { PageProps } from "../pages/contracts";
export type PageRegistry = Record<PageKey, (props: PageProps) => JSX.Element>;
/** The composition root supplies page components; the renderer never owns the catalog. */
export function RouterView(props: PageProps & { pages: PageRegistry }) {
  const { palette, t } = props.appearance;
  return <Each of={[props.navigation.current()]} key={entry => JSON.stringify([entry.key, entry.location.page])}>{entry => {
    const Page = props.pages[entry().location.page as PageKey];
    return <ErrorBoundary fallback={(_error, reset) => <View testID="route-error">
      <Text style={{ color: palette().text }}>{t("router.failed")}</Text>
      <ThemeButton palette={palette} label={t("settings.retry")} onPress={reset} />
      <ThemeButton palette={palette} label={t("nav.back")} onPress={() => void props.navigation.goBack()} disabled={!props.navigation.canGoBack()} />
    </View>}><Page {...props} /></ErrorBoundary>;
  }}</Each>;
}
