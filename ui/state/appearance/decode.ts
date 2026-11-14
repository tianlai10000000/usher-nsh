/** Decodes application-owned theme snapshots before they reach selectors or host effects. */
import * as d from "../../state-runtime/decode";
import bundled from "./bundled.generated.json";
import type { CustomTheme, StyleTheme } from "./contracts";
export const decodeMode = (value: unknown) => d.oneOf(value, ["system", "light", "dark"]);
export const decodeFamily = (value: unknown) => d.oneOf(value, ["nsh", "appkit"]);
const root = bundled.themes.find(theme => theme.id === bundled.defaultThemeId)!;
function tokens(value: unknown, required: readonly string[]) {
  const table = d.object(value);
  for (const key of required) d.string(table[key]);
  for (const item of Object.values(table)) d.requireValue(!d.string(item).includes("var(--"));
}
export function decodeStyleTheme(value: unknown): StyleTheme {
  const theme = d.object(value);
  d.string(theme.id); d.string(theme.name); d.oneOf(theme.material, ["solid", "glass"]);
  for (const name of Object.values(d.object(theme.names))) d.string(name);
  const css = d.object(theme.cssVars);
  // The legacy touch table is authored data; only effective tables drive rendering.
  for (const value of Object.values(d.object(css.touch))) d.string(value);
  tokens(css.touchLight, Object.keys(root.cssVars.light));
  tokens(css.touchDark, Object.keys(root.cssVars.dark));
  tokens(css.light, Object.keys(root.cssVars.light));
  tokens(css.dark, Object.keys(root.cssVars.dark));
  return value as StyleTheme;
}
export function decodeCustomTheme(value: unknown): CustomTheme {
  const theme = d.object(value); d.string(theme.id); d.string(theme.name);
  for (const scheme of [theme.light, theme.dark]) {
    const palette = d.object(scheme);
    for (const field of ["background", "surface", "text", "accent"]) {
      d.requireValue(/^#[0-9a-f]{6}$/i.test(d.string(palette[field])));
    }
  }
  return value as CustomTheme;
}
