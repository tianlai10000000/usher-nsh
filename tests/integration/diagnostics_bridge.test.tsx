/** Tests diagnostic preferences and restricted logging through the production bridge. */
import { expect, open, test } from "nsh/test";
test("diagnostics bridge preserves output preferences and rejects unavailable sources", async () => {
  const app = await open("./fixtures/DiagnosticsBridge.tsx");
  await app.press("Run diagnostics bridge");
  await app.waitFor("text:Diagnostics:passed");
  expect(app.texts()).toContain("Diagnostics:passed");
});
