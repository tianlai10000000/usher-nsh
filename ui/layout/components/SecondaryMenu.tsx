/**
 * The wide secondary menu: the list of pages beside the rail.
 *
 * It is a raised panel with its own surface, a little different from the content panel, so the
 * two read as separate areas. It docks next to the rail on wide desktop windows and appears as a
 * drawer elsewhere. Showing, hiding and the slide are handled by the frame (`../controllers/menuPresence.ts`);
 * this file only draws the panel and reports navigation.
 */
import { Text, View } from "nsh";
import { Button } from "nsh/base";
import { SystemIcon } from "../../components/SystemIcon";
import navigationIcons from "../../config/navigation-icons.json";
import { contentBackgroundColor, menuItemLook } from "../../theme/look";
import { routes, type Navigation, type RouteId } from "../../navigation";
import type { AppearanceStore } from "../../state/appearance";

export function SecondaryMenu(props: {
  appearance: AppearanceStore;
  navigation: Navigation;
  /** Draw the panel's rounded surface; the drawer already has its own background. */
  framed?: boolean;
  onNavigate?: () => void;
}) {
  const { palette, t } = props.appearance;
  const { route, navigate } = props.navigation;

  function item(id: RouteId) {
    const selected = () => route() === id;
    const look = () => menuItemLook(palette, selected());
    const icon = navigationIcons[id];
    return (
      <Button testID={`route-${id}`}
        selected={selected()}
        style={{ ...look().style, paddingHorizontal: palette().metric.sidebarItemPadX, gap: palette().metric.wrapGap }}
        hoverStyle={look().hoverStyle}
        activeStyle={look().activeStyle}
        focusStyle={look().focusStyle}
        onPress={() => {
          navigate(id);
          props.onNavigate?.();
        }}>
        <SystemIcon name={selected() ? icon.selectedSymbol : icon.symbol}
          fallback={icon.fallback} size={palette().metric.controlIcon + 2}
          color={palette().text} />
        <Text class="min-w-0 flex-1"
          style={{ ...look().label, numberOfLines: 1 }}>
          {t(routes[id].titleKey)}
        </Text>
      </Button>
    );
  }

  const framed = () => props.framed === true;
  const radius = () => palette().panelRadius;

  return (
    <View testID="application-sidebar" role="navigation" accessibilityLabel={t("nav.menu")}
      class="min-h-0 flex-1 gap-1 overflow-hidden"
      style={{
        padding: palette().metric.sidebarPadX,
        marginRight: framed() ? palette().metric.panelInset : 0,
        marginBottom: framed() ? palette().metric.panelInset : 0,
        borderRadius: framed() ? radius() : 0,
        backgroundColor: framed()
          ? contentBackgroundColor(palette().menuSurface, props.appearance.contentBackgroundTransparency())
          : undefined,
        boxShadow: framed() ? palette().shellShadow : undefined,
        borderWidth: framed() ? palette().metric.borderWidth : 0,
        borderColor: palette().shellBorder,
      }}>
      {item("home")}
      {item("settings")}
    </View>
  );
}
