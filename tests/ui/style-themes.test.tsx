import { expect, open, test } from "nsh/test";

test("applies each shipped palette through the same theme chooser", async () => {
  const app = await open("../../App.tsx", { platform: "macos", width: 1000, colorScheme: "light", locale: "en-US" });
  await app.press("id:route-settings");
  await app.press("id:settings-section-theme");
  const settingsColor = () => app.spanStyle("id:breadcrumb-current-settings >> text:Settings").color;

  for (const [id, foreground] of [
    ["builtin.nsh-gallery", "#101319"],
    ["builtin.primer", "#1f2328"],
    ["builtin.geist", "#171717"],
    ["builtin.slate", "#0f172a"],
    ["builtin.radix-sand", "#21201c"],
  ]) {
    await app.waitUntil(() => app.enabled(`id:theme-${id}`));
    await app.press(`id:theme-${id}`);
    await app.waitUntil(() => settingsColor().startsWith(foreground));
    expect(app.shows("id:title-bar")).toBe(true);
  }

  await app.press("id:settings-section-general");
  await app.press("id:appearance-dark");
  await app.waitUntil(() => settingsColor().startsWith("#eeeeec"));
  await app.press("id:route-home");
  expect(app.spanStyle("id:breadcrumb-current-home >> text:Home").color).toMatch(/^#eeeeec/);

  // The same chooser lists the Gallery theme; its dark scheme comes from the same file.
  await app.press("id:route-settings");
  await app.press("id:settings-section-theme");
  await app.press("id:theme-builtin.nsh-gallery");
  await app.waitUntil(() => settingsColor().startsWith("#eeeff1"));
});
