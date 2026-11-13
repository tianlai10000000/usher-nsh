/** Home page entry, with retained counter data owned by the window store. */
import { Text, View } from "nsh";
import { ThemeButton } from "../../components/ThemeControls";
import { EffectiveAppearance } from "../../features/settings/EffectiveAppearance";
import { headingLook, spacing } from "../../theme/look";
import type { PageProps } from "../contracts";
export function HomePage(props: PageProps) {
  const { palette, t } = props.appearance;
  return <View class="flex-1 items-center justify-center" style={spacing(palette).stack}>
    <Text style={headingLook(palette, "display")}>{t("home.greeting")}</Text>
    <ThemeButton palette={palette} label={t("home.click")} prominent native={false}
      onPress={() => void props.session.change({ kind: "increment" })} />
    <Text style={{ color: palette().text }}>{t("home.clicks", { count: props.session.value().clicks })}</Text>
    <EffectiveAppearance appearance={props.appearance} />
  </View>;
}
