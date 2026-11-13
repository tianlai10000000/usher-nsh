/** Anchors update details with the library popover; closing never cancels native work. */
import { safeArea, signal, Text, View, windowSize } from "nsh";
import { Button, Popover } from "nsh/base";
import type { JSX } from "nsh/jsx-runtime";
import type { AppearanceStore } from "../../state/appearance";
import type { UpdateActions } from "./index";
import { UpdateDetails } from "./UpdatesSettings";
import { SystemIcon } from "../../components/SystemIcon";
import icons from "../../config/navigation-icons.json";
import { popupMotion, type PopupSide } from "../../theme/motion";
import { iconButtonLook, popupLook } from "../../theme/look";
export function UpdatePanel(
  props: {
    appearance: AppearanceStore;
    updates: UpdateActions;
    trigger: JSX.Element;
  },
) {
  const open = signal(false);
  const side = signal<PopupSide>("right");
  const { palette, t } = props.appearance;
  const motion = () => popupMotion(palette, side());
  const closeLook = () => iconButtonLook(palette, false, true);
  return (
    <Popover
      testID="update-panel"
      trigger={props.trigger}
      open={open()}
      placement="right"
      align="end"
      onPosition={({ placement }) => side.set(placement)}
      margin={palette().metric.panelInset}
      offset={palette().metric.textGap}
      popupStartingStyle={motion().starting}
      popupEndingStyle={motion().ending}
      popupStyle={{
        ...motion().style,
        width: "max-content",
        minWidth: palette().metric.menuWidth + palette().metric.cardPad * 2,
        maxWidth: windowSize().width - safeArea().left - safeArea().right -
          palette().metric.panelInset * 2,
        padding: palette().metric.cardPad,
        gap: palette().metric.groupGap,
        ...popupLook(palette),
      }}
      onOpenChange={({ open: next }) => {
        open.set(next);
        const phase = props.updates.data().domain?.phase;
        if (
          next &&
          (phase?.type === "idle" || phase?.outcome?.type === "up_to_date")
        ) void props.updates.check();
      }}
    >
      <View
        testID="update-panel-header"
        class="flex-row items-center"
        style={{ width: "100%", gap: palette().metric.textGap }}
      >
        <Text
          role="header"
          style={{
            color: palette().text,
            fontSize: palette().metric.fontTitle,
            fontWeight: "600",
            flex: 1,
            minWidth: 0,
            numberOfLines: 0,
          }}
        >
          {t("settings.section.updates")}
        </Text>
        <Button
          testID="update-panel-close"
          accessibilityLabel={t("update.close")}
          style={{ ...closeLook().style, flexShrink: 0 }}
          hoverStyle={closeLook().hoverStyle}
          activeStyle={closeLook().activeStyle}
          focusStyle={closeLook().focusStyle}
          onPress={() => open.set(false)}
        >
          <SystemIcon
            name={icons.close.symbol}
            fallback={icons.close.fallback}
            size={palette().metric.titlebarIcon}
            color={palette().muted}
          />
        </Button>
      </View>
      <UpdateDetails
        testID="update-panel-details"
        appearance={props.appearance}
        updates={props.updates}
      />
    </Popover>
  );
}
