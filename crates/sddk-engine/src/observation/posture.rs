// Copyright (c) SDDK contributors.
// SPDX-License-Identifier: MIT
//
// observation/posture.rs — A4-4bR: subject-general evidence algebra.
//
// Cycle: `p-63676b11dc0ef88f/a4-4br-subject-general-evidence`
// ADR:   ADR-0125 (post-acceptance: A4-4bR section)
//
// # What this module introduces
//
// A4-4b generalized the evidence algebra by reusing
// `EvidenceResolution` — an enum that could only carry a `RelationId` as
// the target of resolution. A4-4bR generalizes that into
// `EvidencePosture<Target>`: a closed four-variant epistemic ADT whose
// target is a type parameter.
//
// The historical `EvidenceResolution` (relation-only) is preserved as a
// Rust type alias over the new shape:
//
// ```text
// pub type EvidenceResolution = EvidencePosture<RelationId>;
// ```
//
// so A4-0 and A4-2 relation-target consumers and tests do not change.
//
// The lens kernel introduces the subject-general specialization
// `LensEvidenceResolution = EvidencePosture<ObservationTargetRef>` so a
// lens may legitimately emit a contribution about a unit, a contract, a
// knowledge assertion, or a relation — **without** inventing synthetic
// `SoftwareRelation`s to satisfy a relation-only API.
//
// # Variants (unchanged shape, four only)
//
// ```text
// Supported      { target, supporting }
// Contradicted   { target, contradicting }
// Conflicted     { target, supporting, contradicting }   <- both sets preserved
// Insufficient   { target, gap }                         <- typed gap, no free-form string
// ```
//
// # Subject kinds
//
// ```text
// ObservationTargetRef
//  ├── Relation(RelationId)
//  ├── Unit(SoftwareUnitRef)
//  ├── Contract(ContractId)
//  └── Knowledge(KnowledgeId)
// ```
//
// `Unit`, `Contract`, `Knowledge` are first-class evidence targets.
// The target kind enters identity (a Unit contribution never collides
// with a Relation contribution even at the same identity-defining
// prefix), per the A4-4bR spec.
//
// # What this module deliberately does NOT introduce
//
// - No second persistence surface. `ObservationSet` is the only substrate.
// - No `SubjectObservationSet`, `LensObservationGraph`, `EvidenceGraph`,
//   `LensSupported2`, `LensUnknown2`, `Aligned2`, `Misaligned2`,
//   `Tension2`, `LensState`. The algebra is reused, not duplicated.
// - No rendered/string targets. Identity uses `canonical_tag()`.

use serde::Serialize;

use crate::architectural_contract::ContractId;
use crate::architecture_graph::SoftwareUnitRef;
use crate::knowledge::KnowledgeId;

use super::types::{ObservationId, RelationId};

// ─── Subject-general target reference ────────────────────────────────────────

/// Canonical subject of an evidence resolution.
///
/// `Relation` is the historical A4-0 case (resolution around a
/// `RelationId`). `Unit`, `Contract`, and `Knowledge` are first-class
/// targets introduced by A4-4bR; lenses may emit contributions on any
/// variant without inventing synthetic `SoftwareRelation`s.
#[derive(Clone, Debug, PartialEq, Eq, Hash, PartialOrd, Ord, Serialize)]
#[serde(rename_all = "snake_case", tag = "kind", content = "subject")]
pub enum ObservationTargetRef {
    /// A software relation — the historical A4-0 case.
    Relation(RelationId),
    /// A software unit on its own (AC7 paradigm-lens target).
    Unit(SoftwareUnitRef),
    /// An architectural contract.
    Contract(ContractId),
    /// A knowledge assertion.
    Knowledge(KnowledgeId),
}

impl ObservationTargetRef {
    /// Canonical short tag for the **target kind**. Used for identity
    /// (different kinds never collide) and for stable serialization.
    pub fn kind_tag(&self) -> &'static str {
        match self {
            ObservationTargetRef::Relation(_) => "relation",
            ObservationTargetRef::Unit(_) => "unit",
            ObservationTargetRef::Contract(_) => "contract",
            ObservationTargetRef::Knowledge(_) => "knowledge",
        }
    }

    /// Canonical full tag (kind + inner identity). Used when a target's
    /// own id is the only stable key. Inner strings come from each
    /// target's own canonical representation; we deliberately do NOT
    /// use rendered text.
    pub fn canonical_tag(&self) -> String {
        match self {
            ObservationTargetRef::Relation(r) => format!("relation:{}", r.as_str()),
            ObservationTargetRef::Unit(u) => format!("unit:{}", u.as_str()),
            ObservationTargetRef::Contract(c) => format!("contract:{}", c.as_str()),
            ObservationTargetRef::Knowledge(k) => format!("knowledge:{}", k.as_str()),
        }
    }
}

// ─── Subject-general evidence algebra ─────────────────────────────────────────

/// What the evidence for one [`ObservationTargetRef`] amounts to.
///
/// Four variants, closed. A4-4bR generalizes the historical
/// `EvidenceResolution` (relation-only) by parameterizing the target
/// type. The four-variant shape is invariant across target kinds;
/// only the `target` field's type changes.
#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case", tag = "posture")]
pub enum EvidencePosture<Target> {
    /// At least one observation affirms and none denies.
    Supported {
        /// The observed target.
        target: Target,
        /// The affirming observations.
        supporting: Vec<ObservationId>,
    },
    /// At least one observation denies and none affirms.
    Contradicted {
        /// The observed target.
        target: Target,
        /// The denying observations.
        contradicting: Vec<ObservationId>,
    },
    /// Both exist. **Not a tie to break**: the disagreement is the information.
    Conflicted {
        /// The observed target.
        target: Target,
        /// The affirming observations.
        supporting: Vec<ObservationId>,
        /// The denying observations.
        contradicting: Vec<ObservationId>,
    },
    /// Nothing covers the target, or provenance is missing. An explicit gap.
    Insufficient {
        /// The observed target.
        target: Target,
        /// Why there is no resolution. **Typed** for identity.
        gap: InsufficientGap,
    },
}

impl<Target> EvidencePosture<Target> {
    /// Canonical short tag for the **posture class**. Used for identity
    /// (the gap message NEVER enters identity — see
    /// [`InsufficientGap::canonical_tag`]).
    pub fn posture_class(&self) -> &'static str {
        match self {
            EvidencePosture::Supported { .. } => "supported",
            EvidencePosture::Contradicted { .. } => "contradicted",
            EvidencePosture::Conflicted { .. } => "conflicted",
            EvidencePosture::Insufficient { .. } => "insufficient",
        }
    }

    /// Whether any observation covered the target.
    pub fn is_covered(&self) -> bool {
        !matches!(self, EvidencePosture::Insufficient { .. })
    }

    /// Borrow the target.
    pub fn target(&self) -> &Target {
        match self {
            EvidencePosture::Supported { target, .. }
            | EvidencePosture::Contradicted { target, .. }
            | EvidencePosture::Conflicted { target, .. }
            | EvidencePosture::Insufficient { target, .. } => target,
        }
    }
}

// ─── Typed gap (no free-form string in identity) ──────────────────────────────

/// Typed reason an [`EvidencePosture::Insufficient`] posture was
/// produced. This replaces the historical `gap: String` field for
/// identity purposes: identity hashes `InsufficientGap::canonical_tag()`
/// (a stable `&'static str`), never a free-form message.
///
/// The wire form (for human-readable serialization) is generated from
/// this enum via [`InsufficientGap::wire_message`].
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, PartialOrd, Ord, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum InsufficientGap {
    /// No observation covers the target at all.
    NoObservation,
    /// Some observation covers the target but its stance is empty (no
    /// affirm, no deny) — the target is unobservable in practice.
    ObservationsWithoutStance,
    /// Provenance is missing (no `basis` or `evidence` attached to the
    /// covering observation). Distinct from "no observation" because the
    /// observation exists but cannot be reasoned about.
    MissingProvenance,
    /// The lens author explicitly declared "insufficient but I have a
    /// typed reason that is none of the above".
    LensDeclaredGap,
}

impl InsufficientGap {
    /// Canonical short tag. Used for identity.
    pub fn canonical_tag(self) -> &'static str {
        match self {
            Self::NoObservation => "no_observation",
            Self::ObservationsWithoutStance => "observations_without_stance",
            Self::MissingProvenance => "missing_provenance",
            Self::LensDeclaredGap => "lens_declared_gap",
        }
    }

    /// Wire form for serialization. This is a serialized message; it
    /// MUST NOT enter identity.
    pub fn wire_message(self) -> String {
        // Stable, deterministic, derived from the enum so the message
        // does not carry clock, locale, or random bytes.
        format!("insufficient:{}", self.canonical_tag())
    }

    /// Decode a wire message produced by [`InsufficientGap::wire_message`].
    ///
    /// This is the inverse of `wire_message` for messages we ourselves
    /// produced. Per A4-4bR the typed enum is the **only** representation
    /// allowed to reach identity; the wire form is a serialization
    /// convenience. Decoding foreign strings (one we did not produce)
    /// falls back to [`InsufficientGap::LensDeclaredGap`] so a malformed
    /// `gap: String` cannot smuggle free-form text into identity.
    pub fn from_wire_message(s: &str) -> Self {
        if let Some(tag) = s.strip_prefix("insufficient:") {
            match tag {
                "no_observation" => Self::NoObservation,
                "observations_without_stance" => Self::ObservationsWithoutStance,
                "missing_provenance" => Self::MissingProvenance,
                "lens_declared_gap" => Self::LensDeclaredGap,
                _ => Self::LensDeclaredGap,
            }
        } else {
            Self::LensDeclaredGap
        }
    }
}

// ─── Tests (subject-general, type-locked) ─────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;
    use crate::architecture_graph::SoftwareUnitRef;
    use crate::observation::types::RelationId;

    fn unit_a() -> SoftwareUnitRef {
        SoftwareUnitRef::new("crate::a")
    }

    fn rel() -> RelationId {
        RelationId::derive("from", "supports", "to")
    }

    #[test]
    fn observation_target_ref_kind_tag_is_stable() {
        // Pin: kind tag is stable across calls and is `&'static str`.
        assert_eq!(ObservationTargetRef::Relation(rel()).kind_tag(), "relation");
        assert_eq!(ObservationTargetRef::Unit(unit_a()).kind_tag(), "unit");
        assert_eq!(
            ObservationTargetRef::Contract(ContractId::new("c-1").expect("valid c-id")).kind_tag(),
            "contract"
        );
        assert_eq!(
            ObservationTargetRef::Knowledge(KnowledgeId::new("k-1").expect("valid k-id"))
                .kind_tag(),
            "knowledge"
        );
    }

    #[test]
    fn observation_target_ref_canonical_tag_includes_kind() {
        // Pin: even if inner strings match, the kind prefix differs.
        let tag_unit = ObservationTargetRef::Unit(SoftwareUnitRef::new("foo")).canonical_tag();
        let tag_relation =
            ObservationTargetRef::Relation(RelationId::derive("foo", "X", "foo")).canonical_tag();
        let tag_contract =
            ObservationTargetRef::Contract(ContractId::new("foo").expect("valid c-id"))
                .canonical_tag();
        let tag_knowledge =
            ObservationTargetRef::Knowledge(KnowledgeId::new("foo").expect("valid k-id"))
                .canonical_tag();
        // All four start with their kind prefix.
        assert!(tag_unit.starts_with("unit:"));
        assert!(tag_relation.starts_with("relation:"));
        assert!(tag_contract.starts_with("contract:"));
        assert!(tag_knowledge.starts_with("knowledge:"));
        // No two kinds ever produce the same canonical tag.
        assert_ne!(tag_unit, tag_relation);
        assert_ne!(tag_unit, tag_contract);
        assert_ne!(tag_unit, tag_knowledge);
        assert_ne!(tag_relation, tag_contract);
        assert_ne!(tag_relation, tag_knowledge);
        assert_ne!(tag_contract, tag_knowledge);
    }

    #[test]
    fn unit_foo_distinct_from_relation_foo_foo() {
        // The A4-4bR pin: Unit(foo) != Relation(foo --X--> foo).
        let unit_foo = ObservationTargetRef::Unit(SoftwareUnitRef::new("foo"));
        let rel_foo_foo = ObservationTargetRef::Relation(RelationId::derive("foo", "X", "foo"));
        assert_ne!(unit_foo, rel_foo_foo);
        assert_ne!(unit_foo.kind_tag(), rel_foo_foo.kind_tag());
    }

    #[test]
    fn posture_class_is_stable() {
        let unit = ObservationTargetRef::Unit(unit_a());
        let p: EvidencePosture<ObservationTargetRef> = EvidencePosture::Insufficient {
            target: unit,
            gap: InsufficientGap::NoObservation,
        };
        assert_eq!(p.posture_class(), "insufficient");
    }

    #[test]
    fn insufficient_gap_canonical_tag_does_not_carry_message() {
        // Identity uses canonical_tag() (a `&'static str`). Different
        // message text NEVER changes identity.
        let gap_a = InsufficientGap::LensDeclaredGap;
        let gap_b = InsufficientGap::LensDeclaredGap;
        assert_eq!(gap_a.canonical_tag(), gap_b.canonical_tag());
        assert_eq!(gap_a.wire_message(), gap_b.wire_message());
    }

    #[test]
    fn conflicted_preserves_both_sets() {
        let unit = ObservationTargetRef::Unit(unit_a());
        let a = ObservationId::derive(
            "s_a",
            "e_a",
            crate::observation::types::ObservationOrigin::DeterministicLocal,
            crate::observation::types::ObservationStance::Affirms,
            "b_a",
        );
        let b = ObservationId::derive(
            "s_b",
            "e_b",
            crate::observation::types::ObservationOrigin::DeterministicLocal,
            crate::observation::types::ObservationStance::Denies,
            "b_b",
        );
        let p: EvidencePosture<ObservationTargetRef> = EvidencePosture::Conflicted {
            target: unit,
            supporting: vec![a.clone()],
            contradicting: vec![b.clone()],
        };
        match p {
            EvidencePosture::Conflicted {
                supporting,
                contradicting,
                ..
            } => {
                assert_eq!(supporting, vec![a]);
                assert_eq!(contradicting, vec![b]);
            }
            _ => panic!("expected Conflicted"),
        }
    }
}
