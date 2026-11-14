/** Runs every button journey against the native renderer, including real AppKit picker cancellation. */
import { expect, open, test } from "nsh/test";
import { registerButtonTests } from "../ui/support/button-journeys";
import config from "./native/config.json";

registerButtonTests({
  buttons: "../ui/fixtures/AsyncButtons.tsx",
  settings: "./native-buttons/SettingsActionsPending.tsx",
  updates: "../ui/fixtures/UpdateActionStates.tsx",
}, {
  captureUpdates: async (app, name) => { await app.screenshot(`update-check-${name}.png`); },
  captureSettings: async (app, name) => {
    const top = app.texts().find(text => text.startsWith("Page top:"))!.slice("Page top:".length);
    await app.screenshot(`settings-${top}-${name}.png`);
  },
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

import { registerInteractionTests } from "../ui/support/interaction-journeys";
registerInteractionTests("../ui/fixtures/InteractionControls.tsx", async (app, name) => {
  const height = app.texts().find(text => text.startsWith("Surface height:"))!.slice("Surface height:".length);
  await app.screenshot(`interaction-${height}-${name}.png`);
});

test("native AppKit locks preserve enabled appearance and restore rejected selections", async () => {
  const app = await open("./native-buttons/InteractionControls.tsx", { platform: "macos", width: 900, height: 800, locale: "en-US" });
  try {
    await app.press("id:native-inspect");
    const expected = "Native state:Contract button:true:-1,Contract popup:true:0,Contract segments:true:0";
    expect(app.texts()).toContain(expected);
    const height = app.texts().find(text => text.startsWith("Surface height:"))!.slice("Surface height:".length);
    await app.hover("id:native-neutral");
    await app.screenshot(`appkit-${height}-before.png`);
    await app.press("id:native-lock");
    await app.press("id:native-activate");
    expect(app.texts()).toContain(expected);
    expect(app.texts()).toContain("Calls:0");
    expect(app.texts()).toContain("Selection:first");
    await app.hover("id:native-neutral");
    await app.screenshot(`appkit-${height}-blocked.png`);
    await app.press("id:native-unlock");
    await app.hover("id:native-neutral");
    await app.screenshot(`appkit-${height}-restored.png`);
    await app.press("id:native-disable");
    await app.hover("id:native-neutral");
    await app.screenshot(`appkit-${height}-unavailable.png`);
    await app.press("id:native-inspect");
    expect(app.texts()).toContain("Native state:Contract button:false:-1,Contract popup:false:0,Contract segments:false:0");
    await app.press("id:native-activate");
    expect(app.texts()).toContain("Calls:0");
    await app.press("id:native-enable");
    await app.press("id:native-activate");
    await app.waitFor("text:Calls:3");
    expect(app.texts()).toContain("Selection:second");
    await app.press("id:native-clear-selection");
    await app.press("id:native-lock");
    await app.press("id:native-activate");
    expect(app.texts()).toContain("Native state:Contract button:true:-1,Contract popup:true:-1,Contract segments:true:-1");
    expect(app.texts()).toContain("Calls:3");
    expect(app.texts()).toContain("Selection:missing");
  } finally { app.dispose(); }
});

import { registerStatusbarJourney } from "../ui/support/statusbar-journey";
registerStatusbarJourney("../../ProductionApp.tsx");
