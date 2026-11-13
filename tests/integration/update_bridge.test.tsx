/** Verifies the real Rust update bridge in isolated engines, without enabling product updates. */
import { expect, open, test } from "nsh/test";

test("update bridge configures once, shares state and validates native claims", async () => {
  const app = await open("./fixtures/UpdateBridge.tsx");
  try {
    await app.waitFor("text:Observed:preparing");
    await app.press("Run update bridge");
    await app.waitFor("text:Update:passed");
    expect(app.texts()).toContain("Update:passed");
  } finally { app.dispose(); }
  const next = await open("./fixtures/UpdateBridge.tsx");
  try { await next.waitFor("text:Observed:preparing"); }
  finally { next.dispose(); }
});
