/** Holds button promises and shared busy state to verify production loading and cleanup behavior. */
import { mount, signal, Text, View } from "nsh";
import { Button } from "nsh/base";
import { AsyncButton, useButtonAction } from "../../../ui/components/AsyncButton";
import { ThemeButton } from "../../../ui/components/ThemeControls";
import { createAppearanceStore } from "../../../ui/state/appearance";
import { buttonLook, spacing } from "../../../ui/theme/look";

function Preview() {
  const appearance = createAppearanceStore();
  const { palette } = appearance;
  const calls = signal(0);
  const themedSize = signal({ width: 0, height: 0 });
  const iconSize = signal({ width: 0, height: 0 });
  const result = signal("idle");
  const shared = signal(false);
  const show = signal(true);
  let settle: ((fail: boolean) => void) | undefined;
  function request() {
    calls.update(value => value + 1);
    result.set("pending");
    return new Promise<boolean>(resolve => {
      settle = fail => { result.set(fail ? "failed" : "done"); resolve(!fail); };
    });
  }
  const rejected = useButtonAction({
    loadingFeedback: true,
    onPress: async () => { await request(); throw new Error("request rejected"); },
  });
  return <View style={{ ...spacing(palette).stack, padding: palette().metric.cardPad }}>
    <Text>Calls:{calls()}</Text>
    <Text>Result:{result()}</Text>
    <Text>Size:async-themed:{themedSize().width},{themedSize().height}</Text>
    <Text>Size:async-icon:{iconSize().width},{iconSize().height}</Text>
    <If when={show()}>
      <View class="self-start" onLayout={({ bounds }) => themedSize.set(bounds.size)}>
        <ThemeButton palette={palette} testID="async-themed" loadingFeedback label="Themed action" onPress={request} />
      </View>
      <View class="self-start" onLayout={({ bounds }) => iconSize.set(bounds.size)}>
        <AsyncButton palette={palette} testID="async-icon" loadingFeedback accessibilityLabel="Icon action" iconOnly
          style={buttonLook(palette).style} onPress={request}><Text>→</Text></AsyncButton>
      </View>
      <ThemeButton palette={palette} testID="async-shared" label="Shared action" busy={shared()} onPress={() => { shared.set(true); }} />
    </If>
    <Button testID="reject-action" disabled={rejected.disabled()}
      onPress={() => { void rejected.run().catch(() => result.set("rejected")); }}><Text>Reject action</Text></Button>
    <Text>Reject busy:{String(rejected.busy())}</Text>
    <ThemeButton palette={palette} testID="immediate-action" label="Immediate action" onPress={request} />
    <ThemeButton palette={palette} testID="sync-action" label="Synchronous action" onPress={() => result.set("sync")} />
    <Button testID="finish-action" onPress={() => settle?.(false)}><Text>Finish action</Text></Button>
    <Button testID="fail-action" onPress={() => settle?.(true)}><Text>Fail action</Text></Button>
    <Button testID="finish-shared" onPress={() => shared.set(false)}><Text>Finish shared</Text></Button>
    <Button testID="toggle-controls" onPress={() => show.set(!show())}><Text>Toggle controls</Text></Button>
  </View>;
}
mount(Preview);
