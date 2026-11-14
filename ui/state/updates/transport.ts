/** Adapts the registered Rust update bridge to the project's ordered state facade. */
import { decodeUpdateEnvelope as decode, decodeClaim } from "./decode";
import { json, subscribeDecoded } from "../../state-runtime/decode";
import { UpdateState } from "usher/nsh";
import type { Bridge, Claim, UpdateData } from "./contracts";
import type { Envelope } from "../../state-runtime/contracts";
export const bridge: Bridge = {
  read: async () => decode(await UpdateState.snapshot()),
  configure: async (value) =>
    decode(await UpdateState.configure(JSON.stringify(value))),
  dispatch: async (value) =>
    decode(await UpdateState.request(JSON.stringify(value))),
  subscribe: observer => subscribeDecoded(receive => UpdateState.subscribe(receive), observer, decode),
  acquire: () => UpdateState.acquire(),
  claim: async (lease) => {
    const value = await UpdateState.claim(lease);
    return value ? decodeClaim(json(value)) : null;
  },
  progress: (token, received, total) =>
    UpdateState.progress(token, received, total),
  complete: (token, value) =>
    UpdateState.complete(token, JSON.stringify(value)),
};
