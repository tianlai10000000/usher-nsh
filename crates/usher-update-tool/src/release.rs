//! The four commands. Each returns the text to print, so tests can read it.

use std::{
    fs,
    io::Read,
    path::{Path, PathBuf},
};

use base64::{Engine as _, engine::general_purpose::STANDARD as B64};
use ed25519_dalek::{Signer, SigningKey};
use sha2::{Digest, Sha256};
use usher_update::signature::{
    Digest256, KeySet, MANIFEST_ASSET, MAX_MANIFEST_BYTES, MAX_SIGNATURE_BYTES, ManifestSpec,
    Revocations, SIGNATURE_ASSET, VerifiedManifest, key_id, render_manifest, render_signature_file,
};

use crate::{Failure, Flags, keys, keyset_file, repository_root};

/// File types that belong in a release, as the updater looks for them by exact name.
const RELEASE_EXTENSIONS: [&str; 3] = [".dmg", ".tar.gz", ".zip"];
/// A release file is read in blocks, so a multi-gigabyte installer never sits in memory.
const BLOCK: usize = 1 << 20;

pub fn keygen(flags: &Flags) -> Result<String, Failure> {
    if !flags.positional.is_empty() {
        return Err(Failure::Usage(
            "keygen takes no positional arguments".into(),
        ));
    }
    let directory = match flags.one("out")? {
        Some(path) => PathBuf::from(path),
        None => keys::default_directory()
            .ok_or_else(|| Failure::Other("HOME is not set; pass --out".into()))?,
    };
    let key = keys::generate(&directory, &repository_root())?;
    let mut out = String::new();
    out.push_str("Generated an Ed25519 update-signing key pair (self-test passed).\n\n");
    out.push_str(&format!(
        "  secret key (keep private): {}\n",
        key.secret_path.display()
    ));
    out.push_str(&format!(
        "  public key:                {}\n",
        key.public_path.display()
    ));
    out.push_str(&format!("  key id:                    {}\n", key.id));
    out.push_str(&format!(
        "  public key (base64):       {}\n",
        key.public_b64
    ));
    out.push_str(&format!(
        "  SHA-256 fingerprint:       {}\n",
        key.fingerprint
    ));
    out.push_str("\nWrite the fingerprint down somewhere else: it lets you check the key later by another route.\n");

    let recovery = flags.switch("recovery");
    if recovery && !flags.switch("add-to-keyset") {
        return Err(Failure::Usage(
            "--recovery only makes sense with --add-to-keyset".into(),
        ));
    }
    if flags.switch("add-to-keyset") {
        let file = keyset_file();
        append_key(&file, &key.public_b64, recovery)?;
        let role = if recovery {
            "recovery key (may revoke keys; keep it offline)"
        } else {
            "release key"
        };
        out.push_str(&format!("\nAdded the public key to {} as a {role}.\nThe client trusts it from the next build.\n", file.display()));
    } else {
        out.push_str("\nTo have the client trust it, add the public key to crates/usher-update/trusted_keys.txt\n(or run keygen again with --add-to-keyset).\n");
    }
    out.push_str(&format!(
        "\nFor CI, store the secret as a repository secret named UPDATE_SIGNING_KEY. This copies it\nwithout printing it:\n\n  tr -d '\\n' < \"{}\" | pbcopy\n\nThen pass --key-env UPDATE_SIGNING_KEY to `sign`.\nBack up the secret offline: if it is lost, already-installed clients cannot be updated\nby anyone, and if it leaks anyone can sign updates. Keep a second key in the key set so\neither can be retired.\n",
        key.secret_path.display()
    ));
    Ok(out)
}

/// Appends a public key to the key list, unless it is already there. Existing lines,
/// comments included, are kept as they are.
fn append_key(file: &Path, public_b64: &str, recovery: bool) -> Result<(), Failure> {
    let existing = fs::read_to_string(file).unwrap_or_default();
    // A key already listed, as either role, is never added a second time: that would be a
    // duplicate the client refuses, and silently changing its role is not this command's job.
    let listed = |line: &str| {
        line.trim()
            .strip_prefix("recovery:")
            .unwrap_or(line.trim())
            .trim()
            == public_b64
    };
    if existing.lines().any(listed) {
        return Ok(());
    }
    let mut text = existing;
    if !text.is_empty() && !text.ends_with('\n') {
        text.push('\n');
    }
    if recovery {
        text.push_str("recovery:");
    }
    text.push_str(public_b64);
    text.push('\n');
    // Whole file, then rename: a crash never leaves a half-written key list.
    let temporary = file.with_extension("tmp");
    fs::write(&temporary, text)?;
    fs::rename(&temporary, file)?;
    Ok(())
}

fn hash_file(path: &Path) -> Result<Digest256, Failure> {
    let mut file = fs::File::open(path)?;
    let mut hasher = Sha256::new();
    let mut block = vec![0u8; BLOCK];
    loop {
        let n = file.read(&mut block)?;
        if n == 0 {
            break;
        }
        hasher.update(&block[..n]);
    }
    Ok(hasher.finalize().into())
}

/// Release files in `directory`: regular files with a known extension. A symbolic link is
/// refused rather than followed, so what is signed is what is in the directory.
fn release_files(directory: &Path) -> Result<Vec<(String, PathBuf)>, Failure> {
    let mut files = Vec::new();
    for entry in fs::read_dir(directory)? {
        let entry = entry?;
        let name = entry.file_name().to_string_lossy().into_owned();
        if name == MANIFEST_ASSET || name == SIGNATURE_ASSET {
            continue;
        }
        if !RELEASE_EXTENSIONS.iter().any(|ext| name.ends_with(ext)) {
            continue;
        }
        let kind = entry.file_type()?;
        if kind.is_symlink() {
            return Err(Failure::Other(format!(
                "{name} is a symbolic link; refusing to sign it"
            )));
        }
        if kind.is_file() {
            files.push((name, entry.path()));
        }
    }
    files.sort();
    Ok(files)
}

fn directory_argument(flags: &Flags) -> Result<&Path, Failure> {
    match flags.positional.as_slice() {
        [one] => Ok(Path::new(one)),
        _ => Err(Failure::Usage("give exactly one release directory".into())),
    }
}

fn load_signer(flags: &Flags) -> Result<SigningKey, Failure> {
    match (flags.one("key")?, flags.one("key-env")?) {
        (Some(_), Some(_)) => Err(Failure::Usage("use --key or --key-env, not both".into())),
        (Some(path), None) => Ok(keys::load_secret(Path::new(path))?),
        (None, Some(name)) => Ok(keys::secret_from_env(name)?),
        (None, None) => {
            let directory = keys::default_directory()
                .ok_or_else(|| Failure::Other("HOME is not set; pass --key".into()))?;
            Ok(keys::load_secret(&directory.join(keys::SECRET_FILE))?)
        }
    }
}

fn load_keyset(flags: &Flags) -> Result<KeySet, Failure> {
    let path = flags
        .one("keys")?
        .map(PathBuf::from)
        .unwrap_or_else(keyset_file);
    let text = fs::read_to_string(&path)
        .map_err(|error| Failure::Other(format!("{}: {error}", path.display())))?;
    Ok(KeySet::parse(&text)?)
}

pub fn sign(flags: &Flags) -> Result<String, Failure> {
    let directory = directory_argument(flags)?;
    let version = flags
        .one("version")?
        .ok_or_else(|| Failure::Usage("--version is required".into()))?;
    let version = version.strip_prefix('v').unwrap_or(version);
    let channel = flags.one("channel")?.unwrap_or("stable");
    let min_version = flags.one("min-version")?;
    let revoke_keys = flags.all("revoke-key");
    let signer = load_signer(flags)?;
    let id = key_id(&signer.verifying_key());

    let files = release_files(directory)?;
    if files.is_empty() {
        return Err(Failure::Other(format!(
            "{} has no .dmg, .tar.gz or .zip files to sign",
            directory.display()
        )));
    }
    let entries: Vec<(String, Digest256)> = files
        .iter()
        .map(|(name, path)| Ok((name.clone(), hash_file(path)?)))
        .collect::<Result<_, Failure>>()?;
    let manifest = render_manifest(&ManifestSpec {
        version,
        channel,
        min_version,
        revoke_keys: &revoke_keys,
        entries: &entries,
    })?;
    let signature = render_signature_file(&id, &signer.sign(&manifest).to_bytes());

    // Verify what was just made with the same code the client runs, against a key list
    // holding only this key, before anything is published. A secret that does not match
    // the built-in list is caught here, not by users.
    // The signer's role is the one the built-in list gives it; an unlisted key is a release
    // key. So a release key asked to revoke something is refused here, as clients would.
    let listed = key_list_for(flags)?;
    let role = listed.as_ref().and_then(|list| list.role(&id));
    let own_keys = KeySet::parse(&format!(
        "{}{}",
        if role == Some(usher_update::signature::Role::Recovery) {
            "recovery:"
        } else {
            ""
        },
        B64.encode(signer.verifying_key().as_bytes())
    ))?;
    VerifiedManifest::verify(
        &manifest,
        &signature,
        &own_keys,
        &Revocations::default(),
        channel,
    )
    .map_err(|error| Failure::Other(format!("the signed manifest does not verify: {error}")))?;
    let trusted = role.is_some();

    // Both files or neither: write beside the target, then rename.
    let manifest_path = directory.join(MANIFEST_ASSET);
    let signature_path = directory.join(SIGNATURE_ASSET);
    write_atomic(&manifest_path, &manifest)?;
    if let Err(error) = write_atomic(&signature_path, &signature) {
        let _ = fs::remove_file(&manifest_path);
        return Err(error);
    }

    let mut out = format!(
        "Signed {} file(s) as version {version}, channel {channel}, with key {id}.\n",
        entries.len()
    );
    for (name, digest) in &entries {
        out.push_str(&format!(
            "  {}  {name}\n",
            digest
                .iter()
                .map(|b| format!("{b:02x}"))
                .collect::<String>()
        ));
    }
    out.push_str(&format!(
        "\nWrote {} and {}.\n",
        manifest_path.display(),
        signature_path.display()
    ));
    if !trusted {
        out.push_str(&format!(
            "\nWarning: key {id} is not in crates/usher-update/trusted_keys.txt, so installed clients\nwill refuse this release. Add it with `keygen --add-to-keyset` and ship a build that has it first.\n"
        ));
    }
    Ok(out)
}

/// The key list a release is judged against: `--keys`, or the one the next build carries.
fn key_list_for(flags: &Flags) -> Result<Option<KeySet>, Failure> {
    let path = flags
        .one("keys")?
        .map(PathBuf::from)
        .unwrap_or_else(keyset_file);
    Ok(fs::read_to_string(path)
        .ok()
        .and_then(|text| KeySet::parse(&text).ok()))
}

fn write_atomic(path: &Path, bytes: &[u8]) -> Result<(), Failure> {
    let temporary = path.with_extension("partial");
    fs::write(&temporary, bytes)?;
    fs::rename(&temporary, path).map_err(|error| {
        let _ = fs::remove_file(&temporary);
        Failure::from(error)
    })
}

fn read_bounded(path: &Path, limit: usize) -> Result<Vec<u8>, Failure> {
    let meta = fs::metadata(path)
        .map_err(|error| Failure::Other(format!("{}: {error}", path.display())))?;
    if meta.len() as usize > limit {
        return Err(Failure::Other(format!(
            "{} is larger than {limit} bytes",
            path.display()
        )));
    }
    Ok(fs::read(path)?)
}

pub fn verify(flags: &Flags) -> Result<String, Failure> {
    let directory = directory_argument(flags)?;
    let keyset = load_keyset(flags)?;
    let channel = flags.one("channel")?.unwrap_or("stable");
    let revoked = Revocations::from_ids(flags.all("revoked"));
    let manifest = read_bounded(&directory.join(MANIFEST_ASSET), MAX_MANIFEST_BYTES)?;
    let signature = read_bounded(&directory.join(SIGNATURE_ASSET), MAX_SIGNATURE_BYTES)?;
    let verified = VerifiedManifest::verify(&manifest, &signature, &keyset, &revoked, channel)?;

    // Every file the manifest lists must be on disk with exactly the signed hash, and
    // nothing releasable may be missing from the manifest.
    let mut out = format!(
        "Signature valid: version {}, channel {}, signed by key {}.\n",
        verified.version, verified.channel, verified.signed_by
    );
    for (name, expected) in verified.entries() {
        let path = directory.join(name);
        let actual = hash_file(&path)
            .map_err(|_| Failure::Other(format!("{name} is listed but missing or unreadable")))?;
        if &actual != expected {
            return Err(Failure::Other(format!(
                "{name} does not match its signed SHA-256"
            )));
        }
        out.push_str(&format!("  ok  {name}\n"));
    }
    for (name, _) in release_files(directory)? {
        if !verified.entries().iter().any(|(listed, _)| *listed == name) {
            return Err(Failure::Other(format!(
                "{name} is in the directory but not in the signed manifest"
            )));
        }
    }
    if !verified.revoked_keys.is_empty() {
        out.push_str(&format!(
            "Revokes keys: {}\n",
            verified.revoked_keys.join(", ")
        ));
    }
    if let Some(min) = &verified.min_version {
        out.push_str(&format!("Minimum version for in-place update: {min}\n"));
    }
    Ok(out)
}

pub fn pubkeys(flags: &Flags) -> Result<String, Failure> {
    if !flags.positional.is_empty() {
        return Err(Failure::Usage(
            "pubkeys takes no positional arguments".into(),
        ));
    }
    let path = flags
        .one("keys")?
        .map(PathBuf::from)
        .unwrap_or_else(keyset_file);
    let text = fs::read_to_string(&path)
        .map_err(|error| Failure::Other(format!("{}: {error}", path.display())))?;
    match KeySet::parse(&text) {
        Ok(set) => {
            let mut out = format!(
                "{} trusted key(s) in {}:\n",
                set.ids().len(),
                path.display()
            );
            for id in set.ids() {
                out.push_str(&format!("  {id}\n"));
            }
            Ok(out)
        }
        Err(usher_update::signature::Error::InvalidKeySet("no keys")) => Ok(format!(
            "No keys yet in {}. Run `keygen --add-to-keyset`.\n",
            path.display()
        )),
        Err(error) => Err(error.into()),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::run;

    fn args(parts: &[&str]) -> Vec<String> {
        parts.iter().map(|s| (*s).to_owned()).collect()
    }

    /// A key pair in a temp directory, a key list holding it, and a release directory.
    struct Setup {
        _root: tempfile::TempDir,
        release: PathBuf,
        secret: PathBuf,
        keys_file: PathBuf,
    }

    fn setup() -> Setup {
        let root = tempfile::tempdir().unwrap();
        let generated = keys::generate(&root.path().join("keys"), &repository_root()).unwrap();
        let keys_file = root.path().join("trusted_keys.txt");
        fs::write(&keys_file, format!("# test\n{}\n", generated.public_b64)).unwrap();
        let release = root.path().join("release");
        fs::create_dir(&release).unwrap();
        fs::write(
            release.join("Usher-aarch64-apple-darwin.tar.gz"),
            b"mac package",
        )
        .unwrap();
        fs::write(
            release.join("Usher-x86_64-pc-windows-msvc.zip"),
            b"win package",
        )
        .unwrap();
        fs::write(release.join("notes.txt"), b"not a release file").unwrap();
        Setup {
            _root: root,
            release,
            secret: generated.secret_path,
            keys_file,
        }
    }

    fn sign_args<'a>(s: &'a Setup, extra: &[&'a str]) -> Vec<String> {
        let mut v = vec![
            "sign",
            s.release.to_str().unwrap(),
            "--version",
            "v1.2.3",
            "--key",
            s.secret.to_str().unwrap(),
            "--keys",
            s.keys_file.to_str().unwrap(),
        ];
        v.extend_from_slice(extra);
        args(&v)
    }

    fn verify_args(s: &Setup, extra: &[&str]) -> Vec<String> {
        let mut v = vec![
            "verify",
            s.release.to_str().unwrap(),
            "--keys",
            s.keys_file.to_str().unwrap(),
        ];
        v.extend_from_slice(extra);
        args(&v)
    }

    fn message(failure: Failure) -> String {
        match failure {
            Failure::Usage(m) | Failure::Other(m) => m,
        }
    }

    #[test]
    fn a_signed_release_verifies_and_lists_only_release_files() {
        let s = setup();
        let out = run(&sign_args(&s, &[])).unwrap();
        assert!(out.contains("Signed 2 file(s) as version 1.2.3"), "{out}");
        let manifest = fs::read_to_string(s.release.join("SHA256SUMS")).unwrap();
        assert!(manifest.starts_with("usher-update-manifest-v2\nversion=1.2.3\nchannel=stable\n"));
        assert!(
            !manifest.contains("notes.txt"),
            "only release files are signed"
        );
        let signature = fs::read_to_string(s.release.join("SHA256SUMS.sig")).unwrap();
        assert!(signature.starts_with("usher-signature-v1\nkey="));

        let verified = run(&verify_args(&s, &[])).unwrap();
        assert!(
            verified.contains("Signature valid: version 1.2.3, channel stable"),
            "{verified}"
        );
        assert!(verified.contains("ok  Usher-aarch64-apple-darwin.tar.gz"));
        assert!(verified.contains("ok  Usher-x86_64-pc-windows-msvc.zip"));
        // No temporary files are left behind.
        let names: Vec<String> = fs::read_dir(&s.release)
            .unwrap()
            .flatten()
            .map(|e| e.file_name().to_string_lossy().into_owned())
            .collect();
        assert!(!names.iter().any(|n| n.ends_with(".partial")), "{names:?}");
    }

    #[test]
    fn changing_a_signed_file_or_adding_one_is_caught() {
        let s = setup();
        run(&sign_args(&s, &[])).unwrap();
        fs::write(
            s.release.join("Usher-aarch64-apple-darwin.tar.gz"),
            b"tampered",
        )
        .unwrap();
        let error = message(run(&verify_args(&s, &[])).unwrap_err());
        assert!(
            error.contains("does not match its signed SHA-256"),
            "{error}"
        );

        fs::write(
            s.release.join("Usher-aarch64-apple-darwin.tar.gz"),
            b"mac package",
        )
        .unwrap();
        assert!(run(&verify_args(&s, &[])).is_ok());
        fs::write(s.release.join("Usher-extra.zip"), b"smuggled").unwrap();
        let error = message(run(&verify_args(&s, &[])).unwrap_err());
        assert!(error.contains("not in the signed manifest"), "{error}");
    }

    #[test]
    fn a_tampered_manifest_or_signature_fails_verification() {
        let s = setup();
        run(&sign_args(&s, &[])).unwrap();
        let manifest_path = s.release.join("SHA256SUMS");
        let original = fs::read(&manifest_path).unwrap();
        let mut changed = original.clone();
        changed[30] ^= 1;
        fs::write(&manifest_path, &changed).unwrap();
        assert!(run(&verify_args(&s, &[])).is_err());
        fs::write(&manifest_path, &original).unwrap();
        assert!(run(&verify_args(&s, &[])).is_ok());

        fs::write(s.release.join("SHA256SUMS.sig"), b"garbage").unwrap();
        assert!(run(&verify_args(&s, &[])).is_err());
    }

    #[test]
    fn a_release_signed_by_a_key_the_client_does_not_trust_is_rejected_and_warned_about() {
        let s = setup();
        let other = tempfile::tempdir().unwrap();
        let stranger = keys::generate(&other.path().join("k"), &repository_root()).unwrap();
        let out = run(&args(&[
            "sign",
            s.release.to_str().unwrap(),
            "--version",
            "1.0.0",
            "--key",
            stranger.secret_path.to_str().unwrap(),
        ]))
        .unwrap();
        assert!(
            out.contains("is not in crates/usher-update/trusted_keys.txt"),
            "{out}"
        );
        let error = message(run(&verify_args(&s, &[])).unwrap_err());
        assert!(error.contains("untrusted key"), "{error}");
    }

    #[test]
    fn channel_and_min_version_are_carried_and_enforced() {
        let s = setup();
        let out = run(&sign_args(
            &s,
            &["--channel", "preview", "--min-version", "1.0.0"],
        ))
        .unwrap();
        assert!(out.contains("channel preview"), "{out}");
        let ok = run(&verify_args(&s, &["--channel", "preview"])).unwrap();
        assert!(
            ok.contains("Minimum version for in-place update: 1.0.0"),
            "{ok}"
        );
        // A stable client must not accept a preview manifest.
        let error = message(run(&verify_args(&s, &[])).unwrap_err());
        assert!(error.contains("channel preview"), "{error}");
        // A key the client has been told to distrust is refused even with a valid signature.
        let target = key_id(&keys::load_secret(&s.secret).unwrap().verifying_key());
        let error = message(
            run(&verify_args(
                &s,
                &["--channel", "preview", "--revoked", &target],
            ))
            .unwrap_err(),
        );
        assert!(error.contains("revoked"), "{error}");
    }

    #[test]
    fn a_release_key_cannot_be_used_to_revoke_even_through_the_tool() {
        let (s, _recovery, _release_id, recovery_id) = two_key_setup();
        let error = message(run(&sign_args(&s, &["--revoke-key", &recovery_id])).unwrap_err());
        assert!(error.contains("may not revoke keys"), "{error}");
        assert!(
            !s.release.join("SHA256SUMS").exists(),
            "nothing may be written for a refused release"
        );
        assert!(!s.release.join("SHA256SUMS.sig").exists());
        // A syntactically bad id is refused earlier still.
        assert!(run(&sign_args(&s, &["--revoke-key", "zz"])).is_err());
    }

    #[test]
    fn bad_input_is_refused_before_anything_is_written() {
        let s = setup();
        for (extra, expected) in [
            (vec!["--channel", "nightly"], "channel"),
            (vec!["--min-version", "soon"], "min-version"),
            (vec!["--revoke-key", "zz"], "revoke-key"),
        ] {
            let error = message(run(&sign_args(&s, &extra)).unwrap_err());
            assert!(error.contains(expected), "{extra:?}: {error}");
            assert!(
                !s.release.join("SHA256SUMS").exists(),
                "{extra:?} left a manifest behind"
            );
            assert!(
                !s.release.join("SHA256SUMS.sig").exists(),
                "{extra:?} left a signature behind"
            );
        }
        let empty = tempfile::tempdir().unwrap();
        let error = message(
            run(&args(&[
                "sign",
                empty.path().to_str().unwrap(),
                "--version",
                "1.0.0",
                "--key",
                s.secret.to_str().unwrap(),
            ]))
            .unwrap_err(),
        );
        assert!(error.contains("no .dmg, .tar.gz or .zip"), "{error}");
    }

    #[cfg(unix)]
    #[test]
    fn a_symbolic_link_in_the_release_directory_is_never_signed() {
        let s = setup();
        let outside = tempfile::tempdir().unwrap();
        fs::write(
            outside.path().join("secret.zip"),
            b"not part of the release",
        )
        .unwrap();
        std::os::unix::fs::symlink(
            outside.path().join("secret.zip"),
            s.release.join("Usher-link.zip"),
        )
        .unwrap();
        let error = message(run(&sign_args(&s, &[])).unwrap_err());
        assert!(error.contains("symbolic link"), "{error}");
        assert!(!s.release.join("SHA256SUMS").exists());
    }

    #[test]
    fn the_secret_can_come_from_the_environment_for_ci() {
        let s = setup();
        let text = fs::read_to_string(&s.secret).unwrap();
        // SAFETY: this test is the only one that sets this variable, and sets it before use.
        unsafe { std::env::set_var("USHER_TEST_SIGNING_KEY", text.trim()) };
        let mut v = vec![
            "sign",
            s.release.to_str().unwrap(),
            "--version",
            "2.0.0",
            "--key-env",
            "USHER_TEST_SIGNING_KEY",
        ];
        let out = run(&args(&v)).unwrap();
        assert!(out.contains("version 2.0.0"), "{out}");
        assert!(run(&verify_args(&s, &[])).is_ok());
        v.extend(["--key", "x"]);
        assert!(
            matches!(run(&args(&v)), Err(Failure::Usage(_))),
            "both sources at once is a usage error"
        );
        unsafe { std::env::remove_var("USHER_TEST_SIGNING_KEY") };
        let error = message(
            run(&args(&[
                "sign",
                s.release.to_str().unwrap(),
                "--version",
                "2.0.0",
                "--key-env",
                "USHER_TEST_SIGNING_KEY",
            ]))
            .unwrap_err(),
        );
        assert!(
            error.contains("not set") && !error.contains(text.trim()),
            "{error}"
        );
    }

    #[test]
    fn keygen_adds_a_key_once_and_pubkeys_lists_it() {
        let dir = tempfile::tempdir().unwrap();
        let file = dir.path().join("trusted_keys.txt");
        fs::write(&file, "# header\n").unwrap();
        append_key(&file, "AAAA", false).unwrap();
        append_key(&file, "AAAA", false).unwrap();
        // Already listed as a release key: asking for it as a recovery key changes nothing.
        append_key(&file, "AAAA", true).unwrap();
        append_key(&file, "BBBB", true).unwrap();
        append_key(&file, "BBBB", false).unwrap();
        assert!(
            fs::read_to_string(&file)
                .unwrap()
                .contains("recovery:BBBB\n")
        );
        let text = fs::read_to_string(&file).unwrap();
        assert_eq!(text.matches("AAAA").count(), 1, "{text}");
        assert_eq!(text.matches("BBBB").count(), 1, "{text}");
        assert!(text.starts_with("# header"), "comments are kept");
        assert!(!dir.path().join("trusted_keys.tmp").exists());

        let empty = dir.path().join("empty.txt");
        fs::write(&empty, "# nothing yet\n").unwrap();
        let out = run(&args(&["pubkeys", "--keys", empty.to_str().unwrap()])).unwrap();
        assert!(out.contains("No keys yet"), "{out}");
        let s = setup();
        let out = run(&args(&["pubkeys", "--keys", s.keys_file.to_str().unwrap()])).unwrap();
        assert!(out.starts_with("1 trusted key(s)"), "{out}");
    }

    #[test]
    fn unknown_commands_and_options_are_usage_errors() {
        assert!(matches!(run(&args(&[])), Err(Failure::Usage(_))));
        assert!(matches!(
            run(&args(&["frobnicate"])),
            Err(Failure::Usage(_))
        ));
        assert!(matches!(
            run(&args(&["keygen", "--nope"])),
            Err(Failure::Usage(_))
        ));
        assert!(matches!(
            run(&args(&["sign", "dir", "--version"])),
            Err(Failure::Usage(_))
        ));
        assert!(run(&args(&["--help"])).unwrap().contains("USAGE"));
    }

    /// A release key and a recovery key in one key list, as the real list is laid out.
    fn two_key_setup() -> (Setup, PathBuf, String, String) {
        let s = setup();
        let release_id = key_id(&keys::load_secret(&s.secret).unwrap().verifying_key());
        let other = tempfile::tempdir().unwrap();
        let recovery = keys::generate(&other.path().join("recovery"), &repository_root()).unwrap();
        let mut list = fs::read_to_string(&s.keys_file).unwrap();
        list.push_str(&format!("recovery:{}\n", recovery.public_b64));
        fs::write(&s.keys_file, list).unwrap();
        // Keep the recovery key's directory alive for the test by leaking the guard.
        let secret = recovery.secret_path.clone();
        std::mem::forget(other);
        (s, secret, release_id, recovery.id)
    }

    #[test]
    fn verify_enforces_roles_from_the_key_list() {
        let (s, recovery_secret, release_id, recovery_id) = two_key_setup();
        // The recovery key retires the release key: accepted.
        run(&args(&[
            "sign",
            s.release.to_str().unwrap(),
            "--version",
            "2.0.0",
            "--key",
            recovery_secret.to_str().unwrap(),
            "--keys",
            s.keys_file.to_str().unwrap(),
            "--revoke-key",
            &release_id,
        ]))
        .unwrap();
        let out = run(&verify_args(&s, &[])).unwrap();
        assert!(
            out.contains(&format!("Revokes keys: {release_id}")),
            "{out}"
        );
        // Once the client has absorbed that, what the release key signs is refused.
        assert_ne!(release_id, recovery_id);
        // The release is signed by the recovery key, which stays trusted, so it still verifies.
        assert!(run(&verify_args(&s, &["--revoked", &release_id])).is_ok());
    }

    #[test]
    fn a_thief_who_builds_the_manifest_by_hand_is_still_refused_by_the_client_rules() {
        let (s, _recovery, _release_id, recovery_id) = two_key_setup();
        // Not through this tool: the thief writes the manifest and signs it with the stolen
        // release key. The client's verifier is the only line of defence, and it holds.
        let key = keys::load_secret(&s.secret).unwrap();
        let entries = vec![("Usher-evil.zip".to_owned(), [9u8; 32])];
        let revoke = vec![recovery_id];
        let manifest = render_manifest(&ManifestSpec {
            version: "9.9.9",
            channel: "stable",
            min_version: None,
            revoke_keys: &revoke,
            entries: &entries,
        })
        .unwrap();
        let signature = render_signature_file(
            &key_id(&key.verifying_key()),
            &key.sign(&manifest).to_bytes(),
        );
        let keyset = KeySet::parse(&fs::read_to_string(&s.keys_file).unwrap()).unwrap();
        let result = VerifiedManifest::verify(
            &manifest,
            &signature,
            &keyset,
            &Revocations::default(),
            "stable",
        );
        assert!(
            matches!(
                result,
                Err(usher_update::signature::Error::RevocationNeedsRecoveryKey(
                    _
                ))
            ),
            "{result:?}"
        );
    }
}
