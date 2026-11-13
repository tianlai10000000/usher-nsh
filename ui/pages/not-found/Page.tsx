/** Renders an unmatched internal target while preserving the history's return path. */
import { Text, View } from "nsh";
import { ThemeButton } from "../../components/ThemeControls";
import { headingLook, spacing } from "../../theme/look";
import type { PageProps } from "../contracts";
export function NotFoundPage(props: PageProps) {
  const { palette, t } = props.appearance;
  return <View testID="route-not-found" style={spacing(palette).stack}>
    <Text style={headingLook(palette, "display")}>{t("router.notFound")}</Text>
    <Text style={{ color: palette().text }}>{props.navigation.current().location.path}</Text>
    <ThemeButton palette={palette} label={t("nav.back")} onPress={() => void props.navigation.goBack()} disabled={!props.navigation.canGoBack()} />
  </View>;
}
