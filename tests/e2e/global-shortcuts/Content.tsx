/** Shows production process health while repeatedly mounting window and status-bar borrowers. */
import { effect, signal, Text, View } from "nsh";
import { Button } from "nsh/base";
import { currentWindow } from "nsh/desktop";
import { createGlobalShortcuts, createShortcuts, shortcutIndicator } from "../../../ui/shortcuts";
import { globalTransport } from "../../../ui/shortcuts/global-transport";
import { createAppState } from "../../../ui/state/app";
import { createRouter } from "../../../ui/router";
import { StatusBar } from "../../../ui/layout/components/StatusBar";
import { bindMainWindowShortcuts } from "../../../ui/shortcuts/modules/main-window";
import { applicationCommands } from "../../../ui/shortcuts/modules/application";
import { fixture } from "./fixture";
function Borrower() {
  const state = createAppState(); const router = createRouter(state.window.routing);
  const shortcuts = createShortcuts(state.window.scope, router);
  bindMainWindowShortcuts(router, state);
  const themeActive = signal(true);
  const host = currentWindow();
  effect(() => {
    state.appearance.mode();
    const active = fixture.active();
    themeActive.set(active);
    // Restore only after recording the native post-command state so the next test button can be pressed.
    if (!active) host.focus();
  });
  let releaseGuard: (() => void) | undefined;
  return <View>
    <Text>{`Path:${router.current().location.path}`}</Text><Text>{`Mode:${state.appearance.mode()}`}</Text>
    <Text>{`Effective:${state.appearance.effectiveScheme()}`}</Text>
    <Text>{`ThemeActive:${themeActive()}`}</Text>
    <Button testID="guard" onPress={() => { if (releaseGuard) { releaseGuard(); releaseGuard = undefined; } else releaseGuard = router.beforeEach(() => false); }}><Text>Toggle route guard</Text></Button>
    <Button testID="command-settings" onPress={() => applicationCommands.dispatch({ module: "window", action: "openSettings" })}><Text>Dispatch settings intent</Text></Button>
    <Button testID="command-theme" onPress={() => applicationCommands.dispatch({ module: "appearance", action: "toggleScheme" })}><Text>Dispatch appearance intent</Text></Button>
    <Button testID="system" onPress={() => state.appearance.choose("system")}><Text>System mode</Text></Button>
    <StatusBar appearance={state} shortcuts={shortcutIndicator(shortcuts, state.preferences.t)} />
  </View>;
}
export function Content() {
  const events = signal(0);
  const transport = globalTransport(fixture.attach());
  const trigger = transport.trigger;
  transport.trigger = ticket => { const command = trigger(ticket); if (command) events.update(value => value + 1); return command; };
  const global = createGlobalShortcuts(transport);
  const receipt = signal("none"); const active = signal(true);
  const input = (key: string) => { receipt.set(fixture.backgroundPress(key)); active.set(fixture.active()); };
  const mounted = signal(true); const available = signal("unknown");
  const host = currentWindow();
  effect(() => {
    const received = events();
    // The borrower is absent: restore test controls after input without consuming its queued business intent.
    if (!mounted() && received) host.focus();
  });
  return <View>
    <Text>{`Global:${global.value().global}`}</Text><Text>{`Connection:${global.connection()}`}</Text>
    <Text>{`Native available:${available()}`}</Text>
    <Text>{`Events:${events()}`}</Text><Text>{`Input:${receipt()}`}</Text><Text>{`Active:${active()}`}</Text>
    <Button testID="input-focus" onPress={() => input("focus")}><Text>Background system focus key</Text></Button>
    <Button testID="input-settings" onPress={() => input("settings")}><Text>Background system settings key</Text></Button>
    <Button testID="input-appearance" onPress={() => input("appearance")}><Text>Background system appearance key</Text></Button>
    <Button testID="read-active" onPress={() => active.set(fixture.active())}><Text>Read active state</Text></Button>
    <Button testID="probe" onPress={() => available.set(String(fixture.available()))}><Text>Probe OS ownership</Text></Button>
    <Button testID="release" onPress={() => fixture.release()}><Text>Release test blocker</Text></Button>
    <Button testID="retry" onPress={() => global.retry()}><Text>Retry global</Text></Button>
    <Button testID="remount" onPress={() => mounted.update(value => !value)}><Text>Remount borrower</Text></Button>
    <If when={mounted()}><Borrower /></If>
  </View>;
}
