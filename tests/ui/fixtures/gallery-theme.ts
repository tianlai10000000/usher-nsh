/** Builds an in-memory Gallery snapshot for visual fixtures without a settings service. */
import gallery from "../../../ui/theme/presets/nsh-gallery.json";
import { fallbackTheme, type StyleTheme } from "../../../ui/theme";

const strings = (tokens: Record<string, string | number>) =>
  Object.fromEntries(Object.entries(tokens).map(([name, value]) => [name, String(value)]));

export const galleryTheme: StyleTheme = {
  ...fallbackTheme,
  id: gallery.id,
  cssVars: {
    touch: strings(gallery.cssVars.touch),
    light: { ...fallbackTheme.cssVars.light, ...strings(gallery.cssVars.shared), ...strings(gallery.cssVars.light) },
    dark: { ...fallbackTheme.cssVars.dark, ...strings(gallery.cssVars.shared), ...strings(gallery.cssVars.dark) },
  },
};
