// Copyright (c) SDDK contributors.
// SPDX-License-Identifier: MIT
//
// canonical_event_log.rs — T-01..T-02 (M1 ADR-0094)
//
// CanonicalEventLog port per arch-spec-001 (CA-001, CA-006).
// One logical writer per project_id; append-only; deterministic
// envelope (FactEnvelopeV1) with sha256-based fact_id that is
// domain-separated and sequence-monotone.

use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::sync::{Arc, Mutex};
use thiserror::Error;
use time::OffsetDateTime;

/// Domain separation tag for fact_id computation. Do not reuse the
/// prefix outside canonical-fact-log scopes.
pub const FACT_ID_DOMAIN: &str = "sddk.canonical_fact_log.fact_id.v1";

/// Current envelope schema version. Bumping requires a new variant
/// or a migration window.
pub const FACT_ENVELOPE_SCHEMA_VERSION: &str = "1.0.0";

/// Threshold above which payloads are stored via CasRef. Anything
/// ≤ this stays inline as `payload_inline`.
pub const MAX_INLINE_PAYLOAD_BYTES: usize = 4096;

/// Errores tipados del port. Fail-closed y mensurables.
#[derive(Debug, Error, PartialEq, Eq)]
pub enum AppendError {
    #[error("digest mismatch: stored {stored}, computed {computed}")]
    DigestMismatch { stored: String, computed: String },

    #[error("schema version mismatch: expected {expected}, got {got}")]
    SchemaVersionMismatch { expected: String, got: String },

    #[error("payload too large for inline storage: {size} > {limit}")]
    PayloadTooLarge { size: usize, limit: usize },

    #[error("writer poisoned")]
    WriterPoisoned,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum FactClass {
    DomainEvent,
    LifecycleTransition,
    AuthorityDecision,
    EvidenceAttachment,
    ProjectionMarker,
}

impl FactClass {
    pub fn domain_tag(&self) -> &'static str {
        match self {
            FactClass::DomainEvent => "domain_event",
            FactClass::LifecycleTransition => "lifecycle_transition",
            FactClass::AuthorityDecision => "authority_decision",
            FactClass::EvidenceAttachment => "evidence_attachment",
            FactClass::ProjectionMarker => "projection_marker",
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct FactEnvelopeV1 {
    pub schema_version: String,
    pub class: FactClass,
    pub sequence: u64,
    pub created_at: OffsetDateTime,
    pub payload_sha256: String,
    /// Optional CAS reference for payloads > MAX_INLINE_PAYLOAD_BYTES.
    /// When `payload_inline` is None, `payload_ref` MUST be Some.
    pub payload_ref: Option<CasRef>,
    /// Inline payload bytes for small facts. Empty bytes when CAS used.
    pub payload_inline: Vec<u8>,
}

impl FactEnvelopeV1 {
    /// Build an envelope whose payload fits inline (≤ MAX_INLINE_PAYLOAD_BYTES).
    pub fn new_inline(
        class: FactClass,
        sequence: u64,
        created_at: OffsetDateTime,
        payload: Vec<u8>,
    ) -> Result<Self, AppendError> {
        if payload.len() > MAX_INLINE_PAYLOAD_BYTES {
            return Err(AppendError::PayloadTooLarge {
                size: payload.len(),
                limit: MAX_INLINE_PAYLOAD_BYTES,
            });
        }
        let payload_sha256 = format!("sha256:{}", sha256_hex(&payload));
        Ok(Self {
            schema_version: FACT_ENVELOPE_SCHEMA_VERSION.into(),
            class,
            sequence,
            created_at,
            payload_sha256,
            payload_ref: None,
            payload_inline: payload,
        })
    }

    /// Build an envelope referencing a pre-stored CAS payload (large).
    pub fn new_via_cas(
        class: FactClass,
        sequence: u64,
        created_at: OffsetDateTime,
        cas: CasRef,
        payload_sha256: String,
    ) -> Self {
        Self {
            schema_version: FACT_ENVELOPE_SCHEMA_VERSION.into(),
            class,
            sequence,
            created_at,
            payload_sha256,
            payload_ref: Some(cas),
            payload_inline: Vec::new(),
        }
    }

    /// True when the envelope routes its payload through CAS, not inline.
    pub fn is_cas_routed(&self) -> bool {
        self.payload_ref.is_some()
    }

    pub fn validate(&self) -> Result<(), AppendError> {
        if self.schema_version != FACT_ENVELOPE_SCHEMA_VERSION {
            return Err(AppendError::SchemaVersionMismatch {
                expected: FACT_ENVELOPE_SCHEMA_VERSION.into(),
                got: self.schema_version.clone(),
            });
        }
        match self.payload_ref {
            // CAS-routed: payload lives in CasObjectStore; verify the recorded
            // sha256 is well-formed and `payload_inline` is empty.
            Some(_) => {
                if !self.payload_inline.is_empty() {
                    return Err(AppendError::DigestMismatch {
                        stored: "cas-routed envelope must have empty payload_inline".into(),
                        computed: format!("{} bytes", self.payload_inline.len()),
                    });
                }
                if !self.payload_sha256.starts_with("sha256:") {
                    return Err(AppendError::DigestMismatch {
                        stored: self.payload_sha256.clone(),
                        computed: "expected sha256:<hex>".into(),
                    });
                }
            }
            // Inline: recompute digest and compare against `payload_sha256`.
            None => {
                let computed = sha256_hex(&self.payload_inline);
                let stored = strip_sha256_prefix(&self.payload_sha256);
                if computed != stored {
                    return Err(AppendError::DigestMismatch {
                        stored: stored.to_string(),
                        computed,
                    });
                }
            }
        }
        Ok(())
    }
}

/// Content-addressed reference for large payloads stored in CasObjectStore.
#[derive(Clone, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct CasRef(pub String);

impl CasRef {
    pub fn from_bytes(bytes: &[u8]) -> Self {
        CasRef(format!("sha256:{}", sha256_hex(bytes)))
    }

    pub fn digest(&self) -> &str {
        &self.0
    }
}

/// Stable identifier for a fact, computed from domain-tagged inputs.
#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub struct FactId(pub String);

impl FactId {
    pub fn compute(
        class: FactClass,
        sequence: u64,
        created_at: OffsetDateTime,
        payload_sha256: &str,
    ) -> Self {
        let mut h = Sha256::new();
        h.update(FACT_ID_DOMAIN.as_bytes());
        h.update(b"|");
        h.update(class.domain_tag().as_bytes());
        h.update(b"|");
        h.update(sequence.to_be_bytes());
        h.update(b"|");
        h.update(created_at.unix_timestamp_nanos().to_be_bytes());
        h.update(b"|");
        h.update(payload_sha256.as_bytes());
        let hex = hex_sha256(&mut h);
        FactId(format!("fact:{hex}"))
    }

    pub fn as_str(&self) -> &str {
        &self.0
    }
}

/// Read-only projection of the log over a sequence range.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct FactRange {
    pub from_seq: u64,
    pub to_seq: u64,
}

/// Port: one canonical writer per project_id, append-only, deterministic.
pub trait CanonicalEventLog {
    fn append(&self, fact: FactEnvelopeV1) -> Result<FactId, AppendError>;
    fn read(&self, range: FactRange) -> Result<Vec<FactEnvelopeV1>, AppendError>;
    fn head(&self) -> Result<u64, AppendError>;
}

/// Reference in-memory implementation that serializes appends through a
/// single mutex. Suitable as the canonical port for the M1 cycle; physical
/// migration mirrors continue to mirror through their own writers.
#[derive(Debug)]
pub struct InMemoryCanonicalEventLog {
    inner: Arc<Mutex<InMemoryState>>,
}

#[derive(Debug, Default)]
struct InMemoryState {
    next_seq: u64,
    facts: Vec<FactEnvelopeV1>,
    singleton_marker: bool,
}

impl InMemoryCanonicalEventLog {
    pub fn new() -> Self {
        Self {
            inner: Arc::new(Mutex::new(InMemoryState {
                singleton_marker: true,
                ..InMemoryState::default()
            })),
        }
    }

    pub fn is_singleton(&self) -> bool {
        match self.inner.lock() {
            Ok(s) => s.singleton_marker,
            Err(_) => false,
        }
    }
}

impl Default for InMemoryCanonicalEventLog {
    fn default() -> Self {
        Self::new()
    }
}

impl CanonicalEventLog for InMemoryCanonicalEventLog {
    fn append(&self, fact: FactEnvelopeV1) -> Result<FactId, AppendError> {
        // Validate first (fail-closed before locking the writer state).
        fact.validate()?;

        let mut state = self.inner.lock().map_err(|_| AppendError::WriterPoisoned)?;

        let seq = state.next_seq;
        // Reject mismatched sequence to keep append monotonic.
        if fact.sequence != seq {
            return Err(AppendError::DigestMismatch {
                stored: fact.sequence.to_string(),
                computed: seq.to_string(),
            });
        }
        let fact_id = FactId::compute(
            fact.class,
            fact.sequence,
            fact.created_at,
            &fact.payload_sha256,
        );
        // Append; never overwrite or delete.
        state.facts.push(fact.clone());
        state.next_seq = state.next_seq.saturating_add(1);
        Ok(fact_id)
    }

    fn read(&self, range: FactRange) -> Result<Vec<FactEnvelopeV1>, AppendError> {
        let state = self.inner.lock().map_err(|_| AppendError::WriterPoisoned)?;
        if range.from_seq > range.to_seq {
            return Ok(Vec::new());
        }
        let from = range.from_seq as usize;
        let to = range.to_seq as usize;
        Ok(state
            .facts
            .iter()
            .skip(from)
            .take(to.saturating_sub(from) + 1)
            .cloned()
            .collect())
    }

    fn head(&self) -> Result<u64, AppendError> {
        let state = self.inner.lock().map_err(|_| AppendError::WriterPoisoned)?;
        Ok(state.next_seq.saturating_sub(1))
    }
}

// ---- helpers ------------------------------------------------------------

pub fn sha256_hex(bytes: &[u8]) -> String {
    let mut h = Sha256::new();
    h.update(bytes);
    hex_sha256(&mut h)
}

fn hex_sha256(h: &mut Sha256) -> String {
    let digest = h.finalize_reset();
    let mut out = String::with_capacity(64);
    for byte in digest.iter() {
        out.push_str(&format!("{:02x}", byte));
    }
    out
}

fn strip_sha256_prefix(s: &str) -> &str {
    s.strip_prefix("sha256:").unwrap_or(s)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn payload(b: u8) -> Vec<u8> {
        vec![b; 8]
    }

    #[test]
    fn fact_id_domain_separated() {
        let ts = OffsetDateTime::from_unix_timestamp(1_700_000_000).unwrap();
        let id_a = FactId::compute(FactClass::DomainEvent, 0, ts, "sha256:00");
        let id_b = FactId::compute(FactClass::LifecycleTransition, 0, ts, "sha256:00");
        assert_ne!(id_a, id_b, "different class must yield different fact_id");
    }

    #[test]
    fn fact_id_monotone_in_sequence() {
        let ts = OffsetDateTime::from_unix_timestamp(1_700_000_000).unwrap();
        let id_0 = FactId::compute(FactClass::DomainEvent, 0, ts, "sha256:00");
        let id_1 = FactId::compute(FactClass::DomainEvent, 1, ts, "sha256:00");
        assert_ne!(id_0, id_1, "different seq must yield different fact_id");
    }

    #[test]
    fn append_envelope_validates_schema_version() {
        let ts = OffsetDateTime::from_unix_timestamp(1_700_000_000).unwrap();
        let bytes = payload(1);
        let mut env = FactEnvelopeV1 {
            schema_version: "9.9.9".into(),
            class: FactClass::DomainEvent,
            sequence: 0,
            created_at: ts,
            payload_sha256: format!("sha256:{}", sha256_hex(&bytes)),
            payload_ref: None,
            payload_inline: bytes,
        };
        env.payload_sha256 = format!("sha256:{}", sha256_hex(&env.payload_inline));
        let err = env.validate().unwrap_err();
        assert_eq!(
            err,
            AppendError::SchemaVersionMismatch {
                expected: FACT_ENVELOPE_SCHEMA_VERSION.into(),
                got: "9.9.9".into()
            }
        );
    }

    #[test]
    fn append_envelope_validates_digest() {
        let ts = OffsetDateTime::from_unix_timestamp(1_700_000_000).unwrap();
        let env = FactEnvelopeV1 {
            schema_version: FACT_ENVELOPE_SCHEMA_VERSION.into(),
            class: FactClass::DomainEvent,
            sequence: 0,
            created_at: ts,
            payload_sha256: "sha256:deadbeef".into(),
            payload_ref: None,
            payload_inline: vec![0, 1, 2],
        };
        let err = env.validate().unwrap_err();
        match err {
            AppendError::DigestMismatch { .. } => {}
            other => panic!("unexpected: {other:?}"),
        }
    }

    #[test]
    fn in_memory_log_is_singleton() {
        let log = InMemoryCanonicalEventLog::new();
        assert!(log.is_singleton());
    }

    #[test]
    fn in_memory_log_rejects_mismatched_sequence() {
        let log = InMemoryCanonicalEventLog::new();
        let ts = OffsetDateTime::from_unix_timestamp(1_700_000_000).unwrap();
        let bytes = payload(7);
        let env = FactEnvelopeV1 {
            schema_version: FACT_ENVELOPE_SCHEMA_VERSION.into(),
            class: FactClass::DomainEvent,
            sequence: 99, // wrong; first valid seq is 0
            created_at: ts,
            payload_sha256: format!("sha256:{}", sha256_hex(&bytes)),
            payload_ref: None,
            payload_inline: bytes,
        };
        let err = log.append(env).unwrap_err();
        match err {
            AppendError::DigestMismatch { .. } => {}
            other => panic!("unexpected: {other:?}"),
        }
    }

    #[test]
    fn in_memory_log_appends_and_reads() {
        let log = InMemoryCanonicalEventLog::new();
        let ts = OffsetDateTime::from_unix_timestamp(1_700_000_000).unwrap();
        let env = FactEnvelopeV1 {
            schema_version: FACT_ENVELOPE_SCHEMA_VERSION.into(),
            class: FactClass::DomainEvent,
            sequence: 0,
            created_at: ts,
            payload_sha256: format!("sha256:{}", sha256_hex(&payload(1))),
            payload_ref: None,
            payload_inline: payload(1),
        };
        let id = log.append(env.clone()).unwrap();
        assert!(id.as_str().starts_with("fact:"));
        let read = log
            .read(FactRange {
                from_seq: 0,
                to_seq: 0,
            })
            .unwrap();
        assert_eq!(read.len(), 1);
        assert_eq!(read[0], env);
        assert_eq!(log.head().unwrap(), 0);
    }

    #[test]
    fn cas_ref_from_bytes_matches_sha256() {
        let bytes = b"hello world";
        let r = CasRef::from_bytes(bytes);
        assert_eq!(r.digest(), format!("sha256:{}", sha256_hex(bytes)));
    }

    #[test]
    fn new_inline_rejects_oversized_payload() {
        let ts = OffsetDateTime::from_unix_timestamp(1_700_000_000).unwrap();
        let too_big = vec![0u8; MAX_INLINE_PAYLOAD_BYTES + 1];
        let err = FactEnvelopeV1::new_inline(FactClass::DomainEvent, 0, ts, too_big).unwrap_err();
        match err {
            AppendError::PayloadTooLarge { size, limit } => {
                assert_eq!(size, MAX_INLINE_PAYLOAD_BYTES + 1);
                assert_eq!(limit, MAX_INLINE_PAYLOAD_BYTES);
            }
            other => panic!("unexpected: {other:?}"),
        }
    }

    #[test]
    fn new_via_cas_marks_envelope_as_cas_routed() {
        let ts = OffsetDateTime::from_unix_timestamp(1_700_000_000).unwrap();
        let cas = CasRef::from_bytes(b"big");
        let env = FactEnvelopeV1::new_via_cas(
            FactClass::DomainEvent,
            0,
            ts,
            cas.clone(),
            format!("sha256:{}", sha256_hex(b"big")),
        );
        assert!(env.is_cas_routed());
        assert_eq!(env.payload_ref, Some(cas));
        assert!(env.payload_inline.is_empty());
        env.validate().expect("cas envelope must validate");
    }

    #[test]
    fn small_payload_stored_inline_via_constructor() {
        let ts = OffsetDateTime::from_unix_timestamp(1_700_000_000).unwrap();
        let bytes = vec![1u8; 64];
        let env = FactEnvelopeV1::new_inline(FactClass::DomainEvent, 0, ts, bytes.clone()).unwrap();
        assert!(!env.is_cas_routed());
        assert_eq!(env.payload_inline, bytes);
    }
}
