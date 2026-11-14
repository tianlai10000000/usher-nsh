/** Displays the effective host appearance through the shared appearance facade. */
import { Text } from "nsh";
import type { AppState } from "../../state/app";

export function EffectiveAppearance(props: { appearance: AppState }) {
  return (
    <Text style={{ color: props.appearance.appearance.palette().muted }}>
      {props.appearance.preferences.t("appearance.effective", { mode: props.appearance.preferences.t(`mode.${props.appearance.appearance.effectiveScheme()}`) })}
    </Text>
  );
}
