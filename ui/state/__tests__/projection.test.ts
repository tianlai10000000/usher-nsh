/** Tests public projection semantics with delayed, reordered and failing transport operations. */
import { expect, test } from "nsh/test";
import { compareSequence, createProjection } from "../projection";
import { controlled, envelope, initial, plainRuntime } from "./support/controlled";

function setup() {
  const fake = controlled();
  const owner = createProjection(initial(), fake.transport, plainRuntime);
  return { fake, owner };
}

test("state sequence compares decimal boundaries without JavaScript number rounding", () => {
  for (const [older, newer] of [["9", "10"], ["99", "100"], ["9007199254740992", "9007199254740993"], ["18446744073709551614", "18446744073709551615"]]) {
    expect(compareSequence(older, newer)).toBe(-1);
    expect(compareSequence(newer, older)).toBe(1);
    expect(compareSequence(newer, newer)).toBe(0);
  }
  for (const invalid of ["01", "-1", "1.0", "", " 1", "1e3"]) {
    expect(() => compareSequence(invalid, "1")).toThrow("state_invalid_sequence");
  }
});

test("state callbacks and command replies share monotonic acceptance", async () => {
  const { fake, owner } = setup();
  const connecting = owner.connect();
  fake.observers[0].snapshot(envelope("9", 9));
  fake.finish(0); await connecting;
  const command = owner.view.dispatch(10);
  fake.observers[0].snapshot(envelope("11", 11));
  fake.replies[0].resolve(envelope("10", 10));
  await command;
  fake.observers[0].snapshot(envelope("9", 9));
  fake.observers[0].snapshot(envelope("11", 999));
  expect(owner.view.value().count).toBe(11);
  expect(owner.view.status()).toBe("live");
  const unsaved = envelope("12", 12); unsaved.value.saved = false;
  fake.observers[0].snapshot(unsaved);
  fake.observers[0].snapshot(envelope("13", 12));
  expect(owner.view.value().revision).toBe(1);
  expect(owner.view.value().saved).toBe(true);
  owner.dispose();
});

test("state disposal closes delayed registrations and ignores their callbacks", async () => {
  const { fake, owner } = setup();
  const connecting = owner.connect();
  owner.dispose(); owner.dispose();
  fake.observers[0].snapshot(envelope("1", 1));
  fake.finish(0); await connecting;
  expect(fake.stops()).toBe(1);
  expect(owner.view.value().count).toBe(0);
  expect(owner.view.status()).toBe("disposed");
  await owner.connect();
  expect(fake.registrations.length).toBe(1);
  await expect(owner.view.refresh()).rejects.toThrow("state_binding_closed");
});

test("state reconnect rejects old bindings and foreign instances", async () => {
  const { fake, owner } = setup();
  const first = owner.connect(); fake.finish(0); await first;
  fake.observers[0].snapshot(envelope("999", 1, "1"));
  const second = owner.connect();
  fake.observers[1].snapshot(envelope("0", 2, "2"));
  fake.observers[0].snapshot(envelope("1000", 3, "1"));
  fake.observers[0].end("error");
  fake.observers[1].snapshot(envelope("1001", 4, "1"));
  fake.finish(1); await second;
  expect(owner.view.value().count).toBe(2);
  expect(owner.view.status()).toBe("live");
  expect(fake.stops()).toBe(1);
  owner.dispose(); expect(fake.stops()).toBe(2);
});

test("state terminal events preserve final data and close late registration handles", async () => {
  const { fake, owner } = setup();
  const connecting = owner.connect();
  fake.observers[0].snapshot(envelope("1", 1));
  fake.observers[0].end("closed");
  fake.observers[0].snapshot(envelope("2", 2));
  fake.finish(0); await connecting;
  expect(owner.view.value().count).toBe(1);
  expect(owner.view.status()).toBe("closed");
  expect(fake.stops()).toBe(1);
  owner.dispose(); expect(fake.stops()).toBe(1);
});

test("state registration errors require explicit reconnect and retain last data", async () => {
  const { fake, owner } = setup();
  const first = owner.connect(); fake.finish(0); await first;
  fake.observers[0].snapshot(envelope("1", 7));
  const retry = owner.connect();
  fake.registrations[1].reject(new Error("injected registration failure")); await retry;
  expect(owner.view.status()).toBe("error");
  expect(owner.view.value().count).toBe(7);
  expect(fake.registrations.length).toBe(2);
  const recovery = owner.connect(); fake.finish(2); await recovery;
  fake.observers[2].snapshot(envelope("0", 8, "2"));
  expect(owner.view.value().count).toBe(8);
  owner.dispose();
});

test("state pending command replies cannot revive a disposed projection", async () => {
  const { fake, owner } = setup();
  const connecting = owner.connect(); fake.finish(0); await connecting;
  const reply = owner.view.dispatch(1);
  const checked = expect(reply).rejects.toThrow("state_binding_closed");
  owner.dispose(); fake.replies[0].resolve(envelope("1", 1));
  await checked;
  expect(owner.view.value().count).toBe(0);
  expect(owner.view.status()).toBe("disposed");
});

test("state malformed notifications fail closed without replacing the last value", async () => {
  const { fake, owner } = setup();
  const connecting = owner.connect(); fake.finish(0); await connecting;
  fake.observers[0].snapshot(envelope("1", 1));
  fake.observers[0].snapshot(envelope("02", 2));
  fake.observers[0].snapshot(envelope("3", 3));
  expect(owner.view.status()).toBe("error");
  expect(owner.view.value().count).toBe(1);
  expect(fake.stops()).toBe(1);
  owner.dispose();
});

test("state borrowed views are immutable and selectors remain transport independent", async () => {
  const { fake, owner } = setup();
  const connecting = owner.connect(); fake.finish(0); await connecting;
  const selected = owner.view.select(value => value.nested.label);
  const incoming = envelope("1", 1);
  Object.freeze(incoming.value);
  fake.observers[0].snapshot(incoming);
  expect(selected()).toBe("1");
  expect(Object.isFrozen(owner.view.value().nested)).toBe(true);
  expect("dispose" in owner.view).toBe(false);
  expect("set" in owner.view.value).toBe(false);
  fake.observers[0].snapshot(envelope("2", 2));
  expect(selected()).toBe("2");
  owner.dispose();
});

test("state explicit read replies cannot replace a more recent callback", async () => {
  const { fake, owner } = setup();
  const connected = owner.connect(); fake.finish(0); await connected;
  fake.observers[0].snapshot(envelope("9", 9));
  const read = owner.view.refresh();
  fake.observers[0].snapshot(envelope("10", 10));
  fake.replies[0].resolve(envelope("9", 9)); await read;
  expect(owner.view.value().count).toBe(10);
  owner.dispose();
});

test("state malformed read replies close synchronization while domain command errors leave it live", async () => {
  const { fake, owner } = setup();
  const connected = owner.connect(); fake.finish(0); await connected;
  fake.observers[0].snapshot(envelope("1", 1));
  const command = owner.view.dispatch(2);
  const rejected = expect(command).rejects.toThrow("invalid_locale");
  fake.replies[0].reject(new Error("invalid_locale")); await rejected;
  expect(owner.view.status()).toBe("live");
  const read = owner.view.refresh();
  const malformed = expect(read).rejects.toThrow("state_invalid_envelope");
  fake.replies[1].resolve(envelope("01", 3)); await malformed;
  expect(owner.view.status()).toBe("error");
  expect(owner.view.value().count).toBe(1);
  expect(fake.stops()).toBe(1);
  owner.dispose();
});
