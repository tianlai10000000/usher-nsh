/** Checks spring convergence, disposal and repeated overlay interactions with and without motion. */
import { expect, open, test } from "nsh/test";

for (const reducedMotion of [false, true]) {
  test(`Gallery motion survives reversal and disposal (reduced=${reducedMotion})`, async () => {
    const app = await open("./fixtures/ThemeMotion.tsx", { platform: "macos", width: 700, height: 950, reducedMotion });
    const measured = () => Number(app.texts().find(text => text.startsWith("measured:"))!.slice(9));
    try {
      await app.press("id:toggle-value");
      await app.press("id:toggle-value");
      await app.press("id:toggle-value");
      await app.wait(800);
      await app.press("id:read-spring");
      expect(Math.abs(measured() - 1) < 0.001).toBe(true);
      await app.press("id:toggle-value");
      await app.press("id:toggle-mount");
      await app.press("id:read-spring");
      const stopped = measured();
      await app.wait(300);
      await app.press("id:read-spring");
      expect(measured()).toBe(stopped);
      await app.press("id:toggle-mount");
      await app.press("id:read-spring");
      expect(measured()).toBe(0);
      await app.press("id:toggle-progress");
      expect(app.shows("id:motion-progress")).toBe(true);
    } finally { app.dispose(); }
  });

  test(`Gallery overlays reopen and remain interactive (reduced=${reducedMotion})`, async () => {
    const app = await open("./fixtures/ThemeMotion.tsx", { platform: "macos", width: 700, height: 950, reducedMotion });
    try {
      for (let i = 0; i < 3; i++) {
        await app.press("id:reopen-popup");
        await app.waitFor("id:popup-close");
        await app.press("id:popup-close");
      }
      await app.waitFor("id:popup-close", { state: "gone" });
      await app.press("id:select-trigger");
      await app.waitFor("text:Second item");
      await app.press("text:Second item");
      await app.waitFor("text:checked:false mounted:true selected:second");
      await app.press("id:select-trigger");
      await app.keyboard.press("Escape");
      await app.waitFor("text:First item", { state: "gone" });
      await app.hover("id:tip-trigger");
      await app.waitFor("id:motion-tooltip");
      await app.hover("id:toggle-value");
      await app.waitFor("id:motion-tooltip", { state: "gone" });
      await app.press("id:sheet-trigger");
      await app.waitFor("id:sheet-close");
      await app.press("id:sheet-close");
      await app.waitFor("id:sheet-close", { state: "gone" });
    } finally { app.dispose(); }
  });
}
