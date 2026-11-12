/** Exercises image selection and independent background preferences through Settings. */
import { expect, open, test } from "nsh/test";
import fixture from "../fixtures/background.json";
import config from "../../ui/config/background.json";

test("selects a user image and keeps background preferences across pages", async () => {
  const app = await open("../../App.tsx", { platform: "macos", width: 1100, height: 950, locale: "en-US" });
  const bytes = await (await fetch(fixture.dataUrl)).bytes();
  app.onPick(() => [{ name: "my-wallpaper.png", type: "image/png", data: bytes }]);
  await app.press("id:route-settings");
  await app.waitUntil(() => app.enabled("id:background-image-pick"));
  expect(app.enabled("id:background-image-clear")).toBe(false);
  await app.press("id:background-image-pick");
  await app.waitFor("text:my-wallpaper.png");
  await app.waitFor("id:window-background-image");
  expect(app.picks()[0].kind).toBe("images");
  expect(app.picks()[0].maxDimension).toBe(config.pickerMaxDimension);
  const contentValue = "id:content-background-transparency-control";
  await app.waitFor(`${contentValue} >> text:${config.contentTransparency.default}%`);
  await app.dragAndHold("label:Content background transparency", { x: 500 });
  await app.waitFor(`${contentValue} >> text:${config.contentTransparency.max}%`);
  expect(app.shows(`id:background-image-opacity-control >> text:${config.defaultOpacity}%`)).toBe(true);
  await app.drop();
  await app.waitUntil(() => app.enabled("id:background-image-pick"));
  await app.dragAndHold("label:Image opacity", { x: 500 });
  await app.waitFor(`id:background-image-opacity-control >> text:${config.maxOpacity}%`);
  await app.drop();
  await app.waitUntil(() => app.enabled("id:background-image-pick"));
  await app.press("id:route-home");
  await app.press("Click me");
  await app.waitFor("text:Clicks: 1");
  expect(app.shows("id:window-background-image")).toBe(true);
  await app.press("id:route-settings");
  await app.waitFor(`id:background-image-opacity-control >> text:${config.maxOpacity}%`);
  await app.waitFor(`${contentValue} >> text:${config.contentTransparency.max}%`);
  app.onPick(() => null);
  await app.press("id:background-image-pick");
  await app.waitUntil(() => app.enabled("id:background-image-pick"));
  expect(app.texts()).toContain("my-wallpaper.png");
  await app.press("id:background-image-clear");
  await app.waitFor("id:window-background-image", { state: "gone" });
  await app.waitFor("text:No image selected");
  expect(app.texts()).toContain(`${config.maxOpacity}%`);
  await app.drag("label:Content background transparency", { x: -500 });
  await app.waitFor(`${contentValue} >> text:${config.contentTransparency.min}%`);
  expect(app.shows(`id:background-image-opacity-control >> text:${config.maxOpacity}%`)).toBe(true);
});

test("transparent content works without an image and keeps the narrow menu usable", async () => {
  const app = await open("../../App.tsx", { platform: "macos", width: 1100, height: 950, locale: "en-US" });
  await app.press("id:route-settings");
  await app.waitUntil(() => app.enabled("id:content-background-transparency"));
  await app.drag("label:Content background transparency", { x: 100 });
  expect(app.shows(`id:content-background-transparency-control >> text:${config.contentTransparency.min}%`)).toBe(false);
  expect(app.shows(`id:content-background-transparency-control >> text:${config.contentTransparency.max}%`)).toBe(false);
  await app.waitUntil(() => app.enabled("id:background-image-pick"));
  await app.drag("label:Content background transparency", { x: 500 });
  await app.waitFor(`id:content-background-transparency-control >> text:${config.contentTransparency.max}%`);
  expect(app.shows("id:window-background-image")).toBe(false);
  await app.setViewport(560, 800);
  await app.press("id:sidebar-toggle");
  await app.waitFor("id:sidebar-drawer");
  await app.press("id:route-home");
  await app.waitFor("id:sidebar-drawer", { state: "gone" });
  await app.press("Click me");
  await app.waitFor("text:Clicks: 1");
  await app.press("id:rail-settings");
  await app.waitFor(`id:content-background-transparency-control >> text:${config.contentTransparency.max}%`);
});

test("an unreadable image falls back to the backdrop and remains removable", async () => {
  const app = await open("../../App.tsx", {
    platform: "macos", width: 1100, height: 950, locale: "en-US",
    modules: { "usher/nsh": { "Settings.loadBackgroundImage": () => { throw new Error("background_image_unavailable"); } } },
  });
  const bytes = await (await fetch(fixture.dataUrl)).bytes();
  app.onPick(() => [{ name: "missing.png", type: "image/png", data: bytes }]);
  await app.press("id:route-settings");
  await app.waitUntil(() => app.enabled("id:background-image-pick"));
  await app.press("id:background-image-pick");
  await app.waitFor("text:The background image could not be loaded. Choose it again or remove it.");
  expect(app.shows("id:window-background-image")).toBe(false);
  await app.press("id:background-image-clear");
  await app.waitFor("id:background-image-problem", { state: "gone" });
  await app.waitFor("text:No image selected");
});

test("invalid uploads preserve the current image and the picker remains usable", async () => {
  const app = await open("../../App.tsx", { platform: "macos", width: 1100, height: 950, locale: "en-US" });
  const bytes = await (await fetch(fixture.dataUrl)).bytes();
  app.onPick(() => [{ name: "valid.png", type: "image/png", data: bytes }]);
  await app.press("id:route-settings");
  await app.waitUntil(() => app.enabled("id:background-image-pick"));
  await app.press("id:background-image-pick");
  await app.waitFor("text:valid.png");
  app.onPick(() => [{ name: "invalid.png", type: "image/png", data: new TextEncoder().encode("invalid image") }]);
  await app.press("id:background-image-pick");
  await app.waitFor("text:Choose a valid raster image.");
  expect(app.texts()).toContain("valid.png");
  expect(app.shows("id:window-background-image")).toBe(true);
  app.onPick(() => [{ name: "replacement.png", type: "image/png", data: bytes }]);
  await app.press("id:background-image-pick");
  await app.waitFor("text:replacement.png");
  await app.press("id:background-image-clear");
  await app.waitFor("id:window-background-image", { state: "gone" });
});
