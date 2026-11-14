/** Isolated settings, session and owner doubles for business-module lifecycle tests. */
import type { SettingsCommand, SettingsData } from "../../settings/contracts";
import type { WindowData, SessionData, OperationKind } from "../../window/contracts";
import type { WindowSession } from "../../window/index";
import type { SyncStatus } from "../../../state-runtime/contracts";
import { initialSettings } from "../../settings/initial";
import { createSettingsOperations, type OperationLifecycle, type SettingsView } from "../../settings/operations";
import { deferred } from "./controlled";

export function businessModules() {
  let data: SettingsData = initialSettings();
  let synchronization: SyncStatus = "live";
  let serial = 0;
  let reconnects = 0;
  const cleanups: (() => void)[] = [];
  const observers: (() => void)[] = [];
  const commands: SettingsCommand[] = [];
  const finishes: { operation: OperationKind; ticket: string; stage: string }[] = [];
  const previews: { opacity: boolean; value: number | null; expected?: string }[] = [];
  const exports: string[][] = [];
  const sessionData: SessionData = { section: "general", clicks: 0, operations: {} };
  const windowData: WindowData = {
    history: { entries: [], cursor: 0, catalogIdentity: "mock" },
    session: sessionData, pending: null, navigationResult: "committed",
  };
  let beginGate: ReturnType<typeof deferred<void>> | undefined;
  let dispatchGate: ReturnType<typeof deferred<void>> | undefined;
  let previewTicket: string | undefined;
  let exportGate: ReturnType<typeof deferred<void>> | undefined;
  const lifecycle: OperationLifecycle = {
    onDispose: run => { cleanups.push(run); },
    observe: run => { observers.push(run); run(); },
  };
  const settings: SettingsView = {
    value: () => data,
    status: () => synchronization,
    select: read => () => read(data),
    refresh: async () => data,
    dispatch: async command => {
      commands.push(command);
      await dispatchGate?.promise;
      return data;
    },
  };
  const session: WindowSession = {
    value: () => sessionData,
    change: async command => {
      if (command.kind === "background_problem") sessionData.backgroundProblem = command.value;
      if (command.kind === "observe_settings" && !sessionData.operations.settings?.active) {
        sessionData.operations.settings = { ticket: "observed", stage: command.stage, active: false, failed: false };
      }
      return windowData;
    },
    begin: async (operation, stage) => {
      const ticket = String(++serial);
      sessionData.operations[operation] = { ticket, stage, active: true, failed: false };
      await beginGate?.promise;
      return ticket;
    },
    finish: async (operation, ticket, stage, error, value, failed) => {
      finishes.push({ operation, ticket, stage });
      if (sessionData.operations[operation]?.ticket === ticket) {
        sessionData.operations[operation] = { ticket, stage, error, value, active: false, failed: failed ?? false };
      }
    },
    preview: async (opacity, value, expected) => {
      previews.push({ opacity, value, expected });
      if (expected !== undefined && expected !== previewTicket) return { kind: "superseded" };
      if (value === null) {
        if (opacity) sessionData.opacityPreview = null;
        else sessionData.transparencyPreview = null;
        return { kind: "applied" };
      }
      previewTicket = String(++serial);
      if (opacity) sessionData.opacityPreview = value;
      else sessionData.transparencyPreview = value;
      return { kind: "ticket", ticket: previewTicket };
    },
    exportDiagnostics: async (from, to) => {
      exports.push([from, to]);
      const ticket = String(++serial);
      sessionData.operations.export = { ticket, stage: "running", active: true, failed: false };
      await exportGate?.promise;
      sessionData.operations.export = { ticket, stage: "done", active: false, failed: false };
    },
  };
  const operations = createSettingsOperations(settings, session, async () => {
    reconnects += 1; synchronization = "live";
  }, lifecycle);
  return {
    settings, session, operations, lifecycle, commands, finishes, previews, exports, sessionData,
    reconnects: () => reconnects,
    setData: (value: Partial<SettingsData>) => { data = { ...data, ...value }; },
    setSynchronization: (value: SyncStatus) => { synchronization = value; },
    dispose: () => cleanups.forEach(run => run()),
    notify: () => observers.forEach(run => run()),
    delayBegin: () => beginGate = deferred<void>(),
    delayDispatch: () => dispatchGate = deferred<void>(),
    delayExport: () => exportGate = deferred<void>(),
  };
}
