/** Validates persisted background metadata and configured opacity ranges. */
import * as d from "../../state-runtime/decode";
import config from "../../config/background.json";
export function decodeBackground(state: Record<string, unknown>) {
  d.optional(state.backgroundImage, value => { const image = d.object(value); d.string(image.id); d.string(image.name); });
  const opacity = d.integer(state.backgroundImageOpacity);
  const transparency = d.integer(state.contentBackgroundTransparency);
  d.requireValue(opacity <= config.maxOpacity && opacity >= config.minOpacity);
  d.requireValue(transparency <= config.contentTransparency.max && transparency >= config.contentTransparency.min);
}
