import { nativeComponent, platform, Text, View } from "nsh";

const NativeSymbol = nativeComponent<{ name: string; size: number; color: string }, object>("UsherSystemSymbol", {
  props: ["name", "size", "color"],
  events: [],
});

/** An SF Symbol on macOS (drawn by AppKit); a plain glyph elsewhere. */
export function SystemIcon(props: { name: string; fallback: string; color: string; size?: number }) {
  const size = props.size ?? 16;
  return (
    <If when={platform.os === "macos"} else={
      <Text accessibilityElementsHidden style={{ color: props.color, fontSize: size }}>{props.fallback}</Text>
    }>
      <View style={{ width: size, height: size, pointerEvents: "none" }}>
        <NativeSymbol accessibilityElementsHidden name={props.name} size={size} color={props.color}
          style={{ width: size, height: size, pointerEvents: "none" }} />
      </View>
    </If>
  );
}
