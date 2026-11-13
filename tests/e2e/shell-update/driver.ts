/** Drives production update actions only in isolated packaged sources; never mocks native I/O. */
import { Settings } from "usher/nsh";
import { runningPackage, shellInfo } from "nsh/updater";
import { onDispose } from "../../../ui/state/runtime/nsh";
import type { Updates } from "../../../ui/features/updates";
import manifest from "../../../nsh.json";
import native from "../native/config.json";
import config from "./config.json";

export function driveShellAcceptance(updates: Updates) {
  const source = (manifest.updater as { url?: string }).url;
  if (
    !manifest.id.startsWith(native.appId + ".") ||
    !source?.startsWith("http://127.0.0.1:")
  ) {
    throw new Error(
      "shell acceptance requires a unique isolated identity and loopback source",
    );
  }
  const endpoint = new URL(config.endpoint, source).href;
  let disposed = false;
  onDispose(() => {
    disposed = true;
  });
  const sleep = () =>
    new Promise<void>((resolve) => setTimeout(resolve, config.pollIntervalMs));
  const send = async (value: object) => {
    const response = await fetch(endpoint, {
      method: "POST",
      headers: { "Content-Type": "application/json" },
      body: JSON.stringify(value),
    });
    if (!response.ok) {
      throw new Error(`acceptance transport: ${response.status}`);
    }
    return response.json() as Promise<{ session: number }>;
  };
  async function snapshot() {
    const settings = await Settings.snapshot();
    return {
      shell: shellInfo(),
      package: runningPackage(),
      update: updates.data(),
      error: updates.error(),
      settings: {
        saved: settings.saved,
        colorMode: settings.colorMode,
        localePreference: settings.localePreference,
      },
    };
  }
  void (async () => {
    while (!disposed && !updates.data().domain) await sleep();
    if (disposed) return;
    const { session } = await send({ kind: "boot", value: await snapshot() });
    let previous = "";
    while (!disposed) {
      const value = await snapshot();
      const encoded = JSON.stringify(value);
      if (encoded !== previous) {
        await send({ kind: "state", session, value });
        previous = encoded;
      }
      const response = await fetch(`${endpoint}?session=${session}`);
      if (!response.ok) {
        throw new Error(`acceptance control: ${response.status}`);
      }
      const command = await response.json() as {
        action?: "check" | "download" | "install";
      };
      if (command.action) {
        await send({ kind: "action", session, action: command.action });
        await updates[command.action]();
      }
      await sleep();
    }
  })().catch((error) =>
    console.error(`SHELL_ACCEPTANCE_FAILED: ${String(error)}`)
  );
}
