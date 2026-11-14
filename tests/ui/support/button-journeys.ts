/** Registers the same button lifecycle journeys for headless and native test adapters. */
import { expect, open, test, type App } from "nsh/test";

type PickerDriver = {
  prepare: (app: App) => void | Promise<void>;
  cancelled: (app: App) => void | Promise<void>;
  captureUpdates?: (app: App, name: string) => Promise<void>;
  captureSettings?: (app: App, name: string) => Promise<void>;
};
export function registerButtonTests(entries: { buttons: string; settings: string; updates: string }, picker: PickerDriver) {
  const entry = entries.buttons;
  const options = { platform: "macos", width: 900, height: 800, locale: "en-US" } as const;
  test("async buttons show internal loading and reject duplicate activation until success or failure", async () => {
    const app = await open(entry, options);
    try {
      const size = (id: string) => app.texts().find(text => text.startsWith(`Size:${id}:`));
      for (const [id, finish, outcome] of [
        ["async-themed", "finish-action", "done"],
        ["async-icon", "fail-action", "failed"],
      ]) {
        await app.waitUntil(() => !!size(id) && size(id) !== `Size:${id}:0,0`);
        const initialSize = size(id);
        if (id === "async-icon") expect(app.texts()).toContain("→");
        await app.press(`id:${id}`);
        await app.waitFor(`id:${id} >> id:${id}-loading`);
        expect(size(id)).toBe(initialSize);
        expect(app.enabled(`id:${id}`)).toBe(false);
        await expect(app.press(`id:${id}`)).rejects.toMatchObject({ code: "ERR_NSH_TEST_DISABLED" });
        expect(app.enabled("id:sync-action")).toBe(true);
        if (id === "async-icon") expect(app.texts().includes("→")).toBe(false);
        await app.press(`id:${finish}`);
        await app.waitFor(`text:Result:${outcome}`);
        await app.waitUntil(() => app.enabled(`id:${id}`));
        expect(app.shows(`id:${id}-loading`)).toBe(false);
        expect(size(id)).toBe(initialSize);
        if (id === "async-icon") expect(app.texts()).toContain("→");
      }
      expect(app.texts()).toContain("Calls:2");
      await app.press("id:sync-action");
      expect(app.shows("id:sync-action-loading")).toBe(false);
    } finally { app.dispose(); }
  });

  test("async buttons retain shared loading after remount and ignore a detached button completion", async () => {
    const app = await open(entry, { ...options, reducedMotion: true });
    try {
      await app.press("id:async-themed");
      await app.waitFor("id:async-themed-loading");
      await app.press("id:async-shared");
      await app.waitFor("id:async-shared-loading");
      await app.press("id:toggle-controls");
      await app.press("id:finish-action");
      await app.press("id:toggle-controls");
      await app.waitFor("id:async-shared >> id:async-shared-loading");
      expect(app.enabled("id:async-shared")).toBe(false);
      expect(app.enabled("id:async-themed")).toBe(true);
      await app.press("id:finish-shared");
      await app.waitUntil(() => app.enabled("id:async-shared"));
    } finally { app.dispose(); }
  });

  test("async button request guards clear after a rejected promise", async () => {
    const app = await open(entry, options);
    try {
      await app.press("id:reject-action");
      await app.waitFor("text:Reject busy:true");
      expect(app.enabled("id:reject-action")).toBe(false);
      await app.press("id:finish-action");
      await app.waitFor("text:Result:rejected");
      expect(app.enabled("id:reject-action")).toBe(true);
      expect(app.texts()).toContain("Reject busy:false");
    } finally { app.dispose(); }
  });

  test("immediate promise actions do not show loading or change control implementation", async () => {
    const app = await open(entry, options);
    try {
      await app.press("id:immediate-action");
      await app.waitFor("text:Result:pending");
      expect(app.shows("id:immediate-action-loading")).toBe(false);
      expect(app.enabled("id:immediate-action")).toBe(true);
      await app.press("id:immediate-action");
      expect(app.texts()).toContain("Calls:1");
      await app.press("id:finish-action");
      await app.waitFor("text:Result:done");
    } finally { app.dispose(); }
  });

  test("settings show loading for scan and export but not immediate appearance changes", async () => {
    const app = await open(entries.settings, { ...options, width: 1200, height: 1400 });
    try {
      await app.waitUntil(() => app.enabled("id:appearance-dark"));
      for (const [id, operation, other] of [
        ["show-status-bar", "statusbar", "appearance-light"],
        ["locale-en", "language", "appearance-light"],
        ["appearance-dark", "appearance", "appearance-light"],
        ["accent-reset", "accent", "accent-apply"],
        ["theme-builtin.primer", "theme", "theme-builtin.slate"],
        ["theme-directory-rescan", "scan", "theme-directory-open"],
        ["diagnostic-export", "export", ""],
      ]) {
        if (id === "accent-reset") await app.press("id:settings-section-theme");
        if (id === "diagnostic-export") await app.press("id:settings-section-diagnostics");
        await app.waitUntil(() => app.enabled(`id:${id}`));
        if (id === "show-status-bar") {
          await app.hover("id:settings-neutral");
          await picker.captureSettings?.(app, "before");
        }
        await app.press(`id:${id}`);
        await app.waitFor(`text:Pending:${operation}`);
        if (id === "show-status-bar") {
          await app.hover("id:settings-neutral");
          await picker.captureSettings?.(app, "blocked");
        }
        if (operation === "scan" || operation === "export") {
          await app.waitFor(`id:${id} >> id:${id}-loading`);
        } else {
          expect(app.shows(`id:${id}-loading`)).toBe(false);
        }
        expect(app.enabled(`id:${id}`)).toBe(false);
        if (other) expect(app.shows(`id:${other}-loading`)).toBe(false);
        await expect(app.press(`id:${id}`)).rejects.toMatchObject({ code: "ERR_NSH_TEST_DISABLED" });
        await app.press("id:release-settings");
        await app.waitFor("text:Pending:none");
        await app.waitUntil(() => app.enabled(`id:${id}`));
        expect(app.shows(`id:${id}-loading`)).toBe(false);
      }
      expect(app.texts()).toContain("Calls:7");
    } finally { app.dispose(); }
  });

  test("async background picker loading survives a section change and clears when picking is cancelled", async () => {
    const app = await open(entries.settings, { ...options, width: 1200, height: 1400 });
    try {
      const originalImage = app.texts().find(text => text.startsWith("Background:"));
      for (const attempt of [1, 2]) {
        await picker.prepare(app);
        await app.waitUntil(() => app.enabled("id:background-image-pick"));
        await app.press("id:background-image-pick");
        await app.waitFor("id:background-image-pick-loading");
        await app.press("id:settings-section-diagnostics");
        await app.press("id:settings-section-general");
        await app.waitFor("id:background-image-pick >> id:background-image-pick-loading");
        expect(app.enabled("id:background-image-pick")).toBe(false);
        await app.press("id:release-settings");
        await picker.cancelled(app);
        await app.waitFor("text:Pending:none");
        await app.waitFor("text:Picker:idle");
        await app.waitUntil(() => app.enabled("id:background-image-pick"));
        expect(app.shows("id:background-image-pick-loading")).toBe(false);
        expect(app.texts()).toContain(originalImage);
        expect(app.texts()).toContain("Picker error:none");
        expect(app.texts()).toContain(`Calls:${attempt}`);
      }
    } finally { app.dispose(); }
  });

  test("update download locks checking without starting unrelated loading and restores it after failure", async () => {
    const app = await open(entries.updates, options);
    try {
      await app.press("id:fail-install");
      await app.hover("id:update-neutral");
      await picker.captureUpdates?.(app, "before");
      await app.press("id:update-download");
      await app.waitFor("id:update-download-loading");
      expect(app.enabled("id:update-check")).toBe(false);
      expect(app.shows("id:update-check-loading")).toBe(false);
      await expect(app.press("id:update-check")).rejects.toMatchObject({ code: "ERR_NSH_TEST_DISABLED" });
      expect(app.texts()).toContain("Checks:0");
      await app.hover("id:update-neutral");
      await picker.captureUpdates?.(app, "downloading");
      await app.press("id:fail-install");
      expect(app.enabled("id:update-check")).toBe(true);
      await app.press("id:update-check");
      await app.waitFor("id:update-check-loading");
      expect(app.texts()).toContain("Checks:1");
    } finally { app.dispose(); }
  });

  test("async install buttons remain busy after command submission and recover from a failed installation", async () => {
    const app = await open(entries.updates, options);
    try {
      await app.press("id:update-install");
      await app.waitFor("id:update-install >> id:update-install-loading");
      expect(app.enabled("id:update-install")).toBe(false);
      await app.press("id:await-launch");
      expect(app.shows("id:update-install-loading")).toBe(true);
      expect(app.enabled("id:update-install")).toBe(false);
      await app.press("id:fail-install");
      await app.waitFor("id:update-download");
      expect(app.enabled("id:update-download")).toBe(true);
      expect(app.enabled("id:update-check")).toBe(true);
      expect(app.shows("id:update-install-loading")).toBe(false);
    } finally { app.dispose(); }
  });

}
