/** Adds opt-in request feedback to the public NSH Button without owning business state. */
import { cleanup, signal, View } from "nsh";
import { Button, type ButtonProps } from "nsh/base";
import type { AppearanceStore } from "../state/appearance";
import { ProgressRing } from "./ProgressRing";

type ActionProps = {
  /** Enable for waitful tasks; immediate settings and navigation do not show loading. */
  loadingFeedback?: boolean;
  busy?: boolean;
  disabled?: boolean;
  onPress?: () => void | Promise<unknown>;
};

/** Tracks only this view's event promise; longer operations supply their authoritative busy state. */
export function useButtonAction(props: ActionProps) {
  const pending = signal(false);
  let locked = false;
  let disposed = false;
  cleanup(() => { disposed = true; });
  const busy = () => props.busy === true || (props.loadingFeedback === true && pending());
  const disabled = () => props.disabled === true || busy();
  async function run() {
    if (disposed || locked || disabled()) return;
    locked = true;
    try {
      const result = props.onPress?.();
      if (result && typeof result.then === "function") {
        pending.set(true);
        await result;
      }
    } finally {
      locked = false;
      if (!disposed) pending.set(false);
    }
  }
  return { busy, disabled, run };
}

type Props = Omit<ButtonProps, "onPress"> & ActionProps & {
  palette: AppearanceStore["palette"];
  /** Icon-only controls replace their existing icon while busy to preserve the hit target's size. */
  iconOnly?: boolean;
};

/** Preserves NSH styling, tooltips and semantics while guarding asynchronous activation. */
export function AsyncButton(props: Props) {
  const action = useButtonAction(props);
  const supportsLoading = () => props.loadingFeedback === true || props.busy !== undefined;
  const iconSize = () => props.iconOnly ? props.palette().metric.titlebarIcon : props.palette().metric.controlIcon;
  const forwarded = () => {
    const { palette, busy, loadingFeedback, iconOnly, ...button } = props;
    return button;
  };
  return (
    <Button {...forwarded()} disabled={action.disabled()} onPress={action.run}>
      <If when={supportsLoading()}>
        <View class="items-center justify-center" style={{ width: iconSize(), height: iconSize(), flexShrink: 0 }}>
          <If when={action.busy()} else={<If when={props.iconOnly}>{props.children}</If>}>
            <ProgressRing
              palette={props.palette}
              size={iconSize()}
              progress={null}
              label={props.accessibilityLabel ?? undefined}
              testID={props.testID ? `${props.testID}-loading` : undefined}
            />
          </If>
        </View>
      </If>
      <If when={!props.iconOnly || !supportsLoading()}>{props.children}</If>
    </Button>
  );
}
