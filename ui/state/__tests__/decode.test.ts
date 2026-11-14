/** Verifies domain schemas and fail-closed decoding across all settings ingress paths. */
import { expect, test } from "nsh/test";
import { decodeSettings, decodeSettingsEnvelope } from "../settings/decode";
import { decodeDiagnosticEnvelope } from "../diagnostics/decode";
import { subscribeWindow } from "../window/stream";
import { decodeWindowEnvelope, decodeReceipt, decodeNavigation } from "../window/decode";
import { decodeUpdateEnvelope } from "../updates/decode";
import { initialSettings } from "../settings/initial";
import { createSettingsTransport } from "../settings/transport";
import { createProjection } from "../../state-runtime/projection";
import { plainRuntime } from "./support/controlled";
import { resolveTheme } from "../../theme";
import bundled from "../appearance/bundled.generated.json";

const valid = () => ({ ...initialSettings(), styleThemes: JSON.parse(JSON.stringify(bundled.themes)) });
const wire = (value: unknown, sequence = "1") => ({ storeId: "1", sequence, value });

test("snapshot decode validates every settings slice and resolved theme table", () => {
  expect(decodeSettings(valid()).colorMode).toBe("system");
  for (const patch of [
    { colorMode: "night" }, { localePreference: "xx" }, { componentFamily: "other" },
    { diagnosticLevel: "verbose" }, { saved: "yes" }, { revision: -1 },
    { backgroundImageOpacity: 101 }, { accent: "blue" }, { backgroundImage: { id: 5, name: "x" } },
    { styleThemes: [{ id: "broken" }] }, { customThemes: [{ id: "custom", name: "x", light: {} }] },
  ]) expect(() => decodeSettings({ ...valid(), ...patch })).toThrow("state_invalid_snapshot");
  const unresolved = valid();
  unresolved.styleThemes[0].cssVars.light.primary = "var(--foreground)";
  expect(() => decodeSettings(unresolved)).toThrow("state_invalid_snapshot");
  expect(() => decodeSettingsEnvelope(wire(valid(), "01"))).toThrow("state_invalid_snapshot");
});

test("snapshot decode rejects malformed diagnostic window and update payloads", () => {
  expect(decodeDiagnosticEnvelope(wire({ dropped: "9007199254740993", problem: null })).value.dropped).toBe("9007199254740993");
  expect(() => decodeDiagnosticEnvelope(wire({ dropped: 2 }))).toThrow("state_invalid_snapshot");
  expect(() => decodeWindowEnvelope(wire(JSON.stringify({ history: { entries: [], cursor: 0 }, session: {} })))).toThrow("state_invalid_snapshot");
  expect(() => decodeUpdateEnvelope(wire("{"))).toThrow("state_invalid_snapshot");
  const update = { domain: null, info: {}, pending: "0", execution: "available" };
  expect(decodeUpdateEnvelope(wire(JSON.stringify(update))).value.domain).toBe(null);
  expect(() => decodeUpdateEnvelope(wire(JSON.stringify({ ...update, domain: {
    shell: "1", running: "1", capability: "in_app", auto_check: true,
    phase: { type: "downloading", candidate: { kind: "package", version: "2" }, downloaded: "bad" },
  } })))).toThrow("state_invalid_snapshot");
});

for (const entry of ["subscription", "read", "command"] as const) {
  test(`snapshot decode ${entry} rejects bad data without replacing the last good snapshot`, async () => {
    let receive!: (event: unknown) => void;
    let next: unknown = wire(valid());
    let stops = 0;
    const transport = createSettingsTransport({ snapshot: async () => next, subscribe: handler => {
      receive = handler; handler({ snapshot: next }); return () => { stops++; };
    } }, async () => {});
    const owner = createProjection(initialSettings(), transport, plainRuntime);
    await owner.connect();
    expect(owner.view.status()).toBe("live");
    next = wire({ ...valid(), colorMode: "invalid" }, "2");
    if (entry === "subscription") receive({ snapshot: next });
    else {
      const result = entry === "read" ? owner.view.refresh() : owner.view.dispatch({ type: "colorMode", value: "dark" });
      expect(await result.then(() => false, () => true)).toBe(true);
    }
    expect(owner.view.status()).toBe("error");
    expect(owner.view.value().colorMode).toBe("system");
    expect(stops).toBe(1);
    receive({ snapshot: wire({ ...valid(), colorMode: "dark" }, "3") });
    expect(owner.view.value().colorMode).toBe("system");
    owner.dispose();
  });
}

test("snapshot decode leaves command rejection recoverable and orders valid callbacks", async () => {
  let receive!: (event: unknown) => void;
  const initial = wire(valid());
  const owner = createProjection(initialSettings(), createSettingsTransport({
    snapshot: async () => initial,
    subscribe: handler => { receive = handler; handler({ snapshot: initial }); return () => {}; },
  }, async () => { throw new Error("invalid_accent"); }), plainRuntime);
  await owner.connect();
  expect(await owner.view.dispatch({ type: "accent", value: null }).then(() => false, () => true)).toBe(true);
  expect(owner.view.status()).toBe("live");
  receive({ snapshot: wire({ ...valid(), colorMode: "dark" }, "3") });
  receive({ snapshot: wire({ ...valid(), colorMode: "light" }, "2") });
  expect(owner.view.value().colorMode).toBe("dark");
  owner.dispose();
});

const windowValue = (identity = "catalog") => ({
  history: { entries: [{ key: "1", location: { id: "home", page: "home", path: "/", params: {}, query: {}, ancestors: [] } }], cursor: 0, catalogIdentity: identity },
  pending: null, navigationResult: "committed",
  session: { section: "general", clicks: 0, operations: {} },
});

test("snapshot decode window streams end once and reject mixed messages and obsolete catalogs", () => {
  for (const failure of ["mixed", "falsyEnd", "catalog", "malformed"] as const) {
    let receive!: (event: unknown) => void;
    let snapshots = 0;
    const endings: string[] = [];
    const stop = subscribeWindow(handler => { receive = handler; return () => {}; }, {
      snapshot: () => { snapshots++; }, end: reason => { endings.push(reason); },
    }, "catalog");
    const event = (identity = "catalog") => ({ snapshot: wire(JSON.stringify(windowValue(identity))) });
    receive(event());
    if (failure === "mixed") receive({ ...event(), end: "closed" });
    else if (failure === "falsyEnd") receive({ ...event(), end: "" });
    else if (failure === "catalog") receive(event("obsolete"));
    else receive({ snapshot: wire("{") });
    receive(event()); receive({ end: "closed" });
    expect(snapshots).toBe(1);
    expect(endings.length).toBe(1);
    expect(endings[0]).toBe(failure === "catalog" ? "cancelled" : "error");
    stop();
  }
});

test("snapshot decode validates window receipts previews and update discriminants", () => {
  for (const ticket of ["", "0", "01", "-1", "x"]) {
    expect(() => decodeReceipt(JSON.stringify({ kind: "ticket", ticket }))).toThrow("state_invalid_snapshot");
  }
  expect(decodeReceipt('{"kind":"ticket","ticket":"9007199254740993"}').kind).toBe("ticket");
  expect(() => decodeNavigation("{")).toThrow("state_invalid_snapshot");
  const state = windowValue();
  expect(() => decodeWindowEnvelope(wire(JSON.stringify(state)), "obsolete")).toThrow("state_invalid_snapshot");
  expect(() => decodeWindowEnvelope(wire(JSON.stringify({ ...state, session: { ...state.session, opacityPreview: 101 } })))).toThrow("state_invalid_snapshot");
  const update = { domain: { shell: "1", running: "1", capability: "in_app", auto_check: true,
    phase: { type: "idle", outcome: { type: "package_update", candidate: { version: "2", kind: "package" } } },
  }, info: {}, pending: "0", execution: "available" };
  expect(() => decodeUpdateEnvelope(wire(JSON.stringify(update)))).toThrow("state_invalid_snapshot");
});

test("snapshot decode excludes legacy diagnostic replicas from settings", () => {
  const preferences = decodeSettings({ ...valid(), diagnosticDropped: 12, diagnosticProblem: "obsolete" });
  expect("diagnosticDropped" in preferences).toBe(false);
  expect("diagnosticProblem" in preferences).toBe(false);
});

test("snapshot decode touch rendering consumes resolved scheme tables without rederiving colors", () => {
  const style = valid().styleThemes[0];
  style.cssVars.touch.primary = "var(--background)";
  style.cssVars.touchLight.primary = "#ffff00";
  style.cssVars.touchLight["primary-foreground"] = "#000000";
  style.cssVars.touchDark.primary = "#102040";
  style.cssVars.touchDark["primary-foreground"] = "#ffffff";
  const theme = decodeSettings({ ...valid(), styleThemes: [style] }).styleThemes[0];
  expect(resolveTheme(theme, "light", { touch: true }).accent).toBe("#ffff00");
  expect(resolveTheme(theme, "dark", { touch: true }).accentText).toBe("#ffffff");
  expect(resolveTheme(theme, "light", { touch: false }).accent).toBe(style.cssVars.light.primary);
});
