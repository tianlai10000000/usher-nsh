/**
 * The bottom status bar: app version and settings save state.
 *
 * The frame mounts it below the menus and the content, so it spans the full window width.
 * Visibility is the saved `showStatusBar` preference; this file only draws the bar.
 * Center both flex lines and their items so wrapping does not pin text to the top of the bar.
 */
import { Text, View } from "nsh";
import appMetadata from "../../../nsh.json";
import type { AppState } from "../../state/app";
import { noteLook } from "../../theme/look";

export function StatusBar(props: { appearance: AppState }) {
  const palette = props.appearance.appearance.palette;
  const statusKey = props.appearance.settings.statusKey;
  const t = props.appearance.preferences.t;
  return (
    <View testID="bottom-status-bar" role="summary" accessibilityLabel={t("statusBar.label")}
      class="flex-row flex-wrap items-center content-center justify-between border-t"
      style={{ borderColor: palette().shellBorder, minHeight: palette().metric.statusBar,
        paddingHorizontal: palette().metric.statusPadX, columnGap: palette().metric.stackGap + 4, rowGap: palette().metric.textGap }}>
      <Text testID="status-bar-version" style={noteLook(palette)}>
        {t("statusBar.version", { version: appMetadata.version })}
      </Text>
      <Text testID="status-bar-settings" style={noteLook(palette)}>
        {t(statusKey())}
      </Text>
    </View>
  );
}
