/**
 * History arrows and breadcrumbs for the title bar.
 *
 * Both read the shared navigation state and send the same semantic navigation actions as the
 * menus. The arrows also bind the platform's back and forward shortcuts for the mounted window.
 */
import { keymap, platform, Text, View, windowSize } from "nsh";
import { Button } from "nsh/base";
import { SystemIcon } from "../../components/SystemIcon";
import type { Navigation } from "../../navigation";
import type { AppearanceStore } from "../../state/appearance";
import { focusRing, iconButtonLook, noteLook, spacing } from "../../theme/look";
import { ButtonTooltip } from "./ButtonTooltip";

type NavigationProps = { appearance: AppearanceStore; navigation: Navigation };

export function HistoryControls(props: NavigationProps) {
  const { palette, t } = props.appearance;
  const { canGoBack, canGoForward, goBack, goForward } = props.navigation;
  const mobile = platform.os === "ios" || platform.os === "android";
  // Raise the SF Symbol artwork by one point to align it with the sidebar icon.
  const arrowAlignment = platform.os === "macos" ? { paddingBottom: 2 } : undefined;

  if (!mobile) {
    // NSH removes these bindings with the mounted window.
    keymap.bind({ "mod+[": goBack, "mod+]": goForward }, { when: "notEditing" });
  }

  return (
    <View class="flex-row items-center" style={spacing(palette).text}>
      <Button testID="nav-back" accessibilityLabel={t("nav.back")} disabled={!canGoBack()}
        tooltip={<ButtonTooltip appearance={props.appearance} testID="nav-back-tooltip" placement="bottom"
          label={t("nav.back")} />}
        style={{ ...iconButtonLook(palette, false, true).style, ...arrowAlignment }}
        hoverStyle={iconButtonLook(palette, false, true).hoverStyle}
        activeStyle={iconButtonLook(palette, false, true).activeStyle}
        focusStyle={iconButtonLook(palette, false, true).focusStyle}
        disabledStyle={iconButtonLook(palette, false, true).disabledStyle} onPress={goBack}>
        <SystemIcon name="arrow.left" fallback="←" size={palette().metric.titlebarIcon} color={palette().text} />
      </Button>
      <Button testID="nav-forward" accessibilityLabel={t("nav.forward")} disabled={!canGoForward()}
        tooltip={<ButtonTooltip appearance={props.appearance} testID="nav-forward-tooltip" placement="bottom"
          label={t("nav.forward")} />}
        style={{ ...iconButtonLook(palette, false, true).style, ...arrowAlignment }}
        hoverStyle={iconButtonLook(palette, false, true).hoverStyle}
        activeStyle={iconButtonLook(palette, false, true).activeStyle}
        focusStyle={iconButtonLook(palette, false, true).focusStyle}
        disabledStyle={iconButtonLook(palette, false, true).disabledStyle} onPress={goForward}>
        <SystemIcon name="arrow.right" fallback="→" size={palette().metric.titlebarIcon} color={palette().text} />
      </Button>
    </View>
  );
}

export function Breadcrumbs(props: NavigationProps) {
  const { palette, t } = props.appearance;
  const { breadcrumbs, navigate } = props.navigation;
  const compact = () => !platform.desktop || windowSize().width < 460;
  const visibleTrail = () => compact() ? breadcrumbs().slice(-1) : breadcrumbs();

  // Muted ancestors, chevron separators and a plain current page, as in the reference.
  return (
    <View role="navigation" accessibilityLabel={t("nav.breadcrumbs")}
      class="min-w-0 flex-row items-center" style={spacing(palette).text}>
      <Each of={visibleTrail()} key={(crumb) => crumb.id}>
        {(crumb) => (
          <If when={crumb().current} else={
            <View class="min-w-0 flex-row items-center" style={spacing(palette).text}>
              <Button testID={`breadcrumb-${crumb().id}`} class="min-w-0"
                style={{ paddingHorizontal: palette().metric.textGap / 2, borderRadius: palette().metric.radiusItem / 2 }}
                hoverStyle={{ opacity: 0.7 }}
                focusStyle={focusRing(palette)}
                onPress={() => navigate(crumb().id)}>
                <Text style={{ ...noteLook(palette), fontSize: palette().metric.fontBody, numberOfLines: 1 }}>{t(crumb().titleKey)}</Text>
              </Button>
              <SystemIcon name="chevron.right" fallback="›" size={9} color={palette().muted} />
            </View>
          }>
            <Text testID={`breadcrumb-current-${crumb().id}`}
              accessibilityState={{ selected: true }} class="min-w-0"
              style={{ color: palette().text, fontSize: palette().metric.fontBody, numberOfLines: 1 }}>
              {t(crumb().titleKey)}
            </Text>
          </If>
        )}
      </Each>
    </View>
  );
}
