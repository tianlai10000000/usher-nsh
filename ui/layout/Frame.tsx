/**
 * The frame layout: how the window is divided before any page is drawn.
 *
 *   title bar (full width): menu toggle, history, breadcrumbs ............ settings button
 *   rail | secondary menu (toggles) | content panel (takes the remaining width)
 *   status bar (full width, optional)
 *
 * The rail is the narrow fixed menu; the secondary menu docks beside it on wide desktop windows
 * and becomes a drawer elsewhere. Hiding the secondary menu animates the content panel to the
 * left. This file only assembles the regions: sizes come from theme tokens, state from
 * `controllers/menuPresence.ts` and `controllers/windowChrome.ts`, and pages are passed in as
 * children. WindowBackdrop owns the material and image layers beneath all interactive regions.
 */
import { platform, safeArea, View } from "nsh";
import { Sheet } from "nsh/base";
import type { JSX } from "nsh/jsx-runtime";
import type { Navigation } from "../router/index";
import type { AppState } from "../state/app";
import { contentBackgroundColor } from "../theme/look";
import type { AccountEntry } from "./contracts/account";
import { ContentPanel } from "./components/ContentPanel";
import { createMenuPresence } from "./controllers/menuPresence";
import { Rail } from "./components/Rail";
import { SecondaryMenu } from "./components/SecondaryMenu";
import { StatusBar } from "./components/StatusBar";
import { TitleBar } from "./components/TitleBar";
import type { UpdateIndicator } from "./contracts/updates";
import { createWindowChrome } from "./controllers/windowChrome";
import { WindowBackdrop } from "./components/WindowBackdrop";

export function Frame(props: {
  appearance: AppState;
  navigation: Navigation;
  /** Drives the rail's update button; the button is disabled without it. */
  updates?: UpdateIndicator;
  /** Drives the rail's avatar button; the button is disabled without it. */
  account?: AccountEntry;
  children: JSX.Element;
}) {
  const palette = props.appearance.appearance.palette;
  const showStatusBar = props.appearance.preferences.showStatusBar;
  const desktop = platform.desktop;
  const chrome = createWindowChrome(props.appearance);
  const menu = createMenuPresence(props.appearance);

  return (
    <View testID="window-frame" class="relative flex-1"
      style={{
        paddingTop: desktop ? 0 : safeArea().top,
        paddingLeft: safeArea().left,
        paddingRight: safeArea().right,
        paddingBottom: safeArea().bottom,
      }}>
      <WindowBackdrop appearance={props.appearance} />

      <TitleBar appearance={props.appearance} navigation={props.navigation} chrome={chrome} menu={menu} />

      {/* The rail reveals the backdrop; the secondary menu owns its themed surface. */}
      <View class="min-h-0 flex-1 flex-row">
        <If when={desktop}>
          <Rail appearance={props.appearance} navigation={props.navigation}
            updates={props.updates} account={props.account} />
        </If>
        <If when={desktop && menu.mounted()}>
          {/* Let the docked panel's shadow escape; clip the menu when its slot collapses. */}
          <View testID="menu-slot" class="min-h-0"
            style={{ width: menu.docked() ? palette().metric.menuWidth : 0,
              overflow: menu.docked() ? "visible" : "hidden" }}>
            <View class="min-h-0 flex-1" style={{ width: palette().metric.menuWidth }}>
              <SecondaryMenu appearance={props.appearance} navigation={props.navigation} framed />
            </View>
          </View>
        </If>
        <ContentPanel appearance={props.appearance}>{props.children}</ContentPanel>
      </View>

      {/* Outside the row above, so it is as wide as the window and not just the content. */}
      <If when={showStatusBar()}>
        <StatusBar appearance={props.appearance} />
      </If>

      <Sheet open={menu.drawerOpen()} side="left"
        popupStyle={{ height: "100%", width: palette().metric.menuWidth,
          backgroundColor: contentBackgroundColor(palette().background, props.appearance.background.contentBackgroundTransparency()),
          paddingTop: safeArea().top + palette().metric.panelInset, paddingBottom: safeArea().bottom }}
        backdropStyle={{ backgroundColor: palette().scrim }}
        onOpenChange={({ open }) => menu.drawerOpen.set(open)}>
        <View testID="sidebar-drawer" class="min-h-0 flex-1">
          <SecondaryMenu appearance={props.appearance} navigation={props.navigation}
            onNavigate={() => menu.drawerOpen.set(false)} />
        </View>
      </Sheet>
    </View>
  );
}
