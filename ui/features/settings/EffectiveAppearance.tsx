import { colorScheme, Text } from "nsh";
import type { AppearanceStore } from "../../state/appearance";

export function EffectiveAppearance(props: { appearance: AppearanceStore }) {
  return (
    <Text style={{ color: props.appearance.palette().muted }}>
      {props.appearance.t("appearance.effective", { mode: props.appearance.t(`mode.${colorScheme()}`) })}
    </Text>
  );
}
