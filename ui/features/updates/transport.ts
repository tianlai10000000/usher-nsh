/** Adapts the registered Rust update bridge to the project's ordered state facade. */
import { UpdateState } from "usher/nsh";
import type { Bridge, Claim, UpdateData } from "./contracts";
import type { Envelope } from "../../state/contracts";
function decode(
  value: { storeId: string; sequence: string; value: string },
): Envelope<UpdateData> {
  const state: UpdateData = JSON.parse(value.value);
  if (
    !state || !("domain" in state) || !state.info ||
    typeof state.pending !== "string"
  ) throw new Error("updates_invalid_snapshot");
  return { ...value, value: state };
}
export const bridge: Bridge = {
  read: async () => decode(await UpdateState.snapshot()),
  configure: async (value) =>
    decode(await UpdateState.configure(JSON.stringify(value))),
  dispatch: async (value) =>
    decode(await UpdateState.request(JSON.stringify(value))),
  subscribe(observer) {
    let active = true;
    const stop = UpdateState.subscribe((event) => {
      if (!active) return;
      try {
        if (event.snapshot && !event.end) {
          observer.snapshot(decode(event.snapshot));
        } else {observer.end(
            event.end === "closed" || event.end === "cancelled"
              ? event.end
              : "error",
          );}
      } catch {
        observer.end("error");
      }
    });
    return () => {
      active = false;
      stop();
    };
  },
  acquire: () => UpdateState.acquire(),
  claim: async (lease) => {
    const value = await UpdateState.claim(lease);
    return value ? JSON.parse(value) as Claim : null;
  },
  progress: (token, received, total) =>
    UpdateState.progress(token, received, total),
  complete: (token, value) =>
    UpdateState.complete(token, JSON.stringify(value)),
};
