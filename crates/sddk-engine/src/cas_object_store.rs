// Copyright (c) SDDK contributors.
// SPDX-License-Identifier: MIT
//
// cas_object_store.rs — T-03 (M1 arch-spec-001 CA-002)
//
// Content-addressed store for immutable `Object`-class payloads.
// Deterministic CasRef, dedupe of identical content, typed error
// for missing references. Bounded inline payloads go through
// canonical_event_log's CasRef field.

use crate::canonical_event_log::CasRef;
use sha2::{Digest, Sha256};
use std::collections::HashMap;
use std::sync::Mutex;
use thiserror::Error;

#[derive(Debug, Error, PartialEq, Eq)]
pub enum CasError {
    #[error("CAS payload not found: {digest}")]
    Missing { digest: String },
}

#[derive(Debug, Default)]
pub struct CasObjectStore {
    inner: Mutex<HashMap<String, Vec<u8>>>,
}

impl CasObjectStore {
    pub fn new() -> Self {
        Self::default()
    }

    /// Store `bytes` and return the deterministic CasRef. Idempotent:
    /// two puts of the same content return the same CasRef.
    pub fn put(&self, bytes: Vec<u8>) -> CasRef {
        let digest = sha256_hex(&bytes);
        let key = digest.clone();
        let mut state = self.inner.lock().expect("cas mutex poisoned");
        // Dedupe: do not double-allocate identical content.
        state.entry(key).or_insert(bytes);
        CasRef(format!("sha256:{digest}"))
    }

    /// Retrieve the bytes for a given reference. Returns
    /// `CasError::Missing { digest }` if absent (no panic).
    pub fn get(&self, reference: &CasRef) -> Result<Vec<u8>, CasError> {
        let state = self.inner.lock().expect("cas mutex poisoned");
        let key = strip_prefix(reference.digest());
        state.get(key).cloned().ok_or_else(|| CasError::Missing {
            digest: reference.digest().to_string(),
        })
    }

    /// Number of unique payloads currently stored.
    pub fn len(&self) -> usize {
        self.inner.lock().expect("cas mutex poisoned").len()
    }

    pub fn is_empty(&self) -> bool {
        self.len() == 0
    }
}

fn sha256_hex(bytes: &[u8]) -> String {
    let mut h = Sha256::new();
    h.update(bytes);
    let digest = h.finalize();
    let mut out = String::with_capacity(64);
    for byte in digest.iter() {
        out.push_str(&format!("{:02x}", byte));
    }
    out
}

fn strip_prefix(s: &str) -> &str {
    s.strip_prefix("sha256:").unwrap_or(s)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn put_returns_deterministic_ref() {
        let store = CasObjectStore::new();
        let bytes = b"canonical-payload";
        let r1 = store.put(bytes.to_vec());
        let r2 = store.put(bytes.to_vec());
        assert_eq!(r1, r2);
        assert!(r1.digest().starts_with("sha256:"));
    }

    #[test]
    fn deduplicates_identical_content() {
        let store = CasObjectStore::new();
        let _ = store.put(b"x".to_vec());
        let _ = store.put(b"x".to_vec());
        let _ = store.put(b"x".to_vec());
        assert_eq!(store.len(), 1);
    }

    #[test]
    fn missing_returns_typed_error() {
        let store = CasObjectStore::new();
        let ghost = CasRef::from_bytes(b"never-put");
        let err = store.get(&ghost).unwrap_err();
        match err {
            CasError::Missing { digest } => {
                assert_eq!(digest, ghost.digest());
            }
        }
    }

    #[test]
    fn get_returns_stored_bytes() {
        let store = CasObjectStore::new();
        let payload = b"round-trip".to_vec();
        let r = store.put(payload.clone());
        let got = store.get(&r).unwrap();
        assert_eq!(got, payload);
    }

    #[test]
    fn distinct_content_yields_distinct_ref() {
        let store = CasObjectStore::new();
        let r1 = store.put(b"alpha".to_vec());
        let r2 = store.put(b"beta".to_vec());
        assert_ne!(r1, r2);
        assert_eq!(store.len(), 2);
    }

    #[test]
    fn empty_store_returns_zero_len() {
        let store = CasObjectStore::new();
        assert!(store.is_empty());
        assert_eq!(store.len(), 0);
    }
}
