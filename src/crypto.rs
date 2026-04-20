//! AES-256-GCM encryption at rest for sensitive data files.
//!
//! Encrypted files are plain text with a `PRAXISENC1:` prefix followed by the
//! hex-encoded nonce (12 bytes = 24 hex chars) and ciphertext (including the
//! 16-byte authentication tag).  This keeps them diff-able enough for git while
//! remaining opaque to casual inspection.
//!
//! The 32-byte master key lives at `master.key` in the Praxis data directory
//! with 0600 permissions.  It is generated on `praxis init` and must never be
//! committed (the generated `.gitignore` excludes it).

use std::{fs, path::Path};

use aes_gcm::{
    Aes256Gcm,
    aead::{Aead, AeadCore, KeyInit, OsRng},
};
use anyhow::{Context, Result, bail};

const MAGIC: &str = "PRAXISENC1:";
const NONCE_HEX_LEN: usize = 24; // 12 bytes × 2

// ── Key management ────────────────────────────────────────────────────────────

/// Load the 32-byte master key from `path`, generating and persisting a new one
/// if the file does not yet exist.
pub fn load_or_generate_key(path: &Path) -> Result<[u8; 32]> {
    if path.exists() {
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            if let Ok(meta) = fs::metadata(path) {
                let mode = meta.permissions().mode() & 0o777;
                if mode != 0o600 {
                    log::warn!(
                        "master key at {} has permissions {:04o} — run: chmod 600 {}",
                        path.display(),
                        mode,
                        path.display()
                    );
                }
            }
        }
        let raw = fs::read(path)
            .with_context(|| format!("failed to read master key {}", path.display()))?;
        if raw.len() != 32 {
            bail!(
                "master key at {} has unexpected length {} (expected 32)",
                path.display(),
                raw.len()
            );
        }
        let mut key = [0u8; 32];
        key.copy_from_slice(&raw);
        return Ok(key);
    }

    let key = Aes256Gcm::generate_key(OsRng);
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)
            .with_context(|| format!("failed to create {}", parent.display()))?;
    }
    fs::write(path, key.as_slice())
        .with_context(|| format!("failed to write master key to {}", path.display()))?;
    set_permissions_600(path);
    let mut out = [0u8; 32];
    out.copy_from_slice(&key);
    Ok(out)
}

// ── Encrypt / decrypt ─────────────────────────────────────────────────────────

/// Encrypt `plaintext` with the given key and return the on-disk encoding:
/// `PRAXISENC1:<hex(nonce 12B)><hex(ciphertext + 16B tag)>`
pub fn encrypt(key: &[u8; 32], plaintext: &[u8]) -> Result<String> {
    let cipher = Aes256Gcm::new_from_slice(key)
        .map_err(|_| anyhow::anyhow!("failed to initialise cipher"))?;
    let nonce = Aes256Gcm::generate_nonce(OsRng);
    let ciphertext = cipher
        .encrypt(&nonce, plaintext)
        .map_err(|e| anyhow::anyhow!("encryption failed: {e}"))?;
    Ok(format!(
        "{MAGIC}{}{}",
        to_hex(nonce.as_slice()),
        to_hex(&ciphertext)
    ))
}

/// Decrypt a `PRAXISENC1:`-prefixed blob produced by [`encrypt`].
pub fn decrypt(key: &[u8; 32], encoded: &str) -> Result<Vec<u8>> {
    let hex = encoded
        .strip_prefix(MAGIC)
        .context("not a praxis encrypted blob")?;
    if hex.len() < NONCE_HEX_LEN {
        bail!("encrypted blob is too short");
    }
    let nonce_bytes = from_hex(&hex[..NONCE_HEX_LEN])?;
    let ct_bytes = from_hex(&hex[NONCE_HEX_LEN..])?;

    let cipher = Aes256Gcm::new_from_slice(key)
        .map_err(|_| anyhow::anyhow!("failed to initialise cipher"))?;
    let nonce = aes_gcm::Nonce::from_slice(&nonce_bytes);
    cipher
        .decrypt(nonce, ct_bytes.as_slice())
        .map_err(|e| anyhow::anyhow!("decryption failed: {e}"))
}

/// True if `raw` carries the encrypted-file magic prefix.
pub fn is_encrypted(raw: &str) -> bool {
    raw.starts_with(MAGIC)
}

// ── Key-aware helpers for file stores ────────────────────────────────────────

/// Decrypt `raw` if it is encrypted; otherwise return it unchanged.
///
/// The master key is loaded from `master.key` in the same directory as `path`.
/// If the key file does not exist the content is assumed to be plaintext.
pub fn maybe_decrypt(path: &Path, raw: &str) -> Result<String> {
    if !is_encrypted(raw) {
        return Ok(raw.to_string());
    }
    let key = load_or_generate_key(&key_path_for(path))?;
    let plaintext = decrypt(&key, raw)?;
    String::from_utf8(plaintext).context("decrypted content is not valid UTF-8")
}

/// Encrypt `raw` before writing if the master key exists next to `path`.
///
/// If no `master.key` is found the content is returned unchanged (plaintext
/// fallback for installations that have not run `praxis init`).
pub fn maybe_encrypt(path: &Path, raw: &str) -> Result<String> {
    let kp = key_path_for(path);
    if !kp.exists() {
        return Ok(raw.to_string());
    }
    let key = load_or_generate_key(&kp)?;
    encrypt(&key, raw.as_bytes())
}

fn key_path_for(file: &Path) -> std::path::PathBuf {
    file.parent()
        .unwrap_or_else(|| Path::new("."))
        .join("master.key")
}

// ── Hex helpers ───────────────────────────────────────────────────────────────

fn to_hex(bytes: &[u8]) -> String {
    bytes.iter().map(|b| format!("{b:02x}")).collect()
}

fn from_hex(s: &str) -> Result<Vec<u8>> {
    if !s.len().is_multiple_of(2) {
        bail!("invalid hex string (odd length)");
    }
    (0..s.len())
        .step_by(2)
        .map(|i| {
            u8::from_str_radix(&s[i..i + 2], 16)
                .map_err(|e| anyhow::anyhow!("invalid hex at position {i}: {e}"))
        })
        .collect()
}

// ── Platform permissions ──────────────────────────────────────────────────────

#[cfg(unix)]
fn set_permissions_600(path: &Path) {
    use std::os::unix::fs::PermissionsExt;
    let _ = fs::set_permissions(path, fs::Permissions::from_mode(0o600));
}

#[cfg(not(unix))]
fn set_permissions_600(_path: &Path) {}

// ── Tests ─────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use tempfile::TempDir;

    use super::*;

    fn tmp_key(dir: &TempDir) -> [u8; 32] {
        load_or_generate_key(&dir.path().join("master.key")).unwrap()
    }

    #[test]
    fn encrypt_decrypt_roundtrip() {
        let dir = TempDir::new().unwrap();
        let key = tmp_key(&dir);
        let plaintext = b"super secret vault data";
        let enc = encrypt(&key, plaintext).unwrap();
        assert!(enc.starts_with("PRAXISENC1:"));
        let dec = decrypt(&key, &enc).unwrap();
        assert_eq!(dec, plaintext);
    }

    #[test]
    fn different_nonces_each_call() {
        let dir = TempDir::new().unwrap();
        let key = tmp_key(&dir);
        let a = encrypt(&key, b"test").unwrap();
        let b = encrypt(&key, b"test").unwrap();
        assert_ne!(a, b, "same plaintext should produce different ciphertext");
    }

    #[test]
    fn wrong_key_fails_decryption() {
        let dir = TempDir::new().unwrap();
        let key1 = tmp_key(&dir);
        let key2 = [0u8; 32];
        let enc = encrypt(&key1, b"secret").unwrap();
        assert!(decrypt(&key2, &enc).is_err());
    }

    #[test]
    fn key_persists_across_loads() {
        let dir = TempDir::new().unwrap();
        let kp = dir.path().join("master.key");
        let k1 = load_or_generate_key(&kp).unwrap();
        let k2 = load_or_generate_key(&kp).unwrap();
        assert_eq!(k1, k2);
    }

    #[test]
    fn plaintext_passthrough() {
        let dir = TempDir::new().unwrap();
        let fake_path = dir.path().join("vault.toml");
        // No master.key yet → plaintext passthrough
        let raw = "[secrets]\n";
        let out = maybe_encrypt(&fake_path, raw).unwrap();
        assert_eq!(out, raw);
    }

    #[test]
    fn maybe_encrypt_then_maybe_decrypt() {
        let dir = TempDir::new().unwrap();
        // Generate key
        load_or_generate_key(&dir.path().join("master.key")).unwrap();
        let fake_path = dir.path().join("vault.toml");
        let raw = "sensitive content";
        let enc = maybe_encrypt(&fake_path, raw).unwrap();
        assert!(is_encrypted(&enc));
        let dec = maybe_decrypt(&fake_path, &enc).unwrap();
        assert_eq!(dec, raw);
    }
}
