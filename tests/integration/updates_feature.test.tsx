/** Real update bridge/controller/UI journeys; native I/O is controlled and never installs an app. */
import { expect, open, test } from "nsh/test";
import updateConfig from "../../ui/config/updates.json";
const entry = "./fixtures/UpdatesFeature.tsx";
const options = {
  platform: "macos",
  width: 1100,
  height: 850,
  locale: "en-US",
} as const;
async function settings(app: Awaited<ReturnType<typeof open>>) {
  await app.waitFor("text:Phase:idle");
  await app.press("id:settings-section-updates");
  await app.waitFor("id:settings-updates");
}

test("updates feature checks whole-app updates, renders progress and preserves ready state after closing the panel", async () => {
  const app = await open(entry, options);
  try {
    await settings(app);
    await app.press("id:update-check");
    await app.waitFor("id:update-download");
    await app.hover("id:rail-update");
    await app.waitFor("id:rail-update-tooltip");
    expect(app.texts()).toContain("Update available");
    await app.press("id:rail-update");
    await app.waitFor("id:update-panel-close");
    await app.press("id:update-panel-close");
    await app.press("id:update-download");
    await app.waitFor("id:rail-update-progress");
    await app.waitFor("id:update-download >> id:update-download-loading");
    expect(app.enabled("id:update-download")).toBe(false);
    await expect(app.press("id:update-download")).rejects.toMatchObject({ code: "ERR_NSH_TEST_DISABLED" });
    expect(app.enabled("id:rail-update")).toBe(false);
    expect(app.texts()).toContain("Downloads:1");
    await app.press("id:settings-section-general");
    await app.press("id:settings-section-updates");
    await app.waitFor("id:update-download >> id:update-download-loading");
    expect(app.enabled("id:update-download")).toBe(false);
    await app.press("id:finish-download");
    await app.waitFor("id:update-install");
    expect(app.enabled("id:rail-update")).toBe(true);
    await app.press("id:update-install");
    await app.waitFor(
      "text:Settings could not be saved. Retry before restarting.",
    );
    expect(app.texts()).toContain("Phase:ready_to_install");
  } finally {
    app.dispose();
  }
});
test("updates feature persists automatic preference and requires confirmation for a package when disabled", async () => {
  const app = await open(entry, options);
  try {
    await settings(app);
    await app.press("id:update-auto-check");
    await app.press("id:scenario-package");
    await app.press("id:update-check");
    await app.waitFor("id:update-download");
    expect(app.texts()).toContain("Downloads:0");
    await app.press("id:settings-section-general");
    await app.press("id:settings-section-updates");
    await app.press("id:update-download");
    await app.waitFor("id:update-install");
    expect(app.texts()).toContain("Downloads:1");
  } finally {
    app.dispose();
  }
});
test("updates feature reports manual installation, offline failures and successful retry", async () => {
  const app = await open(entry, options);
  try {
    await settings(app);
    await app.press("id:scenario-manual");
    await app.press("id:update-check");
    await app.waitFor("id:update-manual");
    expect(app.shows("id:update-download")).toBe(false);
    await app.press("id:scenario-offline");
    await app.press("id:update-check");
    await app.waitFor("text:Could not reach the update source. Try again.");
    await app.waitFor("id:rail-update-symbol");
    expect(app.enabled("id:rail-update")).toBe(true);
    await app.press("id:rail-update");
    await app.waitFor("id:update-panel-close");
    await app.press("id:update-panel-details >> id:update-check");
    await app.waitUntil(() =>
      app.texts().includes("Checks:3") && app.texts().includes("Phase:checked")
    );
    await app.waitFor("id:rail-update-symbol");
    expect(app.shows("id:update-panel-close")).toBe(true);
    expect(app.texts()).toContain(
      "Could not reach the update source. Try again.",
    );
    await app.press("id:update-panel-close");
    expect(app.shows("id:rail-update-symbol")).toBe(true);

    await app.press("id:scenario-latest");
    await app.press("id:update-check");
    await app.waitFor("text:You’re up to date");
    expect(app.shows("id:rail-update")).toBe(false);
  } finally {
    app.dispose();
  }
});
test("updates feature supports unknown progress and reduced motion", async () => {
  const app = await open(entry, { ...options, reducedMotion: true });
  try {
    await settings(app);
    await app.press("id:scenario-unknown");
    await app.press("id:update-check");
    await app.waitFor("id:update-download");
    await app.press("id:update-download");
    await app.waitFor("id:update-progress-ring");
    expect(app.enabled("id:rail-update")).toBe(false);
    await app.press("id:finish-download");
    await app.waitFor("id:update-install");
  } finally {
    app.dispose();
  }
});

test("updates feature keeps its controls usable in a narrow window and opens an anchored panel", async () => {
  const app = await open(entry, { ...options, width: 560 });
  try {
    await app.waitFor("text:Phase:idle");
    await app.waitFor("id:settings-section-picker");
    await app.press("id:settings-section-picker");
    await app.press("text:Updates");
    await app.waitFor("id:settings-updates");
    await app.press("id:update-check");
    await app.waitFor("id:update-download");
    await app.press("id:rail-update");
    await app.waitFor("id:update-panel-close");
    await app.press("id:update-panel-close");
    expect(app.shows("id:settings-updates")).toBe(true);
  } finally {
    app.dispose();
  }
});

test("updates feature shows a ring for automatic package downloads and keeps restart separate from the header close control when ready", async () => {
  const app = await open(entry, options);
  try {
    await settings(app);
    await app.press("id:scenario-package-slow");
    await app.press("id:update-check");
    await app.waitFor("id:update-progress-ring");
    expect(app.enabled("id:rail-update")).toBe(false);
    expect(app.shows("id:update-panel-close")).toBe(false);
    await app.press("id:finish-download");
    await app.waitFor("id:update-install");
    expect(app.shows("id:update-check")).toBe(false);
    expect(app.shows("id:update-restart-note")).toBe(true);
    await app.press("id:rail-update");
    await app.waitFor("id:update-panel-close");
    expect(app.shows("id:update-panel-header >> id:update-panel-close")).toBe(
      true,
    );
    expect(app.shows("id:update-actions >> id:update-panel-close")).toBe(false);
    expect(app.texts().includes("Later")).toBe(false);
    expect(app.shows("id:update-check")).toBe(false);
    await app.press("id:update-panel-close");
    expect(app.texts()).toContain("Phase:ready_to_install");
  } finally {
    app.dispose();
  }
});

test("updates feature hides an idle rail entry, shows a checking ring and hides it after an up-to-date result", async () => {
  const app = await open(entry, { ...options, reducedMotion: true });
  try {
    await settings(app);
    expect(app.shows("id:rail-update")).toBe(false);
    await app.press("id:scenario-checking");
    await app.press("id:update-check");
    await app.waitFor("id:update-checking-ring");
    await app.waitFor("id:update-check >> id:update-check-loading");
    expect(app.enabled("id:update-check")).toBe(false);
    expect(app.shows("id:update-check >> text:Check for updates")).toBe(true);
    await expect(app.press("id:update-check")).rejects.toMatchObject({
      code: "ERR_NSH_TEST_DISABLED",
    });
    expect(app.texts()).toContain("Checks:1");
    expect(app.enabled("id:rail-update")).toBe(false);
    expect(app.shows("id:rail-update-progress")).toBe(false);
    expect(app.shows("id:rail-update-symbol")).toBe(false);
    await app.hover("id:rail-update");
    await app.waitFor("id:rail-update-tooltip");
    expect(app.texts()).toContain("Checking for updates…");
    await app.press("id:finish-check");
    await app.waitFor("text:You’re up to date");
    await app.waitFor("id:rail-update", { state: "gone" });
    expect(app.enabled("id:update-check")).toBe(true);
    expect(app.shows("id:update-check-loading")).toBe(false);
  } finally {
    app.dispose();
  }
});

test("updates feature keeps background check failures quiet while preserving the reason in settings", async () => {
  const app = await open(entry, { ...options, clock: {} });
  try {
    await settings(app);
    await app.press("id:scenario-offline");
    await app.clock!.runFor(updateConfig.startupDelayMs);
    await app.waitFor("text:Could not reach the update source. Try again.");
    expect(app.shows("id:rail-update")).toBe(false);
    expect(app.shows("id:update-panel-close")).toBe(false);
    expect(app.enabled("id:update-check")).toBe(true);
    await app.press("id:update-check");
    await app.waitFor("id:rail-update-symbol");
    expect(app.enabled("id:rail-update")).toBe(true);
  } finally {
    app.dispose();
  }
});

test("updates feature shows busy feedback inside the popover action without changing rail interaction", async () => {
  const app = await open(entry, options);
  try {
    await settings(app);
    await app.press("id:scenario-offline");
    await app.press("id:update-check");
    await app.waitFor("id:rail-update-symbol");
    await app.press("id:scenario-checking");
    await app.press("id:rail-update");
    await app.waitFor("id:update-panel-close");
    expect(app.texts()).toContain("Checks:1");

    const action = "id:update-panel-details >> id:update-check";
    await app.press(action);
    await app.waitFor(`${action} >> id:update-check-loading`);
    expect(app.shows(`${action} >> text:Check for updates`)).toBe(true);
    expect(app.enabled(action)).toBe(false);
    await expect(app.press(action)).rejects.toMatchObject({
      code: "ERR_NSH_TEST_DISABLED",
    });
    expect(app.texts()).toContain("Checks:2");
    expect(app.shows("id:update-checking-ring")).toBe(true);
    expect(app.enabled("id:rail-update")).toBe(false);

    await app.press("id:update-panel-close");
    expect(app.shows("id:update-checking-ring")).toBe(true);
    expect(app.texts()).toContain("Checks:2");
    await app.press("id:finish-check");
    await app.waitFor("id:rail-update", { state: "gone" });
    expect(app.enabled("id:update-check")).toBe(true);
  } finally {
    app.dispose();
  }
});
