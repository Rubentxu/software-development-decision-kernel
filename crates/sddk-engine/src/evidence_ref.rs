// Copyright (c) SDDK contributors.
// SPDX-License-Identifier: MIT
//
// evidence_ref.rs — T-05 (M1 ADR-0100 arch-spec-001 CA-002)
//
// Universal EvidenceRef + EvidenceBundle. Reused across planning,
// governed capabilities, and authority receipts. Dedupe via BTreeSet
// ordering computed from sha256(kind || locator || cas).

use crate::canonical_event_log::CasRef;
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::collections::BTreeSet;

/// Domain separation tag for evidence ref ordering.
const EVIDENCE_REF_DOMAIN: &str = "sddk.evidence_ref.v1";

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

// ---- Adapter: legacy EvidenceAttachmentV1 -> universal EvidenceRef ----
//
// Kept minimal so existing call sites can migrate without losing data.
// The legacy shape is preserved as a tuple (kind_string, locator, payload_bytes).

/// Legacy attachment shape, kept here only as an adapter source.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct EvidenceAttachmentV1 {
    pub kind_string: String,
    pub locator: String,
    pub payload_bytes: Vec<u8>,
}

impl From<EvidenceAttachmentV1> for EvidenceRef {
    fn from(v1: EvidenceAttachmentV1) -> Self {
        let kind = match v1.kind_string.as_str() {
            "planning" => EvidenceKind::Planning,
            "governance" => EvidenceKind::Governance,
            "authority" => EvidenceKind::Authority,
            "decision_memory" => EvidenceKind::DecisionMemory,
            _ => EvidenceKind::Adhoc,
        };
        let cas = if v1.payload_bytes.is_empty() {
            None
        } else {
            Some(CasRef::from_bytes(&v1.payload_bytes))
        };
        EvidenceRef {
            kind,
            locator: v1.locator,
            cas,
        }
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
    fn adapter_converts_v1_without_data_loss() {
        let v1 = EvidenceAttachmentV1 {
            kind_string: "governance".into(),
            locator: "rule/G-42".into(),
            payload_bytes: b"rule-payload".to_vec(),
        };
        let r: EvidenceRef = v1.clone().into();
        assert_eq!(r.kind, EvidenceKind::Governance);
        assert_eq!(r.locator, "rule/G-42");
        let cas = r.cas.expect("non-empty payload must yield CasRef");
        assert_eq!(cas, CasRef::from_bytes(b"rule-payload"));
        // Adapter does not modify the input.
        assert_eq!(v1.locator, "rule/G-42");
    }

    #[test]
    fn empty_payload_in_adapter_skips_cas() {
        let v1 = EvidenceAttachmentV1 {
            kind_string: "adhoc".into(),
            locator: "note/note-1".into(),
            payload_bytes: Vec::new(),
        };
        let r: EvidenceRef = v1.into();
        assert!(r.cas.is_none());
        assert_eq!(r.kind, EvidenceKind::Adhoc);
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
    fn unknown_kind_falls_back_to_adhoc() {
        let v1 = EvidenceAttachmentV1 {
            kind_string: "no_such_kind".into(),
            locator: "x".into(),
            payload_bytes: Vec::new(),
        };
        let r: EvidenceRef = v1.into();
        assert_eq!(r.kind, EvidenceKind::Adhoc);
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
