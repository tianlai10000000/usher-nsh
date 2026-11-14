/** Drives the production host binding with delayed mock synchronization and host events. */
import { mount, signal, Text, View } from "nsh";
import { Button } from "nsh/base";
import { bindAppearanceHost } from "../../appearance/host";
import type { SyncStatus } from "../../../state-runtime/contracts";
import type { ColorMode } from "../../settings/contracts";

function Preview() {
  const status = signal<SyncStatus>("connecting");
  const mode = signal<ColorMode>("system");
  const scheme = signal<"light" | "dark">("light");
  const calls = signal("");
  const log: ColorMode[] = [];
  const attached = signal(true);
  function Binding() {
    const effective = bindAppearanceHost(mode, status, {
      effectiveScheme: scheme,
      applyMode(value) { log.push(value); calls.set(log.join(",")); },
    });
    return <Text>{`Effective:${effective()}`}</Text>;
  }
  return <View>
    <Text>{`Calls:${calls() || "none"}`}</Text>
    <If when={attached()}><Binding /></If>
    <Button testID="dark" onPress={() => mode.set("dark")}><Text>Dark</Text></Button>
    <Button testID="light" onPress={() => mode.set("light")}><Text>Light</Text></Button>
    <Button testID="system" onPress={() => mode.set("system")}><Text>System</Text></Button>
    <Button testID="live" onPress={() => status.set("live")}><Text>Loaded</Text></Button>
    <Button testID="reconnect" onPress={() => status.set("connecting")}><Text>Reconnect</Text></Button>
    <Button testID="error" onPress={() => status.set("error")}><Text>Error</Text></Button>
    <Button testID="host-dark" onPress={() => scheme.set("dark")}><Text>Host dark</Text></Button>
    <Button testID="detach" onPress={() => attached.set(false)}><Text>Detach</Text></Button>
  </View>;
}
mount(Preview);
