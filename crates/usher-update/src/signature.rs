//! Release authentication: a set of trusted keys, a signed manifest, revocation.
//!
//! Differences from a single compiled-in key:
//!
//! - The client trusts a *set* of public keys, each named by an id. A signature file says
//!   which key signed it, so a backup key can be added to the set ahead of time and a
//!   release can be signed with it the day the primary key is retired.
//! - A signed manifest can revoke key ids and set a lowest allowed version. The caller
//!   stores what a verified manifest revokes, so a leaked key stops working once any
//!   release signed by another key has been seen.
//! - The manifest names the channel, so a signed stable manifest cannot be replayed as a
//!   preview one, and the version, so an old manifest cannot be replayed on a new release.
//!
//! Untrusted bytes reach the parser only after the signature checks out.
//!
//! ```text
//! usher-update-manifest-v2
//! version=<version without leading v>
//! channel=<stable|preview>
//! min-version=<version>            (optional)
//! revoke-key=<key id>              (optional, repeated)
//! <64 lowercase hex sha256><two spaces><file name>
//! ```
//!
//! ```text
//! usher-signature-v1
//! key=<key id>
//! <base64 of the 64-byte Ed25519 signature>
//! ```

use std::{collections::BTreeSet, fmt};

use base64::{Engine as _, engine::general_purpose::STANDARD as B64};
use ed25519_dalek::{Signature, VerifyingKey};
use sha2::{Digest as _, Sha256};

pub const MANIFEST_ASSET: &str = "SHA256SUMS";
pub const SIGNATURE_ASSET: &str = "SHA256SUMS.sig";
/// A real manifest is a few hundred bytes; bound what is downloaded before verifying.
pub const MAX_MANIFEST_BYTES: usize = 64 * 1024;
pub const MAX_SIGNATURE_BYTES: usize = 512;

const MANIFEST_HEADER: &str = "usher-update-manifest-v2";
const SIGNATURE_HEADER: &str = "usher-signature-v1";
const MAX_ENTRIES: usize = 256;

pub type Digest256 = [u8; 32];

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum Error {
    InvalidKey,
    /// The key set holds two keys with one id, or none at all.
    InvalidKeySet(&'static str),
    InvalidSignatureFile(&'static str),
    /// The signature names a key this client does not trust.
    UnknownKey(String),
    /// The signing key was revoked by an earlier signed manifest.
    RevokedKey(String),
    BadSignature,
    Malformed(&'static str),
    VersionMismatch {
        signed: String,
        release: String,
    },
    ChannelMismatch {
        signed: String,
        expected: String,
    },
    AssetNotListed(String),
    DuplicateAsset(String),
    DigestMismatch(String),
    /// Only a recovery key may revoke keys; this manifest was signed by a release key.
    RevocationNeedsRecoveryKey(String),
    /// A manifest revokes the key that signed it, which no honest release does.
    RevokesItsOwnSigner(String),
}

impl fmt::Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::InvalidKey => write!(f, "a built-in public key is invalid"),
            Self::InvalidKeySet(why) => write!(f, "the built-in key set is invalid: {why}"),
            Self::InvalidSignatureFile(why) => write!(f, "the signature file is malformed: {why}"),
            Self::UnknownKey(id) => write!(f, "the release is signed by an untrusted key {id}"),
            Self::RevokedKey(id) => write!(f, "the signing key {id} was revoked"),
            Self::BadSignature => write!(f, "the release signature is not valid"),
            Self::Malformed(why) => write!(f, "the release manifest is malformed: {why}"),
            Self::VersionMismatch { signed, release } => {
                write!(
                    f,
                    "the signed manifest is for version {signed}, not {release}"
                )
            }
            Self::ChannelMismatch { signed, expected } => {
                write!(
                    f,
                    "the signed manifest is for channel {signed}, not {expected}"
                )
            }
            Self::AssetNotListed(name) => write!(f, "the signed manifest does not list {name}"),
            Self::DuplicateAsset(name) => write!(f, "the signed manifest lists {name} twice"),
            Self::DigestMismatch(name) => {
                write!(f, "the downloaded {name} does not match its signed SHA-256")
            }
            Self::RevocationNeedsRecoveryKey(id) => {
                write!(f, "key {id} may not revoke keys: only a recovery key can")
            }
            Self::RevokesItsOwnSigner(id) => {
                write!(f, "the manifest revokes key {id}, which signed it")
            }
        }
    }
}

impl std::error::Error for Error {}

/// The id of a public key: the first 8 bytes of its SHA-256, in hex.
pub fn key_id(key: &VerifyingKey) -> String {
    let digest = Sha256::digest(key.as_bytes());
    digest[..8]
        .iter()
        .map(|byte| format!("{byte:02x}"))
        .collect()
}

fn is_key_id(text: &str) -> bool {
    text.len() == 16 && text.bytes().all(|b| matches!(b, b'0'..=b'9' | b'a'..=b'f'))
}

/// The public keys a client trusts to sign releases.
#[derive(Clone, Debug)]
pub struct KeySet {
    keys: Vec<(String, VerifyingKey, Role)>,
}

/// What a trusted key may do.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum Role {
    /// Signs ordinary releases. Cannot revoke anything: a thief holding it must not be able
    /// to strand clients by revoking the keys that could recover them.
    Release,
    /// Kept offline and used only to retire a leaked release key. May revoke other keys.
    Recovery,
}

impl KeySet {
    /// Parses base64 public keys, one per line (blank lines and `#` comments are ignored).
    /// A line `recovery:<key>` is a recovery key; any other line is a release key.
    pub fn parse(text: &str) -> Result<Self, Error> {
        let mut keys: Vec<(String, VerifyingKey, Role)> = Vec::new();
        for line in text
            .lines()
            .map(str::trim)
            .filter(|l| !l.is_empty() && !l.starts_with('#'))
        {
            let (role, line) = match line.strip_prefix("recovery:") {
                Some(rest) => (Role::Recovery, rest.trim()),
                None => (Role::Release, line),
            };
            let raw: [u8; 32] = B64
                .decode(line)
                .map_err(|_| Error::InvalidKey)?
                .try_into()
                .map_err(|_| Error::InvalidKey)?;
            let key = VerifyingKey::from_bytes(&raw).map_err(|_| Error::InvalidKey)?;
            let id = key_id(&key);
            if keys.iter().any(|(existing, _, _)| *existing == id) {
                return Err(Error::InvalidKeySet("the same key is listed twice"));
            }
            keys.push((id, key, role));
        }
        if keys.is_empty() {
            return Err(Error::InvalidKeySet("no keys"));
        }
        Ok(Self { keys })
    }

    pub fn ids(&self) -> Vec<&str> {
        self.keys.iter().map(|(id, _, _)| id.as_str()).collect()
    }

    pub fn role(&self, id: &str) -> Option<Role> {
        self.keys
            .iter()
            .find(|(known, _, _)| known == id)
            .map(|(_, _, role)| *role)
    }

    fn get(&self, id: &str) -> Option<&VerifyingKey> {
        self.keys
            .iter()
            .find(|(known, _, _)| known == id)
            .map(|(_, key, _)| key)
    }
}

/// Key ids that earlier signed manifests revoked. The caller persists it.
#[derive(Clone, Debug, Default, Eq, PartialEq)]
pub struct Revocations(BTreeSet<String>);

impl Revocations {
    pub fn from_ids<I: IntoIterator<Item = String>>(ids: I) -> Self {
        Self(ids.into_iter().filter(|id| is_key_id(id)).collect())
    }

    pub fn contains(&self, id: &str) -> bool {
        self.0.contains(id)
    }

    pub fn ids(&self) -> impl Iterator<Item = &str> {
        self.0.iter().map(String::as_str)
    }

    /// Adds what `manifest` revokes. Returns whether anything is new, so the caller knows
    /// to save.
    pub fn absorb(&mut self, manifest: &VerifiedManifest) -> bool {
        let before = self.0.len();
        self.0.extend(manifest.revoked_keys.iter().cloned());
        self.0.len() != before
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct VerifiedManifest {
    pub version: String,
    pub channel: String,
    /// Clients older than this cannot update in place and need the full installer.
    pub min_version: Option<String>,
    pub revoked_keys: Vec<String>,
    /// The key that signed it.
    pub signed_by: String,
    entries: Vec<(String, Digest256)>,
}

impl VerifiedManifest {
    /// Verifies `signature` over the exact `manifest` bytes with a key from `keys` that is
    /// not in `revoked`, then parses them. `channel` is the channel being checked.
    pub fn verify(
        manifest: &[u8],
        signature: &[u8],
        keys: &KeySet,
        revoked: &Revocations,
        channel: &str,
    ) -> Result<Self, Error> {
        let (key_name, signature) = parse_signature_file(signature)?;
        if revoked.contains(&key_name) {
            return Err(Error::RevokedKey(key_name));
        }
        let key = keys
            .get(&key_name)
            .ok_or_else(|| Error::UnknownKey(key_name.clone()))?;
        // `verify_strict` rejects the malleable and small-order encodings plain `verify` accepts.
        key.verify_strict(manifest, &signature)
            .map_err(|_| Error::BadSignature)?;
        let parsed = parse_manifest(manifest, key_name)?;
        if parsed.channel != channel {
            return Err(Error::ChannelMismatch {
                signed: parsed.channel,
                expected: channel.to_owned(),
            });
        }
        if !parsed.revoked_keys.is_empty() {
            // Only a recovery key may retire keys. Otherwise whoever steals a release key
            // could revoke the recovery key and leave clients trusting only the stolen one.
            if keys.role(&parsed.signed_by) != Some(Role::Recovery) {
                return Err(Error::RevocationNeedsRecoveryKey(parsed.signed_by));
            }
            // And never itself: the signer always stays trusted, so at least one key does,
            // and clients can still be updated after any revocation.
            if parsed.revoked_keys.contains(&parsed.signed_by) {
                return Err(Error::RevokesItsOwnSigner(parsed.signed_by));
            }
        }
        Ok(parsed)
    }

    /// Every file the manifest lists, with the SHA-256 it commits to.
    pub fn entries(&self) -> &[(String, Digest256)] {
        &self.entries
    }

    /// The SHA-256 the signature commits to for `name` in release `release_version`.
    pub fn digest_for(&self, release_version: &str, name: &str) -> Result<Digest256, Error> {
        if self.version != release_version {
            return Err(Error::VersionMismatch {
                signed: self.version.clone(),
                release: release_version.to_owned(),
            });
        }
        let mut matches = self.entries.iter().filter(|(entry, _)| entry == name);
        let Some((_, digest)) = matches.next() else {
            return Err(Error::AssetNotListed(name.to_owned()));
        };
        if matches.next().is_some() {
            return Err(Error::DuplicateAsset(name.to_owned()));
        }
        Ok(*digest)
    }
}

/// What a release manifest says. `render_manifest` turns it into the signed text.
pub struct ManifestSpec<'a> {
    pub version: &'a str,
    pub channel: &'a str,
    pub min_version: Option<&'a str>,
    pub revoke_keys: &'a [String],
    pub entries: &'a [(String, Digest256)],
}

/// The manifest text for `spec`, entries sorted by name so the same release always gives
/// the same bytes. The result is parsed again before it is returned: the signing side can
/// never produce a manifest this client would reject.
pub fn render_manifest(spec: &ManifestSpec<'_>) -> Result<Vec<u8>, Error> {
    let mut entries: Vec<&(String, Digest256)> = spec.entries.iter().collect();
    entries.sort_by(|a, b| a.0.cmp(&b.0));
    if let Some(pair) = entries.windows(2).find(|pair| pair[0].0 == pair[1].0) {
        return Err(Error::DuplicateAsset(pair[0].0.clone()));
    }
    let mut text = format!(
        "{MANIFEST_HEADER}\nversion={}\nchannel={}\n",
        spec.version, spec.channel
    );
    if let Some(min) = spec.min_version {
        text.push_str(&format!("min-version={min}\n"));
    }
    for id in spec.revoke_keys {
        text.push_str(&format!("revoke-key={id}\n"));
    }
    for (name, digest) in entries {
        text.push_str(&format!(
            "{}  {name}\n",
            digest
                .iter()
                .map(|b| format!("{b:02x}"))
                .collect::<String>()
        ));
    }
    let bytes = text.into_bytes();
    parse_manifest(&bytes, String::new())?;
    Ok(bytes)
}

/// The signature file for a signature made by key `key_id`.
pub fn render_signature_file(key_id: &str, signature: &[u8; 64]) -> Vec<u8> {
    format!(
        "{SIGNATURE_HEADER}\nkey={key_id}\n{}\n",
        B64.encode(signature)
    )
    .into_bytes()
}

/// The public keys built into this client (`trusted_keys.txt`, one base64 key per line).
/// An empty list means no release key has been set up yet, which `KeySet::parse` reports
/// as `InvalidKeySet("no keys")`; the caller treats that as "updates are not configured".
pub fn built_in_keys() -> Result<KeySet, Error> {
    KeySet::parse(include_str!("../trusted_keys.txt"))
}

fn parse_signature_file(bytes: &[u8]) -> Result<(String, Signature), Error> {
    use Error::InvalidSignatureFile as Bad;
    let text = std::str::from_utf8(bytes).map_err(|_| Bad("not UTF-8"))?;
    let mut lines = text.trim_end_matches('\n').split('\n');
    if lines.next() != Some(SIGNATURE_HEADER) {
        return Err(Bad("unknown format header"));
    }
    let key = lines
        .next()
        .and_then(|line| line.strip_prefix("key="))
        .filter(|id| is_key_id(id))
        .ok_or(Bad("missing key id"))?;
    let encoded = lines.next().ok_or(Bad("missing signature"))?;
    if lines.next().is_some() {
        return Err(Bad("trailing data"));
    }
    let raw = B64
        .decode(encoded)
        .map_err(|_| Bad("signature is not base64"))?;
    let signature =
        Signature::from_slice(&raw).map_err(|_| Bad("signature has the wrong length"))?;
    Ok((key.to_owned(), signature))
}

fn is_version_text(text: &str) -> bool {
    !text.is_empty() && !text.contains(char::is_whitespace) && semver::Version::parse(text).is_ok()
}

fn parse_manifest(manifest: &[u8], signed_by: String) -> Result<VerifiedManifest, Error> {
    use Error::Malformed as Bad;
    let text = std::str::from_utf8(manifest).map_err(|_| Bad("not UTF-8"))?;
    if text.contains('\r') {
        return Err(Bad("carriage return"));
    }
    let body = text
        .strip_suffix('\n')
        .ok_or(Bad("missing final newline"))?;
    let mut lines = body.split('\n').peekable();
    if lines.next() != Some(MANIFEST_HEADER) {
        return Err(Bad("unknown format header"));
    }
    let version = lines
        .next()
        .and_then(|line| line.strip_prefix("version="))
        .filter(|v| is_version_text(v))
        .ok_or(Bad("missing or invalid version"))?
        .to_owned();
    let channel = lines
        .next()
        .and_then(|line| line.strip_prefix("channel="))
        .filter(|c| matches!(*c, "stable" | "preview"))
        .ok_or(Bad("missing or unknown channel"))?
        .to_owned();
    let mut min_version = None;
    if let Some(value) = lines
        .peek()
        .and_then(|line| line.strip_prefix("min-version="))
    {
        if !is_version_text(value) {
            return Err(Bad("invalid min-version"));
        }
        min_version = Some(value.to_owned());
        lines.next();
    }
    let mut revoked_keys = Vec::new();
    while let Some(id) = lines
        .peek()
        .and_then(|line| line.strip_prefix("revoke-key="))
    {
        if !is_key_id(id) || revoked_keys.iter().any(|known| known == id) {
            return Err(Bad("invalid or repeated revoke-key"));
        }
        revoked_keys.push(id.to_owned());
        lines.next();
    }
    let mut entries = Vec::new();
    for line in lines {
        let (hex, name) = line
            .split_once("  ")
            .ok_or(Bad("entry is not '<sha256>  <name>'"))?;
        let digest = parse_hex_digest(hex).ok_or(Bad("invalid SHA-256"))?;
        if name.is_empty() || name.contains(['/', '\\']) || name.contains(char::is_whitespace) {
            return Err(Bad("invalid file name"));
        }
        entries.push((name.to_owned(), digest));
        if entries.len() > MAX_ENTRIES {
            return Err(Bad("too many entries"));
        }
    }
    if entries.is_empty() {
        return Err(Bad("no entries"));
    }
    Ok(VerifiedManifest {
        version,
        channel,
        min_version,
        revoked_keys,
        signed_by,
        entries,
    })
}

fn parse_hex_digest(hex: &str) -> Option<Digest256> {
    if hex.len() != 64 || !hex.bytes().all(|b| matches!(b, b'0'..=b'9' | b'a'..=b'f')) {
        return None;
    }
    let mut digest = [0u8; 32];
    for (byte, pair) in digest.iter_mut().zip(hex.as_bytes().chunks(2)) {
        *byte = u8::from_str_radix(std::str::from_utf8(pair).ok()?, 16).ok()?;
    }
    Some(digest)
}

/// Hashes the package while it streams to disk, so what is compared is what was written.
#[derive(Default)]
pub struct StreamingDigest(Sha256);

impl StreamingDigest {
    pub fn update(&mut self, chunk: &[u8]) {
        self.0.update(chunk);
    }

    pub fn verify(self, name: &str, expected: &Digest256) -> Result<(), Error> {
        let actual: Digest256 = self.0.finalize().into();
        if &actual == expected {
            Ok(())
        } else {
            Err(Error::DigestMismatch(name.to_owned()))
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use ed25519_dalek::{Signer, SigningKey};

    const ASSET: &str = "Usher-aarch64-apple-darwin.tar.gz";

    fn signing_key(seed: u8) -> SigningKey {
        SigningKey::from_bytes(&[seed; 32])
    }

    fn id(seed: u8) -> String {
        key_id(&signing_key(seed).verifying_key())
    }

    fn key_set(seeds: &[u8]) -> KeySet {
        let text: Vec<String> = seeds
            .iter()
            .map(|s| B64.encode(signing_key(*s).verifying_key().as_bytes()))
            .collect();
        KeySet::parse(&text.join("\n")).unwrap()
    }

    /// `release` keys sign releases; `recovery` keys are the offline ones that may revoke.
    fn roled_set(release: &[u8], recovery: &[u8]) -> KeySet {
        let encode = |seed: &u8| B64.encode(signing_key(*seed).verifying_key().as_bytes());
        let mut lines: Vec<String> = release.iter().map(encode).collect();
        lines.extend(
            recovery
                .iter()
                .map(|seed| format!("recovery:{}", encode(seed))),
        );
        KeySet::parse(&lines.join("\n")).unwrap()
    }

    fn hex(bytes: &[u8]) -> String {
        bytes.iter().map(|b| format!("{b:02x}")).collect()
    }

    fn sha(bytes: &[u8]) -> Digest256 {
        Sha256::digest(bytes).into()
    }

    fn manifest(version: &str, channel: &str, extra: &str, entries: &[(&str, &[u8])]) -> Vec<u8> {
        let mut text = format!("{MANIFEST_HEADER}\nversion={version}\nchannel={channel}\n{extra}");
        for (name, content) in entries {
            text.push_str(&format!("{}  {name}\n", hex(&sha(content))));
        }
        text.into_bytes()
    }

    fn sign(manifest: &[u8], seed: u8) -> Vec<u8> {
        let signature = signing_key(seed).sign(manifest).to_bytes();
        format!(
            "{SIGNATURE_HEADER}\nkey={}\n{}\n",
            id(seed),
            B64.encode(signature)
        )
        .into_bytes()
    }

    fn verified(
        manifest: &[u8],
        signature: &[u8],
        seeds: &[u8],
    ) -> Result<VerifiedManifest, Error> {
        VerifiedManifest::verify(
            manifest,
            signature,
            &key_set(seeds),
            &Revocations::default(),
            "stable",
        )
    }

    #[test]
    fn a_valid_signature_yields_the_signed_digest() {
        let archive = b"package bytes";
        let m = manifest(
            "1.2.3",
            "stable",
            "",
            &[(ASSET, archive), ("other.zip", b"x")],
        );
        let v = verified(&m, &sign(&m, 1), &[1]).unwrap();
        assert_eq!(v.digest_for("1.2.3", ASSET), Ok(sha(archive)));
        assert_eq!(v.signed_by, id(1));
    }

    #[test]
    fn a_backup_key_in_the_set_can_sign_instead_of_the_primary() {
        let m = manifest("1.2.3", "stable", "", &[(ASSET, b"p")]);
        // The client trusts both; the release is signed by the second.
        assert!(verified(&m, &sign(&m, 2), &[1, 2]).is_ok());
        // A client that only knows the primary refuses it, naming the key.
        assert_eq!(
            verified(&m, &sign(&m, 2), &[1]).unwrap_err(),
            Error::UnknownKey(id(2))
        );
    }

    #[test]
    fn a_signature_cannot_claim_another_trusted_key() {
        let m = manifest("1.2.3", "stable", "", &[(ASSET, b"p")]);
        // Signed by key 3 but labelled as key 1: the labelled key does not verify it.
        let signature = signing_key(3).sign(&m).to_bytes();
        let forged = format!(
            "{SIGNATURE_HEADER}\nkey={}\n{}\n",
            id(1),
            B64.encode(signature)
        );
        assert_eq!(
            verified(&m, forged.as_bytes(), &[1, 3]).unwrap_err(),
            Error::BadSignature
        );
    }

    #[test]
    fn any_change_to_the_manifest_invalidates_the_signature() {
        let m = manifest("1.2.3", "stable", "", &[(ASSET, b"package")]);
        let signature = sign(&m, 1);
        for index in 0..m.len() {
            let mut tampered = m.clone();
            tampered[index] ^= 1;
            assert_eq!(
                verified(&tampered, &signature, &[1]).unwrap_err(),
                Error::BadSignature,
                "byte {index}"
            );
        }
        let mut longer = m.clone();
        longer.push(b'\n');
        assert!(verified(&longer, &signature, &[1]).is_err());
    }

    #[test]
    fn a_signature_for_another_manifest_cannot_be_swapped_in() {
        let good = manifest("1.2.3", "stable", "", &[(ASSET, b"package")]);
        let evil = manifest("1.2.3", "stable", "", &[(ASSET, b"evil")]);
        assert_eq!(
            verified(&evil, &sign(&good, 1), &[1]).unwrap_err(),
            Error::BadSignature
        );
    }

    #[test]
    fn a_revoked_key_no_longer_verifies_even_with_a_correct_signature() {
        let m = manifest("1.2.3", "stable", "", &[(ASSET, b"p")]);
        let revoked = Revocations::from_ids([id(1)]);
        let result =
            VerifiedManifest::verify(&m, &sign(&m, 1), &key_set(&[1, 2]), &revoked, "stable");
        assert_eq!(result.unwrap_err(), Error::RevokedKey(id(1)));
    }

    // Key 1 is the day-to-day release key; key 2 is the offline recovery key.

    #[test]
    fn a_recovery_key_can_revoke_the_leaked_release_key() {
        let extra = format!("revoke-key={}\n", id(1));
        let m = manifest("1.2.4", "stable", &extra, &[(ASSET, b"p")]);
        let keys = roled_set(&[1], &[2]);
        let mut revoked = Revocations::default();
        let v = VerifiedManifest::verify(&m, &sign(&m, 2), &keys, &revoked, "stable").unwrap();
        assert_eq!(v.revoked_keys, vec![id(1)]);
        assert!(
            revoked.absorb(&v),
            "the caller must save the new revocation"
        );
        assert!(!revoked.absorb(&v), "absorbing it again changes nothing");

        // From now on a release signed by the leaked key is refused...
        let old = manifest("9.9.9", "stable", "", &[(ASSET, b"evil")]);
        let result = VerifiedManifest::verify(&old, &sign(&old, 1), &keys, &revoked, "stable");
        assert_eq!(result.unwrap_err(), Error::RevokedKey(id(1)));
        // ...and the recovery key still signs.
        let fresh = manifest("1.2.5", "stable", "", &[(ASSET, b"p")]);
        assert!(
            VerifiedManifest::verify(&fresh, &sign(&fresh, 2), &keys, &revoked, "stable").is_ok()
        );
    }

    #[test]
    fn a_stolen_release_key_cannot_revoke_anything() {
        // The attack the roles exist for: a thief with the release key tries to revoke the
        // recovery key, so that the only key clients trust is the one they hold.
        let extra = format!("revoke-key={}\n", id(2));
        let m = manifest("9.9.9", "stable", &extra, &[(ASSET, b"evil")]);
        let keys = roled_set(&[1], &[2]);
        assert_eq!(
            VerifiedManifest::verify(&m, &sign(&m, 1), &keys, &Revocations::default(), "stable")
                .unwrap_err(),
            Error::RevocationNeedsRecoveryKey(id(1))
        );
        // Revoking its own key, or another release key, is refused for the same reason.
        for target in [id(1), id(3)] {
            let extra = format!("revoke-key={target}\n");
            let m = manifest("9.9.9", "stable", &extra, &[(ASSET, b"evil")]);
            let keys = roled_set(&[1, 3], &[2]);
            let result = VerifiedManifest::verify(
                &m,
                &sign(&m, 1),
                &keys,
                &Revocations::default(),
                "stable",
            );
            assert!(
                matches!(result, Err(Error::RevocationNeedsRecoveryKey(_))),
                "{target}"
            );
        }
    }

    #[test]
    fn a_recovery_key_cannot_revoke_itself_and_so_one_key_always_survives() {
        let extra = format!("revoke-key={}\nrevoke-key={}\n", id(1), id(2));
        let m = manifest("1.2.4", "stable", &extra, &[(ASSET, b"p")]);
        assert_eq!(
            VerifiedManifest::verify(
                &m,
                &sign(&m, 2),
                &roled_set(&[1], &[2]),
                &Revocations::default(),
                "stable"
            )
            .unwrap_err(),
            Error::RevokesItsOwnSigner(id(2))
        );
    }

    #[test]
    fn a_recovery_key_may_also_sign_an_ordinary_release() {
        let m = manifest("1.2.3", "stable", "", &[(ASSET, b"p")]);
        assert!(
            VerifiedManifest::verify(
                &m,
                &sign(&m, 2),
                &roled_set(&[1], &[2]),
                &Revocations::default(),
                "stable"
            )
            .is_ok()
        );
    }

    #[test]
    fn roles_are_read_from_the_key_list() {
        let keys = roled_set(&[1], &[2]);
        assert_eq!(keys.role(&id(1)), Some(Role::Release));
        assert_eq!(keys.role(&id(2)), Some(Role::Recovery));
        assert_eq!(keys.role(&id(3)), None);
        let plain = B64.encode(signing_key(4).verifying_key().as_bytes());
        assert_eq!(
            KeySet::parse(&format!("  recovery:  {plain}  "))
                .unwrap()
                .role(&id(4)),
            Some(Role::Recovery)
        );
        assert!(KeySet::parse("recovery:not base64").is_err());
        // The same key listed as both roles is a duplicate, not two keys.
        assert!(KeySet::parse(&format!("{plain}\nrecovery:{plain}")).is_err());
    }

    #[test]
    fn the_signed_version_and_channel_must_match() {
        let m = manifest("1.2.2", "stable", "", &[(ASSET, b"p")]);
        let v = verified(&m, &sign(&m, 1), &[1]).unwrap();
        assert_eq!(
            v.digest_for("1.2.3", ASSET),
            Err(Error::VersionMismatch {
                signed: "1.2.2".into(),
                release: "1.2.3".into()
            })
        );
        let preview = manifest("1.2.3", "preview", "", &[(ASSET, b"p")]);
        let result = VerifiedManifest::verify(
            &preview,
            &sign(&preview, 1),
            &key_set(&[1]),
            &Revocations::default(),
            "stable",
        );
        assert_eq!(
            result.unwrap_err(),
            Error::ChannelMismatch {
                signed: "preview".into(),
                expected: "stable".into()
            }
        );
    }

    #[test]
    fn min_version_is_carried_and_validated() {
        let m = manifest("2.0.0", "stable", "min-version=1.5.0\n", &[(ASSET, b"p")]);
        assert_eq!(
            verified(&m, &sign(&m, 1), &[1])
                .unwrap()
                .min_version
                .as_deref(),
            Some("1.5.0")
        );
        let bad = manifest("2.0.0", "stable", "min-version=soon\n", &[(ASSET, b"p")]);
        assert!(verified(&bad, &sign(&bad, 1), &[1]).is_err());
    }

    #[test]
    fn the_selected_asset_must_be_listed_exactly_once() {
        let missing = manifest("1.2.3", "stable", "", &[("other.zip", b"x")]);
        let v = verified(&missing, &sign(&missing, 1), &[1]).unwrap();
        assert_eq!(
            v.digest_for("1.2.3", ASSET),
            Err(Error::AssetNotListed(ASSET.into()))
        );
        let twice = manifest("1.2.3", "stable", "", &[(ASSET, b"a"), (ASSET, b"b")]);
        let v = verified(&twice, &sign(&twice, 1), &[1]).unwrap();
        assert_eq!(
            v.digest_for("1.2.3", ASSET),
            Err(Error::DuplicateAsset(ASSET.into()))
        );
    }

    #[test]
    fn signed_but_malformed_manifests_are_rejected() {
        let h = hex(&sha(b"x"));
        let cases: Vec<(String, &str)> = vec![
            (String::new(), "empty"),
            (
                format!("{MANIFEST_HEADER}\nversion=1.0.0\nchannel=stable\n"),
                "no entries",
            ),
            (
                format!("{MANIFEST_HEADER}\nchannel=stable\n{h}  a.zip\n"),
                "no version",
            ),
            (
                format!("{MANIFEST_HEADER}\nversion=latest\nchannel=stable\n{h}  a.zip\n"),
                "not semver",
            ),
            (
                format!("{MANIFEST_HEADER}\nversion=1.0.0\n{h}  a.zip\n"),
                "no channel",
            ),
            (
                format!("{MANIFEST_HEADER}\nversion=1.0.0\nchannel=nightly\n{h}  a.zip\n"),
                "unknown channel",
            ),
            (
                format!("usher-update-manifest-v1\nversion=1.0.0\nchannel=stable\n{h}  a.zip\n"),
                "old header",
            ),
            (
                format!("{MANIFEST_HEADER}\nversion=1.0.0\nchannel=stable\n{h}  a.zip"),
                "no final newline",
            ),
            (
                format!("{MANIFEST_HEADER}\r\nversion=1.0.0\r\nchannel=stable\r\n{h}  a.zip\r\n"),
                "crlf",
            ),
            (
                format!("{MANIFEST_HEADER}\nversion=1.0.0\nchannel=stable\n{h} a.zip\n"),
                "one space",
            ),
            (
                format!(
                    "{MANIFEST_HEADER}\nversion=1.0.0\nchannel=stable\n{}  a.zip\n",
                    h.to_uppercase()
                ),
                "uppercase",
            ),
            (
                format!("{MANIFEST_HEADER}\nversion=1.0.0\nchannel=stable\n{h}  ../a.zip\n"),
                "path in name",
            ),
            (
                format!("{MANIFEST_HEADER}\nversion=1.0.0\nchannel=stable\n\n{h}  a.zip\n"),
                "blank line",
            ),
            (
                format!(
                    "{MANIFEST_HEADER}\nversion=1.0.0\nchannel=stable\nrevoke-key=zz\n{h}  a.zip\n"
                ),
                "bad key id",
            ),
            (
                format!(
                    "{MANIFEST_HEADER}\nversion=1.0.0\nchannel=stable\nrevoke-key={0}\nrevoke-key={0}\n{h}  a.zip\n",
                    id(2)
                ),
                "repeated revoke",
            ),
        ];
        for (text, why) in cases {
            assert!(
                verified(text.as_bytes(), &sign(text.as_bytes(), 1), &[1, 2]).is_err(),
                "{why}"
            );
        }
    }

    #[test]
    fn malformed_signature_files_are_rejected() {
        let m = manifest("1.2.3", "stable", "", &[(ASSET, b"p")]);
        let good = String::from_utf8(sign(&m, 1)).unwrap();
        let sig_line = good.lines().nth(2).unwrap().to_owned();
        let cases = [
            String::new(),
            "usher-signature-v0\nkey=0000000000000000\nAAAA\n".into(),
            format!("{SIGNATURE_HEADER}\nkey=short\n{sig_line}\n"),
            format!("{SIGNATURE_HEADER}\nkey={}\nnot base64!\n", id(1)),
            format!("{SIGNATURE_HEADER}\nkey={}\nAAAA\n", id(1)),
            format!("{good}extra\n"),
        ];
        for bad in cases {
            assert!(
                matches!(
                    verified(&m, bad.as_bytes(), &[1]),
                    Err(Error::InvalidSignatureFile(_))
                ),
                "{bad:?}"
            );
        }
        assert!(verified(&m, &[0xff, 0xfe], &[1]).is_err());
    }

    #[test]
    fn key_sets_reject_bad_input() {
        let one = B64.encode(signing_key(1).verifying_key().as_bytes());
        assert!(KeySet::parse(&format!("# primary\n{one}\n\n")).is_ok());
        assert_eq!(
            KeySet::parse("").unwrap_err(),
            Error::InvalidKeySet("no keys")
        );
        assert_eq!(
            KeySet::parse("# only a comment\n").unwrap_err(),
            Error::InvalidKeySet("no keys")
        );
        assert_eq!(KeySet::parse("not base64!").unwrap_err(), Error::InvalidKey);
        assert_eq!(KeySet::parse("AAAA").unwrap_err(), Error::InvalidKey);
        assert_eq!(
            KeySet::parse(&format!("{one}\n{one}")).unwrap_err(),
            Error::InvalidKeySet("the same key is listed twice")
        );
    }

    #[test]
    fn the_streaming_digest_accepts_only_the_signed_bytes() {
        let content = b"downloaded package";
        let expected = sha(content);
        let mut digest = StreamingDigest::default();
        for chunk in content.chunks(5) {
            digest.update(chunk);
        }
        assert!(digest.verify(ASSET, &expected).is_ok());
        for tampered in [
            &b"downloaded packagE"[..],
            b"downloaded package ",
            b"",
            b"x",
        ] {
            let mut digest = StreamingDigest::default();
            digest.update(tampered);
            assert_eq!(
                digest.verify(ASSET, &expected),
                Err(Error::DigestMismatch(ASSET.into()))
            );
        }
    }

    #[test]
    fn revocations_ignore_malformed_ids_and_round_trip() {
        let r = Revocations::from_ids(["zz".to_owned(), id(1), id(1)]);
        assert_eq!(r.ids().collect::<Vec<_>>(), vec![id(1)]);
        assert!(r.contains(&id(1)) && !r.contains(&id(2)));
    }
}

#[cfg(test)]
mod render_tests {
    use super::*;
    use ed25519_dalek::{Signer, SigningKey};

    fn digest(byte: u8) -> Digest256 {
        [byte; 32]
    }

    fn spec<'a>(entries: &'a [(String, Digest256)], revoke: &'a [String]) -> ManifestSpec<'a> {
        ManifestSpec {
            version: "1.2.3",
            channel: "stable",
            min_version: None,
            revoke_keys: revoke,
            entries,
        }
    }

    #[test]
    fn rendered_manifests_are_sorted_and_accepted_by_the_client() {
        let key = SigningKey::from_bytes(&[7; 32]);
        let id = key_id(&key.verifying_key());
        let entries = vec![
            ("b.zip".to_owned(), digest(2)),
            ("a.tar.gz".to_owned(), digest(1)),
        ];
        let manifest = render_manifest(&spec(&entries, &[])).unwrap();
        let text = String::from_utf8(manifest.clone()).unwrap();
        assert!(
            text.find("a.tar.gz").unwrap() < text.find("b.zip").unwrap(),
            "{text}"
        );
        // The same release always gives the same bytes.
        let shuffled = vec![entries[1].clone(), entries[0].clone()];
        assert_eq!(render_manifest(&spec(&shuffled, &[])).unwrap(), manifest);

        let signature = render_signature_file(&id, &key.sign(&manifest).to_bytes());
        let keys = KeySet::parse(&B64.encode(key.verifying_key().as_bytes())).unwrap();
        let verified = VerifiedManifest::verify(
            &manifest,
            &signature,
            &keys,
            &Revocations::default(),
            "stable",
        )
        .unwrap();
        assert_eq!(verified.entries().len(), 2);
        assert_eq!(verified.digest_for("1.2.3", "a.tar.gz"), Ok(digest(1)));
    }

    #[test]
    fn the_renderer_refuses_what_the_parser_would_refuse() {
        let ok = vec![("a.zip".to_owned(), digest(1))];
        let bad_version = ManifestSpec {
            version: "latest",
            ..spec(&ok, &[])
        };
        assert!(render_manifest(&bad_version).is_err());
        let bad_channel = ManifestSpec {
            channel: "nightly",
            ..spec(&ok, &[])
        };
        assert!(render_manifest(&bad_channel).is_err());
        let bad_min = ManifestSpec {
            min_version: Some("soon"),
            ..spec(&ok, &[])
        };
        assert!(render_manifest(&bad_min).is_err());
        let revoke = vec!["zz".to_owned()];
        assert!(render_manifest(&spec(&ok, &revoke)).is_err());
        let spaced = vec![("a b.zip".to_owned(), digest(1))];
        assert!(render_manifest(&spec(&spaced, &[])).is_err());
        assert!(render_manifest(&spec(&[], &[])).is_err(), "no entries");
        let twice = vec![
            ("a.zip".to_owned(), digest(1)),
            ("a.zip".to_owned(), digest(2)),
        ];
        assert_eq!(
            render_manifest(&spec(&twice, &[])).unwrap_err(),
            Error::DuplicateAsset("a.zip".into())
        );
    }

    #[test]
    fn the_built_in_key_list_parses_and_holds_the_release_key() {
        // The list is compiled into every client. A malformed line, a duplicate or an
        // empty list would ship an updater that trusts nothing, so it fails the build here.
        let keys = built_in_keys().expect("trusted_keys.txt is malformed or empty");
        assert!(!keys.ids().is_empty());
    }
}
