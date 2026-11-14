/** Owns one executor per application scope; native promises are never cancelled or replayed. */
import type {
  Bridge,
  Claim,
  Config,
  Failure,
  Native,
  UpdateData,
} from "./contracts";
import { compareSequence } from "../../state-runtime/projection";

export function classifyFailure(error: unknown): Failure {
  const code = String(error).toLowerCase();
  if (code.includes("already in progress")) return "busy";
  if (
    /signature|hash|checksum|not newer|archive holds|app.?id|runtime|certificate|manifest|size.*match/
      .test(code)
  ) return "rejected";
  if (/permission|disk|space|read.only|rename|write|install/.test(code)) {
    return "disk";
  }
  if (/network|connect|timeout|timed out|offline|dns|http|fetch/.test(code)) {
    return "network";
  }
  return "unknown";
}
function parts(version: string): string[] {
  const match = /^(0|[1-9]\d*)\.(0|[1-9]\d*)\.(0|[1-9]\d*)(?:[-+].*)?$/.exec(
    version,
  );
  if (!match) throw new Error("invalid manifest version");
  return match.slice(1, 4);
}
function compareVersion(a: string, b: string) {
  const right = parts(b);
  for (const [index, value] of parts(a).entries()) {
    const order = compareSequence(value, right[index]);
    if (order) return order;
  }
  return 0;
}
export async function check(
  native: Native,
  shell: ReturnType<Native["shellInfo"]>,
) {
  // Never turn a failed package check into a whole-app fallback: trust failures must propagate.
  const pack = shell.packages ? await native.checkForPackage() : null;
  const app = await native.checkForUpdate();
  const chosen =
    pack && (!app || compareVersion(pack.version, app.version) >= 0)
      ? pack
      : app;
  if (chosen === pack && pack && shell.version) {
    const packageLine = parts(pack.version).slice(0, 2).join(".");
    const shellLine = parts(shell.version).slice(0, 2).join(".");
    if (
      packageLine !== shellLine &&
      (!app || compareVersion(pack.version, app.version) !== 0)
    ) {
      // Each NSH check caches its own artifact. Never label one version and install another.
      throw new Error("Update source changed during check; try again");
    }
  }
  return {
    type: "checked",
    manual_url: app?.url ?? null,
    offer: chosen
      ? {
        version: chosen.version,
        has_package: pack?.version === chosen.version,
        can_install: app?.canInstall ?? false,
      }
      : null,
  };
}
export interface Clock {
  set(run: () => void, ms: number): unknown;
  clear(timer: unknown): void;
}
export const clock: Clock = {
  set: (run, ms) => setTimeout(run, ms),
  clear: (timer) => clearTimeout(timer as ReturnType<typeof setTimeout>),
};

/** Start/detach manage view demand; detach retains the lease and any native operation until it settles. */
export function createController(
  bridge: Bridge,
  native: Native,
  config: Config,
  report: (code: string | null) => void,
  timers: Clock = clock,
) {
  let lease: string | undefined;
  let shell: ReturnType<Native["shellInfo"]>;
  let stop: (() => void) | undefined;
  let timer: unknown;
  let alive = false;
  let pumping = false;
  let wake = false;
  let halted = false;
  let automatic = false;
  let startup: Promise<void> | undefined;
  let latestSequence: string | undefined;
  let storeId: string | undefined;
  let generation = 0;
  function stopTimer() {
    if (timer !== undefined) timers.clear(timer);
    timer = undefined;
  }
  function schedule(delay: number) {
    stopTimer();
    if (!alive || !lease || !automatic || halted) return;
    timer = timers.set(() => {
      timer = undefined;
      void bridge.dispatch({ type: "check", background: true }).catch(() =>
        report("update.connectionFailed")
      )
        .finally(() => schedule(config.checkIntervalMs));
    }, delay);
  }
  function receive(state: UpdateData) {
    const next = !!state.domain?.auto_check &&
      state.domain.capability !== "unsupported" &&
      state.domain.capability !== "system_managed";
    if (next !== automatic) {
      automatic = next;
      schedule(config.startupDelayMs);
    }
    if (state.pending !== "0") void pump();
  }
  async function initialize(epoch: number) {
    let snapshot = await bridge.read();
    storeId = snapshot.storeId;
    try {
      shell = await native.shellInfo();
    } catch {
      shell = { version: null, build: null, nshVersion: "", packages: false };
    }
    if (!snapshot.value.domain) {
      let running: Awaited<ReturnType<Native["runningPackage"]>> = null;
      try {
        running = await native.runningPackage();
      } catch { /* Built-in code needs no package identity. */ }
      try {
        snapshot = await bridge.configure({
          shell: shell.version ?? null,
          running: running?.version ?? shell.version ?? null,
          capability: shell.version ? "in_app" : "unsupported",
          failure_limit: String(config.failureLimit),
          shell_build: shell.build == null ? null : String(shell.build),
          package_build: running ? String(running.build) : null,
        });
      } catch (error) {
        // Another window can win configuration; never suppress a genuine configuration failure.
        if (!String(error).includes("updates_invalid_command")) throw error;
        snapshot = await bridge.read();
        if (!snapshot.value.domain) throw error;
      }
    }
    if (!lease) {
      try {
        lease = await bridge.acquire();
      } catch (error) {
        if (!String(error).includes("updates_busy")) throw error;
      }
    }
    if (!alive || generation !== epoch) return;
    const cancel = await bridge.subscribe({
      snapshot(value) {
        if (
          !alive || generation !== epoch || value.storeId !== storeId ||
          (latestSequence !== undefined &&
            compareSequence(value.sequence, latestSequence) <= 0)
        ) return;
        latestSequence = value.sequence;
        receive(value.value);
      },
      end() {
        if (!alive || generation !== epoch) return;
        halted = true;
        stopTimer();
        report("update.connectionFailed");
      },
    });
    if (!alive || generation !== epoch) {
      cancel();
      return;
    }
    stop = cancel;
    if (
      latestSequence === undefined ||
      compareSequence(snapshot.sequence, latestSequence) > 0
    ) {
      latestSequence = snapshot.sequence;
      receive(snapshot.value);
    }
    schedule(config.startupDelayMs);
    void pump();
  }
  async function execute(claim: Claim) {
    const effect = claim.effect;
    let reply: object;
    try {
      if (effect.type === "check") reply = await check(native, shell);
      else if (effect.type === "notify") reply = { type: "notified" };
      else if (effect.type === "download") {
        let progress = Promise.resolve();
        const onProgress: Parameters<Native["downloadPackage"]>[0] = (
          value,
        ) => {
          progress = progress.then(() =>
            bridge.progress(
              claim.token,
              String(value.received),
              value.total == null ? null : String(value.total),
            )
          );
          // The promise remains owned and awaited even if reporting fails during native I/O.
          void progress.catch(() => report("update.connectionFailed"));
        };
        await (effect.candidate?.kind === "package"
          ? native.downloadPackage(onProgress)
          : native.downloadUpdate(onProgress));
        await progress.catch(() => report("update.connectionFailed"));
        reply = { type: "downloaded" };
      } else {
        await (effect.candidate?.kind === "package"
          ? native.restartToPackage()
          : native.installAndRestart());
        // NSH settles installation only on failure. Do not pretend a mocked resolution proves a restart.
        halted = true;
        stopTimer();
        return;
      }
    } catch (error) {
      reply = {
        type: "failed",
        failure: classifyFailure(error),
        detail: String(error),
      };
    }
    // A transport failure must not re-enter native execution or be reported as a native failure.
    await bridge.complete(claim.token, reply);
  }
  async function pump() {
    if (!alive || !lease || halted) return;
    wake = true;
    if (pumping) return;
    pumping = true;
    try {
      do {
        wake = false;
        while (alive && !halted) {
          const claim = await bridge.claim(lease);
          if (!claim) break;
          await execute(claim);
        }
        // A publication can arrive while an empty claim is resolving. Preserve that wakeup.
      } while (alive && !halted && wake);
    } catch {
      halted = true;
      stopTimer();
      report("update.connectionFailed");
    } finally {
      pumping = false;
    }
  }

  return {
    start() {
      alive = true;
      if (!startup) {
        startup = initialize(++generation).catch(() =>
          report("update.connectionFailed")
        );
      }
      return startup;
    },
    detach() {
      generation++;
      alive = false;
      stopTimer();
      stop?.();
      stop = undefined;
      startup = undefined;
      latestSequence = undefined;
    },
  };
}
