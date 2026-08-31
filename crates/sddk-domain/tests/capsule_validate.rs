//! Proptest: `ContextCapsuleRef::validate()` enforces its three invariants:
//! 1. SHA-256 format: 64 lowercase hex chars (no prefix).
//! 2. Size bound: ≤ 4096 bytes for the inline summary.
//! 3. Digest integrity: recomputed sha256 matches the declared sha256.
//!
//! Plus: `Pointer` variant always passes (CID resolution is a runtime concern).
//!
//! Cycle 3 REQ-K3-002 acceptance scenario 4.
//!
//! Strategy: generate a valid `Inline` capsule from random bytes, then assert
//! `validate()` returns Ok. Then mutate each invariant and assert the
//! corresponding `CapsuleError` variant. 100 iterations per arm.

#![cfg(test)]

use proptest::prelude::*;
use sddk_domain::workflow_run::{CapsuleError, ContextCapsuleRef};
use sha2::Digest;

/// Build a valid `Inline` capsule from `payload` (≤ 4096 bytes).
fn valid_inline(payload: &[u8]) -> ContextCapsuleRef {
    let sha = format!("{:064x}", sha2::Sha256::digest(payload));
    ContextCapsuleRef::Inline {
        summary: String::from_utf8_lossy(payload).to_string(),
        sha256: sha,
    }
}

proptest! {
    #![proptest_config(ProptestConfig::with_cases(100))]

    /// Property: a pointer capsule always validates.
    #[test]
    fn pointer_always_valid(cid in "\\PC+") {
        let cap = ContextCapsuleRef::Pointer { cid };
        prop_assert!(cap.validate().is_ok());
    }

    /// Property: a well-formed inline capsule (correct sha256, ≤ 4096 bytes)
    /// validates. Use printable ASCII so the payload roundtrips through
    /// `String::from_utf8_lossy` cleanly.
    #[test]
    fn valid_inline_validates(payload in "[\\x20-\\x7e]{0,4096}") {
        let cap = valid_inline(payload.as_bytes());
        prop_assert!(cap.validate().is_ok(), "valid inline must validate");
    }

    /// Property: an inline capsule with a corrupted sha256 returns
    /// `Sha256Mismatch`.
    #[test]
    fn corrupt_sha256_fails(payload in "[\\x20-\\x7e]{1,1024}") {
        let mut cap = valid_inline(payload.as_bytes());
        if let ContextCapsuleRef::Inline { sha256, .. } = &mut cap {
            // Flip a hex char in the middle (zero out the first 8 chars).
            let replacement: String = "0".repeat(8);
            *sha256 = format!("{replacement}{}", &sha256[8..]);
        }
        let result = cap.validate();
        prop_assert!(
            matches!(result, Err(CapsuleError::Sha256Mismatch { .. })),
            "corrupted sha256 must yield Sha256Mismatch, got {:?}",
            result
        );
    }

    /// Property: a sha256 with uppercase hex chars is rejected as
    /// `Sha256Malformed` (lowercase only). We find the first lowercase
    /// letter in the sha256 (a-f) and flip it to uppercase — guaranteed
    /// to produce a malformed sha256 because the digest's first 8 chars
    /// must contain an a-f letter.
    #[test]
    fn uppercase_sha256_rejected(payload in "[a-f]{1,64}") {
        let mut cap = valid_inline(payload.as_bytes());
        if let ContextCapsuleRef::Inline { sha256, .. } = &mut cap {
            // Find the first lowercase letter (a-f) and uppercase it.
            let bytes: Vec<char> = sha256.chars().collect();
            let pos = bytes
                .iter()
                .position(|c| matches!(c, 'a'..='f'))
                .expect("sha256 must contain at least one lowercase hex letter");
            let upper = bytes[pos].to_ascii_uppercase();
            let mut new_bytes = bytes;
            new_bytes[pos] = upper;
            *sha256 = new_bytes.into_iter().collect();
        }
        let result = cap.validate();
        prop_assert!(
            matches!(result, Err(CapsuleError::Sha256Malformed(_))),
            "uppercase sha256 must yield Sha256Malformed, got {:?}",
            result
        );
    }

    /// Property: a sha256 of wrong length (≠ 64 chars) is rejected as
    /// `Sha256Malformed`.
    #[test]
    fn wrong_length_sha256_rejected(payload in "[\\x20-\\x7e]{0,256}") {
        let mut cap = valid_inline(payload.as_bytes());
        if let ContextCapsuleRef::Inline { sha256, .. } = &mut cap {
            *sha256 = "deadbeef".to_string(); // 8 chars, not 64
        }
        let result = cap.validate();
        prop_assert!(
            matches!(result, Err(CapsuleError::Sha256Malformed(_))),
            "wrong-length sha256 must yield Sha256Malformed, got {:?}",
            result
        );
    }

    /// Property: an inline capsule larger than 4096 bytes returns
    /// `InlineTooLarge`.
    #[test]
    fn oversize_inline_rejected(payload in "[\\x20-\\x7e]{4097,8192}") {
        let cap = valid_inline(payload.as_bytes());
        let result = cap.validate();
        prop_assert!(
            matches!(result, Err(CapsuleError::InlineTooLarge { .. })),
            "oversize inline must yield InlineTooLarge, got {:?}",
            result
        );
    }
}

/// Regression test: a payload of exactly 4096 bytes (boundary) must pass.
#[test]
fn boundary_4096_bytes_passes() {
    let payload = vec![b'a'; 4096];
    let cap = valid_inline(&payload);
    assert!(cap.validate().is_ok(), "4096-byte inline must validate");
}
