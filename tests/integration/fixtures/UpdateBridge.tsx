/** Exercises the production update transport with deterministic replies and no network or installer. */
import { mount, signal, Text, View } from "nsh";
import { Button } from "nsh/base";
import { UpdateState } from "usher/nsh";
import { onDispose } from "../../../ui/state-runtime/nsh";

function Probe() {
  const status = signal("idle");
  const observed = signal("waiting");
  const stop = UpdateState.subscribe(event => {
    if (event.snapshot) {
      const value = JSON.parse(event.snapshot.value);
      observed.set(value.domain?.phase.type ?? "preparing");
    }
  });
  onDispose(stop);
  const request = (value: object) => UpdateState.request(JSON.stringify(value));
  const require = (condition: boolean, message: string) => { if (!condition) throw new Error(message); };
  async function rejected(work: () => Promise<unknown>, code: string) {
    try { await work(); } catch (error) {
      require(String(error).includes(code), `unexpected error: ${error}`);
      return;
    }
    throw new Error(`expected ${code}`);
  }
  async function run() {
    try {
      const before = await UpdateState.snapshot();
      require(JSON.parse(before.value).domain === null, "initially unconfigured");
      await rejected(() => UpdateState.acquire(), "updates_not_configured");
      await rejected(() => UpdateState.claim("unknown"), "updates_not_configured");
      const configuration = JSON.stringify({ shell: "1.0.0", running: "1.0.0", capability: "in_app", failure_limit: "2" });
      await UpdateState.configure(configuration);
      await rejected(() => UpdateState.configure(configuration), "updates_invalid_command");
      await request({ type: "auto_check", enabled: false });
      const lease = await UpdateState.acquire();
      await rejected(() => UpdateState.acquire(), "updates_busy");
      await request({ type: "check", background: false });
      const check = JSON.parse((await UpdateState.claim(lease))!);
      require(check.effect.type === "check", "check claim");
      await UpdateState.complete(check.token, JSON.stringify({ type: "checked", offer: { version: "1.0.1", has_package: true } }));
      await rejected(() => UpdateState.complete(check.token, JSON.stringify({ type: "downloaded" })), "updates_stale_execution");
      const notice = JSON.parse((await UpdateState.claim(lease))!);
      await UpdateState.complete(notice.token, JSON.stringify({ type: "notified" }));
      await request({ type: "download" });
      const download = JSON.parse((await UpdateState.claim(lease))!);
      await UpdateState.progress(download.token, "10", "100");
      await rejected(() => UpdateState.progress(download.token, "9", "100"), "updates_invalid_command");
      await UpdateState.complete(download.token, JSON.stringify({ type: "downloaded" }));
      const ready = await UpdateState.snapshot();
      require(JSON.parse(ready.value).domain.phase.type === "ready_to_install", "ready state");
      require(ready.storeId === before.storeId, "same application service");
      stop();
      const afterStop = await UpdateState.snapshot();
      require(afterStop.storeId === before.storeId, "subscription does not own service");
      await rejected(() => UpdateState.acquire(), "updates_busy");
      status.set("passed");
    } catch (error) { status.set(`failed:${error}`); }
  }
  return <View>
    <Text>Update:{status()}</Text><Text>Observed:{observed()}</Text>
    <Button onPress={run}><Text>Run update bridge</Text></Button>
  </View>;
}
mount(Probe);
