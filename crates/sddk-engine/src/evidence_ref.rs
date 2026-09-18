// Copyright (c) SDDK contributors.
// SPDX-License-Identifier: MIT.
//
// evidence_ref.rs — T-05 (M1 ADR-0100 arch-spec-001 CA-002)
//
// Universal EvidenceRef + EvidenceBundle. Reused across planning,
// governed capabilities, and authority receipts. Dedupe via BTreeSet
// ordering computed from sha256(kind || locator || cas).
//
// Post-A5-EVIDENCE-ATTACHMENT-MIGRATION-V1: the legacy adapter struct
// `EvidenceAttachmentV1` (carrying kind_string/locator/payload_bytes)
// has been removed. Universal `EvidenceRef` is the only productive
// evidence attachment shape. Producers must build `EvidenceRef`
// (or `EvidenceBundle`) directly; legacy compatibility for
// pre-MIGRATION_19 rows lives at the storage read boundary
// (`EvidenceAttachmentRecord::from_legacy_kind_tag`), not in this type.
//
// `EvidenceKind::from_domain_tag` now returns `Result<Self,
// UnknownEvidenceKind>` so a typo or unknown tag can never be silently
// absorbed into `Adhoc` (A5-EVIDENCE-ATTACHMENT-MIGRATION-V1 §M2).

use crate::canonical_event_log::CasRef;
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::collections::BTreeSet;

/// Domain separation tag for evidence ref ordering.
const EVIDENCE_REF_DOMAIN: &str = "sddk.evidence_ref.v1";

/// Typed error returned when an unknown `kind` tag cannot be resolved
/// against the closed [`EvidenceKind`] vocabulary.
///
/// Replaces the pre-migration `_ => EvidenceKind::Adhoc` silent fallback.
/// Fail-closed (A5-EVIDENCE-ATTACHMENT-MIGRATION-V1 §M2).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct UnknownEvidenceKind(pub String);

impl std::fmt::Display for UnknownEvidenceKind {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "unknown evidence kind: {:?} (expected: planning, governance, authority, decision_memory, adhoc)",
            self.0
        )
    }
}

impl std::error::Error for UnknownEvidenceKind {}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Serialize, Deserialize, PartialOrd, Ord)]
pub enum EvidenceKind {
    Planning,
    Governance,
    Authority,
    DecisionMemory,
    Adhoc,
}

impl EvidenceKind {
    pub fn domain_tag(&self) -> &'static str {
        match self {
            EvidenceKind::Planning => "planning",
            EvidenceKind::Governance => "governance",
            EvidenceKind::Authority => "authority",
            EvidenceKind::DecisionMemory => "decision_memory",
            EvidenceKind::Adhoc => "adhoc",
        }
    }

    /// Inverse of [`Self::domain_tag`] for the **closed** vocabulary.
    ///
    /// This is a codec for a closed enum, not locator parsing: the input is
    /// the literal tag this type emits, and the mapping is total over
    /// `ALL`. Unknown tags return `Err(UnknownEvidenceKind)` (fail closed)
    /// rather than a default, so a typo can never be silently absorbed
    /// into `Adhoc`.
    pub fn from_domain_tag(tag: &str) -> Result<Self, UnknownEvidenceKind> {
        match tag {
            "planning" => Ok(EvidenceKind::Planning),
            "governance" => Ok(EvidenceKind::Governance),
            "authority" => Ok(EvidenceKind::Authority),
            "decision_memory" => Ok(EvidenceKind::DecisionMemory),
            "adhoc" => Ok(EvidenceKind::Adhoc),
            other => Err(UnknownEvidenceKind(other.to_string())),
        }
    }
}

/// Universal evidence reference. Either carries an inline locator
/// string OR a CAS pointer (or both, but at least one).
///
/// `PartialEq`, `Eq`, and `Hash` are implemented manually so `BTreeSet`
/// dedupe remains field-level; ordering is intentionally separate
/// (`ordering_key` + `Ord`) from equality.
#[derive(Clone, Debug, Eq, Serialize, Deserialize)]
pub struct EvidenceRef {
    pub kind: EvidenceKind,
    pub locator: String,
    pub cas: Option<CasRef>,
}

impl EvidenceRef {
    pub fn new(kind: EvidenceKind, locator: impl Into<String>) -> Self {
        Self {
            kind,
            locator: locator.into(),
            cas: None,
        }
    }

    pub fn with_cas(mut self, cas: CasRef) -> Self {
        self.cas = Some(cas);
        self
    }

    /// Deterministic ordering key (sha256 of domain-tagged fields).
    /// Used by `EvidenceBundle` to enforce BTreeSet ordering.
    pub fn ordering_key(&self) -> String {
        let mut h = Sha256::new();
        h.update(EVIDENCE_REF_DOMAIN.as_bytes());
        h.update(b"|");
        h.update(self.kind.domain_tag().as_bytes());
        h.update(b"|");
        h.update(self.locator.as_bytes());
        h.update(b"|");
        h.update(
            self.cas
                .as_ref()
                .map(|c| c.digest().as_bytes())
                .unwrap_or(b""),
        );
        let digest = h.finalize();
        let mut out = String::with_capacity(64);
        for byte in digest.iter() {
            out.push_str(&format!("{:02x}", byte));
        }
        out
    }
}

/// `Ord` derives from `ordering_key()` so two `EvidenceRef`s compare
/// deterministically regardless of field-level declaration order.
impl PartialOrd for EvidenceRef {
    fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
        Some(self.cmp(other))
    }
}

impl Ord for EvidenceRef {
    fn cmp(&self, other: &Self) -> std::cmp::Ordering {
        self.ordering_key().cmp(&other.ordering_key())
    }
}

/// Ordered, deduplicated set of evidence references.
#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct EvidenceBundle {
    refs: BTreeSet<EvidenceRef>,
}

impl EvidenceBundle {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn from_refs<I: IntoIterator<Item = EvidenceRef>>(iter: I) -> Self {
        let mut s: BTreeSet<EvidenceRef> = BTreeSet::new();
        for r in iter {
            s.insert(r);
        }
        Self { refs: s }
    }

    pub fn add(&mut self, r: EvidenceRef) -> bool {
        self.refs.insert(r)
    }

    pub fn len(&self) -> usize {
        self.refs.len()
    }

    pub fn is_empty(&self) -> bool {
        self.refs.is_empty()
    }

    /// Iterates refs in deterministic (sha256-ascending) order.
    pub fn iter(&self) -> impl Iterator<Item = &EvidenceRef> {
        self.refs.iter()
    }

    /// True iff the bundle contains a ref whose ordering_key equals `r`'s.
    /// Uses `Ord`, since `BTreeSet` requires both `Eq` + `Ord`; but our
    /// `Ord` is derived from `ordering_key`, which is itself deterministic
    /// from the fields — so equal-key refs are equal by content here.
    pub fn contains(&self, r: &EvidenceRef) -> bool {
        self.refs.contains(r)
    }
}

/// Field-level equality is also satisfied when ordering_keys match, but
/// we keep `derive(PartialEq)` because content equality is the more
/// natural meaning for callers — `BTreeSet` content equality holds.
impl PartialEq for EvidenceRef {
    fn eq(&self, other: &Self) -> bool {
        self.kind == other.kind && self.locator == other.locator && self.cas == other.cas
    }
}

/// Hash mirrors PartialEq field-by-field; ordering_key is its own thing.
impl std::hash::Hash for EvidenceRef {
    fn hash<H: std::hash::Hasher>(&self, state: &mut H) {
        self.kind.hash(state);
        self.locator.hash(state);
        self.cas.hash(state);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn ordering_key_is_deterministic() {
        let a = EvidenceRef::new(EvidenceKind::Planning, "spec/SP-001");
        let b = EvidenceRef::new(EvidenceKind::Planning, "spec/SP-001");
        assert_eq!(a.ordering_key(), b.ordering_key());
    }

    #[test]
    fn from_domain_tag_total_over_closed_vocabulary() {
        for tag in [
            "planning",
            "governance",
            "authority",
            "decision_memory",
            "adhoc",
        ] {
            assert!(
                EvidenceKind::from_domain_tag(tag).is_ok(),
                "{tag} must resolve to a closed EvidenceKind variant"
            );
        }
    }

    #[test]
    fn from_domain_tag_fails_closed_on_unknown() {
        // Pre-migration this returned `Some(Adhoc)`. The post-migration
        // contract is typed error (A5-EVIDENCE-ATTACHMENT-MIGRATION-V1
        // §M2): a typo can never be silently absorbed.
        for unknown in ["", "log", "metric", "snapshot", "no_such_kind", "PLAN"] {
            let err =
                EvidenceKind::from_domain_tag(unknown).expect_err("unknown tags must fail closed");
            assert_eq!(err, UnknownEvidenceKind(unknown.to_string()));
            assert!(
                err.to_string().contains(unknown),
                "error message must carry the offending tag"
            );
        }
    }

    #[test]
    fn bundle_orders_refs_deterministically() {
        let mut bundle = EvidenceBundle::new();
        bundle.add(EvidenceRef::new(EvidenceKind::Authority, "auth-1"));
        bundle.add(EvidenceRef::new(EvidenceKind::Planning, "plan-1"));
        bundle.add(EvidenceRef::new(EvidenceKind::Planning, "plan-2"));
        // identical ref: dedupe (insertion returns false)
        assert!(!bundle.add(EvidenceRef::new(EvidenceKind::Planning, "plan-1")));
        assert_eq!(bundle.len(), 3);

        let keys: Vec<String> = bundle.iter().map(|r| r.ordering_key()).collect();
        let mut sorted = keys.clone();
        sorted.sort();
        assert_eq!(keys, sorted, "bundle iteration must be deterministic");
    }

    #[test]
    fn cross_call_site_happy_path() {
        let cas = CasRef::from_bytes(b"shared-payload");
        let mut bundle = EvidenceBundle::new();
        bundle.add(EvidenceRef::new(EvidenceKind::Planning, "workitem/W-1").with_cas(cas.clone()));
        bundle.add(EvidenceRef::new(EvidenceKind::Authority, "receipt/R-1").with_cas(cas.clone()));
        bundle.add(EvidenceRef::new(EvidenceKind::Governance, "rule/G-1").with_cas(cas.clone()));
        assert_eq!(bundle.len(), 3);
        let with_cas = EvidenceRef::new(EvidenceKind::Planning, "workitem/W-1").with_cas(cas);
        assert!(bundle.contains(&with_cas));
    }

    #[test]
    fn bundle_from_iter_dedupes_identical_refs() {
        let r1 = EvidenceRef::new(EvidenceKind::Planning, "shared");
        let r2 = EvidenceRef::new(EvidenceKind::Planning, "shared");
        let r3 = EvidenceRef::new(EvidenceKind::Planning, "other");
        let bundle = EvidenceBundle::from_refs([r1, r2, r3]);
        assert_eq!(bundle.len(), 2);
    }

    #[test]
    fn same_cas_two_refs_dedupe_by_full_key() {
        let cas = CasRef::from_bytes(b"shared");
        let r1 = EvidenceRef::new(EvidenceKind::Planning, "a").with_cas(cas.clone());
        let r2 = EvidenceRef::new(EvidenceKind::Planning, "b").with_cas(cas.clone());
        // Different locator → different ordering_key → both retained.
        let mut bundle = EvidenceBundle::new();
        assert!(bundle.add(r1));
        assert!(bundle.add(r2));
        assert_eq!(bundle.len(), 2);
    }
}
