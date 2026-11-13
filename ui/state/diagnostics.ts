/** Projects the process diagnostic status independently of persisted preferences. */
import { createProjection } from "./projection";
import { nshRuntime, onDispose } from "./runtime/nsh";
import { createDiagnosticTransport } from "./transport";
export type DiagnosticData = { problem?: string | null; dropped: string };
export function createDiagnosticState() {
  const owner = createProjection<DiagnosticData, never>({ problem: null, dropped: "0" }, createDiagnosticTransport(), nshRuntime);
  onDispose(() => owner.dispose()); void owner.connect();
  return owner.view;
}
