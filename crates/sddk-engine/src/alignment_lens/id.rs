// Copyright (c) SDDK contributors.
// SPDX-License-Identifier: MIT
//
// alignment_lens/id.rs — A4-4b: content-addressed LensContributionId.
//
// A `LensContributionId` is `sha256(domain | lens_id | lens_version |
// concern | observation_set_canonical_tag | target_kind_tag |
// target_canonical_tag | posture_class_tag |
// sorted_evidence_ref_ordering_keys)`. It is the durable-shape
// identifier of a `LensContribution`. Identity **deliberately excludes**:
//
// - wall clock (`SystemTime::now`, `Instant::now`);
// - message text (renderer output, free-form `String` — including
//   `InsufficientGap::wire_message`, which is a wire-format
//   convenience derived from
//   [`InsufficientGap`](super::types::InsufficientGap) but **never**
//   enters identity);
// - registration order;
// - vector insertion order (refs are sorted by
//   [`EvidenceRef::ordering_key`] before hashing).
//
// A4-4bR: identity now also includes the **target kind** (Relation /
// Unit / Contract / Knowledge) and the target's own canonical tag.
// Two contributions with the same `(lens, lens_version, concern,
// observation_set, posture, refs)` but different target kinds (or
// different target ids of the same kind) carry DISTINCT ids.
//
// Two contributions produced from the same `(lens, lens_version,
// concern, observation_set, target_kind, target_canonical_tag,
// posture_class, sorted_evidence_refs)` carry the SAME id, regardless
// of when or where they were produced. This is the A4-4b pin
// **identity-excludes-clock-message-order** (spec §3 X15, pins
// §6.9 / §6.10) extended by A4-4bR to include target identity.

use serde::Serialize;
use sha2::{Digest, Sha256};

use crate::evidence_ref::EvidenceRef;
use crate::observation::LensEvidenceResolution;

use super::types::{LensId, LensVersion};
use crate::intent_universal_concern::UniversalConcern;

/// Domain separation tag for the contribution identity.
pub const LENS_CONTRIBUTION_ID_DOMAIN: &str = "sddk.alignment_lens.contribution.id.v1|";

/// Content-addressed identity of one [`LensContribution`](super::LensContribution).
///
/// Two contributions share an id iff all identity-defining inputs are
/// equal. See module docs.
#[derive(Clone, Debug, PartialEq, Eq, Hash, PartialOrd, Ord, Serialize)]
#[serde(transparent)]
pub struct LensContributionId(pub String);

impl LensContributionId {
    /// Borrow the underlying hex.
    pub fn as_str(&self) -> &str {
        &self.0
    }

    /// Construct from a raw hex. Caller is responsible for ensuring
    /// the hex was produced by [`derive_contribution_id`].
    pub fn from_hex(s: impl Into<String>) -> Self {
        Self(s.into())
    }
}

impl std::fmt::Display for LensContributionId {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(&self.0)
    }
}

/// Derive the contribution id deterministically.
///
/// `observation_set_canonical_tag` is a stable string summarizing the
/// observation set. The kernel constructs it from a sorted
/// concatenation of observation ids (or from a stable adapter inside
/// `ObservationSet`). The exact derivation is documented in
/// [`ObservationSet::canonical_tag`](crate::observation::ObservationSet::canonical_tag).
///
/// `evidence_refs` need NOT be pre-sorted; this function sorts them
/// internally by [`EvidenceRef::ordering_key`] before hashing. The
/// kernel never lets a `LensContribution` carry refs in non-deterministic
/// order — the kernel's contribution assembly pre-sorts (see
/// `kernel.rs`).
pub fn derive_contribution_id(
    lens_id: LensId,
    lens_version: LensVersion,
    concern: UniversalConcern,
    observation_set_canonical_tag: &str,
    evidence_resolution: &LensEvidenceResolution,
    evidence_refs: &[EvidenceRef],
) -> LensContributionId {
    let mut h = Sha256::new();
    h.update(LENS_CONTRIBUTION_ID_DOMAIN.as_bytes());
    h.update(lens_id.as_str().as_bytes());
    h.update(b"|");
    h.update(lens_version.canonical_tag().as_bytes());
    h.update(b"|");
    h.update(concern.canonical().as_bytes());
    h.update(b"|");
    h.update(observation_set_canonical_tag.as_bytes());
    h.update(b"|");
    // A4-4bR: identity includes the target kind AND the target's
    // canonical tag. Different targets → different ids, even at the
    // same posture class.
    h.update(evidence_resolution.target().kind_tag().as_bytes());
    h.update(b"|");
    h.update(evidence_resolution.target().canonical_tag().as_bytes());
    h.update(b"|");
    // Note: we hash only the posture class (`posture_class()` returns
    // a `&'static str` like "supported"), never the gap wire-message
    // of `Insufficient` or the observation-id vectors it carries. The
    // observation id vectors ARE hashed below via `sorted_ref_keys`,
    // but only when the lens opted to materialize them as
    // `evidence_refs` — those refs are the lens's claim of which
    // observations it depended on, not the resolution's internal
    // bookkeeping.
    h.update(evidence_resolution.posture_class().as_bytes());
    h.update(b"|");

    // Sort refs by their deterministic ordering key before hashing.
    // `EvidenceRef::ordering_key` returns owned `String`s; we sort
    // owned strings (no leaks, no `Box::leak`).
    let mut key_strings: Vec<String> = evidence_refs.iter().map(|r| r.ordering_key()).collect();
    key_strings.sort();
    for k in &key_strings {
        h.update(k.as_bytes());
        h.update(b"|");
    }

    let digest = h.finalize();
    let mut hex = String::with_capacity(64);
    for byte in digest.iter() {
        hex.push_str(&format!("{:02x}", byte));
    }
    LensContributionId(hex)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::evidence_ref::{EvidenceKind, EvidenceRef};
    use crate::intent_universal_concern::UniversalConcern;
    use crate::observation::types::ObservationId;
    use crate::observation::{InsufficientGap, ObservationTargetRef};

    fn lens() -> LensId {
        LensId::new("test_lens")
    }
    fn version() -> LensVersion {
        LensVersion::new(1, 0)
    }
    fn ref_a() -> EvidenceRef {
        EvidenceRef::new(EvidenceKind::Planning, "spec/A")
    }
    fn ref_b() -> EvidenceRef {
        EvidenceRef::new(EvidenceKind::Governance, "rule/B")
    }
    fn insufficient_unit() -> LensEvidenceResolution {
        let unit = crate::architecture_graph::SoftwareUnitRef::new("foo");
        LensEvidenceResolution::Insufficient {
            target: ObservationTargetRef::Unit(unit),
            gap: InsufficientGap::NoObservation,
        }
    }
    fn observation_tag() -> &'static str {
        "OBSTAG|universal_concern|unit_ref"
    }

    #[test]
    fn same_inputs_produce_same_id() {
        let id1 = derive_contribution_id(
            lens(),
            version(),
            UniversalConcern::Freshness,
            observation_tag(),
            &insufficient_unit(),
            &[ref_a(), ref_b()],
        );
        let id2 = derive_contribution_id(
            lens(),
            version(),
            UniversalConcern::Freshness,
            observation_tag(),
            &insufficient_unit(),
            &[ref_a(), ref_b()],
        );
        assert_eq!(id1, id2);
    }

    #[test]
    fn ref_insertion_order_does_not_change_id() {
        let id1 = derive_contribution_id(
            lens(),
            version(),
            UniversalConcern::Freshness,
            observation_tag(),
            &insufficient_unit(),
            &[ref_a(), ref_b()],
        );
        let id2 = derive_contribution_id(
            lens(),
            version(),
            UniversalConcern::Freshness,
            observation_tag(),
            &insufficient_unit(),
            &[ref_b(), ref_a()],
        );
        assert_eq!(id1, id2, "vector insertion order must not reach identity");
    }

    #[test]
    fn insufficient_gap_does_not_reach_identity() {
        // Identity uses posture_class() (a `&'static str` like
        // "insufficient"). Different gap wire-message texts MUST NOT
        // change identity.
        let mut r1 = insufficient_unit();
        let mut r2 = insufficient_unit();
        if let LensEvidenceResolution::Insufficient { ref mut gap, .. } = r1 {
            *gap = InsufficientGap::NoObservation;
        }
        if let LensEvidenceResolution::Insufficient { ref mut gap, .. } = r2 {
            *gap = InsufficientGap::MissingProvenance;
        }
        let id1 = derive_contribution_id(
            lens(),
            version(),
            UniversalConcern::Freshness,
            observation_tag(),
            &r1,
            &[ref_a()],
        );
        let id2 = derive_contribution_id(
            lens(),
            version(),
            UniversalConcern::Freshness,
            observation_tag(),
            &r2,
            &[ref_a()],
        );
        assert_eq!(
            id1, id2,
            "InsufficientGap variant MUST NOT reach identity — only posture_class() does"
        );
    }

    #[test]
    fn different_concern_changes_id() {
        let id1 = derive_contribution_id(
            lens(),
            version(),
            UniversalConcern::DependencyDirection,
            observation_tag(),
            &insufficient_unit(),
            &[ref_a()],
        );
        let id2 = derive_contribution_id(
            lens(),
            version(),
            UniversalConcern::Freshness,
            observation_tag(),
            &insufficient_unit(),
            &[ref_a()],
        );
        assert_ne!(id1, id2);
    }

    #[test]
    fn different_observation_set_canonical_tag_changes_id() {
        let id1 = derive_contribution_id(
            lens(),
            version(),
            UniversalConcern::DependencyDirection,
            "OBSTAG|A",
            &insufficient_unit(),
            &[ref_a()],
        );
        let id2 = derive_contribution_id(
            lens(),
            version(),
            UniversalConcern::DependencyDirection,
            "OBSTAG|B",
            &insufficient_unit(),
            &[ref_a()],
        );
        assert_ne!(id1, id2);
    }

    #[test]
    fn lens_version_is_in_identity() {
        let id1 = derive_contribution_id(
            lens(),
            LensVersion::new(1, 0),
            UniversalConcern::DependencyDirection,
            observation_tag(),
            &insufficient_unit(),
            &[ref_a()],
        );
        let id2 = derive_contribution_id(
            lens(),
            LensVersion::new(1, 1),
            UniversalConcern::DependencyDirection,
            observation_tag(),
            &insufficient_unit(),
            &[ref_a()],
        );
        assert_ne!(
            id1, id2,
            "different LensVersion -> different contribution id"
        );
    }

    #[test]
    fn different_posture_class_changes_id() {
        let supported = LensEvidenceResolution::Supported {
            target: ObservationTargetRef::Unit(crate::architecture_graph::SoftwareUnitRef::new(
                "foo",
            )),
            supporting: Vec::new(),
        };
        let id_supported = derive_contribution_id(
            lens(),
            version(),
            UniversalConcern::Freshness,
            observation_tag(),
            &supported,
            &[ref_a()],
        );
        let id_insufficient = derive_contribution_id(
            lens(),
            version(),
            UniversalConcern::Freshness,
            observation_tag(),
            &insufficient_unit(),
            &[ref_a()],
        );
        assert_ne!(
            id_supported, id_insufficient,
            "different posture class -> different id"
        );
    }

    #[test]
    fn different_target_kind_changes_id() {
        // A4-4bR pin: Unit(foo) and Relation(foo->foo) produce
        // distinct identities even at the same posture class.
        let unit =
            ObservationTargetRef::Unit(crate::architecture_graph::SoftwareUnitRef::new("foo"));
        let rel = ObservationTargetRef::Relation(crate::observation::types::RelationId::derive(
            "foo", "X", "foo",
        ));
        let p_unit: LensEvidenceResolution = LensEvidenceResolution::Insufficient {
            target: unit,
            gap: InsufficientGap::NoObservation,
        };
        let p_rel: LensEvidenceResolution = LensEvidenceResolution::Insufficient {
            target: rel,
            gap: InsufficientGap::NoObservation,
        };
        let id_unit = derive_contribution_id(
            lens(),
            version(),
            UniversalConcern::Freshness,
            observation_tag(),
            &p_unit,
            &[ref_a()],
        );
        let id_rel = derive_contribution_id(
            lens(),
            version(),
            UniversalConcern::Freshness,
            observation_tag(),
            &p_rel,
            &[ref_a()],
        );
        assert_ne!(
            id_unit, id_rel,
            "different target kind (Unit vs Relation) -> different id"
        );
    }

    #[test]
    fn different_target_within_same_kind_changes_id() {
        // A4-4bR pin: Unit(foo) and Unit(bar) produce distinct
        // identities even at the same posture class.
        let a: LensEvidenceResolution = LensEvidenceResolution::Insufficient {
            target: ObservationTargetRef::Unit(crate::architecture_graph::SoftwareUnitRef::new(
                "foo",
            )),
            gap: InsufficientGap::NoObservation,
        };
        let b: LensEvidenceResolution = LensEvidenceResolution::Insufficient {
            target: ObservationTargetRef::Unit(crate::architecture_graph::SoftwareUnitRef::new(
                "bar",
            )),
            gap: InsufficientGap::NoObservation,
        };
        let id_a = derive_contribution_id(
            lens(),
            version(),
            UniversalConcern::Freshness,
            observation_tag(),
            &a,
            &[ref_a()],
        );
        let id_b = derive_contribution_id(
            lens(),
            version(),
            UniversalConcern::Freshness,
            observation_tag(),
            &b,
            &[ref_a()],
        );
        assert_ne!(id_a, id_b, "different target id -> different id");
    }

    #[test]
    fn observation_id_is_inside_resolved_refs_only_when_lens_says_so() {
        let id_a = derive_contribution_id(
            lens(),
            version(),
            UniversalConcern::DependencyDirection,
            observation_tag(),
            &insufficient_unit(),
            &[ref_a()],
        );
        let id_b = derive_contribution_id(
            lens(),
            version(),
            UniversalConcern::DependencyDirection,
            observation_tag(),
            &insufficient_unit(),
            &[ref_b()],
        );
        assert_ne!(id_a, id_b);
        let _ = ObservationId::derive(
            "s",
            "e",
            crate::observation::types::ObservationOrigin::DeterministicLocal,
            crate::observation::types::ObservationStance::Affirms,
            "b",
        );
    }
}
