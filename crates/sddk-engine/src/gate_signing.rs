//! Local gate-receipt signing (Phase 9).
//!
//! Gate receipts are signed with HMAC-SHA256 using a local key stored in the
//! XDG state area. `release verify` re-verifies signatures — tampered receipts
//! fail closed.

use std::fs;
use std::path::Path;

use hmac::{Hmac, Mac};
use sha2::Sha256;

/// Errors emitted by gate signing.
#[derive(Debug, thiserror::Error)]
pub enum SignError {
    /// Key could not be created.
    #[error("key error: {0}")]
    Key(String),
    /// Signing failed.
    #[error("sign error: {0}")]
    Sign(String),
}

type HmacSha256 = Hmac<Sha256>;

/// Loads (or creates) the local gate-signing key.
///
/// The key lives at `<dir>/gate-signing.key` with mode 600; it is created
/// randomly on first use (idempotent).
pub fn load_or_create_key(dir: &Path) -> Result<String, SignError> {
    fs::create_dir_all(dir).map_err(|e| SignError::Key(format!("create dir: {e}")))?;
    let key_path = dir.join("gate-signing.key");
    if key_path.exists() {
        let key = fs::read_to_string(&key_path)
            .map_err(|e| SignError::Key(format!("read key: {e}")))?
            .trim()
            .to_string();
        if key.is_empty() {
            return Err(SignError::Key("key file is empty".into()));
        }
        return Ok(key);
    }
    // Generate a random 32-byte hex key.
    let mut bytes = [0u8; 32];
    getrandom::getrandom(&mut bytes).map_err(|e| SignError::Key(format!("rng: {e}")))?;
    let key = bytes.iter().map(|b| format!("{b:02x}")).collect::<String>();
    #[cfg(unix)]
    {
        use std::os::unix::fs::OpenOptionsExt;
        let mut options = fs::OpenOptions::new();
        options.write(true).create_new(true).mode(0o600);
        let mut file = options
            .open(&key_path)
            .map_err(|e| SignError::Key(format!("create key file: {e}")))?;
        use std::io::Write;
        file.write_all(key.as_bytes())
            .map_err(|e| SignError::Key(format!("write key: {e}")))?;
    }
    #[cfg(not(unix))]
    {
        fs::write(&key_path, &key).map_err(|e| SignError::Key(format!("write key: {e}")))?;
    }
    Ok(key)
}

/// Signs a payload with the key, returning the hex HMAC-SHA256.
pub fn sign_payload(payload: &str, key: &str) -> Result<String, SignError> {
    let mut mac = HmacSha256::new_from_slice(key.as_bytes())
        .map_err(|e| SignError::Sign(format!("key: {e}")))?;
    mac.update(payload.as_bytes());
    Ok(mac
        .finalize()
        .into_bytes()
        .iter()
        .map(|b| format!("{b:02x}"))
        .collect())
}

/// Verifies a payload against a signature (constant-time via HMAC compare).
pub fn verify_payload(payload: &str, signature: &str, key: &str) -> bool {
    let expected = match sign_payload(payload, key) {
        Ok(sig) => sig,
        Err(_) => return false,
    };
    // Constant-time comparison.
    if expected.len() != signature.len() {
        return false;
    }
    let a = expected.as_bytes();
    let b = signature.as_bytes();
    let mut diff = 0u8;
    for (x, y) in a.iter().zip(b.iter()) {
        diff |= x ^ y;
    }
    diff == 0
}

/// Canonical payload format for `archive.vault.complete` gate receipts.
///
/// Fields (pipe-delimited): `receipt_id|gate|transition|cycle_id|delivery_kind|content_hash|timestamp`
///
/// This format is verified by the CLI's `run_release_vault()` when emitting `vault-receipt.json`.
pub fn archive_vault_complete_payload(
    receipt_id: &str,
    gate: &str,
    transition: &str,
    cycle_id: &str,
    delivery_kind: &str,
    content_hash: &str,
    timestamp: &str,
) -> String {
    format!(
        "{}|{}|{}|{}|{}|{}|{}",
        receipt_id, gate, transition, cycle_id, delivery_kind, content_hash, timestamp
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn sign_then_verify_roundtrip() {
        let key = "k".repeat(64);
        let payload = r#"{"receipt_id":"r-1","outcome":"passed"}"#;
        let signature = sign_payload(payload, &key).unwrap();
        assert!(verify_payload(payload, &signature, &key));
    }

    #[test]
    fn tampered_payload_fails_verification() {
        let key = "k".repeat(64);
        let payload = r#"{"receipt_id":"r-1","outcome":"passed"}"#;
        let signature = sign_payload(payload, &key).unwrap();
        let tampered = r#"{"receipt_id":"r-1","outcome":"failed"}"#;
        assert!(!verify_payload(tampered, &signature, &key));
    }

    #[test]
    fn wrong_key_fails_verification() {
        let key = "k".repeat(64);
        let other = "j".repeat(64);
        let payload = "payload";
        let signature = sign_payload(payload, &key).unwrap();
        assert!(!verify_payload(payload, &signature, &other));
    }

    #[test]
    fn key_creation_is_idempotent() {
        let dir = std::env::temp_dir().join(format!("sddk-sign-test-{}", std::process::id()));
        let _ = fs::remove_dir_all(&dir);
        let key1 = load_or_create_key(&dir).unwrap();
        let key2 = load_or_create_key(&dir).unwrap();
        assert_eq!(key1, key2);
        assert_eq!(key1.len(), 64);
        fs::remove_dir_all(&dir).ok();
    }
}
