/** Composes the window material, theme tint and user image below every interactive region. */
import { cleanup, effect, Image, nativeComponent, platform, signal, View } from "nsh";
import { Settings } from "usher/nsh";
import type { AppearanceStore } from "../../state/appearance";
import backgroundConfig from "../../config/background.json";

const WindowMaterial = nativeComponent<object, { ready: { installed: boolean } }>("UsherMacWindowMaterial", {
  props: [],
  events: ["ready"],
});

export function WindowBackdrop(props: { appearance: AppearanceStore }) {
  const { palette, backgroundImageId } = props.appearance;
  const hasMaterial = platform.os === "macos" && Settings.windowMaterialAvailable();
  const materialReady = signal(false);
  const imageSource = signal<string | undefined>();

  effect(() => {
    const id = backgroundImageId();
    let active = true;
    let objectURL: string | undefined;
    imageSource.set(undefined);
    cleanup(() => {
      active = false;
      if (objectURL) URL.revokeObjectURL(objectURL);
    });
    if (!id) {
      props.appearance.reportBackgroundLoadFailure(false);
      return;
    }
    // Ignore late loads after replacement or unmount; only the current image owns its URL.
    void Settings.loadBackgroundImage(id).then((bytes) => {
      if (!active) return;
      objectURL = URL.createObjectURL(new Blob([bytes]));
      imageSource.set(objectURL);
      props.appearance.reportBackgroundLoadFailure(false);
    }).catch(() => {
      if (active) props.appearance.reportBackgroundLoadFailure(true);
    });
  });

  return (
    <View testID={materialReady() ? "window-frosted" : "window-solid"} accessibilityElementsHidden
      style={{ position: "absolute", left: 0, top: 0, right: 0, bottom: 0, pointerEvents: "none",
        backgroundColor: materialReady() ? palette().windowTint : palette().background }}>
      <If when={hasMaterial}>
        <WindowMaterial testID="window-material" accessibilityElementsHidden
          style={{ position: "absolute", width: 1, height: 1, pointerEvents: "none" }}
          onReady={({ installed }) => materialReady.set(installed)} />
      </If>
      <If when={imageSource()}>
        <Image testID="window-background-image" src={imageSource()} accessibilityElementsHidden
          style={{ position: "absolute", left: 0, top: 0, right: 0, bottom: 0, objectFit: "cover",
            opacity: props.appearance.backgroundImageOpacity() / backgroundConfig.maxOpacity,
            pointerEvents: "none" }}
          onError={() => {
            imageSource.set(undefined);
            props.appearance.reportBackgroundLoadFailure(true);
          }} />
      </If>
    </View>
  );
}
