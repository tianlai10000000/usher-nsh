//! `usher-update-tool`: the release side of Usher's updater.
//!
//! ```text
//! usher-update-tool keygen  [--out DIR] [--add-to-keyset] [--recovery]
//! usher-update-tool sign    DIR --version V [--channel stable|preview] [--min-version V]
//!                           [--revoke-key ID]... [--key FILE | --key-env NAME] [--keys FILE]
//! usher-update-tool verify  DIR [--keys FILE] [--channel C] [--revoked ID]...
//! usher-update-tool pubkeys [--keys FILE]
//! ```
//!
//! No framework is used for arguments: the tool has five commands, and a hand-written
//! parser keeps the release pipeline free of a heavy dependency and of surprises.

mod keys;
mod release;

use std::{path::PathBuf, process::ExitCode};

const USAGE: &str = "\
usher-update-tool: key generation and release signing for Usher updates

USAGE:
  usher-update-tool keygen  [--out DIR] [--add-to-keyset] [--recovery]
      Generate an Ed25519 signing key pair. The secret goes to DIR (default
      ~/.usher-nsh-update-signing, mode 0600, never inside the repository, never
      overwritten). With --add-to-keyset the public key is appended to the client's
      trusted_keys.txt.
      With --recovery the key is a recovery key: the only kind that may revoke other
      keys. Keep it offline and use it only to retire a leaked release key.

  usher-update-tool sign    DIR --version V [--channel stable|preview] [--min-version V]
                            [--revoke-key ID]... [--key FILE | --key-env NAME] [--keys FILE]
      Write DIR/SHA256SUMS and DIR/SHA256SUMS.sig for the release files in DIR, then verify
      them as the client would. The secret comes from --key, or from the environment
      variable --key-env (CI), or from the default key file. Only a recovery key (see
      keygen --recovery) may use --revoke-key; --keys names the key list that says which
      key is which (default: the one the next build carries).

  usher-update-tool verify  DIR [--keys FILE] [--channel C] [--revoked ID]...
      Check DIR/SHA256SUMS against its signature and the files on disk.

  usher-update-tool pubkeys [--keys FILE]
      List the trusted public keys with their ids.
";

fn main() -> ExitCode {
    let args: Vec<String> = std::env::args().skip(1).collect();
    match run(&args) {
        Ok(output) => {
            print!("{output}");
            ExitCode::SUCCESS
        }
        Err(Failure::Usage(message)) => {
            eprintln!("error: {message}\n\n{USAGE}");
            ExitCode::from(2)
        }
        Err(Failure::Other(message)) => {
            eprintln!("error: {message}");
            ExitCode::FAILURE
        }
    }
}

#[derive(Debug)]
pub enum Failure {
    Usage(String),
    Other(String),
}

impl<E: std::fmt::Display> From<E> for Failure {
    fn from(error: E) -> Self {
        Self::Other(error.to_string())
    }
}

fn usage<T>(message: impl Into<String>) -> Result<T, Failure> {
    Err(Failure::Usage(message.into()))
}

/// The repository this tool lives in: where secrets must never be written.
fn repository_root() -> PathBuf {
    let manifest = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    // crates/usher-update-tool -> the repository, without a `..` in printed paths.
    manifest
        .parent()
        .and_then(|crates| crates.parent())
        .map_or(manifest.clone(), PathBuf::from)
}

pub fn keyset_file() -> PathBuf {
    repository_root().join("crates/usher-update/trusted_keys.txt")
}

/// A simple `--flag value` / `--flag` reader. Unknown flags are errors, never ignored.
struct Flags {
    positional: Vec<String>,
    values: Vec<(String, String)>,
    switches: Vec<String>,
}

impl Flags {
    fn parse(args: &[String], with_value: &[&str], switches: &[&str]) -> Result<Self, Failure> {
        let mut flags = Flags {
            positional: vec![],
            values: vec![],
            switches: vec![],
        };
        let mut index = 0;
        while index < args.len() {
            let arg = &args[index];
            if let Some(name) = arg.strip_prefix("--") {
                if with_value.contains(&name) {
                    let Some(value) = args.get(index + 1) else {
                        return usage(format!("--{name} needs a value"));
                    };
                    flags.values.push((name.to_owned(), value.clone()));
                    index += 2;
                    continue;
                }
                if switches.contains(&name) {
                    flags.switches.push(name.to_owned());
                    index += 1;
                    continue;
                }
                return usage(format!("unknown option --{name}"));
            }
            flags.positional.push(arg.clone());
            index += 1;
        }
        Ok(flags)
    }

    fn one(&self, name: &str) -> Result<Option<&str>, Failure> {
        let mut found = self
            .values
            .iter()
            .filter(|(n, _)| n == name)
            .map(|(_, v)| v.as_str());
        let first = found.next();
        if found.next().is_some() {
            return usage(format!("--{name} was given more than once"));
        }
        Ok(first)
    }

    fn all(&self, name: &str) -> Vec<String> {
        self.values
            .iter()
            .filter(|(n, _)| n == name)
            .map(|(_, v)| v.clone())
            .collect()
    }

    fn switch(&self, name: &str) -> bool {
        self.switches.iter().any(|s| s == name)
    }
}

pub fn run(args: &[String]) -> Result<String, Failure> {
    let Some((command, rest)) = args.split_first() else {
        return usage("no command given");
    };
    match command.as_str() {
        "keygen" => release::keygen(&Flags::parse(
            rest,
            &["out"],
            &["add-to-keyset", "recovery"],
        )?),
        "sign" => release::sign(&Flags::parse(
            rest,
            &[
                "version",
                "channel",
                "min-version",
                "revoke-key",
                "key",
                "key-env",
                "keys",
            ],
            &[],
        )?),
        "verify" => release::verify(&Flags::parse(rest, &["keys", "channel", "revoked"], &[])?),
        "pubkeys" => release::pubkeys(&Flags::parse(rest, &["keys"], &[])?),
        "-h" | "--help" | "help" => Ok(USAGE.to_owned()),
        other => usage(format!("unknown command {other}")),
    }
}
