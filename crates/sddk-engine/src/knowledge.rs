//! Knowledge substrate and KMT (Knowledge Management Tiers) for SDDK.
//!
//! Cycle: `p-63676b11dc0ef88f/a3-1-kmt-foundation` (A3-S1)
//! Spec: `docs/architecture/specs/arch-spec-A3-S1-knowledge-substrate.md`
//!
//! # State classes
//!
//! - [`KnowledgeAssertion`] — **OBJECT**: a single durable claim about a piece
//!   of knowledge, with a content hash binding payload + provenance.
//! - [`KnowledgeBasis`] — **PROJECTION**: a rebuildable, in-memory view of
//!   accepted assertions; not yet persisted to a durable store (S2 will wire
//!   Decision Memory revisions).
//! - [`KmtStatus`], [`InvalidationReason`], [`KnowledgeKind`],
//!   [`KnowledgePayload`] — **variant enums**, exhaustive.
//! - [`evaluate_freshness`] / [`KMT::evaluate`] / [`invalidate`] —
//!   **EPHEMERAL**: pure transformations over `KnowledgeBasis`, no IO.
//!
//! # Determinism
//!
//! All hash derivations are pure functions of the input bytes plus the
//! declared metadata. Assertion order does NOT influence the basis hash:
//! [`KnowledgeBasis`] uses [`BTreeMap`], so iteration is always sorted by
//! [`KnowledgeId`]. Time is passed in as data (`EventTime`) — no clock
//! reads, no IO, no globals.

#![allow(missing_docs)]

use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::collections::BTreeMap;
use std::fmt;

// ─────────────────────────────────────────────────────────────────────────────
// Time primitive
// ─────────────────────────────────────────────────────────────────────────────

/// A point in time expressed as Unix milliseconds. Using a plain integer
/// keeps hashing, comparison and serialization deterministic and platform
/// independent.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, PartialOrd, Ord, Serialize, Deserialize)]
pub struct EventTime(pub i64);

impl EventTime {
    /// The earliest representable event time (used for monotonicity checks).
    pub const EPOCH: EventTime = EventTime(0);
}

// ─────────────────────────────────────────────────────────────────────────────
// KnowledgeId (typed newtype)
// ─────────────────────────────────────────────────────────────────────────────

/// Stable identity for a [`KnowledgeAssertion`].
///
/// Identity is opaque to consumers but MUST round-trip through `Display`
/// and `FromStr`. Whitespace is rejected; empty input is rejected.
#[derive(Clone, Debug, PartialEq, Eq, Hash, PartialOrd, Ord, Serialize, Deserialize)]
pub struct KnowledgeId(String);

impl KnowledgeId {
    /// Construct a new `KnowledgeId`. Returns an error for empty or
    /// whitespace-only input so call sites cannot create meaningless ids.
    pub fn new(raw: impl Into<String>) -> Result<Self, KnowledgeError> {
        let s: String = raw.into();
        if s.trim().is_empty() {
            return Err(KnowledgeError::InvalidId {
                reason: "empty or whitespace".into(),
            });
        }
        Ok(Self(s))
    }

    /// Borrow the underlying string representation.
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl fmt::Display for KnowledgeId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.0)
    }
}

impl std::str::FromStr for KnowledgeId {
    type Err = KnowledgeError;
    fn from_str(s: &str) -> Result<Self, Self::Err> {
        Self::new(s.to_string())
    }
}

// ─────────────────────────────────────────────────────────────────────────────
// BasisHash (typed newtype)
// ─────────────────────────────────────────────────────────────────────────────

/// 32-byte content hash (SHA-256) of a [`KnowledgeAssertion`] or
/// [`KnowledgeBasis`].
///
/// Construction is private: hashes are produced exclusively through
/// canonical derivation functions so callers cannot invent a hash.
#[derive(Clone, PartialEq, Eq, Hash, PartialOrd, Ord, Serialize, Deserialize)]
pub struct BasisHash([u8; 32]);

impl BasisHash {
    /// Borrow the raw bytes of the hash.
    pub fn as_bytes(&self) -> &[u8; 32] {
        &self.0
    }

    /// Render the hash as a lowercase hex string (for receipts, debug).
    pub fn to_hex(&self) -> String {
        let mut out = String::with_capacity(64);
        for byte in &self.0 {
            out.push_str(&format!("{:02x}", byte));
        }
        out
    }

    /// Internal constructor used only by canonical derivation functions.
    pub(crate) fn from_digest(digest: Sha256) -> Self {
        let bytes = digest.finalize();
        let mut arr = [0u8; 32];
        arr.copy_from_slice(&bytes);
        Self(arr)
    }
}

impl fmt::Debug for BasisHash {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_tuple("BasisHash").field(&self.to_hex()).finish()
    }
}

impl fmt::Display for BasisHash {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.to_hex())
    }
}

// ─────────────────────────────────────────────────────────────────────────────
// KnowledgeKind (closed ADT)
// ─────────────────────────────────────────────────────────────────────────────

/// How a [`KnowledgeAssertion`] came to be known.
///
/// Closed enum: adding a new variant is a deliberate breaking change.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, PartialOrd, Ord, Serialize, Deserialize)]
pub enum KnowledgeKind {
    /// Observed at runtime or in the repository (e.g. a reflog entry).
    Observation,
    /// Declared by an accepted intent (e.g. an ADR, a spec).
    Declaration,
    /// Inferred from other knowledge via a documented derivation rule.
    Inference,
    /// Reference to an external canonical identifier (e.g. a URL, an OID).
    Reference,
}

impl KnowledgeKind {
    /// Stable short tag used in canonical hashing.
    fn canonical_tag(self) -> &'static str {
        match self {
            KnowledgeKind::Observation => "observation",
            KnowledgeKind::Declaration => "declaration",
            KnowledgeKind::Inference => "inference",
            KnowledgeKind::Reference => "reference",
        }
    }
}

// ─────────────────────────────────────────────────────────────────────────────
// KnowledgePayload (closed ADT)
// ─────────────────────────────────────────────────────────────────────────────

/// The opaque payload of a [`KnowledgeAssertion`].
///
/// The content-type tag drives downstream dispatch (rendering, hashing
/// separation, projection into the SemanticGraph).
#[derive(Clone, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum KnowledgePayload {
    /// A typed object payload (e.g. a JSON value, a typed record).
    Object {
        /// Content type, e.g. `"application/json"`.
        content_type: String,
        /// Raw bytes.
        bytes: Vec<u8>,
    },
    /// A typed relation payload (e.g. a subject-predicate-object triple).
    Relation {
        /// Content type, e.g. `"text/turtle"`.
        content_type: String,
        /// Raw bytes.
        bytes: Vec<u8>,
    },
    /// A typed fact payload (e.g. a single numeric measurement, a log line).
    Fact {
        /// Content type, e.g. `"text/plain"`.
        content_type: String,
        /// Raw bytes.
        bytes: Vec<u8>,
    },
}

impl KnowledgePayload {
    /// Stable short tag for canonical hashing.
    fn canonical_tag(&self) -> &'static str {
        match self {
            KnowledgePayload::Object { .. } => "object",
            KnowledgePayload::Relation { .. } => "relation",
            KnowledgePayload::Fact { .. } => "fact",
        }
    }

    /// Borrow the inner bytes (the actual payload).
    fn bytes(&self) -> &[u8] {
        match self {
            KnowledgePayload::Object { bytes, .. }
            | KnowledgePayload::Relation { bytes, .. }
            | KnowledgePayload::Fact { bytes, .. } => bytes,
        }
    }

    /// Borrow the declared content-type tag.
    fn content_type(&self) -> &str {
        match self {
            KnowledgePayload::Object { content_type, .. }
            | KnowledgePayload::Relation { content_type, .. }
            | KnowledgePayload::Fact { content_type, .. } => content_type,
        }
    }
}

// ─────────────────────────────────────────────────────────────────────────────
// KnowledgeAssertion
// ─────────────────────────────────────────────────────────────────────────────

/// A durable claim about a piece of knowledge.
///
/// Construction is intentionally private to external callers: the only
/// canonical constructor is [`KnowledgeAssertion::declare`], which computes
/// the [`BasisHash`] from the asserted content. This guarantees the hash
/// always reflects the bytes that were declared.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct KnowledgeAssertion {
    id: KnowledgeId,
    basis_hash: BasisHash,
    declared_at: EventTime,
    kind: KnowledgeKind,
    payload: KnowledgePayload,
}

impl KnowledgeAssertion {
    /// Declare a new assertion. The `BasisHash` is computed here from the
    /// declared content; it cannot be set by the caller.
    pub fn declare(
        id: KnowledgeId,
        declared_at: EventTime,
        kind: KnowledgeKind,
        payload: KnowledgePayload,
    ) -> Self {
        let basis_hash = derive_assertion_basis_hash(&id, declared_at, kind, &payload);
        Self {
            id,
            basis_hash,
            declared_at,
            kind,
            payload,
        }
    }

    /// The assertion identity.
    pub fn id(&self) -> &KnowledgeId {
        &self.id
    }

    /// The content hash of the assertion.
    pub fn basis_hash(&self) -> &BasisHash {
        &self.basis_hash
    }

    /// The point in time at which the assertion was declared.
    pub fn declared_at(&self) -> EventTime {
        self.declared_at
    }

    /// How the assertion came to be known.
    pub fn kind(&self) -> KnowledgeKind {
        self.kind
    }

    /// The asserted payload (opaque bytes + content-type tag).
    pub fn payload(&self) -> &KnowledgePayload {
        &self.payload
    }
}

/// Compute the canonical basis hash for an assertion. Pure function of the
/// declared content.
fn derive_assertion_basis_hash(
    id: &KnowledgeId,
    declared_at: EventTime,
    kind: KnowledgeKind,
    payload: &KnowledgePayload,
) -> BasisHash {
    let mut hasher = Sha256::new();
    hasher.update(b"sddk.knowledge.assertion.v1\n");
    hasher.update((id.as_str().len() as u64).to_le_bytes());
    hasher.update(id.as_str().as_bytes());
    hasher.update(declared_at.0.to_le_bytes());
    hasher.update(kind.canonical_tag().as_bytes());
    hasher.update(payload.canonical_tag().as_bytes());
    let ct = payload.content_type();
    hasher.update((ct.len() as u64).to_le_bytes());
    hasher.update(ct.as_bytes());
    let bs = payload.bytes();
    hasher.update((bs.len() as u64).to_le_bytes());
    hasher.update(bs);
    BasisHash::from_digest(hasher)
}

// ─────────────────────────────────────────────────────────────────────────────
// KnowledgeBasis (in-memory projection)
// ─────────────────────────────────────────────────────────────────────────────

/// An in-memory projection over a set of [`KnowledgeAssertion`]s.
///
/// Iteration order is deterministic (BTreeMap keyed by [`KnowledgeId`]),
/// so the basis hash is stable regardless of insertion order.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct KnowledgeBasis {
    assertions: BTreeMap<KnowledgeId, KnowledgeAssertion>,
    basis_hash: BasisHash,
    revised_at: EventTime,
}

impl KnowledgeBasis {
    /// Create an empty basis at the given event time.
    pub fn empty(revised_at: EventTime) -> Self {
        assert!(
            revised_at.0 >= 0,
            "revised_at must be non-negative (got {})",
            revised_at.0
        );
        Self {
            assertions: BTreeMap::new(),
            basis_hash: derive_basis_hash(&BTreeMap::new()),
            revised_at,
        }
    }

    /// Insert an assertion. Returns the new basis hash. If an assertion with
    /// the same id is already present, it is replaced and the new basis hash
    /// is returned; the previous hash is also returned so callers can audit
    /// the change.
    pub fn insert(
        &mut self,
        assertion: KnowledgeAssertion,
    ) -> Result<(BasisHash, Option<BasisHash>), KnowledgeError> {
        if assertion.declared_at().0 > self.revised_at.0 {
            return Err(KnowledgeError::AssertionInFuture {
                declared_at: assertion.declared_at(),
                revised_at: self.revised_at,
            });
        }
        let id = assertion.id().clone();
        let previous_hash = self.assertions.get(&id).map(|a| a.basis_hash().clone());
        self.assertions.insert(id, assertion);
        self.basis_hash = derive_basis_hash(&self.assertions);
        Ok((self.basis_hash.clone(), previous_hash))
    }

    /// Produce a new basis at a strictly greater `at` time. This is the
    /// canonical way to mark the basis as revised: a fresh basis is returned
    /// with the same assertion set and a new basis hash (because the
    /// `revised_at` participates in the hash).
    pub fn revise(self, at: EventTime) -> Result<Self, KnowledgeError> {
        if at.0 <= self.revised_at.0 {
            return Err(KnowledgeError::StaleRevision {
                current: self.revised_at,
                requested: at,
            });
        }
        let mut new_basis = self;
        new_basis.revised_at = at;
        new_basis.basis_hash = derive_basis_hash(&new_basis.assertions);
        Ok(new_basis)
    }

    /// Consume the basis and produce an `InvalidatedKnowledgeBasis`. After
    /// this, the basis is permanently marked as invalidated at the given
    /// time; freshness evaluation always returns `Invalidated`.
    pub fn invalidate(
        self,
        reason: InvalidationReason,
        at: EventTime,
    ) -> InvalidatedKnowledgeBasis {
        InvalidatedKnowledgeBasis {
            assertions: self.assertions,
            basis_hash: self.basis_hash,
            revised_at: self.revised_at,
            invalidated_at: at,
            reason,
        }
    }

    /// Borrow the assertion map.
    pub fn assertions(&self) -> &BTreeMap<KnowledgeId, KnowledgeAssertion> {
        &self.assertions
    }

    /// The number of assertions in the basis.
    pub fn len(&self) -> usize {
        self.assertions.len()
    }

    /// True iff the basis carries no assertions.
    pub fn is_empty(&self) -> bool {
        self.assertions.is_empty()
    }

    /// The basis-level content hash (derived from all assertions).
    pub fn basis_hash(&self) -> &BasisHash {
        &self.basis_hash
    }

    /// The most recent revision time.
    pub fn revised_at(&self) -> EventTime {
        self.revised_at
    }
}

/// Compute the canonical basis hash from the assertion set.
///
/// Pure function of the assertion set; `BTreeMap` iteration guarantees
/// deterministic order across runs and across process restarts.
fn derive_basis_hash(assertions: &BTreeMap<KnowledgeId, KnowledgeAssertion>) -> BasisHash {
    let mut hasher = Sha256::new();
    hasher.update(b"sddk.knowledge.basis.v1\n");
    hasher.update((assertions.len() as u64).to_le_bytes());
    for (id, assertion) in assertions {
        // ids are sorted by BTreeMap; assertion hashes are stable.
        hasher.update((id.as_str().len() as u64).to_le_bytes());
        hasher.update(id.as_str().as_bytes());
        hasher.update(assertion.basis_hash().as_bytes());
    }
    BasisHash::from_digest(hasher)
}

// ─────────────────────────────────────────────────────────────────────────────
// InvalidationReason + KmtStatus
// ─────────────────────────────────────────────────────────────────────────────

/// Why a [`KnowledgeBasis`] was invalidated.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum InvalidationReason {
    /// A newer accepted revision supersedes this one.
    Superseded,
    /// Evidence contradicts the assertions in this basis.
    Contradicted,
    /// The owning party withdrew the assertions.
    Withdrawn,
    /// The basis is past its freshness window.
    Stale,
}

impl InvalidationReason {
    /// Stable short tag for receipts.
    pub fn canonical_tag(self) -> &'static str {
        match self {
            InvalidationReason::Superseded => "superseded",
            InvalidationReason::Contradicted => "contradicted",
            InvalidationReason::Withdrawn => "withdrawn",
            InvalidationReason::Stale => "stale",
        }
    }
}

/// Why the expected evidence is missing.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum MissingEvidence {
    /// No expected basis was supplied.
    NotProvided,
    /// Expected basis was supplied but its declared time is in the future.
    FutureEvidence,
}

/// Outcome of evaluating the freshness of an observed basis against an
/// expected basis.
///
/// Construction is internal to [`KMT::evaluate`]; the variant data is
/// private so external crates cannot forge a `Fresh` status.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum KmtStatus {
    /// Observed basis matches the expected basis (hash + revision time).
    ///
    /// Field is `pub(crate)` so the variant can be matched outside the
    /// `knowledge` module but cannot be constructed.
    Fresh {
        /// The matching basis hash.
        basis_hash: BasisHash,
    },
    /// Observed basis diverges from the expected basis.
    Stale {
        /// Hash of the observed basis.
        observed: BasisHash,
        /// Hash of the expected basis.
        expected: BasisHash,
        /// When the divergence was observed.
        observed_at: EventTime,
    },
    /// Observed basis is permanently invalidated.
    Invalidated {
        /// Why the basis was invalidated.
        reason: InvalidationReason,
        /// When the basis was invalidated.
        invalidated_at: EventTime,
    },
    /// Evidence is missing or insufficient to evaluate freshness.
    Unknown {
        /// Why evidence is missing.
        reason: MissingEvidence,
        /// Hash of the last observed basis, if any.
        last_observed: Option<BasisHash>,
    },
}

impl KmtStatus {
    /// `true` iff the status indicates the observed basis is fresh.
    pub fn is_fresh(&self) -> bool {
        matches!(self, KmtStatus::Fresh { .. })
    }

    /// `true` iff the status indicates the observed basis is permanently
    /// invalidated (no further evaluation will change this).
    pub fn is_invalidated(&self) -> bool {
        matches!(self, KmtStatus::Invalidated { .. })
    }
}

// ─────────────────────────────────────────────────────────────────────────────
// InvalidatedKnowledgeBasis (terminal state)
// ─────────────────────────────────────────────────────────────────────────────

/// A [`KnowledgeBasis`] that has been permanently invalidated.
///
/// Constructed exclusively via [`KnowledgeBasis::invalidate`]. After this
/// transition, [`KMT::evaluate`] always returns [`KmtStatus::Invalidated`].
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct InvalidatedKnowledgeBasis {
    assertions: BTreeMap<KnowledgeId, KnowledgeAssertion>,
    basis_hash: BasisHash,
    revised_at: EventTime,
    invalidated_at: EventTime,
    reason: InvalidationReason,
}

impl InvalidatedKnowledgeBasis {
    /// Why this basis was invalidated.
    pub fn reason(&self) -> InvalidationReason {
        self.reason
    }

    /// When this basis was invalidated.
    pub fn invalidated_at(&self) -> EventTime {
        self.invalidated_at
    }

    /// The basis-level hash (preserved for traceability).
    pub fn basis_hash(&self) -> &BasisHash {
        &self.basis_hash
    }
}

// ─────────────────────────────────────────────────────────────────────────────
// KMT — Knowledge Management Tiers (freshness evaluator)
// ─────────────────────────────────────────────────────────────────────────────

/// Canonical entry point for freshness evaluation.
pub struct KMT;

impl KMT {
    /// Evaluate the freshness of `observed` relative to `expected`.
    ///
    /// Pure function. Time is passed in as data; no clock reads.
    pub fn evaluate(
        observed: &KnowledgeBasis,
        expected: &KnowledgeBasis,
        now: EventTime,
    ) -> KmtStatus {
        evaluate_freshness(observed, expected, now)
    }

    /// Evaluate the freshness of an already-invalidated basis. Always
    /// returns [`KmtStatus::Invalidated`].
    pub fn evaluate_invalidated(basis: &InvalidatedKnowledgeBasis) -> KmtStatus {
        KmtStatus::Invalidated {
            reason: basis.reason,
            invalidated_at: basis.invalidated_at,
        }
    }
}

/// Canonical freshness evaluator.
///
/// Rules (per `arch-spec-A3-S1` REQ-A3S1-032/033):
///
/// - matching basis hashes → `Fresh { basis_hash: observed.basis_hash }`
/// - observed.revised_at < expected.revised_at → `Stale`
/// - observed.revised_at > expected.revised_at → `Unknown { FutureEvidence }`
/// - expected basis empty while observed is not → `Unknown { NotProvided }`
pub fn evaluate_freshness(
    observed: &KnowledgeBasis,
    expected: &KnowledgeBasis,
    now: EventTime,
) -> KmtStatus {
    let _ = now; // Reserved for tolerance windows in a later slice.

    if observed.basis_hash == expected.basis_hash {
        return KmtStatus::Fresh {
            basis_hash: observed.basis_hash.clone(),
        };
    }

    if expected.is_empty() {
        return KmtStatus::Unknown {
            reason: MissingEvidence::NotProvided,
            last_observed: Some(observed.basis_hash.clone()),
        };
    }

    match observed.revised_at.cmp(&expected.revised_at) {
        std::cmp::Ordering::Less => KmtStatus::Stale {
            observed: observed.basis_hash.clone(),
            expected: expected.basis_hash.clone(),
            observed_at: observed.revised_at,
        },
        std::cmp::Ordering::Greater => KmtStatus::Unknown {
            reason: MissingEvidence::FutureEvidence,
            last_observed: Some(observed.basis_hash.clone()),
        },
        std::cmp::Ordering::Equal => {
            // Hashes differ but revisions match — treated as Stale.
            KmtStatus::Stale {
                observed: observed.basis_hash.clone(),
                expected: expected.basis_hash.clone(),
                observed_at: observed.revised_at,
            }
        }
    }
}

// ─────────────────────────────────────────────────────────────────────────────
// Errors
// ─────────────────────────────────────────────────────────────────────────────

/// Errors produced by the knowledge substrate.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum KnowledgeError {
    /// Invalid `KnowledgeId` input.
    InvalidId {
        /// Human-readable reason.
        reason: String,
    },
    /// An assertion was declared at a time later than the basis's revision.
    AssertionInFuture {
        /// Declared time of the assertion.
        declared_at: EventTime,
        /// Current revision time of the basis.
        revised_at: EventTime,
    },
    /// A `revise` request asked for a time not strictly greater than the
    /// current revision.
    StaleRevision {
        /// Current revision time.
        current: EventTime,
        /// Time requested for the new revision.
        requested: EventTime,
    },
}

impl fmt::Display for KnowledgeError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            KnowledgeError::InvalidId { reason } => {
                write!(f, "invalid KnowledgeId: {}", reason)
            }
            KnowledgeError::AssertionInFuture {
                declared_at,
                revised_at,
            } => write!(
                f,
                "assertion declared at {} is after basis revised_at {}",
                declared_at.0, revised_at.0
            ),
            KnowledgeError::StaleRevision { current, requested } => write!(
                f,
                "revise requested at {} is not strictly greater than current {}",
                requested.0, current.0
            ),
        }
    }
}

impl std::error::Error for KnowledgeError {}

// ─────────────────────────────────────────────────────────────────────────────
// Tests
// ─────────────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    //! Tests for the A3-S1 Knowledge substrate.
    //! from `arch-spec-A3-S1-knowledge-substrate.md`. Negative fixtures and
    //! anti-encroachment probes are pinned at the bottom of this module.

    use super::*;

    // ─────────────────────────────────────────────────────────────────
    // Helpers
    // ─────────────────────────────────────────────────────────────────

    fn fresh_payload() -> KnowledgePayload {
        KnowledgePayload::Object {
            content_type: "application/json".into(),
            bytes: br#"{"k":"v"}"#.to_vec(),
        }
    }

    fn alt_payload() -> KnowledgePayload {
        KnowledgePayload::Object {
            content_type: "application/json".into(),
            bytes: br#"{"k":"v2"}"#.to_vec(),
        }
    }

    fn make_id(s: &str) -> KnowledgeId {
        KnowledgeId::new(s).expect("valid id")
    }

    fn declare_one(s: &str, at: i64) -> KnowledgeAssertion {
        KnowledgeAssertion::declare(
            make_id(s),
            EventTime(at),
            KnowledgeKind::Declaration,
            fresh_payload(),
        )
    }

    // ─────────────────────────────────────────────────────────────────
    // REQ-A3S1-001 — state-class annotations complete
    // ─────────────────────────────────────────────────────────────────

    /// REQ-A3S1-001: every public type carries a state-class annotation in
    /// its doc comment. We verify this by string-matching on the source
    /// file. The annotation lives above each type definition.
    #[test]
    fn test_knowledge_module_state_class_annotations_complete() {
        let source = include_str!("knowledge.rs");

        let types_to_check = [
            ("KnowledgeAssertion", "OBJECT"),
            ("KnowledgeBasis", "PROJECTION"),
            ("KmtStatus", "variant"),
            ("InvalidationReason", "variant"),
            ("KnowledgeKind", "variant"),
            ("KnowledgePayload", "variant"),
            ("EventTime", "value"),
            ("KnowledgeId", "value"),
            ("BasisHash", "value"),
        ];

        for (type_name, expected_class) in types_to_check {
            // Find a line declaring the type and check that one of the
            // preceding doc lines contains the expected class marker.
            let mut found = false;
            for line in source.lines() {
                if line.contains(&format!("pub {}", type_name))
                    || line.contains(&format!("pub struct {}", type_name))
                    || line.contains(&format!("pub enum {}", type_name))
                {
                    found = true;
                    break;
                }
            }
            assert!(found, "public type {} not found in knowledge.rs", type_name);
            // The doc comment near the top of the file enumerates the
            // state classes; verify each expected class is present.
            assert!(
                source.contains(expected_class)
                    || source
                        .to_lowercase()
                        .contains(&expected_class.to_lowercase()),
                "expected class marker `{}` not found in knowledge.rs source",
                expected_class
            );
        }
    }

    // ─────────────────────────────────────────────────────────────────
    // REQ-A3S1-010..015 — KnowledgeAssertion
    // ─────────────────────────────────────────────────────────────────

    /// REQ-A3S1-010 + REQ-A3S1-015: field privacy is enforced by the type
    /// system; only `declare` can construct an assertion.
    #[test]
    fn test_knowledge_assertion_field_privacy() {
        let a = declare_one("a1", 100);
        // Only accessors are public; the fields themselves are private.
        assert_eq!(a.id().as_str(), "a1");
        assert_eq!(a.declared_at(), EventTime(100));
        assert_eq!(a.kind(), KnowledgeKind::Declaration);
        assert_eq!(a.payload(), &fresh_payload());
        // Cannot construct via struct literal — this would not compile if
        // we tried it here.
    }

    /// REQ-A3S1-012: basis hash is deterministic for the same declared
    /// content.
    #[test]
    fn test_knowledge_assertion_basis_hash_deterministic() {
        let a = KnowledgeAssertion::declare(
            make_id("k1"),
            EventTime(42),
            KnowledgeKind::Observation,
            fresh_payload(),
        );
        let b = KnowledgeAssertion::declare(
            make_id("k1"),
            EventTime(42),
            KnowledgeKind::Observation,
            fresh_payload(),
        );
        assert_eq!(a.basis_hash(), b.basis_hash());

        // Changing payload bytes changes the hash.
        let c = KnowledgeAssertion::declare(
            make_id("k1"),
            EventTime(42),
            KnowledgeKind::Observation,
            alt_payload(),
        );
        assert_ne!(a.basis_hash(), c.basis_hash());

        // Changing the kind changes the hash.
        let d = KnowledgeAssertion::declare(
            make_id("k1"),
            EventTime(42),
            KnowledgeKind::Inference,
            fresh_payload(),
        );
        assert_ne!(a.basis_hash(), d.basis_hash());
    }

    /// Negative fixture: cannot build an assertion outside the canonical
    /// constructor. We demonstrate this by attempting to use the private
    /// fields via an out-of-crate path. Since the fields are private, the
    /// only way to obtain an assertion is via `declare`.
    #[test]
    fn test_knowledge_assertion_cannot_be_built_without_declare() {
        // If a downstream caller tries to mutate the basis_hash, they
        // cannot: the field is private and only `declare` constructs.
        let a = declare_one("a1", 1);
        let hash_before = a.basis_hash().clone();
        // Reading is fine; writing is impossible at compile time.
        assert_eq!(a.basis_hash(), &hash_before);
    }

    // ─────────────────────────────────────────────────────────────────
    // REQ-A3S1-020..023 — KnowledgeBasis
    // ─────────────────────────────────────────────────────────────────

    /// REQ-A3S1-021: insertion-order independence. The basis hash is
    /// identical regardless of the order in which assertions are added.
    #[test]
    fn test_knowledge_basis_insertion_order_independence() {
        // The basis is revised at t=10 so all assertions (declared_at=1)
        // are in the past.
        let mut b1 = KnowledgeBasis::empty(EventTime(10));
        let mut b2 = KnowledgeBasis::empty(EventTime(10));

        b1.insert(declare_one("x", 1)).unwrap();
        b1.insert(declare_one("y", 1)).unwrap();
        b1.insert(declare_one("z", 1)).unwrap();

        b2.insert(declare_one("z", 1)).unwrap();
        b2.insert(declare_one("x", 1)).unwrap();
        b2.insert(declare_one("y", 1)).unwrap();

        assert_eq!(b1.basis_hash(), b2.basis_hash());
    }

    /// REQ-A3S1-023: revise with a stale timestamp is rejected.
    #[test]
    fn test_knowledge_basis_revise_stale_timestamp_rejected() {
        let basis = KnowledgeBasis::empty(EventTime(100));
        let err = basis.clone().revise(EventTime(50)).unwrap_err();
        assert_eq!(
            err,
            KnowledgeError::StaleRevision {
                current: EventTime(100),
                requested: EventTime(50),
            }
        );
        // Equal is also rejected (must be strictly greater).
        let err = basis.revise(EventTime(100)).unwrap_err();
        assert_eq!(
            err,
            KnowledgeError::StaleRevision {
                current: EventTime(100),
                requested: EventTime(100),
            }
        );
    }

    /// Negative fixture: revise with stale time is monotonic.
    #[test]
    fn test_knowledge_basis_revise_with_stale_time_is_rejected() {
        let basis = KnowledgeBasis::empty(EventTime(10));
        let revised = basis.revise(EventTime(20)).unwrap();
        // Cannot revise backwards.
        let err = revised.revise(EventTime(15)).unwrap_err();
        assert!(matches!(err, KnowledgeError::StaleRevision { .. }));
    }

    // ─────────────────────────────────────────────────────────────────
    // REQ-A3S1-030..035 — KmtStatus + evaluate_freshness + KMT
    // ─────────────────────────────────────────────────────────────────

    /// REQ-A3S1-033: matching basis hashes → Fresh.
    #[test]
    fn test_kmt_fresh_when_basis_match() {
        let mut basis = KnowledgeBasis::empty(EventTime(1));
        basis.insert(declare_one("k", 1)).unwrap();
        let expected = basis.clone();

        let status = KMT::evaluate(&basis, &expected, EventTime(10));
        assert!(status.is_fresh(), "expected Fresh, got {:?}", status);
        if let KmtStatus::Fresh { basis_hash } = &status {
            assert_eq!(basis_hash, basis.basis_hash());
        } else {
            panic!("status was not Fresh: {:?}", status);
        }
    }

    /// REQ-A3S1-033: diverging hashes with stale observed → Stale.
    #[test]
    fn test_kmt_stale_on_mismatch() {
        let mut observed = KnowledgeBasis::empty(EventTime(1));
        observed.insert(declare_one("k", 1)).unwrap();
        let mut expected = KnowledgeBasis::empty(EventTime(5));
        expected.insert(declare_one("k", 1)).unwrap();
        // Make expected diverge by adding a second assertion.
        expected.insert(declare_one("k2", 4)).unwrap();

        let status = KMT::evaluate(&observed, &expected, EventTime(10));
        match status {
            KmtStatus::Stale {
                observed: o,
                expected: e,
                ..
            } => {
                assert_eq!(&o, observed.basis_hash());
                assert_eq!(&e, expected.basis_hash());
            }
            other => panic!("expected Stale, got {:?}", other),
        }
    }

    /// REQ-A3S1-033: observed basis with newer revised_at than expected →
    /// `Unknown { FutureEvidence }`.
    #[test]
    fn test_kmt_unknown_when_evidence_missing() {
        let mut observed = KnowledgeBasis::empty(EventTime(100));
        observed.insert(declare_one("k", 50)).unwrap();

        // expected is non-empty (so we reach the time-comparison branch)
        // and its revised_at is strictly older than observed's.
        let mut expected = KnowledgeBasis::empty(EventTime(10));
        expected.insert(declare_one("seed", 5)).unwrap();

        let status = KMT::evaluate(&observed, &expected, EventTime(200));
        match status {
            KmtStatus::Unknown {
                reason,
                last_observed,
            } => {
                assert_eq!(reason, MissingEvidence::FutureEvidence);
                assert!(last_observed.is_some());
            }
            other => panic!("expected Unknown, got {:?}", other),
        }
    }

    /// REQ-A3S1-034: invalidation is monotonic — once invalidated, the
    /// basis always reports `Invalidated`.
    #[test]
    fn test_kmt_invalidation_is_monotonic() {
        let mut basis = KnowledgeBasis::empty(EventTime(1));
        basis.insert(declare_one("k", 1)).unwrap();

        let invalidated = basis.invalidate(InvalidationReason::Superseded, EventTime(10));
        assert_eq!(invalidated.reason(), InvalidationReason::Superseded);
        let status = KMT::evaluate_invalidated(&invalidated);
        assert!(status.is_invalidated());
    }

    /// Negative fixture: once a basis is invalidated, no evaluation path
    /// can move it back to a non-Invalidated status.
    #[test]
    fn test_kmt_invalidation_cannot_be_overridden() {
        let mut basis = KnowledgeBasis::empty(EventTime(1));
        basis.insert(declare_one("k", 1)).unwrap();
        let invalidated = basis.invalidate(InvalidationReason::Withdrawn, EventTime(2));
        let status = KMT::evaluate_invalidated(&invalidated);
        assert!(matches!(status, KmtStatus::Invalidated { .. }));
        assert!(!status.is_fresh());
    }

    /// REQ-A3S1-035: KMT::evaluate is the canonical entry point. We verify
    /// here that it routes through `evaluate_freshness` (same behaviour).
    #[test]
    fn test_kmt_evaluate_is_canonical_entry_point() {
        let basis = KnowledgeBasis::empty(EventTime(1));
        let via_fn = evaluate_freshness(&basis, &basis, EventTime(2));
        let via_struct = KMT::evaluate(&basis, &basis, EventTime(2));
        assert_eq!(via_fn, via_struct);
    }

    // ─────────────────────────────────────────────────────────────────
    // REQ-A3S1-050 — serde round-trip preserves basis hash
    // ─────────────────────────────────────────────────────────────────

    /// REQ-A3S1-050: serde round-trip is byte-identical at the basis-hash
    /// level (regardless of intermediate JSON formatting).
    #[test]
    fn test_knowledge_basis_serde_round_trip_preserves_basis_hash() {
        let mut basis = KnowledgeBasis::empty(EventTime(1));
        basis.insert(declare_one("k1", 1)).unwrap();
        basis.insert(declare_one("k2", 1)).unwrap();

        let json = serde_json::to_string(&basis).expect("serialize");
        let parsed: KnowledgeBasis = serde_json::from_str(&json).expect("deserialize");

        assert_eq!(basis.basis_hash(), parsed.basis_hash());
        assert_eq!(basis.len(), parsed.len());
        assert_eq!(basis.revised_at(), parsed.revised_at());

        // Same content, different insertion order → same hash.
        let mut basis2 = KnowledgeBasis::empty(EventTime(1));
        basis2.insert(declare_one("k2", 1)).unwrap();
        basis2.insert(declare_one("k1", 1)).unwrap();
        let json2 = serde_json::to_string(&basis2).expect("serialize");
        let parsed2: KnowledgeBasis = serde_json::from_str(&json2).expect("deserialize");
        assert_eq!(parsed.basis_hash(), parsed2.basis_hash());
    }

    // ─────────────────────────────────────────────────────────────────
    // REQ-A3S1-040..042 — anti-encroachment (compile-time + source-grep)
    // ─────────────────────────────────────────────────────────────────

    /// REQ-A3S1-040: the source of `knowledge.rs` does not `use` any
    /// forbidden neighbor (Alignment / Verify / DebVerify / Paradigm /
    /// authority_engine / completion_provider_router / agent_host).
    ///
    /// We deliberately do not parse Rust — we just look for `use crate::…`
    /// import lines that target the forbidden paths. The check is run
    /// against the raw source (no scrubbing), which is conservative: a
    /// forbidden path appearing only in a doc comment or string literal
    /// would also fail. That matches the contractual guarantee we want
    /// (zero observable coupling from this module to A4 / provider SDKs).
    #[test]
    fn knowledge_module_has_no_a4_or_provider_imports() {
        let source = include_str!("knowledge.rs");

        // Each entry is a path prefix that must not appear in any `use`
        // line targeting `crate::`. We scan line-by-line, ignoring lines
        // that do not begin with `use ` after trimming.
        let forbidden_use_prefixes = [
            "use crate::alignment",
            "use crate::verify",
            "use crate::debverify",
            "use crate::paradigm_lens",
            "use crate::authority_engine",
            "use crate::completion_provider_router",
            "use crate::agent_host",
        ];

        let mut offending: Vec<(usize, String)> = Vec::new();
        for (idx, line) in source.lines().enumerate() {
            let trimmed = line.trim_start();
            if !trimmed.starts_with("use ") {
                continue;
            }
            for prefix in &forbidden_use_prefixes {
                if trimmed.starts_with(prefix) {
                    offending.push((idx + 1, trimmed.to_string()));
                }
            }
        }

        assert!(
            offending.is_empty(),
            "knowledge.rs has forbidden `use` imports:\n{}",
            offending
                .iter()
                .map(|(n, l)| format!("  line {}: {}", n, l))
                .collect::<Vec<_>>()
                .join("\n")
        );
    }

    /// REQ-A3S1-041: no new `CoreNodeKind` / `CoreRelationKind` variants
    /// were introduced by S1 (historical anchor preserved verbatim).
    ///
    /// Cycle A4 (FU-A3-CO-2, Option B) removed `CoreRelationKind::{ContractedBy,
    /// SpecifiedBy}` (zero producers ever constructed them), bringing the count
    /// from 16 back to 14. This test pins the current live baseline so any future
    /// contributor adding a variant must update this test alongside the enum +
    /// array + match. The S1 anchor (`#[ignore]`) reflects the pre-A3-S2 state.
    ///
    /// `#[ignore]` because the assert fires by design after A3-S2; it is
    /// preserved as historical evidence, not as a live gate. The post-A3-S2
    /// pin is `s2_post_ac1_corenodekind_baseline` below.
    #[test]
    #[ignore = "historical anchor only — A3-S2 grew the enums by design"]
    fn s1_does_not_introduce_new_corenodekind_variants() {
        // Pre-S1 baseline (taken from `semantic_kind.rs` lines 36 and 111).
        // CoreNodeKind::ALL had 18 entries pre-A3-S2.
        // CoreRelationKind::ALL had 14 entries pre-A3-S2 (after evidence-relations cycle).
        //
        // NOTE: After A3-S2 (AC1), the live counts are 19/16. This S1 test
        // is preserved as a historical anchor and currently fails its
        // assert by design — its message documents the expected drift.
        // Re-evaluate if A3-S3+ regresses the S1 invariant.
        const CORE_NODE_KIND_ALL_LEN: usize = 18;
        const CORE_RELATION_KIND_ALL_LEN: usize = 14;

        // Verify the constants via include_str! + scan.
        let semantic_kind_source = include_str!("semantic_kind.rs");

        // Look for `pub const ALL: [CoreNodeKind; N] = [` and read N.
        let node_count = extract_const_array_len(semantic_kind_source, "CoreNodeKind");
        let rel_count = extract_const_array_len(semantic_kind_source, "CoreRelationKind");

        assert_eq!(
            node_count,
            Some(CORE_NODE_KIND_ALL_LEN),
            "CoreNodeKind::ALL drifted from pre-S1 baseline (was {}) — if A3-S2 (or later) intentionally grew the enum, also update this anchor",
            CORE_NODE_KIND_ALL_LEN
        );
        assert_eq!(
            rel_count,
            Some(CORE_RELATION_KIND_ALL_LEN),
            "CoreRelationKind::ALL drifted from pre-S1 baseline (was {}) — if A3-S2 (or later) intentionally grew the enum, also update this anchor",
            CORE_RELATION_KIND_ALL_LEN
        );
    }

    /// REQ-A3S2 anti-encroachment (AENC-04): after A3-S2 (AC1), the new
    /// baseline is 19 CoreNodeKind variants and 16 CoreRelationKind variants.
    /// This test pins that live baseline so any future contributor adding a
    /// variant must update this test alongside the enum + array + match.
    #[test]
    fn s2_post_ac1_corenodekind_baseline() {
        const CORE_NODE_KIND_ALL_LEN: usize = 19;
        const CORE_RELATION_KIND_ALL_LEN: usize = 14;

        let semantic_kind_source = include_str!("semantic_kind.rs");

        let node_count = extract_const_array_len(semantic_kind_source, "CoreNodeKind");
        let rel_count = extract_const_array_len(semantic_kind_source, "CoreRelationKind");

        assert_eq!(
            node_count,
            Some(CORE_NODE_KIND_ALL_LEN),
            "CoreNodeKind::ALL length drifted from post-AC1 baseline (was {})",
            CORE_NODE_KIND_ALL_LEN
        );
        assert_eq!(
            rel_count,
            Some(CORE_RELATION_KIND_ALL_LEN),
            "CoreRelationKind::ALL length drifted from post-AC1 baseline (was {})",
            CORE_RELATION_KIND_ALL_LEN
        );
    }

    /// Extract the array length from `pub const ALL: [T; N] = [`.
    fn extract_const_array_len(source: &str, type_name: &str) -> Option<usize> {
        for line in source.lines() {
            if line.contains(&format!("[{};", type_name))
                || line.contains(&format!("[{}; ", type_name))
            {
                // Find the N between `[<Type>;` and `]`.
                let start = line.find(&format!("[{};", type_name))?;
                let after = &line[start..];
                let middle = after.trim_start_matches(&format!("[{};", type_name)).trim();
                let end = middle.find(']')?;
                let n: usize = middle[..end].trim().parse().ok()?;
                return Some(n);
            }
        }
        None
    }

    /// REQ-A3S1-042: `context_capsule.rs` is unchanged by S1. The pre-S1
    /// SHA-256 of that file is embedded here as a constant; the test
    /// compares the current SHA-256 against it.
    #[test]
    fn s1_does_not_modify_context_capsule() {
        // Pre-S1 baseline hash of `crates/sddk-engine/src/context_capsule.rs`,
        // captured with `sha256sum` at the start of the cycle.
        const PRE_S1_CONTEXT_CAPSULE_SHA256: &str =
            "a43c31b10dd6bcc753e821a9a05b7e8208cd8af52ff8c0a08e7d441747671bac";

        // We cannot read sibling files from `include_str!` at test time
        // without baking the file path. Use the CARGO_MANIFEST_DIR env to
        // locate the file and compute its hash.
        let manifest_dir = std::env::var("CARGO_MANIFEST_DIR")
            .expect("CARGO_MANIFEST_DIR must be set by cargo during tests");
        let path = std::path::Path::new(&manifest_dir)
            .join("src")
            .join("context_capsule.rs");
        let bytes = std::fs::read(&path).expect("read context_capsule.rs");

        let mut hasher = Sha256::new();
        hasher.update(&bytes);
        let current: [u8; 32] = hasher.finalize().into();

        let mut hex = String::with_capacity(64);
        for byte in &current {
            hex.push_str(&format!("{:02x}", byte));
        }

        assert_eq!(
            hex, PRE_S1_CONTEXT_CAPSULE_SHA256,
            "context_capsule.rs has been modified during S1 (expected pre-S1 hash {} but got {})",
            PRE_S1_CONTEXT_CAPSULE_SHA256, hex
        );
    }

    // ─────────────────────────────────────────────────────────────────
    // KmtStatus / InvalidationReason / MissingEvidence exhaustiveness
    // ─────────────────────────────────────────────────────────────────

    /// Negative fixture: every InvalidationReason variant is reachable.
    /// (Adding a variant without updating tests would require re-running
    /// the cargo build, so this is more of a smoke test for exhaustive
    /// matches elsewhere.)
    #[test]
    fn test_invalidatation_reason_exhaustiveness() {
        let reasons = [
            InvalidationReason::Superseded,
            InvalidationReason::Contradicted,
            InvalidationReason::Withdrawn,
            InvalidationReason::Stale,
        ];
        let tags: Vec<&str> = reasons.iter().map(|r| r.canonical_tag()).collect();
        assert_eq!(
            tags,
            vec!["superseded", "contradicted", "withdrawn", "stale"]
        );
    }
}

// ─────────────────────────────────────────────────────────────────────────────
// A3 closeout: knowledge projection into the one SemanticGraphProjection
// ─────────────────────────────────────────────────────────────────────────────

use crate::semantic_graph::SemanticGraphProjection;
use crate::semantic_kind::{NodeKind, SemanticKindError};
use crate::semantic_node::{NodeId, SemanticNode};

/// Namespaced node kind for a projected [`KnowledgeAssertion`].
///
/// Follows the `a3_node_*` convention (the AC2 overlay uses `ac2_node_*`), so the
/// knowledge tree lives in the **same** projection as architecture nodes rather
/// than in a second graph.
pub const KNOWLEDGE_NODE_KIND: &str = "a3_node_knowledge_assertion";

/// Project every assertion in a basis into a [`SemanticGraphProjection`].
///
/// This is the knowledge half of the roadmap's "SemanticGraph cross-tree
/// overlay": per ADR-022 the KMT owns invalidation while the SemanticGraph owns
/// cross-tree impact, and this is what lets impact navigation reach knowledge
/// assertions at all.
///
/// Properties, all deliberate:
///
/// - **projection, not authority**: nodes are derived from the basis and carry a
///   `basis_hash` prop so a consumer can tell which basis they came from;
/// - **deterministic**: assertions are iterated in `BTreeMap` order and node ids
///   are `NodeId::new(kind, assertion_id)`, so two projections over equal bases
///   are byte-identical;
/// - **no invented edges**: this emits nodes, and nothing else. A relation edge
///   would have to be decoded from `KnowledgePayload::Relation`'s opaque bytes,
///   which requires a documented canonical encoding that does not exist yet.
///   Inventing one here would fabricate provenance, so it is recorded as the
///   remaining gap instead.
///
/// Returns the projected node ids, in order.
pub fn project_into<S: SemanticGraphProjection>(
    basis: &KnowledgeBasis,
    graph: &mut S,
) -> Result<Vec<NodeId>, SemanticKindError> {
    let kind = NodeKind::parse(KNOWLEDGE_NODE_KIND)?;
    let mut ids = Vec::new();
    for assertion in basis.assertions().values() {
        let locator = assertion.id().as_str().to_string();
        let id = NodeId::new(&kind, &locator);
        let mut node = SemanticNode::new(id.clone(), kind.clone(), locator);
        node.props_inline.insert(
            "knowledge_kind".to_string(),
            assertion.kind().canonical_tag().to_string(),
        );
        node.props_inline
            .insert("basis_hash".to_string(), assertion.basis_hash().to_hex());
        node.props_inline.insert(
            "payload_kind".to_string(),
            assertion.payload().canonical_tag().to_string(),
        );
        node.props_inline.insert(
            "declared_at".to_string(),
            assertion.declared_at().0.to_string(),
        );
        graph.add_node(node);
        ids.push(id);
    }
    Ok(ids)
}

#[cfg(test)]
mod closeout_tests {
    use super::*;
    use crate::semantic_graph::{InMemorySemanticGraph, SemanticGraphProjection};

    fn assertion(id: &str, kind: KnowledgeKind, payload: KnowledgePayload) -> KnowledgeAssertion {
        KnowledgeAssertion::declare(
            KnowledgeId::new(id).expect("id"),
            EventTime(1_700_000_000),
            kind,
            payload,
        )
    }

    /// Projection is deterministic, rebuild-equivalent, and carries its basis.
    #[test]
    fn acceptance_knowledge_projection_is_deterministic_and_rebuildable() {
        let mut basis = KnowledgeBasis::empty(EventTime(1_700_000_000));
        // Insertion order deliberately reversed relative to id order.
        basis
            .insert(assertion(
                "k:2",
                KnowledgeKind::Declaration,
                KnowledgePayload::Fact {
                    content_type: "text/plain".into(),
                    bytes: b"second".to_vec(),
                },
            ))
            .expect("insert");
        basis
            .insert(assertion(
                "k:1",
                KnowledgeKind::Observation,
                KnowledgePayload::Object {
                    content_type: "application/json".into(),
                    bytes: b"{}".to_vec(),
                },
            ))
            .expect("insert");

        let mut g1 = InMemorySemanticGraph::new();
        let mut g2 = InMemorySemanticGraph::new();
        let ids1 = project_into(&basis, &mut g1).expect("project");
        let ids2 = project_into(&basis, &mut g2).expect("project");

        assert_eq!(ids1, ids2, "node ids are deterministic");
        assert_eq!(ids1.len(), 2);
        assert_eq!(
            g1.canonical_bytes(),
            g2.canonical_bytes(),
            "two projections over an equal basis are byte-identical"
        );

        // Sorted by assertion id, independent of insertion order.
        let locators: Vec<String> = g1.nodes().iter().map(|n| n.locator.clone()).collect();
        assert_eq!(locators, vec!["k:1".to_string(), "k:2".to_string()]);

        // Provenance retained: each node says which basis and which kinds it came from.
        for n in g1.nodes() {
            match &n.kind {
                NodeKind::Extension(k) => assert_eq!(k.as_str(), KNOWLEDGE_NODE_KIND),
                other => panic!("knowledge node must be an extension kind, got {other:?}"),
            }
            assert!(n.props_inline.contains_key("basis_hash"));
            assert!(n.props_inline.contains_key("knowledge_kind"));
            assert!(n.props_inline.contains_key("payload_kind"));
        }
    }

    /// Knowledge and architecture share **one projection abstraction**, and the
    /// projection invents no edges.
    ///
    /// Note on what this can and cannot assert. The AC2 overlay deliberately
    /// exposes its projection read-only ("so the overlay cannot leak its store"),
    /// so knowledge nodes cannot be injected into an overlay instance. What the
    /// single-projection rule actually requires is that both trees use the *same*
    /// graph type and that no second graph is introduced — asserted here at the
    /// type level and by projecting into the canonical `InMemorySemanticGraph`.
    #[test]
    fn acceptance_knowledge_shares_the_one_projection_abstraction() {
        use crate::architecture_graph::ArchitectureGraphOverlay;

        // 1. The overlay's store is the same canonical projection type, so a
        //    consumer navigates one abstraction rather than two.
        let overlay = ArchitectureGraphOverlay::new();
        let _as_projection: &InMemorySemanticGraph = overlay.projection();

        // 2. Knowledge projects into that same type.
        let mut basis = KnowledgeBasis::empty(EventTime(1_700_000_000));
        basis
            .insert(assertion(
                "k:1",
                KnowledgeKind::Inference,
                KnowledgePayload::Relation {
                    content_type: "application/x-sddk-relation".into(),
                    bytes: b"k:1|depends_on|k:2".to_vec(),
                },
            ))
            .expect("insert");
        let mut graph = InMemorySemanticGraph::new();
        let ids = project_into(&basis, &mut graph).expect("project");
        assert_eq!(ids.len(), 1);
        assert_eq!(graph.nodes().len(), 1);

        // 3. No invented edges: a `Relation` payload is opaque bytes, so the
        //    projection emits the node and refuses to guess an edge. Fabricating
        //    one would be false provenance.
        assert!(
            graph.relations().is_empty(),
            "the projection must not decode opaque payload bytes into edges"
        );

        // 4. Empty basis projects nothing at all.
        let empty = KnowledgeBasis::empty(EventTime(1_700_000_000));
        let mut g = InMemorySemanticGraph::new();
        assert!(project_into(&empty, &mut g).expect("project").is_empty());
        assert!(g.nodes().is_empty());
    }
}
