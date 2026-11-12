// Made by `nsh test BackgroundPreview.tsx`: the steps of standard input.
import { expect, open, test } from "nsh/test";

test("steps", async () => {
const app = await open("/Users/yuankai/yuankai/Demo/rust/usher-nsh/tests/ui/fixtures/BackgroundPreview.tsx", { network: "live" });
await app.wait(1200);
await app.setViewport(1100, 950);
await app.waitUntil(() => app.enabled("id:locale-en"));
await app.press("id:locale-en");
await app.waitUntil(() => app.enabled("id:load-preview-image"));
await app.press("id:load-preview-image");
await app.waitFor("id:window-background-image");
await app.wait(400);
await app.screenshot("/private/tmp/usher-background-image-30.png");
await app.press("label:Image opacity");
await app.keyboard.press("End");
await app.waitFor("text:100%");
await app.wait(400);
await app.screenshot("/private/tmp/usher-background-image-100.png");
await app.press("id:sidebar-toggle");
await app.waitFor("id:application-sidebar", { state: "gone" });
await app.press("id:sidebar-toggle");
await app.waitFor("id:application-sidebar");
await app.press("id:background-image-clear");
await app.waitFor("id:window-background-image", { state: "gone" });

});
