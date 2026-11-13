/** Holds a diagnostic save at the view boundary to inspect transient control styling. */
import { mount, signal, Text, View } from "nsh";
import { Button } from "nsh/base";
import { SettingsPage } from "../../../ui/features/settings/SettingsPage";
import { createAppearanceStore, type DiagnosticOutput } from "../../../ui/state/appearance";

function Preview() {
  const appearance = createAppearanceStore();
  const pending = signal(false);
  let release: (() => void) | undefined;
  const delayed = {
    ...appearance,
    busy: () => pending() || appearance.busy(),
    async chooseDiagnosticOutput(output: DiagnosticOutput, value: boolean) {
      pending.set(true);
      try {
        await new Promise<void>(resolve => { release = resolve; });
        return await appearance.chooseDiagnosticOutput(output, value);
      } finally { release = undefined; pending.set(false); }
    },
  };
  appearance.setSettingsSection("diagnostics");
  return <View class="flex-1" style={{ backgroundColor: appearance.palette().panel }}>
    <View class="flex-row">
      <Button testID="finish-save" onPress={() => release?.()}><Text>Finish save</Text></Button>
      <Text>Saving:{pending() ? "pending" : "ready"}</Text>
      <Text>Outputs:{["file", "console", "structured"].map(output => `${output}=${appearance.diagnosticOutput(output as DiagnosticOutput)}`).join(",")}</Text>
    </View>
    <SettingsPage appearance={delayed} />
  </View>;
}
mount(Preview);
