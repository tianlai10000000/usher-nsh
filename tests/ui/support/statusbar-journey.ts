/** Exercises real shell relayout and repeated status-bar commits with the Gallery animation theme. */
import { expect, open, test } from "nsh/test";
export function registerStatusbarJourney(entry: string) {
  test("status bar remains responsive across repeated Gallery toggles in the full application", async () => {
    const app = await open(entry, { platform: "macos", width: 1200, height: 1000, locale: "en-US" });
    try {
      await app.press("id:route-settings");
      await app.press("id:settings-section-theme");
      await app.waitUntil(() => app.enabled("id:theme-builtin.nsh-gallery"));
      await app.press("id:theme-builtin.nsh-gallery");
      await app.waitUntil(() => app.enabled("id:theme-builtin.nsh-gallery"));
      await app.press("id:settings-section-general");
      for (const shown of [false, true, false, true]) {
        await app.waitUntil(() => app.enabled("id:show-status-bar"));
        await app.press("id:show-status-bar");
        await app.waitFor("id:bottom-status-bar", { state: shown ? "shown" : "gone" });
        await app.waitUntil(() => app.enabled("id:show-status-bar"));
        expect(app.enabled("id:appearance-dark")).toBe(true);
      }
      await app.press("id:route-home");
      await app.waitFor("id:breadcrumb-current-home");
    } finally { app.dispose(); }
  });
}
