/** Guards the Gallery part mapping, switch travel and custom-accent rendering regressions. */
import { expect, open, test } from "nsh/test";

for (const scheme of ["light", "dark"] as const) {
  test(`Gallery finishes preserve component roles in ${scheme}`, async () => {
    const app = await open("./fixtures/ThemeFinishes.tsx", { platform: "macos", width: 700, height: 650, colorScheme: scheme, reducedMotion: true });
    const read = () => JSON.parse(app.texts().find(text => text.startsWith("contract:"))!.slice("contract:".length));
    try {
      await app.waitUntil(() => app.texts().some(text => text.startsWith("contract:")));
      let styles = read();
      expect(styles.switch.thumbStyle.backgroundColor).toBe("#ffffff");
      expect(styles.switch.thumbStyle.borderWidth).toBe(1);
      expect(styles.switch.thumbStyle.transform[0].translateX).toBe(12);
      expect(styles.switch.trackStyle.borderWidth).toBe(1);
      expect(styles.switch.trackStyle.padding).toBe(1);
      expect(styles.switch.trackStyle.boxShadow).toContain("inset");
      expect(styles.slider.indicatorStyle.height).toBe(6);
      expect(styles.slider.rangeStyle.backgroundImage).toContain("linear-gradient");
      expect(styles.progress.indicatorStyle.backgroundImage).toBe(styles.slider.rangeStyle.backgroundImage);
      expect(styles.selected.style.backgroundColor).toBe(scheme === "light" ? "#dbdce0" : "#4b4f57");
      expect(styles.selected.label.color).toBe(scheme === "light" ? "#101319" : "#eeeff1");
      expect(styles.field.style.backgroundColor).toBe(scheme === "light" ? "#ffffff" : "#16191f");
      expect(styles.tab.indicatorStyle.borderColor).toBe(scheme === "light" ? "#c1c2c4" : "rgba(0,0,0,0.5)");
      expect(styles.popup.boxShadow).toContain(scheme === "light" ? "28px" : "32px");
      await app.press("id:sample-switch");
      styles = read();
      expect(styles.switch.thumbStyle.transform[0].translateX).toBe(0);
      expect(styles.switch.trackStyle.backgroundColor).toBe(scheme === "light" ? "#e9eaed" : "#33363c");
      await app.press("id:override-accent");
      styles = read();
      expect(styles.primary.style.backgroundColor).toBe("#b42318");
      expect(styles.primary.style.backgroundImage).toBeUndefined();
      expect(styles.slider.rangeStyle.backgroundColor).toBe("#b42318");
      expect(styles.slider.rangeStyle.backgroundImage).toBeUndefined();
      expect(styles.progress.indicatorStyle.backgroundImage).toBeUndefined();
    } finally { app.dispose(); }
  });
}
