/**
 * The narrow fixed menu (the rail) down the window's left edge.
 *
 * Top: primary destinations. Bottom: the update button, the settings button and the account
 * button. It never hides; the wider secondary menu next to it is the part that toggles. The
 * update and account buttons are driven by optional contracts: without one the button is shown
 * disabled instead of pretending to work. System symbols come from the shared icon catalog;
 * bottom utility icons use a smaller theme metric than primary destinations.
 */
import { View } from "nsh";
import { Avatar, Button } from "nsh/base";
import { SystemIcon } from "../../components/SystemIcon";
import navigationIcons from "../../config/navigation-icons.json";
import type { MessageKey } from "../../i18n";
import type { Navigation, RouteId } from "../../navigation";
import type { AppearanceStore } from "../../state/appearance";
import { iconButtonLook } from "../../theme/look";
import type { AccountEntry } from "../contracts/account";
import type { UpdateIndicator } from "../contracts/updates";
import { ButtonTooltip } from "./ButtonTooltip";

const primaryRoutes: readonly RouteId[] = ["home"];

export function Rail(props: {
  appearance: AppearanceStore;
  navigation: Navigation;
  updates?: UpdateIndicator;
  account?: AccountEntry;
}) {
  const { palette, t } = props.appearance;
  const { route, navigate } = props.navigation;

  function routeButton(id: RouteId, labelKey: MessageKey, testID: string, iconSize: () => number) {
    const selected = () => route() === id;
    const look = () => iconButtonLook(palette, selected());
    const icon = navigationIcons[id];
    return (
      <Button testID={testID} accessibilityLabel={t(labelKey)} selected={selected()}
        tooltip={<ButtonTooltip appearance={props.appearance} testID={`${testID}-tooltip`} placement="right"
          label={t(labelKey)} />}
        style={look().style} hoverStyle={look().hoverStyle} activeStyle={look().activeStyle}
        focusStyle={look().focusStyle} onPress={() => navigate(id)}>
        <SystemIcon name={selected() ? icon.selectedSymbol : icon.symbol}
          fallback={icon.fallback} size={iconSize()}
          color={palette().text} />
      </Button>
    );
  }

  const look = () => iconButtonLook(palette);
  const updateAvailable = () => props.updates?.available() === true;

  return (
    <View testID="application-rail" role="navigation" accessibilityLabel={t("nav.rail")}
      class="min-h-0 items-center"
      style={{ width: palette().metric.railWidth, paddingBottom: palette().metric.panelInset,
        gap: palette().metric.textGap }}>
      <Each of={primaryRoutes}>
        {(id) => routeButton(id(), `nav.${id()}`, `rail-${id()}`, () => palette().metric.railIcon)}
      </Each>
      <View class="flex-1" />
      <Button testID="rail-update" accessibilityLabel={t(updateAvailable() ? "nav.updateAvailable" : "nav.update")}
        tooltip={<ButtonTooltip appearance={props.appearance} testID="rail-update-tooltip" placement="right"
          label={t(updateAvailable() ? "nav.updateAvailable" : "nav.update")} />}
        disabled={props.updates === undefined}
        style={look().style} hoverStyle={look().hoverStyle} activeStyle={look().activeStyle}
        focusStyle={look().focusStyle} disabledStyle={look().disabledStyle}
        onPress={() => props.updates?.request()}>
        <SystemIcon name={navigationIcons.update.symbol} fallback={navigationIcons.update.fallback}
          size={palette().metric.railUtilityIcon}
          color={palette().text} />
        <If when={updateAvailable()}>
          <View testID="rail-update-badge" class="absolute"
            style={{ top: 4, right: 4, width: palette().metric.badge, height: palette().metric.badge,
              borderRadius: palette().metric.badge / 2, backgroundColor: palette().accent }} />
        </If>
      </Button>
      {routeButton("settings", "nav.openSettings", "rail-settings", () => palette().metric.railUtilityIcon)}
      <Button testID="rail-account" accessibilityLabel={t("nav.account")}
        tooltip={<ButtonTooltip appearance={props.appearance} testID="rail-account-tooltip" placement="right"
          label={t("nav.account")} />}
        disabled={props.account === undefined}
        style={{ width: palette().metric.avatar, height: palette().metric.avatar,
          borderRadius: palette().metric.avatar / 2 }}
        focusStyle={look().focusStyle} disabledStyle={look().disabledStyle}
        onPress={() => props.account?.open()}>
        <Avatar name={props.account?.name() ?? null} src={props.account?.picture() ?? null}
          accessibilityLabel={t("nav.account")}
          style={{ width: palette().metric.avatar, height: palette().metric.avatar,
            borderRadius: palette().metric.avatar / 2 }}
          imageStyle={{ borderRadius: palette().metric.avatar / 2 }}
          fallbackStyle={{ borderRadius: palette().metric.avatar / 2, backgroundColor: palette().menuSelected,
            alignItems: "center", justifyContent: "center" }}
          fallback={<SystemIcon name={navigationIcons.account.symbol} fallback={navigationIcons.account.fallback}
            size={palette().metric.railUtilityIcon}
            color={palette().text} />} />
      </Button>
    </View>
  );
}
