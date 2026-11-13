/** Exercises real settings/update services and production UI with a controlled native executor. */
import { mount, signal, Text, View } from "nsh";
import { Button } from "nsh/base";
import { Frame } from "../../../ui/layout/Frame";
import { createRouter, RouterProvider } from "../../../ui/router";
import { createAppearanceStore } from "../../../ui/state/appearance";
import { createWindowState } from "../../../ui/state/window";
import { SettingsProvider } from "../../../ui/state/app";
import { createUpdates } from "../../../ui/features/updates";
import { SettingsPage } from "../../../ui/features/settings/SettingsPage";
import type { Native } from "../../../ui/features/updates/contracts";

function Probe() {
  const window = createWindowState();
  const appearance = createAppearanceStore(window.session);
  const navigation = createRouter(window.routing);
  const scenario = signal("shell");
  const calls = signal(0);
  let finish: (() => void) | undefined;
  const updater: Native = {
    shellInfo: () => ({
      version: "1.0.0",
      build: 1,
      packages: true,
      nshVersion: "0.5.0",
    }),
    runningPackage: () => null,
    checkForPackage: async () => {
      if (scenario() === "offline") throw new Error("network offline");
      return scenario().startsWith("package")
        ? { version: "1.0.1", build: 2 }
        : null;
    },
    checkForUpdate: async () =>
      scenario() === "latest" ? null : ({
        version: scenario().startsWith("package") ? "1.0.1" : "1.1.0",
        canInstall: scenario() !== "manual",
        url: "https://example.test/update",
      }),
    downloadUpdate: async (progress) => {
      calls.update((n) => n + 1);
      progress?.({
        received: 25,
        total: scenario() === "unknown" ? null : 100,
      });
      await new Promise<void>((resolve) => {
        finish = resolve;
      });
      if (scenario() === "disk") throw new Error("disk write failed");
    },
    downloadPackage: async (progress) => {
      calls.update((n) => n + 1);
      progress?.({ received: 25, total: 100 });
      if (scenario() === "package-slow") {
        await new Promise<void>((resolve) => {
          finish = resolve;
        });
      }
      progress?.({ received: 100, total: 100 });
    },
    installAndRestart: async () => {
      throw new Error("disk permission denied");
    },
    restartToPackage: async () => {
      throw new Error("restart failed");
    },
  };
  const updates = createUpdates(appearance, updater);
  return (
    <SettingsProvider value={appearance.state}>
      <RouterProvider value={navigation}>
        <Frame
          appearance={appearance}
          navigation={navigation}
          updates={updates.indicator}
        >
          <View>
            <View class="flex-row flex-wrap">
              <Each
                of={[
                  "shell",
                  "package",
                  "package-slow",
                  "offline",
                  "manual",
                  "latest",
                  "unknown",
                  "disk",
                ]}
              >
                {(name) => (
                  <Button
                    testID={`scenario-${name()}`}
                    onPress={() => scenario.set(name())}
                  >
                    <Text>{name()}</Text>
                  </Button>
                )}
              </Each>
              <Button testID="finish-download" onPress={() => finish?.()}>
                <Text>Finish download</Text>
              </Button>
            </View>
            <Text>Downloads:{calls()}</Text>
            <Text testID="feature-phase">
              Phase:{updates.data().domain?.phase.type ?? "preparing"}
            </Text>
            <SettingsPage appearance={appearance} updates={updates} />
          </View>
        </Frame>
      </RouterProvider>
    </SettingsProvider>
  );
}
mount(Probe);
