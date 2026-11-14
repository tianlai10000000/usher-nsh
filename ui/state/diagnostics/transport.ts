/** Adapts process diagnostic status to the ordered projection contract. */
import { DiagnosticState } from "usher/nsh";
import { subscribeDecoded } from "../../state-runtime/decode";
import type { StateTransport } from "../../state-runtime/contracts";
import { decodeDiagnosticEnvelope } from "./decode";
/** Diagnostic updates have their own version stream; legacy settings DTO fields remain compatible. */
export function createDiagnosticTransport(): StateTransport<import("./health").DiagnosticData, never> {
  return {
    read: async () => decodeDiagnosticEnvelope(await DiagnosticState.snapshot()),
    dispatch: async () => { throw new Error("diagnostics_read_only"); },
    subscribe: observer => subscribeDecoded(receive => DiagnosticState.subscribe(receive), observer, decodeDiagnosticEnvelope),
  };
}
