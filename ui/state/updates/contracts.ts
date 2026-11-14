/** Transport and executor contracts; shared business data is projected from Rust. */
import type { Envelope, StateTransport } from "../../state-runtime/contracts";
export type Kind = "package" | "shell";
export type Candidate = { version: string; kind: Kind };
export type Failure =
  | "network"
  | "busy"
  | "rejected"
  | "disk"
  | "cancelled"
  | "unknown";
export type Outcome = {
  type:
    | "up_to_date"
    | "package_update"
    | "shell_update"
    | "manual_upgrade"
    | "system_managed"
    | "not_applicable"
    | "failed";
  candidate?: Candidate;
  failure?: Failure;
  reason?: string;
};
export type Phase = {
  type:
    | "idle"
    | "checking"
    | "checked"
    | "downloading"
    | "ready_to_install"
    | "installing"
    | "awaiting_first_launch"
    | "install_failed";
  candidate?: Candidate;
  outcome?: Outcome;
  failure?: Failure;
  downloaded?: string;
  total?: string | null;
};
export type UpdateData = {
  domain: {
    shell: string;
    running: string;
    capability: string;
    auto_check: boolean;
    phase: Phase;
  } | null;
  info: {
    manual_check?: boolean;
    shell_build?: string | null;
    package_build?: string | null;
    last_checked?: string | null;
    last_result?: string | null;
    manual_url?: string | null;
  };
  pending: string;
  execution: string;
};
export type Command = { type: "check"; background: boolean } | {
  type: "auto_check";
  enabled: boolean;
} | { type: "download" | "install" | "confirm_first_launch" };
export type Claim = {
  token: string;
  effect: {
    type: "check" | "download" | "install" | "notify";
    candidate?: Candidate;
  };
};
export interface Bridge extends StateTransport<UpdateData, Command> {
  configure(value: object): Promise<Envelope<UpdateData>>;
  acquire(): Promise<string>;
  claim(lease: string): Promise<Claim | null>;
  complete(token: string, value: object): Promise<void>;
  progress(
    token: string,
    received: string,
    total: string | null,
  ): Promise<void>;
}
export type Native = typeof import("nsh/updater");
export type Config = {
  startupDelayMs: number;
  checkIntervalMs: number;
  failureLimit: number;
};
export const initial: UpdateData = {
  domain: null,
  info: {},
  pending: "0",
  execution: "available",
};
