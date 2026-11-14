/** Uses a registry-generated Gallery snapshot without implementing theme resolution in tests. */
import { decodeStyleTheme } from "../../../ui/state/appearance/decode";
import bundled from "../../../ui/state/appearance/bundled.generated.json";
import source from "../../../ui/theme/presets/nsh-gallery.json";
import type { StyleTheme } from "../../../ui/state/appearance/contracts";
export const galleryTheme: StyleTheme = decodeStyleTheme(bundled.themes.find(theme => theme.id === source.id));
