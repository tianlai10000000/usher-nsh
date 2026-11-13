/** Real update bridge/controller/UI journeys; native I/O is controlled and never installs an app. */
import { expect, open, test } from "nsh/test";
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
    await app.press("id:rail-update");
    await app.waitFor("id:update-panel-close");
    await app.press("id:update-panel-close");
    await app.press("id:update-download");
    await app.waitFor("id:rail-update-progress");
    expect(app.enabled("id:rail-update")).toBe(false);
    expect(app.texts()).toContain("Downloads:1");
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
    await app.press("id:rail-update");
    await app.waitFor("id:update-panel-header >> id:update-panel-close");
    expect(app.shows("id:update-actions >> id:update-panel-close")).toBe(false);
    await app.press("id:update-panel-close");
    expect(app.shows("id:update-panel-header")).toBe(false);
    expect(app.texts()).toContain("Could not reach the update source. Try again.");
    await app.press("id:scenario-latest");
    await app.press("id:update-check");
    await app.waitFor("text:You’re up to date");
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
