/** Checks image loading, transparent content fills and overlay interaction on headless or native hosts. */
import { expect, open, test } from "nsh/test";
import config from "../../ui/config/background.json";

test("the background image remains below navigation and settings controls", async () => {
  const app = await open("./fixtures/BackgroundPreview.tsx", {
    platform: "macos", width: 1100, height: 950, locale: "en-US", colorScheme: "light",
  });
  await app.waitUntil(() => app.enabled("id:load-preview-image"));
  await app.press("id:load-preview-image");
  await app.waitFor("id:window-background-image");
  await app.drag("label:Image opacity", { x: 500 });
  await app.waitFor(`text:${config.maxOpacity}%`);
  await app.waitUntil(() => app.enabled("id:background-image-pick"));
  await app.drag("label:Content background transparency", { x: 500 });
  await app.waitFor(`id:content-background-transparency-control >> text:${config.contentTransparency.max}%`);
  expect(app.shows("id:route-settings")).toBe(true);
  await app.waitUntil(() => app.enabled("id:titlebar-color-mode"));
  await app.press("id:sidebar-toggle");
  await app.waitFor("id:application-sidebar", { state: "gone" });
  expect(app.shows("id:window-background-image")).toBe(true);
  await app.waitUntil(() => app.enabled("id:titlebar-color-mode"));
  await app.press("id:sidebar-toggle");
  await app.waitFor("id:application-sidebar");
  await app.waitUntil(() => app.enabled("id:titlebar-color-mode"));
  await app.press("id:titlebar-color-mode");
  expect(app.shows("id:window-background-image")).toBe(true);
  await app.waitFor(`id:content-background-transparency-control >> text:${config.contentTransparency.max}%`);
  await app.waitUntil(() => app.enabled("id:background-image-clear"));
  await app.press("id:background-image-clear");
  await app.waitFor("id:window-background-image", { state: "gone" });
  await app.waitFor("text:No image selected");
});
