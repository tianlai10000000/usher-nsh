/** Verifies real macOS spring frames and overlay rendering in the isolated theme host. */
import { expect, open, test } from "nsh/test";

for (const reducedMotion of [false, true]) {
  test(`native Gallery motion frames (reduced=${reducedMotion})`, async () => {
    const app = await open("../ui/fixtures/ThemeMotion.tsx", { platform: "macos", width: 800, height: 950, reducedMotion });
    try {
      await app.press("id:sample-spring");
      await app.waitUntil(() => app.texts().some(text => text.startsWith("samples:") && text.includes("800:")));
      const trace = app.texts().find(text => text.startsWith("samples:"))!.slice(8);
      const values = trace.split(";").filter(Boolean).map(point => Number(point.split(":")[1]));
      expect(values.length).toBe(4);
      expect(Math.abs(values[3] - 1) < 0.001).toBe(true);
      if (reducedMotion) expect(values.every(value => value === 1)).toBe(true);
      else expect(values.some(value => value > 0 && value < 0.99)).toBe(true);
      await app.screenshot(reducedMotion ? "reduced.png" : "animated.png");
      await app.press("id:toggle-progress");
      await app.wait(200);
      await app.screenshot(reducedMotion ? "progress-reduced.png" : "progress-animated.png");
      await app.drag("id:motion-slider", { x: 0 });
      await app.hover("id:motion-slider");
      await app.wait(200);
      await app.screenshot(reducedMotion ? "hover-reduced.png" : "hover-animated.png");
      await app.press("id:popup-trigger");
      await app.waitFor("id:popup-close");
      await app.wait(200);
      await app.screenshot(reducedMotion ? "popup-reduced.png" : "popup-animated.png");
      await app.press("id:popup-close");
      await app.press("id:reopen-popup");
      await app.waitFor("id:popup-close");
      await app.press("id:popup-close");
      await app.waitFor("id:popup-close", { state: "gone" });
      await app.press("id:select-trigger");
      await app.waitFor("text:Second item");
      await app.wait(200);
      await app.screenshot(reducedMotion ? "select-reduced.png" : "select-animated.png");
      await app.press("text:Second item");
      await app.press("id:sheet-trigger");
      await app.waitFor("id:sheet-close");
      await app.wait(500);
      await app.screenshot(reducedMotion ? "sheet-reduced.png" : "sheet-animated.png");
      await app.press("id:sheet-close");
      await app.waitFor("id:sheet-close", { state: "gone" });
    } finally { app.dispose(); }
  });
}
