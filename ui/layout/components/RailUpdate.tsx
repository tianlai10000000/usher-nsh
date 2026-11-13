/** Renders update status using theme metrics and the public Canvas and Button APIs. */
import { Canvas, reduceMotion, View } from "nsh";
import { Button } from "nsh/base";
import { SystemIcon } from "../../components/SystemIcon";
import icons from "../../config/navigation-icons.json";
import type { AppearanceStore } from "../../state/appearance";
import { iconButtonLook } from "../../theme/look";
import type { UpdateIndicator } from "../contracts/updates";
import { ButtonTooltip } from "./ButtonTooltip";
export function RailUpdate(
  props: { appearance: AppearanceStore; updates?: UpdateIndicator },
) {
  const { palette, t } = props.appearance;
  const state = () => props.updates?.state();
  const look = () => iconButtonLook(palette);
  const checking = () => state()?.shape === "checking";
  const ring = () => checking() || state()?.shape === "progress";
  const emphasis = () => state()?.shape === "emphasis";
  const color = () =>
    state()?.warning ? palette().updateWarning : palette().accent;
  const spin = () => ({
    animationName: reduceMotion() ? "none" as const : "spin" as const,
    animationDuration: palette().metric.updateSpinMs,
    animationTimingFunction: "linear" as const,
    animationIterationCount: "infinite" as const,
  });
  const button = (
    <Button
      testID="rail-update"
      accessibilityLabel={state()?.label ?? t("nav.update")}
      tooltip={
        <ButtonTooltip
          appearance={props.appearance}
          testID="rail-update-tooltip"
          placement="right"
          label={state()?.label ?? t("nav.update")}
        />
      }
      disabled={state()?.disabled ?? true}
      style={look().style}
      hoverStyle={emphasis()
        ? { opacity: 1 - palette().metric.hoverWash }
        : look().hoverStyle}
      activeStyle={look().activeStyle}
      focusStyle={look().focusStyle}
      disabledStyle={ring() ? { opacity: 1 } : look().disabledStyle}
    >
      <If
        when={ring()}
        else={
          <View
            testID="rail-update-symbol"
            class="items-center justify-center"
            style={{
              width: palette().metric.buttonHeightSmall,
              height: palette().metric.buttonHeightSmall,
              borderRadius: palette().metric.buttonHeightSmall / 2,
              backgroundColor: emphasis() ? color() : "transparent",
            }}
          >
            <SystemIcon
              name={emphasis() ? icons.update.readySymbol : icons.update.symbol}
              fallback={emphasis()
                ? icons.update.readyFallback
                : icons.update.fallback}
              size={emphasis()
                ? palette().metric.updateIcon
                : palette().metric.railUtilityIcon}
              color={emphasis()
                ? state()?.warning
                  ? palette().updateWarningText
                  : palette().accentText
                : palette().text}
            />
          </View>
        }
      >
        <View
          testID={checking() ? "rail-update-checking" : "rail-update-progress"}
          class="items-center justify-center"
          style={{
            width: palette().metric.buttonHeightSmall,
            height: palette().metric.buttonHeightSmall,
          }}
        >
          <Canvas
            testID={checking()
              ? "update-checking-ring"
              : "update-progress-ring"}
            role="progressbar"
            accessibilityLabel={state()?.label}
            style={{
              width: palette().metric.buttonHeightSmall,
              height: palette().metric.buttonHeightSmall,
              ...(checking() || state()?.progress === null ? spin() : {}),
            }}
            draw={(cx, size) => {
              const radius = (Math.min(size.width, size.height) -
                palette().metric.borderWidth * 2) / 2;
              cx.lineWidth = palette().metric.borderWidth * 2;
              cx.lineCap = "round";
              cx.beginPath();
              cx.strokeStyle = palette().border;
              cx.arc(size.width / 2, size.height / 2, radius, 0, Math.PI * 2);
              cx.stroke();
              cx.beginPath();
              cx.strokeStyle = palette().muted;
              cx.arc(
                size.width / 2,
                size.height / 2,
                radius,
                -Math.PI / 2,
                -Math.PI / 2 +
                  (checking()
                      ? palette().metric.updateSegment
                      : (state()?.progress ?? palette().metric.updateSegment)) *
                    Math.PI * 2,
              );
              cx.stroke();
            }}
          />
          <If when={!checking()}>
            <View
              class="absolute"
              accessibilityElementsHidden
              style={{ pointerEvents: "none" }}
            >
              <SystemIcon
                name={icons.update.progressSymbol}
                fallback={icons.update.progressFallback}
                size={palette().metric.badge}
                color={palette().border}
              />
            </View>
          </If>
        </View>
      </If>
    </Button>
  );
  return props.updates ? props.updates.present(button) : button;
}
