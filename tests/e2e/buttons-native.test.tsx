/** Runs every button journey against the native renderer, including real AppKit picker cancellation. */
import { expect } from "nsh/test";
import { registerButtonTests } from "../ui/support/button-journeys";
import config from "./native/config.json";

registerButtonTests({
  buttons: "../ui/fixtures/AsyncButtons.tsx",
  settings: "./native-buttons/SettingsActionsPending.tsx",
  updates: "../ui/fixtures/UpdateActionStates.tsx",
}, {
  prepare: async app => {
    await app.press("id:native-picker-arm");
    await app.waitFor("text:Native picker:armed");
  },
  cancelled: async app => {
    await app.waitUntil(() => !app.texts().includes("Native picker:armed"), { timeout: config.conditionTimeoutMs });
    expect(app.texts()).toContain("Native picker:closed");
    const calls = app.texts().find(text => text.startsWith("Calls:"))?.slice("Calls:".length);
    expect(app.texts()).toContain(`Native cancellations:${calls}`);
  },
});
