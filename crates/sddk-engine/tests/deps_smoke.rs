//! Smoke test: tokio runtime constructable from new deps.
//!
//! Confirms the tokio runtime dependency wires correctly into the crate —
//! `Runtime::new()` succeeds without missing-symbol errors.

use rand::{RngCore, SeedableRng};
use std::result::Result as StdResult;

#[test]
fn tokio_runtime_constructable() {
    // tokio::runtime::Runtime::new() returns Result<Runtime, std::io::Error> in tokio 1.x.
    let result: StdResult<tokio::runtime::Runtime, std::io::Error> = tokio::runtime::Runtime::new();
    assert!(
        result.is_ok(),
        "tokio::runtime::Runtime::new() must succeed — check that tokio features [rt, rt-multi-thread, time, sync] are correct"
    );
    let _runtime = result.unwrap();
}

#[test]
fn rand_crate_available() {
    // rand 0.8 is used in RetryPolicy for deterministic jitter.
    let mut rng = rand::rngs::StdRng::from_entropy();
    let mut buf = [0u8; 4];
    rng.fill_bytes(&mut buf);
    let sample = u32::from_ne_bytes(buf);
    assert_ne!(sample, 0, "rand::rngs::StdRng must produce non-zero bytes");
}

#[test]
fn sha2_crate_available() {
    // sha2 0.10 is re-exported via workspace but we test direct use.
    use sha2::{Digest, Sha256};
    let mut hasher = Sha256::new();
    hasher.update(b"hello world");
    let result = hasher.finalize();
    assert_eq!(result.len(), 32, "SHA-256 produces 32-byte digest");
}
