/** Fences window streams by catalog identity while reusing the common protocol decoder. */
import { subscribeDecoded } from "../../state-runtime/decode";
import type { Observer } from "../../state-runtime/contracts";
import type { WindowData } from "./contracts";
import { decodeWindowEnvelope } from "./decode";
export function subscribeWindow(
  subscribe: (receive: (event: unknown) => void) => () => void,
  observer: Observer<WindowData>,
  catalogIdentity: string,
) {
  let ended = false;
  return subscribeDecoded(subscribe, {
    snapshot(value) {
      if (ended) return;
      if (value.value.history.catalogIdentity !== catalogIdentity) {
        ended = true;
        observer.end("cancelled");
      } else observer.snapshot(value);
    },
    end(reason) {
      if (!ended) { ended = true; observer.end(reason); }
    },
  }, decodeWindowEnvelope);
}
