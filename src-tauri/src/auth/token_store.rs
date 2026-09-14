//! Encrypted token file. See docs/02-arquitectura.md section 7.
//!
//! `tokens.bin` = `nonce(12) || AES-256-GCM(ciphertext)` with a fresh nonce on every write.
//! Key = HKDF-SHA256(ikm = machine-id || uid, salt = "ugc-token-store-v1"). The key is never
//! stored. Tokens are read and written only here; nothing in this module logs them.

use std::collections::BTreeMap;
use std::path::{Path, PathBuf};

use aes_gcm::aead::{Aead, KeyInit, OsRng};
use aes_gcm::{AeadCore, Aes256Gcm, Key, Nonce};
use hkdf::Hkdf;
use serde::{Deserialize, Serialize};
use sha2::Sha256;

use crate::error::AppError;

const SALT: &[u8] = b"ugc-token-store-v1";
const NONCE_LEN: usize = 12;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Default)]
pub struct AccountTokens {
    pub refresh_token: String,
    pub access_token: String,
    /// Unix seconds.
    pub expires_at: i64,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Default)]
pub struct TokenFile {
    pub accounts: BTreeMap<String, AccountTokens>,
    /// Secret iCal addresses by account id (docs/99 "Calendarios iCal por URL").
    #[serde(default)]
    pub ical_urls: BTreeMap<String, String>,
}

/// Where the file lives and what the key is derived from.
#[derive(Clone)]
pub struct TokenStore {
    path: PathBuf,
    key: Key<Aes256Gcm>,
}

impl std::fmt::Debug for TokenStore {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("TokenStore")
            .field("path", &self.path)
            .finish_non_exhaustive()
    }
}

fn machine_id() -> Result<String, AppError> {
    let raw = std::fs::read_to_string("/etc/machine-id")
        .or_else(|_| std::fs::read_to_string("/var/lib/dbus/machine-id"))
        .map_err(|e| AppError::Auth(format!("cannot read machine-id: {e}")))?;
    Ok(raw.trim().to_string())
}

fn current_uid() -> u32 {
    use std::os::unix::fs::MetadataExt;
    std::fs::metadata("/proc/self")
        .map(|m| m.uid())
        .unwrap_or(0)
}

pub fn derive_key(machine_id: &str, uid: u32) -> Key<Aes256Gcm> {
    let ikm = format!("{machine_id}:{uid}");
    let hk = Hkdf::<Sha256>::new(Some(SALT), ikm.as_bytes());
    let mut okm = [0u8; 32];
    // 32 bytes is always a valid HKDF-SHA256 output length.
    hk.expand(b"aes-256-gcm", &mut okm).unwrap_or_default();
    *Key::<Aes256Gcm>::from_slice(&okm)
}

impl TokenStore {
    /// Store at the default path with the key of this machine and user.
    pub fn open_default() -> Result<TokenStore, AppError> {
        Ok(TokenStore::with_material(
            crate::config::tokens_path(),
            &machine_id()?,
            current_uid(),
        ))
    }

    pub fn with_material(path: PathBuf, machine_id: &str, uid: u32) -> TokenStore {
        TokenStore {
            path,
            key: derive_key(machine_id, uid),
        }
    }

    pub fn path(&self) -> &Path {
        &self.path
    }

    /// Read and decrypt. A missing file is an empty store; a tampered file is an error.
    pub fn load(&self) -> Result<TokenFile, AppError> {
        let bytes = match std::fs::read(&self.path) {
            Ok(b) => b,
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => return Ok(TokenFile::default()),
            Err(e) => return Err(AppError::Auth(format!("cannot read token file: {e}"))),
        };
        if bytes.len() < NONCE_LEN + 16 {
            return Err(AppError::Auth("token file is truncated".into()));
        }
        let (nonce, ciphertext) = bytes.split_at(NONCE_LEN);
        let cipher = Aes256Gcm::new(&self.key);
        let plain = cipher
            .decrypt(Nonce::from_slice(nonce), ciphertext)
            .map_err(|_| {
                AppError::Auth(
                    "token file failed authentication (wrong machine or tampered file)".into(),
                )
            })?;
        serde_json::from_slice(&plain)
            .map_err(|e| AppError::Auth(format!("token file is malformed: {e}")))
    }

    /// Encrypt with a new nonce and write atomically with mode 0600.
    pub fn save(&self, file: &TokenFile) -> Result<(), AppError> {
        use std::io::Write;
        use std::os::unix::fs::OpenOptionsExt;
        if let Some(parent) = self.path.parent() {
            std::fs::create_dir_all(parent)?;
        }
        let plain = serde_json::to_vec(file)?;
        let cipher = Aes256Gcm::new(&self.key);
        let nonce = Aes256Gcm::generate_nonce(&mut OsRng);
        let ciphertext = cipher
            .encrypt(&nonce, plain.as_ref())
            .map_err(|_| AppError::Auth("encryption failed".into()))?;
        let mut out = Vec::with_capacity(NONCE_LEN + ciphertext.len());
        out.extend_from_slice(&nonce);
        out.extend_from_slice(&ciphertext);
        let tmp = self.path.with_extension("bin.tmp");
        {
            let mut f = std::fs::OpenOptions::new()
                .write(true)
                .create(true)
                .truncate(true)
                .mode(0o600)
                .open(&tmp)?;
            f.write_all(&out)?;
            f.sync_all()?;
        }
        std::fs::rename(&tmp, &self.path)?;
        std::fs::set_permissions(
            &self.path,
            std::os::unix::fs::PermissionsExt::from_mode(0o600),
        )?;
        Ok(())
    }

    pub fn get(&self, account_id: &str) -> Result<Option<AccountTokens>, AppError> {
        Ok(self.load()?.accounts.get(account_id).cloned())
    }

    pub fn put(&self, account_id: &str, tokens: AccountTokens) -> Result<(), AppError> {
        let mut file = self.load()?;
        file.accounts.insert(account_id.to_string(), tokens);
        self.save(&file)
    }

    pub fn remove(&self, account_id: &str) -> Result<(), AppError> {
        let mut file = self.load()?;
        let removed = file.accounts.remove(account_id).is_some()
            | file.ical_urls.remove(account_id).is_some();
        if removed {
            self.save(&file)?;
        }
        Ok(())
    }

    pub fn get_ical_url(&self, account_id: &str) -> Result<Option<String>, AppError> {
        Ok(self.load()?.ical_urls.get(account_id).cloned())
    }

    pub fn put_ical_url(&self, account_id: &str, url: &str) -> Result<(), AppError> {
        let mut file = self.load()?;
        file.ical_urls
            .insert(account_id.to_string(), url.to_string());
        self.save(&file)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::os::unix::fs::PermissionsExt;

    fn store(dir: &Path) -> TokenStore {
        TokenStore::with_material(
            dir.join("tokens.bin"),
            "0123456789abcdef0123456789abcdef",
            1000,
        )
    }

    #[test]
    fn round_trip_and_permissions() {
        let dir = tempfile::tempdir().unwrap();
        let s = store(dir.path());
        assert_eq!(s.load().unwrap(), TokenFile::default());
        s.put(
            "acc1",
            AccountTokens {
                refresh_token: "r1".into(),
                access_token: "a1".into(),
                expires_at: 42,
            },
        )
        .unwrap();
        s.put(
            "acc2",
            AccountTokens {
                refresh_token: "r2".into(),
                access_token: "a2".into(),
                expires_at: 43,
            },
        )
        .unwrap();
        let f = s.load().unwrap();
        assert_eq!(f.accounts.len(), 2);
        assert_eq!(f.accounts["acc1"].refresh_token, "r1");
        let mode = std::fs::metadata(s.path()).unwrap().permissions().mode() & 0o777;
        assert_eq!(mode, 0o600);
        // Plaintext never appears on disk.
        let bytes = std::fs::read(s.path()).unwrap();
        assert!(!bytes.windows(2).any(|w| w == b"r1"));
        s.remove("acc1").unwrap();
        assert!(s.get("acc1").unwrap().is_none());
        assert!(s.get("acc2").unwrap().is_some());
        s.put_ical_url(
            "ical1",
            "https://calendar.google.com/calendar/ical/x/private-abc/basic.ics",
        )
        .unwrap();
        assert!(s
            .get_ical_url("ical1")
            .unwrap()
            .unwrap()
            .contains("private-abc"));
        let bytes = std::fs::read(s.path()).unwrap();
        assert!(
            !bytes.windows(11).any(|w| w == b"private-abc"),
            "url is encrypted"
        );
        s.remove("ical1").unwrap();
        assert!(s.get_ical_url("ical1").unwrap().is_none());
    }

    #[test]
    fn nonce_changes_on_every_write() {
        let dir = tempfile::tempdir().unwrap();
        let s = store(dir.path());
        let f = TokenFile::default();
        s.save(&f).unwrap();
        let a = std::fs::read(s.path()).unwrap();
        s.save(&f).unwrap();
        let b = std::fs::read(s.path()).unwrap();
        assert_ne!(a[..12], b[..12]);
    }

    #[test]
    fn tampered_byte_fails_authentication() {
        let dir = tempfile::tempdir().unwrap();
        let s = store(dir.path());
        s.put(
            "acc1",
            AccountTokens {
                refresh_token: "r1".into(),
                access_token: "a1".into(),
                expires_at: 42,
            },
        )
        .unwrap();
        let mut bytes = std::fs::read(s.path()).unwrap();
        let last = bytes.len() - 1;
        bytes[last] ^= 0x01;
        std::fs::write(s.path(), &bytes).unwrap();
        let err = s.load().unwrap_err();
        assert!(matches!(err, AppError::Auth(_)));
        assert!(err.to_string().contains("authentication"));
    }

    #[test]
    fn different_machine_or_uid_cannot_read() {
        let dir = tempfile::tempdir().unwrap();
        let s = store(dir.path());
        s.put("acc1", AccountTokens::default()).unwrap();
        let other = TokenStore::with_material(
            dir.path().join("tokens.bin"),
            "ffffffffffffffffffffffffffffffff",
            1000,
        );
        assert!(other.load().is_err());
        let other_uid = TokenStore::with_material(
            dir.path().join("tokens.bin"),
            "0123456789abcdef0123456789abcdef",
            1001,
        );
        assert!(other_uid.load().is_err());
    }
}
