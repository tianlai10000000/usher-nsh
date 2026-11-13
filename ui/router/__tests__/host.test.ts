/** Host lifecycle and URL trust contracts, independent of platform event delivery. */
import { expect, test } from "nsh/test";
import { bindHostRouting, linkTarget } from "../host";
import type { Navigation } from "../index";
import manifest from "../../../nsh.json";
import policy from "../../config/routing.json";
const allowed = { schemes: manifest.schemes, hosts: policy.hosts };
test("router host rejects foreign authorities and preserves raw paths for Rust validation", () => {
  for (const url of ["https://app/settings", "usher://other/settings", "usher://app:42/settings", "usher://user@app/settings", "usher://app/settings#secret", "usher://app\\settings", " usher://app/settings"]) expect(linkTarget(url, allowed)).toBe(null);
  expect(linkTarget("usher://app/settings", allowed)).toEqual({ kind: "path", location: "/settings" });
  expect(linkTarget("USHER://APP", allowed)).toEqual({ kind: "path", location: "/" });
  expect(linkTarget("usher://app/%2e%2e", allowed)).toEqual({ kind: "path", location: "/%2e%2e" });
});
test("router host queues launch links and fences disposal and inactive window back", async () => {
  let status: ReturnType<Navigation["status"]> = "connecting";
  let back = false, owns = true, stopped = 0, backs = 0;
  const paths: unknown[] = [];
  let open!: (url: string) => void, press!: () => boolean, ready!: () => void;
  const dispose = bindHostRouting({
    status: () => status, resolve: target => { if (target.kind === "path" && target.location === "/invalid") throw new Error("invalid_target"); return {} as ReturnType<Navigation["resolve"]>; },
    push: async target => { paths.push(target); return "committed"; },
    canGoBack: () => back, goBack: async () => { backs++; return "cancelled"; },
  }, {
    onOpen: callback => { open = callback; callback("usher://app/"); return () => { stopped++; }; },
    onBack: callback => { press = callback; return () => { stopped++; }; },
    observeReady: callback => { ready = callback; callback(); return () => { stopped++; }; },
    ownsBack: () => owns,
  }, allowed);
  open("usher://app/settings"); expect(paths.length).toBe(0);
  status = "live"; ready(); ready(); expect(paths).toEqual([{ kind: "path", location: "/settings" }]);
  open("usher://other/settings"); open("usher://app/invalid");
  expect(paths.length).toBe(1);
  expect(press()).toBe(false); back = true; owns = false; expect(press()).toBe(false);
  owns = true; expect(press()).toBe(true); expect(backs).toBe(1);
  dispose(); dispose(); open("usher://app/"); ready(); expect(press()).toBe(false);
  expect(stopped).toBe(3); expect(paths.length).toBe(1); expect(backs).toBe(1);
});
