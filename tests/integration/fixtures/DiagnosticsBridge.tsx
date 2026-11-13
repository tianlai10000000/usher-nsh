/** Verifies supported output preferences through the real NSH settings transport. */
import { mount, signal, Text, View } from "nsh";
import { Button } from "nsh/base";
import { Settings, SettingsState } from "usher/nsh";
function Probe() {
  const result = signal("idle");
  async function run() {
    const require = (condition: boolean, message: string) => { if (!condition) throw new Error(message); };
    try {
      const initial = await Settings.startDiagnostics();
      require(!initial.diagnosticConsole && initial.diagnosticFile && !initial.diagnosticStructured, "compatibility defaults");
      await Settings.setDiagnosticOutput("console", true);
      await Settings.setDiagnosticOutput("file", false);
      await Settings.setDiagnosticOutput("structured", true);
      const saved = await SettingsState.snapshot();
      require(saved.value.diagnosticConsole && !saved.value.diagnosticFile && saved.value.diagnosticStructured, "shared projection");
      await Settings.setDiagnosticLevel("OFF");
      const off = await SettingsState.snapshot();
      require(off.value.diagnosticLevel === "OFF" && off.value.diagnosticConsole && off.value.diagnosticStructured, "level preserves outputs");
      for (const output of ["terminal", "memory", "../file"]) {
        let rejected = false;
        try { await Settings.setDiagnosticOutput(output, true); } catch (error) { rejected = String(error).includes("invalid_diagnostic_output"); }
        require(rejected, "unsupported collector or path must be rejected");
      }
      let rejected = false;
      try { await Settings.logEvent("INFO", "ui", "free text"); } catch (error) { rejected = String(error).includes("invalid_log_event"); }
      require(rejected, "short identifiers only");
      result.set("passed");
    } catch (error) { result.set(`failed:${error}`); }
  }
  return <View><Text>Diagnostics:{result()}</Text><Button onPress={run}><Text>Run diagnostics bridge</Text></Button></View>;
}
mount(Probe);
