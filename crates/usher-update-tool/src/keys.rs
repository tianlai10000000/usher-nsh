//! Signing key files: generate, store privately, load.
//!
//! Choices worth stating, since a leaked or lost key is the worst failure an updater has:
//!
//! - the secret key is written with mode 0600 inside a 0700 directory, created in one
//!   step (no window where it is world-readable) and never overwritten;
//! - the directory may not be inside the repository, so `git add .` cannot pick it up;
//! - a fresh key is checked end to end before anything is published: sign, rebuild the
//!   public key from the stored 32 bytes, verify, and confirm a tampered message fails;
//! - nothing here prints the secret.

use std::{
    fs::{self, OpenOptions},
    io::{self, Write},
    path::{Path, PathBuf},
};

use base64::{Engine as _, engine::general_purpose::STANDARD as B64};
use ed25519_dalek::{Signature, Signer, SigningKey, VerifyingKey};
use usher_update::signature::key_id;

pub const SECRET_FILE: &str = "update-signing.secret";
pub const PUBLIC_FILE: &str = "update-signing.pub";

#[derive(Debug)]
pub enum KeyError {
    Io(io::Error),
    /// The output directory is inside the repository.
    InsideRepository(PathBuf),
    /// A key file is already there; keys are never overwritten.
    Exists(PathBuf),
    NoRandomness,
    Malformed(&'static str),
    /// The key file is readable by other users.
    LooseMode(PathBuf),
    SelfTestFailed(&'static str),
}

impl From<io::Error> for KeyError {
    fn from(error: io::Error) -> Self {
        Self::Io(error)
    }
}

impl std::fmt::Display for KeyError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Io(error) => write!(f, "{error}"),
            Self::InsideRepository(path) => write!(
                f,
                "{} is inside the repository; the secret key could be committed. Use a directory outside it",
                path.display()
            ),
            Self::Exists(path) => write!(
                f,
                "{} already exists and is never overwritten. To rotate keys, move the old files away after backing them up",
                path.display()
            ),
            Self::NoRandomness => write!(f, "the system random number generator is unavailable"),
            Self::Malformed(what) => write!(f, "the key file is malformed: {what}"),
            Self::LooseMode(path) => write!(
                f,
                "{} is readable by other users; run chmod 600 on it, and rotate the key if it was exposed",
                path.display()
            ),
            Self::SelfTestFailed(what) => write!(f, "the new key failed its self-test: {what}"),
        }
    }
}

impl std::error::Error for KeyError {}

pub struct GeneratedKey {
    pub secret_path: PathBuf,
    pub public_path: PathBuf,
    pub public_b64: String,
    pub id: String,
    /// SHA-256 of the raw public key, for checking the key by another route.
    pub fingerprint: String,
}

/// `~/.usher-nsh-update-signing`, outside any repository.
pub fn default_directory() -> Option<PathBuf> {
    std::env::var_os("HOME").map(|home| PathBuf::from(home).join(".usher-nsh-update-signing"))
}

/// Whether `directory` is inside `repository` once both are resolved. The directory is
/// resolved through its nearest existing ancestor, so a path that does not exist yet (or a
/// symbolic link to the repository) is judged by where it would really be.
pub fn is_inside(directory: &Path, repository: &Path) -> io::Result<bool> {
    let repository = repository.canonicalize()?;
    let mut existing = directory;
    let mut tail = Vec::new();
    let resolved = loop {
        match existing.canonicalize() {
            Ok(path) => break path,
            Err(_) => {
                let (Some(parent), Some(name)) = (existing.parent(), existing.file_name()) else {
                    break directory.to_path_buf();
                };
                tail.push(name.to_owned());
                existing = parent;
            }
        }
    };
    let full = tail
        .iter()
        .rev()
        .fold(resolved, |path, name| path.join(name));
    Ok(full.starts_with(repository))
}

fn random_seed() -> Result<[u8; 32], KeyError> {
    let mut seed = [0u8; 32];
    getrandom::fill(&mut seed).map_err(|_| KeyError::NoRandomness)?;
    Ok(seed)
}

fn create_private_directory(directory: &Path) -> io::Result<()> {
    #[cfg(unix)]
    {
        use std::os::unix::fs::DirBuilderExt;
        fs::DirBuilder::new()
            .recursive(true)
            .mode(0o700)
            .create(directory)
    }
    #[cfg(not(unix))]
    {
        fs::create_dir_all(directory)
    }
}

fn write_new(path: &Path, bytes: &[u8], mode: u32) -> Result<(), KeyError> {
    let mut options = OpenOptions::new();
    // `create_new` fails if the file exists, so a key can never be overwritten, and
    // the mode is set at creation, so it is never readable by others even briefly.
    options.write(true).create_new(true);
    #[cfg(unix)]
    std::os::unix::fs::OpenOptionsExt::mode(&mut options, mode);
    #[cfg(not(unix))]
    let _ = mode;
    let mut file = options.open(path).map_err(|error| {
        if error.kind() == io::ErrorKind::AlreadyExists {
            KeyError::Exists(path.to_owned())
        } else {
            KeyError::Io(error)
        }
    })?;
    file.write_all(bytes)?;
    file.sync_all()?;
    Ok(())
}

/// Checks a key the way the client will use it: from the stored raw bytes.
fn self_test(signing: &SigningKey, public_raw: &[u8; 32]) -> Result<(), KeyError> {
    let rebuilt = VerifyingKey::from_bytes(public_raw)
        .map_err(|_| KeyError::SelfTestFailed("the stored public key does not decode"))?;
    let message = b"usher update signing self-test";
    let signature: Signature = signing.sign(message);
    rebuilt.verify_strict(message, &signature).map_err(|_| {
        KeyError::SelfTestFailed("the public key does not verify the secret key's signature")
    })?;
    if rebuilt
        .verify_strict(b"usher update signing self-test!", &signature)
        .is_ok()
    {
        return Err(KeyError::SelfTestFailed("a changed message still verified"));
    }
    Ok(())
}

pub fn fingerprint(public_raw: &[u8; 32]) -> String {
    use sha2::{Digest, Sha256};
    Sha256::digest(public_raw)
        .iter()
        .map(|b| format!("{b:02x}"))
        .collect()
}

/// Generates a key pair into `directory`. `repository` is where keys must not go.
pub fn generate(directory: &Path, repository: &Path) -> Result<GeneratedKey, KeyError> {
    if is_inside(directory, repository)? {
        return Err(KeyError::InsideRepository(directory.to_owned()));
    }
    let secret_path = directory.join(SECRET_FILE);
    let public_path = directory.join(PUBLIC_FILE);
    for path in [&secret_path, &public_path] {
        if path.exists() {
            return Err(KeyError::Exists(path.clone()));
        }
    }
    let seed = random_seed()?;
    let signing = SigningKey::from_bytes(&seed);
    let public_raw = signing.verifying_key().to_bytes();
    // Test first: nothing touches the disk for a key that does not work.
    self_test(&signing, &public_raw)?;

    create_private_directory(directory)?;
    let public_b64 = B64.encode(public_raw);
    // Public first. If writing the secret then fails, nothing secret was left behind and a
    // lone public file only blocks a retry with a clear "exists" message.
    write_new(
        &secret_path,
        format!("{}\n", B64.encode(seed)).as_bytes(),
        0o600,
    )?;
    write_new(&public_path, format!("{public_b64}\n").as_bytes(), 0o644)?;
    Ok(GeneratedKey {
        id: key_id(&signing.verifying_key()),
        fingerprint: fingerprint(&public_raw),
        secret_path,
        public_path,
        public_b64,
    })
}

/// Loads a secret key, refusing one that other users can read.
pub fn load_secret(path: &Path) -> Result<SigningKey, KeyError> {
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        if fs::metadata(path)?.permissions().mode() & 0o077 != 0 {
            return Err(KeyError::LooseMode(path.to_owned()));
        }
    }
    let text = fs::read_to_string(path)?;
    let seed: [u8; 32] = B64
        .decode(text.trim())
        .map_err(|_| KeyError::Malformed("not base64"))?
        .try_into()
        .map_err(|_| KeyError::Malformed("not 32 bytes"))?;
    Ok(SigningKey::from_bytes(&seed))
}

/// The secret key from the environment, for CI: `name` holds the file's base64 line.
pub fn secret_from_env(name: &str) -> Result<SigningKey, KeyError> {
    let value = std::env::var(name)
        .map_err(|_| KeyError::Malformed("the environment variable is not set"))?;
    let seed: [u8; 32] = B64
        .decode(value.trim())
        .map_err(|_| KeyError::Malformed("not base64"))?
        .try_into()
        .map_err(|_| KeyError::Malformed("not 32 bytes"))?;
    Ok(SigningKey::from_bytes(&seed))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn outside_repo() -> (tempfile::TempDir, tempfile::TempDir) {
        (tempfile::tempdir().unwrap(), tempfile::tempdir().unwrap())
    }

    #[test]
    fn a_generated_pair_is_private_complete_and_usable() {
        let (keys, repo) = outside_repo();
        let dir = keys.path().join("signing");
        let key = generate(&dir, repo.path()).unwrap();
        assert!(key.secret_path.exists() && key.public_path.exists());
        assert_eq!(key.id.len(), 16);
        assert_eq!(key.fingerprint.len(), 64);
        assert_eq!(
            fs::read_to_string(&key.public_path).unwrap().trim(),
            key.public_b64
        );

        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            let mode = |p: &Path| fs::metadata(p).unwrap().permissions().mode() & 0o777;
            assert_eq!(mode(&dir), 0o700);
            assert_eq!(mode(&key.secret_path), 0o600);
            assert_eq!(mode(&key.public_path), 0o644);
        }
        // The stored secret loads and matches the published public key.
        let loaded = load_secret(&key.secret_path).unwrap();
        assert_eq!(
            B64.encode(loaded.verifying_key().as_bytes()),
            key.public_b64
        );
        assert_eq!(key_id(&loaded.verifying_key()), key.id);
    }

    #[test]
    fn two_pairs_are_different() {
        let (keys, repo) = outside_repo();
        let a = generate(&keys.path().join("a"), repo.path()).unwrap();
        let b = generate(&keys.path().join("b"), repo.path()).unwrap();
        assert_ne!(a.public_b64, b.public_b64);
    }

    #[test]
    fn keys_are_never_overwritten() {
        let (keys, repo) = outside_repo();
        let dir = keys.path().join("signing");
        let first = generate(&dir, repo.path()).unwrap();
        let before = fs::read(&first.secret_path).unwrap();
        assert!(matches!(
            generate(&dir, repo.path()),
            Err(KeyError::Exists(_))
        ));
        assert_eq!(fs::read(&first.secret_path).unwrap(), before);
        // Even a lone public file blocks a new pair, so the two can never get out of step.
        fs::remove_file(&first.secret_path).unwrap();
        assert!(matches!(
            generate(&dir, repo.path()),
            Err(KeyError::Exists(_))
        ));
    }

    #[test]
    fn a_directory_inside_the_repository_is_refused_before_anything_is_written() {
        let (_keys, repo) = outside_repo();
        let inside = repo.path().join("secrets");
        assert!(matches!(
            generate(&inside, repo.path()),
            Err(KeyError::InsideRepository(_))
        ));
        assert!(
            !inside.exists(),
            "nothing may be created inside the repository"
        );
        let deeper = repo.path().join("a/b/c");
        assert!(matches!(
            generate(&deeper, repo.path()),
            Err(KeyError::InsideRepository(_))
        ));
        assert!(!repo.path().join("a").exists());
    }

    #[cfg(unix)]
    #[test]
    fn a_link_into_the_repository_does_not_get_around_the_check() {
        let (keys, repo) = outside_repo();
        let link = keys.path().join("innocent");
        std::os::unix::fs::symlink(repo.path(), &link).unwrap();
        assert!(matches!(
            generate(&link.join("signing"), repo.path()),
            Err(KeyError::InsideRepository(_))
        ));
    }

    #[test]
    fn a_name_that_merely_starts_like_the_repository_is_not_inside_it() {
        let parent = tempfile::tempdir().unwrap();
        let repo = parent.path().join("usher");
        let sibling = parent.path().join("usher-keys");
        fs::create_dir(&repo).unwrap();
        assert!(!is_inside(&sibling, &repo).unwrap());
        assert!(is_inside(&repo.join("x"), &repo).unwrap());
    }

    #[cfg(unix)]
    #[test]
    fn a_secret_readable_by_others_is_refused() {
        use std::os::unix::fs::PermissionsExt;
        let (keys, repo) = outside_repo();
        let key = generate(&keys.path().join("signing"), repo.path()).unwrap();
        fs::set_permissions(&key.secret_path, fs::Permissions::from_mode(0o644)).unwrap();
        assert!(matches!(
            load_secret(&key.secret_path),
            Err(KeyError::LooseMode(_))
        ));
        fs::set_permissions(&key.secret_path, fs::Permissions::from_mode(0o600)).unwrap();
        assert!(load_secret(&key.secret_path).is_ok());
    }

    #[test]
    fn malformed_secrets_are_rejected_without_echoing_them() {
        let dir = tempfile::tempdir().unwrap();
        for (text, expected) in [
            ("not base64!\n", "not base64"),
            ("AAAA\n", "not 32 bytes"),
            ("", "not 32 bytes"),
        ] {
            let path = dir.path().join("k");
            let _ = fs::remove_file(&path);
            write_new(&path, text.as_bytes(), 0o600).unwrap();
            let error = load_secret(&path).unwrap_err().to_string();
            assert!(error.contains(expected), "{error}");
            assert!(!error.contains(text.trim()) || text.trim().is_empty());
        }
    }

    #[test]
    fn the_self_test_catches_a_mismatched_public_key() {
        let signing = SigningKey::from_bytes(&[1; 32]);
        let other = SigningKey::from_bytes(&[2; 32]).verifying_key().to_bytes();
        assert!(matches!(
            self_test(&signing, &other),
            Err(KeyError::SelfTestFailed(_))
        ));
        assert!(self_test(&signing, &signing.verifying_key().to_bytes()).is_ok());
    }
}
