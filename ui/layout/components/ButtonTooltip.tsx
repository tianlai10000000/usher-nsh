/** Shares themed button hints while NSH owns their triggers, overlay and collision handling. */
import { safeArea, Text, windowSize } from "nsh";
import { Tooltip, type TooltipProps } from "nsh/base";
import type { AppearanceStore } from "../../state/appearance";

/** Attach to a button's `tooltip` prop so disabled buttons retain their native hint behavior. */
export function ButtonTooltip(props: {
  appearance: AppearanceStore;
  label: string;
  placement: TooltipProps["placement"];
  testID: string;
}) {
  const { palette } = props.appearance;

  return (
    <Tooltip testID={props.testID} placement={props.placement}
      offset={palette().metric.textGap} margin={palette().metric.panelInset}
      style={{
        backgroundColor: palette().background,
        borderColor: palette().border,
        borderWidth: palette().metric.borderWidth,
        borderRadius: palette().metric.radiusControl,
        paddingHorizontal: palette().metric.buttonPadXSmall,
        paddingVertical: palette().metric.textGap,
        boxShadow: palette().shellShadow,
        // Flip/shift handles position; a width limit lets longer translations wrap inside it.
        maxWidth: Math.max(0, windowSize().width - safeArea().left - safeArea().right
          - palette().metric.panelInset * 2),
      }}>
      <Text style={{ color: palette().text, fontSize: palette().metric.fontNote }}>{props.label}</Text>
    </Tooltip>
  );
}
