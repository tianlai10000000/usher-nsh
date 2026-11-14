/** Real bridge/keymap fixture with one lazy page command, remount controls and a borrowed status bar. */
import { initializeFixtureGlobals } from "./global";
import { cleanup, mount, platform, signal, Text, View } from "nsh";
import { Button, Input } from "nsh/base";
import * as native from "usher/nsh";
import { createAppState } from "../../../ui/state/app";
import { createRouter } from "../../../ui/router";
import { RouterView, type PageRegistry } from "../../../ui/router/RouterView";
import { createShortcuts, shortcutIndicator } from "../../../ui/shortcuts";
import { shortcutTransport } from "../../../ui/shortcuts/transport";
import { StatusBar } from "../../../ui/layout/components/StatusBar";
const fixture = (native as typeof native & { ShortcutFixture: { attach(window: string, os: string): unknown; loads(): number } }).ShortcutFixture;
const pages: PageRegistry = { home: () => <Text>Mounted:home</Text>, settings: () => <Text>Mounted:settings</Text>, "not-found": () => <Text>Mounted:not-found</Text> };
function Content() {
  const state = createAppState(); const router = createRouter(state.window.routing);
  const shortcuts = createShortcuts(state.window.scope, router, shortcutTransport(state.window.scope, platform.os, fixture.attach(state.window.scope, platform.os)));
  const status = shortcutIndicator(shortcuts, state.preferences.t);
  const visible = signal(true); const loads = signal(fixture.loads());
  const blocked = signal(false); let unblock: (() => void) | undefined;
  cleanup(() => unblock?.());
  return <View>
    <Text>{`Store:${state.window.scope}`}</Text>
    <Text>{`Path:${router.current().location.path}`}</Text>
    <Text>{`Startup:${shortcuts.value().startup}`}</Text><Text>{`Page:${shortcuts.value().page}`}</Text>
    <Text>{`Connection:${shortcuts.connection()}`}</Text>
    <Text>{`Loads:${loads()}`}</Text>
    <Button testID="read-loads" onPress={() => loads.set(fixture.loads())}><Text>Read loads</Text></Button>
    <Button testID="go-settings" onPress={() => router.navigate("settings")}><Text>Settings</Text></Button>
    <Button testID="go-home" onPress={() => router.navigate("home")}><Text>Home</Text></Button>
    <Button testID="toggle-status" onPress={() => visible.update(v => !v)}><Text>Toggle status</Text></Button>
    <Button testID="block" onPress={() => { if (blocked()) { unblock?.(); unblock = undefined; } else unblock = shortcuts.block(); blocked.set(!blocked()); }}><Text>{blocked() ? "Unblock" : "Block"}</Text></Button>
    <Input testID="editor" placeholder="Type here" />
    <RouterView pages={pages} appearance={state} session={state.window.session} navigation={router} shortcuts={shortcuts} />
    <If when={visible()}><StatusBar appearance={state} shortcuts={status} /></If>
  </View>;
}
function Harness() {
  initializeFixtureGlobals();
  const mounted = signal(true);
  return <View><Button testID="remount" onPress={() => mounted.update(v => !v)}><Text>Remount</Text></Button><If when={mounted()}><Content /></If></View>;
}
mount(Harness);
