/** Typed application shortcut transport; no layout or NSH types enter the controller. */
import type { Envelope, Observer } from "../state-runtime/contracts";
export type Phase = "validating" | "loading" | "installing" | "ready" | "failed" | "notRequested" | "notLoaded" | "inactive" | "unsupported" | "recovering";
export type Health = {
  startup: Phase; page: Phase; global: Phase; entry: string | null;
  modules: Array<{ id: string; kind: "global" | "application" | "page"; page: string | null; phase: Phase; error: string | null }>;
  bindings: Array<{ id: string; module: string; keystroke: string; state: string }>;
};
export type Command = { module: "navigation"; action: "back" | "forward" } | { module: "window"; action: "focusMain" | "openSettings" } | { module: "appearance"; action: "toggleScheme" };
export type NativeEffect = { kind: "install" | "release"; ticket: string; binding: string; keystroke: string; editing: "auto" | "always" | "notEditing"; repeat: boolean; passThrough: boolean };
export type Request = { kind: "initialize" | "retry" } | { kind: "context"; focused: boolean; blocked: boolean }
  | { kind: "beginPage"; entry: string; page: string } | { kind: "finishPage" | "leavePage"; ticket: string }
  | { kind: "acknowledge"; ticket: string; success: boolean; release: boolean };
export type Reply = { snapshot: Envelope<Health>; effects: NativeEffect[]; ticket: string | null };
export type Transport = {
  initial: Envelope<Health>;
  read(): Promise<Envelope<Health>>;
  subscribe(observer: Observer<Health>): () => void;
  execute(request: Request): Reply;
  trigger(ticket: string): Command | null;
  close(): void;
};
export type InputBackend = { bind(effect: NativeEffect, run: () => void): () => void };
