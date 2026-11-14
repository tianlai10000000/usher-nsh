/** Toggles only the application's color preference through the existing appearance command. */
import type { AppState } from "../../state/app";
export async function toggleScheme(appearance: Pick<AppState["appearance"], "mode" | "effectiveScheme" | "choose">) {
  const saved = appearance.mode();
  const effective = saved === "system" ? appearance.effectiveScheme() : saved;
  await appearance.choose(effective === "dark" ? "light" : "dark");
}
