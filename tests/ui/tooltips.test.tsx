/** Verifies localized framework tooltips on title-bar and rail icons across input and state changes. */
import { expect, open, test } from "nsh/test";

type App = Awaited<ReturnType<typeof open>>;

async function showTooltip(app: App, buttonID: string, label: string) {
  await app.hover(`id:${buttonID}`);
  await app.waitFor(`id:${buttonID}-tooltip`);
  expect(app.texts()).toContain(label);
}

test("every title-bar and rail icon has a hover tooltip, including disabled buttons", async () => {
  const app = await open("../../App.tsx", {
    platform: "macos", width: 900, height: 740, locale: "en-US", colorScheme: "light", clock: {},
  });
  await app.waitUntil(() => app.enabled("id:titlebar-color-mode"));
  expect(app.enabled("id:nav-back")).toBe(false);
  expect(app.enabled("id:nav-forward")).toBe(false);
  expect(app.shows("id:rail-update")).toBe(false);
  expect(app.enabled("id:rail-account")).toBe(false);

  const hints = [
    ["sidebar-toggle", "Hide sidebar"],
    ["nav-back", "Back"],
    ["nav-forward", "Forward"],
    ["titlebar-color-mode", "Switch to light mode"],
    ["rail-home", "Home"],
    ["rail-settings", "Open settings"],
    ["rail-account", "Account"],
  ] as const;

  for (const [buttonID, label] of hints) {
    await showTooltip(app, buttonID, label);
    // Breadcrumbs have no hint and provide a neutral target outside the icon buttons.
    await app.hover("id:breadcrumb-current-home");
    await app.waitFor(`id:${buttonID}-tooltip`, { state: "gone" });
  }

  await showTooltip(app, "rail-settings", "Open settings");
  await app.keyboard.press("escape");
  await app.waitFor("id:rail-settings-tooltip", { state: "gone" });
});

test("tooltips follow the sidebar action, appearance action and selected language", async () => {
  const app = await open("../../App.tsx", {
    platform: "macos", width: 900, height: 740, locale: "en-US", colorScheme: "light", clock: {},
  });
  await app.waitUntil(() => app.enabled("id:titlebar-color-mode"));
  await showTooltip(app, "sidebar-toggle", "Hide sidebar");
  await app.press("id:sidebar-toggle");
  await app.waitFor("id:sidebar-toggle-tooltip", { state: "gone" });
  await app.waitFor("id:application-sidebar", { state: "gone" });
  await app.hover("id:breadcrumb-current-home");
  await showTooltip(app, "sidebar-toggle", "Show sidebar");

  await showTooltip(app, "titlebar-color-mode", "Switch to light mode");
  await app.press("id:titlebar-color-mode");
  await app.waitUntil(() => app.enabled("label:Switch to dark mode"));
  await app.hover("id:breadcrumb-current-home");
  await showTooltip(app, "titlebar-color-mode", "Switch to dark mode");
  await app.press("id:titlebar-color-mode");
  await app.waitFor("id:titlebar-color-mode-tooltip", { state: "gone" });
  await app.waitFor("text:Current appearance: Dark");
  await app.waitUntil(() => app.enabled("id:titlebar-color-mode"));
  await app.hover("id:breadcrumb-current-home");
  await showTooltip(app, "titlebar-color-mode", "Switch to system appearance");

  await app.press("id:rail-settings");
  await app.waitUntil(() => app.enabled("id:locale-zh-Hans"));
  await app.scroll("id:main-content", 500);
  await app.press("id:locale-zh-Hans");
  await app.waitFor("text:设置");
  const translatedHints = [
    ["sidebar-toggle", "显示侧边栏"],
    ["nav-back", "后退"],
    ["nav-forward", "前进"],
    ["titlebar-color-mode", "切换到跟随系统外观"],
    ["rail-home", "主页"],
    ["rail-settings", "打开设置"],
    ["rail-account", "账户"],
  ] as const;
  for (const [buttonID, label] of translatedHints) {
    await showTooltip(app, buttonID, label);
  }

  await app.press("id:locale-en");
  await app.waitFor("text:Settings");
  await showTooltip(app, "rail-home", "Home");
  await showTooltip(app, "rail-settings", "Open settings");
});

test("keyboard focus shows a title-bar tooltip and Escape dismisses it", async () => {
  const app = await open("../../App.tsx", {
    platform: "macos", width: 900, height: 740, locale: "en-US", clock: {},
  });
  await app.waitUntil(() => app.enabled("id:sidebar-toggle"));
  await app.keyboard.press("tab");
  await app.waitFor("id:sidebar-toggle-tooltip");
  expect(app.texts()).toContain("Hide sidebar");
  await app.keyboard.press("escape");
  await app.waitFor("id:sidebar-toggle-tooltip", { state: "gone" });
  await app.keyboard.press("tab");
  await app.waitFor("id:titlebar-color-mode-tooltip");
  expect(app.shows("id:sidebar-toggle-tooltip")).toBe(false);
});
