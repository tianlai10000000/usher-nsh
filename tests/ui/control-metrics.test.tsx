/** Verifies theme metrics and neutral selected-button contrast through the application. */
import { expect, open, test } from "nsh/test";

// A theme's metrics must reach the screen: the same button is 28, 32 or 36 points tall
// depending on the selected design system, and the change needs no code.
test("controls take their size from the selected theme", async () => {
  const app = await open("./fixtures/Measure.tsx", { platform: "macos", width: 600, colorScheme: "light", locale: "en-US" });
  const height = (id: string) => Number(app.texts().find((text) => text.startsWith(`${id} `))?.split(" ")[1]);

  await app.waitUntil(() => height("button") > 0);
  expect(height("button")).toBe(28);

  for (const [pick, id, button, field] of [
    ["pick-primer", "builtin.primer", 32, 32],
    ["pick-slate", "builtin.slate", 36, 36],
    ["pick-gallery", "builtin.nsh-gallery", 28, 32],
  ] as const) {
    await app.waitUntil(() => app.enabled(`id:${pick}`));
    await app.press(`id:${pick}`);
    await app.waitFor(`text:theme ${id}`);
    await app.waitUntil(() => height("button") === button);
    expect(height("field")).toBe(field);
  }
});

// Selected neutral buttons retain the theme foreground on their neutral selected surface.
test("a selected theme button retains neutral text under every shipped theme", async () => {
  const app = await open("../../App.tsx", { platform: "macos", width: 1100, height: 720, colorScheme: "light", locale: "en-US" });
  await app.press("id:route-settings");
  await app.waitUntil(() => app.shows("id:settings-section-theme"));
  await app.press("id:settings-section-theme");

  for (const [id, label, ink] of [
    ["builtin.slate", "Slate · shadcn/ui", "#0f172a"],
    ["builtin.primer", "Primer · GitHub", "#1f2328"],
    ["builtin.nsh-gallery", "NSH Gallery", "#101319"],
  ] as const) {
    await app.waitUntil(() => app.enabled(`id:theme-${id}`));
    await app.press(`id:theme-${id}`);
    await app.waitUntil(() => app.spanStyle(`id:theme-${id} >> text:${label}`).color.startsWith(ink));
    expect(app.texts()).toContain(label);
  }
});
