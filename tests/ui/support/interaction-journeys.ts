/** Shares public interaction-contract assertions between headless and macOS renderer acceptance. */
import { expect, open, test, type App } from "nsh/test";

type Capture = (app: App, name: string) => Promise<void>;
export function registerInteractionTests(entry: string, capture?: Capture) {
  test("shared controls retain reactive values and accessibility labels across repeated input", async () => {
    const app = await open(entry, { platform: "macos", width: 900, height: 800 });
    try {
      await app.press("id:commit-values");
      await app.press("id:protected-switch");
      expect(app.texts()).toContain("Checked:false");
      await app.press("id:protected-switch");
      expect(app.texts()).toContain("Checked:true");
      for (let stop = 0; stop < 4; stop++) await app.keyboard.press("tab");
      await app.keyboard.press("right");
      expect(app.texts()).toContain("Value:51");
      await app.keyboard.press("right");
      expect(app.texts()).toContain("Value:52");
      await app.press("id:protected-select");
      await app.press("text:Second");
      expect(app.texts()).toContain("Choice:second");
      await app.press("id:protected-select");
      await app.press("text:First");
      expect(app.texts()).toContain("Choice:first");
      await app.press("id:change-icon-label");
      expect(app.shows("label:Changed icon")).toBe(true);
      expect(app.shows("label:Protected icon")).toBe(false);
    } finally { app.dispose(); }
  });
  test("shared slider completes an accepted gesture when its change acquires a lock", async () => {
    const app = await open(entry, { platform: "macos", width: 900, height: 800 });
    try {
      await app.press("id:arm-slider-lock");
      for (let stop = 0; stop < 4; stop++) await app.keyboard.press("tab");
      await app.keyboard.press("right");
      expect(app.texts()).toContain("Blocked:true");
      expect(app.texts()).toContain("Ends:1");
    } finally { app.dispose(); }
  });
  test("shared interaction locks preserve appearance and reject pointer and keyboard changes", async () => {
    const app = await open(entry, { platform: "macos", width: 900, height: 800, locale: "en-US", reducedMotion: true });
    try {
      await app.waitUntil(() => app.texts().some(text => text.startsWith("Surface height:") && text !== "Surface height:0"));
      await app.hover("id:interaction-neutral");
      await capture?.(app, "before");
      await app.keyboard.press("tab");
      await app.keyboard.press("enter");
      expect(app.texts()).toContain("Calls:1");
      await app.press("id:lock-controls");
      for (const id of ["protected-button", "protected-icon", "protected-switch", "protected-slider", "protected-select", "unavailable-button"]) {
        expect(app.enabled(`id:${id}`)).toBe(false);
        await expect(app.press(`id:${id}`)).rejects.toMatchObject({ code: "ERR_NSH_TEST_DISABLED" });
      }
      await app.keyboard.press("space");
      await app.keyboard.press("enter");
      expect(app.texts()).toContain("Calls:1");
      await app.hover("id:interaction-neutral");
      await capture?.(app, "blocked");
      expect(app.enabled("id:independent-button")).toBe(true);
      await app.press("id:independent-button");
      expect(app.texts()).toContain("Calls:2");
      await app.press("id:unlock-controls");
      await app.press("id:protected-switch");
      expect(app.texts()).toContain("Calls:3");
      await app.hover("id:interaction-neutral");
      await capture?.(app, "restored");
      await app.press("id:disable-controls");
      await app.hover("id:interaction-neutral");
      await capture?.(app, "unavailable");
      expect(app.enabled("id:protected-button")).toBe(false);
      expect(app.enabled("id:unavailable-button")).toBe(false);
    } finally { app.dispose(); }
  });
}
