/**
 * The app's title bar: menu toggle, history arrows, breadcrumbs and a light/dark switch.
 * Settings opens from the rail, so the bar carries no settings button.
 *
 * Desktop hides the system title bar and draws this one across the whole window width, leaving
 * room for the system window controls at either end. Everything that is not a control is a drag
 * handle for moving the window. Mobile uses the same bar below the safe area.
 */
import { colorScheme, platform, View } from "nsh";
import { Button } from "nsh/base";
import { SystemIcon } from "../../components/SystemIcon";
import type { Navigation } from "../../router";
import type { AppearanceStore } from "../../state/appearance";
import { iconButtonLook, spacing } from "../../theme/look";
import type { MenuPresence } from "../controllers/menuPresence";
import { Breadcrumbs, HistoryControls } from "./Navigation";
import type { WindowChrome } from "../controllers/windowChrome";
import { ButtonTooltip } from "./ButtonTooltip";

export function TitleBar(props: {
  appearance: AppearanceStore;
  navigation: Navigation;
  chrome: WindowChrome;
  menu: MenuPresence;
}) {
  const { palette, t } = props.appearance;
  const desktop = platform.desktop;
  const toggleLook = () => iconButtonLook(palette);
  // The effective scheme, so a "system" preference still flips to the opposite of what is shown.
  const dark = () => colorScheme() === "dark";
  const colorLook = () => iconButtonLook(palette, false, true);

  return (
    // The whole bar drags the window. Buttons inside stay clickable because views with press
    // handlers are excluded, so no separate drag layer (which the spacer would cover) is needed.
    <View testID="title-bar" class="min-w-0 flex-row items-center" desktop={{ windowDrag: true }}
      style={{
        height: props.chrome.barHeight(),
        paddingLeft: desktop ? props.chrome.leftInset() : palette().metric.panelInset + palette().metric.textGap,
        paddingRight: props.chrome.rightInset(),
        gap: palette().metric.wrapGap,
      }}>
      <View class="flex-row items-center" style={spacing(palette).text}>
        <Button testID="sidebar-toggle"
          accessibilityLabel={t(props.menu.docked() ? "nav.closeSidebar" : "nav.sidebar")}
          tooltip={<ButtonTooltip appearance={props.appearance} testID="sidebar-toggle-tooltip" placement="bottom"
            label={t(props.menu.docked() ? "nav.closeSidebar" : "nav.sidebar")} />}
          style={toggleLook().style} hoverStyle={toggleLook().hoverStyle}
          activeStyle={toggleLook().activeStyle} focusStyle={toggleLook().focusStyle}
          onPress={props.menu.toggle}>
          <SystemIcon name="sidebar.left" fallback="▯" color={palette().text} size={palette().metric.toolbarIcon} />
        </Button>
        <HistoryControls appearance={props.appearance} navigation={props.navigation} />
      </View>
      <Breadcrumbs appearance={props.appearance} navigation={props.navigation} />
      <View class="h-full min-w-4 flex-1" />
      <Button testID="titlebar-color-mode" accessibilityLabel={t(dark() ? "nav.switchToLight" : "nav.switchToDark")}
        tooltip={<ButtonTooltip appearance={props.appearance} testID="titlebar-color-mode-tooltip" placement="bottom"
          label={t(dark() ? "nav.switchToLight" : "nav.switchToDark")} />}
        disabled={props.appearance.busy()}
        style={colorLook().style} hoverStyle={colorLook().hoverStyle}
        activeStyle={colorLook().activeStyle} focusStyle={colorLook().focusStyle}
        disabledStyle={colorLook().disabledStyle}
        onPress={() => void props.appearance.choose(dark() ? "light" : "dark")}>
        <SystemIcon name={dark() ? "sun.max" : "moon"} fallback={dark() ? "☀" : "☾"}
          size={palette().metric.titlebarIcon} color={palette().text} />
      </Button>
    </View>
  );
}
