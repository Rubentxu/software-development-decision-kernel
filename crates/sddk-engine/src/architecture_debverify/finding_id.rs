//! Deterministic identity for an AC5 DebVerify finding (REQ-A3S15-001..003).
//!
//! A3-S14 made findings inspectable and showed that `subject` is not an identity:
//! `comp:dup` produces both a `shadow_authority` and a `contradiction`. The
//! traversal this cycle adds needs a handle that can be read from
//! `architecture findings` and passed back to `why architecture`, so an identity
//! is required and `subject` cannot supply it.

use crate::architectural_contract::ContractId;
use crate::architecture_debverify::DebVerifyFindingKind;
use sha2::{Digest, Sha256};

/// The substrate a finding id is derived against.
///
/// Every field is **clock-stable by construction**, and that is load-bearing
/// rather than tidy. The workflow is two invocations:
///
/// ```text
/// sddk architecture findings            -> shadow_authority <finding-id>
/// sddk why architecture <finding-id>     -> explanation
/// ```
///
/// With the default wall clock those are different `now` values. If the basis
/// moved with the clock the id could never be fed back, and the feature would be
/// unusable in exactly its primary use case.
///
/// `semantic_graph_digest` is therefore **not** part of the basis: the AC2
/// overlay embeds each linkage claim's `evaluated_at`, so that digest changes with
/// the clock. `contract_set_digest`, `revision` and `knowledge_basis` are measured
/// stable.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct FindingBasis {
    /// Exact named revision (the declaration's `revision`).
    pub revision: String,
    /// Human-readable knowledge basis.
    pub knowledge_basis: String,
    /// sha256 over the sorted contract basis hashes (AC4's digest, shared).
    pub contract_set_digest: [u8; 32],
}

impl FindingBasis {
    /// Construct a basis.
    pub fn new(
        revision: impl Into<String>,
        knowledge_basis: impl Into<String>,
        contract_set_digest: [u8; 32],
    ) -> Self {
        Self {
            revision: revision.into(),
            knowledge_basis: knowledge_basis.into(),
            contract_set_digest,
        }
    }
}

/// Deterministic identity of one AC5 finding on one basis.
#[derive(Clone, Debug, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct FindingId(pub String);

impl FindingId {
    /// Domain prefix for the derivation.
    pub const DOMAIN_PREFIX: &'static str = "sddk.architecture_finding.id.v1|";

    /// Derive the id from the basis and the finding's discriminating fields.
    ///
    /// Excluded on purpose:
    ///
    /// - **`message`** — free text that can change for a redaction alone, so it
    ///   must never participate in identity.
    /// - **`severity`** — a pure function of `kind` (`FindingSeverity` has three
    ///   variants assigned per kind). Including it would add no discrimination
    ///   while implying it could vary independently of the kind.
    ///
    /// `subjects` and `contract_ids` are taken as the audit already orders them
    /// (sorted, deduplicated), so the id is order-independent by construction.
    pub fn derive(
        basis: &FindingBasis,
        kind: DebVerifyFindingKind,
        subjects: &[String],
        contract_ids: &[ContractId],
    ) -> Self {
        let mut h = Sha256::new();
        h.update(Self::DOMAIN_PREFIX.as_bytes());
        h.update(basis.revision.as_bytes());
        h.update(b"|");
        h.update(basis.knowledge_basis.as_bytes());
        h.update(b"|");
        h.update(basis.contract_set_digest);
        h.update(b"|kind|");
        h.update(kind.canonical_tag().as_bytes());
        h.update(b"|subjects|");
        for s in subjects {
            h.update(s.as_bytes());
            h.update(b",");
        }
        h.update(b"|contracts|");
        for c in contract_ids {
            h.update(c.as_str().as_bytes());
            h.update(b",");
        }
        let hex: String = h.finalize().iter().map(|b| format!("{b:02x}")).collect();
        Self(hex)
    }

    /// Borrow the raw string.
    pub fn as_str(&self) -> &str {
        &self.0
    }

    /// Whether a string is shaped like a finding id (64 lowercase hex).
    ///
    /// **For diagnostics only.** Resolution never uses this: a declaration may
    /// legally carry a contract id of 64 hex, so shape cannot decide a namespace
    /// (`FindingId` resolution is namespace-based and fails closed on ambiguity).
    pub fn looks_like_id(s: &str) -> bool {
        s.len() == 64
            && s.bytes()
                .all(|b| b.is_ascii_digit() || (b'a'..=b'f').contains(&b))
    }
}

impl std::fmt::Display for FindingId {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(&self.0)
    }
}
