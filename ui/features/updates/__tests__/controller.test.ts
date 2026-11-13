/** Deterministic executor tests: native calls, timers, disposal and failed receipts are separate boundaries. */
import { expect, test } from "nsh/test";
import {
  check,
  classifyFailure,
  type Clock,
  createController,
} from "../controller";
import {
  type Bridge,
  type Claim,
  initial,
  type Native,
  type UpdateData,
} from "../contracts";
import type { Observer } from "../../../state/contracts";
import { presentation } from "../indicator";

const config = { startupDelayMs: 30, checkIntervalMs: 600, failureLimit: 2 };
const shell = {
  version: "1.0.0",
  build: 1,
  packages: true,
  nshVersion: "0.5.0",
};
function native(): Native {
  return {
    shellInfo: () => shell,
    runningPackage: () => null,
    checkForPackage: async () => null,
    checkForUpdate: async () => null,
    downloadPackage: async () => {},
    downloadUpdate: async () => {},
    restartToPackage: async () => {
      throw new Error("restart returned");
    },
    installAndRestart: async () => {
      throw new Error("install returned");
    },
  };
}
function harness() {
  let sequence = 0;
  let observer: Observer<UpdateData> | undefined;
  let state: UpdateData = JSON.parse(JSON.stringify(initial));
  const jobs: Claim[] = [];
  const replies: object[] = [];
  const tasks = new Map<number, { run(): void; ms: number }>();
  let timerId = 0;
  const timers: Clock = {
    set(run, ms) {
      const id = ++timerId;
      tasks.set(id, { run, ms });
      return id;
    },
    clear(id) {
      tasks.delete(id as number);
    },
  };
  const envelope = () => ({
    storeId: "1",
    sequence: String(sequence),
    value: JSON.parse(JSON.stringify(state)),
  });
  const publish = () => {
    sequence++;
    observer?.snapshot(envelope());
  };
  const bridge: Bridge = {
    read: async () => envelope(),
    configure: async () => {
      state.domain = {
        shell: "1.0.0",
        running: "1.0.0",
        auto_check: true,
        capability: "in_app",
        phase: { type: "idle" },
      };
      publish();
      return envelope();
    },
    subscribe(next) {
      observer = next;
      next.snapshot(envelope());
      return () => {
        if (observer === next) observer = undefined;
      };
    },
    acquire: async () => "lease",
    claim: async () => jobs.shift() ?? null,
    complete: async (_token, reply) => {
      replies.push(reply);
      state.pending = String(jobs.length);
      publish();
    },
    progress: async () => {},
    dispatch: async (command) => {
      if (command.type === "auto_check") {
        state.domain!.auto_check = command.enabled;
      }
      publish();
      return envelope();
    },
  };
  return {
    bridge,
    timers,
    tasks,
    replies,
    publish,
    state: () => state,
    enqueue(claim: Claim) {
      jobs.push(claim);
      state.pending = String(jobs.length);
      publish();
    },
  };
}
async function settle() {
  for (let n = 0; n < 30; n++) await Promise.resolve();
}

test("updates controller keeps package and shell checks serial and propagates trust failures", async () => {
  const calls: string[] = [];
  const updater = native();
  updater.checkForPackage = async () => {
    calls.push("package");
    throw new Error("signature invalid");
  };
  updater.checkForUpdate = async () => {
    calls.push("shell");
    return null;
  };
  let failure = "";
  try {
    await check(updater, shell);
  } catch (error) {
    failure = classifyFailure(error);
  }
  expect(failure).toBe("rejected");
  expect(calls).toEqual(["package"]);
  updater.checkForPackage = async () => ({ version: "1.0.1", build: 2 });
  updater.checkForUpdate = async () => ({
    version: "1.1.0",
    canInstall: false,
    url: "https://example.test/update",
  });
  expect((await check(updater, shell)).offer).toEqual({
    version: "1.1.0",
    has_package: false,
    can_install: false,
  });
});
test("updates controller owns timers and responds immediately to automatic preference changes", async () => {
  const h = harness();
  const c = createController(h.bridge, native(), config, () => {}, h.timers);
  await c.start();
  expect([...h.tasks.values()].map((task) => task.ms)).toEqual([
    config.startupDelayMs,
  ]);
  await h.bridge.dispatch({ type: "auto_check", enabled: false });
  expect(h.tasks.size).toBe(0);
  await h.bridge.dispatch({ type: "auto_check", enabled: true });
  expect(h.tasks.size).toBe(1);
  c.detach();
  expect(h.tasks.size).toBe(0);
  await c.start();
  expect(h.tasks.size).toBe(1);
  c.detach();
});
test("updates controller retains an in-flight native operation across view detach and never replays it", async () => {
  const h = harness();
  const updater = native();
  let downloads = 0;
  let finish!: () => void;
  updater.downloadPackage = () => {
    downloads++;
    return new Promise((resolve) => {
      finish = resolve;
    });
  };
  const c = createController(h.bridge, updater, config, () => {}, h.timers);
  await c.start();
  h.enqueue({
    token: "download",
    effect: {
      type: "download",
      candidate: { version: "1.0.1", kind: "package" },
    },
  });
  await settle();
  c.detach();
  await c.start();
  expect(downloads).toBe(1);
  finish();
  await settle();
  expect(h.replies).toEqual([{ type: "downloaded" }]);
  expect(downloads).toBe(1);
  c.detach();
});
test("updates controller never retries native work when its completion transport fails", async () => {
  const h = harness();
  const updater = native();
  let downloads = 0;
  const errors: (string | null)[] = [];
  updater.downloadUpdate = async () => {
    downloads++;
  };
  h.bridge.complete = async () => {
    throw new Error("bridge closed");
  };
  const c = createController(
    h.bridge,
    updater,
    config,
    (error) => errors.push(error),
    h.timers,
  );
  await c.start();
  h.enqueue({
    token: "download",
    effect: {
      type: "download",
      candidate: { version: "1.1.0", kind: "shell" },
    },
  });
  await settle();
  h.publish();
  await settle();
  expect(downloads).toBe(1);
  expect(errors).toContain("update.connectionFailed");
  expect(h.tasks.size).toBe(0);
  c.detach();
});
test("updates projection distinguishes preparing, download rings, manual failures and unknown progress", () => {
  expect(presentation(initial).key).toBe("update.preparing");
  expect(presentation(initial).visible).toBe(false);
  const state: UpdateData = {
    ...initial,
    info: {},
    domain: {
      shell: "1.0.0",
      running: "1.0.0",
      capability: "in_app",
      auto_check: true,
      phase: {
        type: "downloading",
        candidate: { version: "1.0.1", kind: "package" },
        downloaded: "5",
        total: null,
      },
    },
  };
  expect(presentation(state).shape).toBe("progress");
  expect(presentation(state).disabled).toBe(true);
  state.domain!.phase.total = "20";
  expect(presentation(state).progress).toBe(0.25);
  state.domain!.phase.total = null;
  state.domain!.phase.candidate!.kind = "shell";
  expect(presentation(state).shape).toBe("progress");
  expect(presentation(state).progress).toBe(null);
  state.domain!.phase = {
    type: "checked",
    outcome: { type: "failed", failure: "network" },
  };
  expect(presentation(state).shape).toBe("quiet");
  expect(presentation(state).visible).toBe(false);
  state.info.manual_check = true;
  expect(presentation(state).shape).toBe("emphasis");
  expect(presentation(state).warning).toBe(true);
  expect(presentation(state).visible).toBe(true);
  expect(presentation(state).canRetryCheck).toBe(true);
  state.domain!.phase = { type: "checking" };
  expect(presentation(state).visible).toBe(true);
  expect(presentation(state).shape).toBe("checking");
  expect(presentation(state).disabled).toBe(true);
  state.domain!.phase = { type: "checked", outcome: { type: "up_to_date" } };
  expect(presentation(state).visible).toBe(false);
  state.domain!.phase = {
    type: "install_failed",
    candidate: { version: "1.0.1", kind: "package" },
    failure: "network",
  };
  expect(presentation(state).visible).toBe(true);
  expect(presentation(state).warning).toBe(true);
});

test("updates controller configures unsupported environments and schedules no network work", async () => {
  const h = harness();
  const updater = native();
  const configurations: object[] = [];
  updater.shellInfo = () => {
    throw new Error("no shell");
  };
  h.bridge.configure = async (value) => {
    configurations.push(value);
    h.state().domain = {
      shell: "0.1.0",
      running: "0.1.0",
      auto_check: true,
      capability: "unsupported",
      phase: { type: "idle" },
    };
    return h.bridge.read();
  };
  const c = createController(h.bridge, updater, config, () => {}, h.timers);
  await c.start();
  expect(configurations).toEqual([{
    shell: null,
    running: null,
    capability: "unsupported",
    failure_limit: "2",
    shell_build: null,
    package_build: null,
  }]);
  expect(h.tasks.size).toBe(0);
  c.detach();
});
test("updates controller fences delayed subscriptions and endings from a disposed view generation", async () => {
  const h = harness();
  const registrations: {
    observer: Observer<UpdateData>;
    resolve(cancel: () => void): void;
  }[] = [];
  const cancelled: number[] = [];
  const errors: (string | null)[] = [];
  h.bridge.subscribe = (observer) =>
    new Promise((resolve) => registrations.push({ observer, resolve }));
  const c = createController(
    h.bridge,
    native(),
    config,
    (error) => errors.push(error),
    h.timers,
  );
  const first = c.start();
  await settle();
  c.detach();
  const second = c.start();
  await settle();
  registrations[1].resolve(() => cancelled.push(1));
  await second;
  registrations[0].resolve(() => cancelled.push(0));
  await first;
  registrations[0].observer.end("closed");
  expect(errors).toEqual([]);
  expect(cancelled).toEqual([0]);
  expect(h.tasks.size).toBe(1);
  c.detach();
  expect(cancelled).toEqual([0, 1]);
});
test("updates controller drains ordered progress before completion and reports native install failures", async () => {
  const h = harness();
  const updater = native();
  const calls: string[] = [];
  updater.downloadUpdate = async (progress) => {
    progress?.({ received: 1, total: 2 });
    progress?.({ received: 2, total: 2 });
  };
  h.bridge.progress = async (_token, received) => {
    await Promise.resolve();
    calls.push(received);
  };
  h.bridge.complete = async (_token, reply) => {
    calls.push((reply as { type: string }).type);
    h.replies.push(reply);
  };
  const c = createController(h.bridge, updater, config, () => {}, h.timers);
  await c.start();
  h.enqueue({
    token: "download",
    effect: {
      type: "download",
      candidate: { version: "1.1.0", kind: "shell" },
    },
  });
  await settle();
  expect(calls).toEqual(["1", "2", "downloaded"]);
  updater.installAndRestart = async () => {
    throw new Error("disk full");
  };
  h.enqueue({
    token: "install",
    effect: { type: "install", candidate: { version: "1.1.0", kind: "shell" } },
  });
  await settle();
  expect(h.replies[1]).toEqual({
    type: "failed",
    failure: "disk",
    detail: "Error: disk full",
  });
  c.detach();
});

test("updates controller schedules periodic checks after the startup delay without restarting native work", async () => {
  const h = harness();
  const commands: object[] = [];
  const dispatch = h.bridge.dispatch;
  h.bridge.dispatch = async (command) => {
    commands.push(command);
    return dispatch(command);
  };
  const c = createController(h.bridge, native(), config, () => {}, h.timers);
  await c.start();
  const [id, task] = [...h.tasks.entries()][0];
  h.tasks.delete(id);
  task.run();
  await settle();
  expect(commands).toEqual([{ type: "check", background: true }]);
  expect([...h.tasks.values()].map((task) => task.ms)).toEqual([
    config.checkIntervalMs,
  ]);
  c.detach();
});

test("updates controller preserves a wakeup racing with an empty claim", async () => {
  const h = harness();
  const claim = h.bridge.claim;
  let first = true;
  let empty!: (value: null) => void;
  h.bridge.claim = (lease) => {
    if (first) {
      first = false;
      return new Promise((resolve) => {
        empty = resolve;
      });
    }
    return claim(lease);
  };
  const c = createController(h.bridge, native(), config, () => {}, h.timers);
  await c.start();
  await settle();
  h.enqueue({ token: "wake", effect: { type: "notify" } });
  empty(null);
  await settle();
  expect(h.replies).toEqual([{ type: "notified" }]);
  c.detach();
});

test("updates controller refuses a package requiring a shell that the native check did not find", async () => {
  const updater = native();
  updater.checkForPackage = async () => ({ version: "1.1.0", build: 3 });
  for (
    const app of [null, {
      version: "1.0.1",
      canInstall: true,
      url: "https://example.test/update",
    }]
  ) {
    updater.checkForUpdate = async () => app;
    let failed = false;
    try {
      await check(updater, shell);
    } catch {
      failed = true;
    }
    expect(failed).toBe(true);
  }
  updater.checkForUpdate = async () => ({
    version: "1.1.0",
    canInstall: true,
    url: "https://example.test/update",
  });
  expect((await check(updater, shell)).offer?.version).toBe("1.1.0");
});

// These messages come from the pinned NSH updater's real archive validation path.
test("updates controller classifies archive identity refusal before generic installation errors", () => {
  expect(
    classifyFailure(
      new Error(
        'update install: the archive holds Some("foreign.app"), this app is "expected.app"',
      ),
    ),
  ).toBe("rejected");
  expect(
    classifyFailure(
      new Error("update install: its code signature is not valid"),
    ),
  ).toBe("rejected");
  expect(
    classifyFailure(
      new Error("update install: Permission denied (os error 13)"),
    ),
  ).toBe("disk");
});
