/** Displays shortcut health and details through a borrowed layout contract, never registering keys. */
import { signal, Text, View, safeArea, windowSize } from "nsh";
import { Button, Popover } from "nsh/base";
import type { AppState } from "../../state/app";
import type { ShortcutIndicator } from "../contracts/shortcuts";
import { SystemIcon } from "../../components/SystemIcon";
import { ButtonTooltip } from "./ButtonTooltip";
import { ThemeButton } from "../../components/ThemeControls";
import { buttonLook, noteLook, popupLook } from "../../theme/look";
export function ShortcutStatus(props: { appearance: AppState; indicator: ShortcutIndicator }) {
  const palette = props.appearance.appearance.palette;
  const t = props.appearance.preferences.t;
  const open = signal(false);
  const triggerLook = () => buttonLook(palette, { kind: "ghost", size: "small" });
  const label = () => props.indicator.warning() ? t("shortcuts.attention") : props.indicator.summary();
  const color = () => props.indicator.warning() ? palette().updateWarningText : palette().muted;

  return <Popover open={open()} onOpenChange={({ open: next }) => open.set(next)} placement="top" align="end"
    offset={palette().metric.textGap} margin={palette().metric.panelInset}
    popupStyle={{ ...popupLook(palette), padding: palette().metric.cardPad, gap: palette().metric.textGap,
      maxWidth: windowSize().width - safeArea().left - safeArea().right - palette().metric.panelInset * 2 }}
    trigger={<Button testID="shortcut-status" style={triggerLook().style}
      hoverStyle={triggerLook().hoverStyle} activeStyle={triggerLook().activeStyle} focusStyle={triggerLook().focusStyle}
      tooltip={<ButtonTooltip appearance={props.appearance} placement="top" testID="shortcut-status-tooltip"
        label={`${t("shortcuts.details")} · ${props.indicator.pageSummary()}`} />}
      accessibilityLabel={`${props.indicator.summary()}. ${props.indicator.pageSummary()}`}>
      <SystemIcon name={props.indicator.warning() ? "exclamationmark.triangle" : "keyboard"}
        fallback={props.indicator.warning() ? "!" : "⌨"} size={palette().metric.controlIcon} color={color()} />
      <Text style={{ ...noteLook(palette), color: color() }}>{label()}</Text>
    </Button>}>
    <View testID="shortcut-details" style={{ gap: palette().metric.textGap }}>
      <Text style={noteLook(palette)}>{t("shortcuts.details")}</Text>
      <Each of={props.indicator.details()} key={row => row.id}>{row => <Text testID={`shortcut-detail-${row().id}`} style={noteLook(palette)}>{`${row().label}: ${row().value}`}</Text>}</Each>
      <If when={props.indicator.retryable()}><ThemeButton palette={palette} testID="shortcut-retry" label={t("settings.retry")} loadingFeedback onPress={props.indicator.retry} /></If>
    </View>
  </Popover>;
}
