//! Contradiction-preserving resolution (REQ-A4S0-010..013).
//!
//! A pure function over an immutable set. It never mutates the set, never sorts the
//! evidence away, and never picks a winner.
//!
//! # A4-4bR: subject-general evidence resolution
//!
//! The historical [`EvidenceResolution`] enum (relation-only,
//! `gap: String`) is preserved as-is for A4-0/A4-2 consumers and
//! tests. The new subject-general [`EvidencePosture<Target>`] shape
//! lives in `super::posture`. The lens kernel uses
//! `EvidencePosture<ObservationTargetRef>` natively; this module
//! exposes the relation-only posture view as a thin convenience for
//! tests and consumers that want the typed gap on the historical
//! relation-only path.

use serde::Serialize;

use super::posture::{EvidencePosture, InsufficientGap, ObservationTargetRef};
use super::types::{ObservationId, ObservationSet, ObservationStance, RelationId};

// ─── Historical relation-only EvidenceResolution ──────────────────────────────

/// What the evidence for one relation amounts to.
///
/// A closed ADT, never a confidence number (REQ-A4S0-012).
///
/// The `gap: String` is preserved verbatim for A4-0/A4-2 wire
/// compatibility. The new subject-general [`EvidencePosture<Target>`]
/// uses a typed [`InsufficientGap`] enum for identity (no free-form
/// string in identity).
#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case", tag = "resolution")]
pub enum EvidenceResolution {
    /// At least one observation affirms and none denies.
    Supported {
        /// The relation.
        relation: RelationId,
        /// The affirming observations.
        supporting: Vec<ObservationId>,
    },
    /// At least one observation denies and none affirms.
    Contradicted {
        /// The relation.
        relation: RelationId,
        /// The denying observations.
        contradicting: Vec<ObservationId>,
    },
    /// Both exist. **Not a tie to break**: the disagreement is the information.
    Conflicted {
        /// The relation.
        relation: RelationId,
        /// The affirming observations.
        supporting: Vec<ObservationId>,
        /// The denying observations.
        contradicting: Vec<ObservationId>,
    },
    /// Nothing covers the relation, or provenance is missing. An explicit gap.
    Insufficient {
        /// The relation.
        relation: RelationId,
        /// Why there is no resolution.
        gap: String,
    },
}

impl EvidenceResolution {
    /// Canonical short tag.
    pub fn canonical_tag(&self) -> &'static str {
        match self {
            EvidenceResolution::Supported { .. } => "supported",
            EvidenceResolution::Contradicted { .. } => "contradicted",
            EvidenceResolution::Conflicted { .. } => "conflicted",
            EvidenceResolution::Insufficient { .. } => "insufficient",
        }
    }

    /// Whether any observation covered the relation.
    pub fn is_covered(&self) -> bool {
        !matches!(self, EvidenceResolution::Insufficient { .. })
    }
}

/// Lift a relation-only `EvidenceResolution` into the lens-side
/// `EvidencePosture<ObservationTargetRef>`.
///
/// This is the bridge used by relation-target lenses (Lens A and any
/// other lens whose subject lives in the relation substrate). The
/// relation's `RelationId` becomes an `ObservationTargetRef::Relation`.
/// A `gap: String` produced by an A4-0 path is decoded into a typed
/// [`InsufficientGap`] via [`InsufficientGap::from_wire_message`]; any
/// string we did not produce falls back to
/// [`InsufficientGap::LensDeclaredGap`], so foreign free-form text
/// cannot smuggle into identity.
impl From<EvidenceResolution> for EvidencePosture<ObservationTargetRef> {
    fn from(r: EvidenceResolution) -> Self {
        match r {
            EvidenceResolution::Supported {
                relation,
                supporting,
            } => EvidencePosture::Supported {
                target: ObservationTargetRef::Relation(relation),
                supporting,
            },
            EvidenceResolution::Contradicted {
                relation,
                contradicting,
            } => EvidencePosture::Contradicted {
                target: ObservationTargetRef::Relation(relation),
                contradicting,
            },
            EvidenceResolution::Conflicted {
                relation,
                supporting,
                contradicting,
            } => EvidencePosture::Conflicted {
                target: ObservationTargetRef::Relation(relation),
                supporting,
                contradicting,
            },
            EvidenceResolution::Insufficient { relation, gap } => EvidencePosture::Insufficient {
                target: ObservationTargetRef::Relation(relation),
                gap: InsufficientGap::from_wire_message(&gap),
            },
        }
    }
}

/// Resolve the evidence for one relation. Pure and deterministic.
///
/// Order-independent by construction: the inputs are partitioned by stance and the
/// ids are already in canonical order, so insertion order cannot reach the result
/// (REQ-A4S0-011).
pub fn resolve_relation(set: &ObservationSet, relation: &RelationId) -> EvidenceResolution {
    let covering = set.for_relation(relation);
    if covering.is_empty() {
        return EvidenceResolution::Insufficient {
            relation: relation.clone(),
            gap: "no observation covers this relation".to_string(),
        };
    }

    let mut supporting: Vec<ObservationId> = Vec::new();
    let mut contradicting: Vec<ObservationId> = Vec::new();
    for o in covering {
        match o.stance {
            ObservationStance::Affirms => supporting.push(o.id.clone()),
            ObservationStance::Denies => contradicting.push(o.id.clone()),
        }
    }
    supporting.sort();
    supporting.dedup();
    contradicting.sort();
    contradicting.dedup();

    match (supporting.is_empty(), contradicting.is_empty()) {
        (false, true) => EvidenceResolution::Supported {
            relation: relation.clone(),
            supporting,
        },
        (true, false) => EvidenceResolution::Contradicted {
            relation: relation.clone(),
            contradicting,
        },
        (false, false) => EvidenceResolution::Conflicted {
            relation: relation.clone(),
            supporting,
            contradicting,
        },
        // Cannot happen: `covering` is non-empty, so at least one side has an entry.
        (true, true) => EvidenceResolution::Insufficient {
            relation: relation.clone(),
            gap: "observations carry no stance".to_string(),
        },
    }
}

// ─── A4-4bR: subject-general posture resolution ──────────────────────────────

/// Resolve the evidence for one [`ObservationTargetRef`] into the
/// subject-general [`EvidencePosture<ObservationTargetRef>`] shape.
///
/// This is the **canonical A4-4bR resolver**. The lens kernel consumes
/// its output. The historical [`resolve_relation`] is preserved for
/// A4-0/A4-2 consumers.
///
/// Behavior:
/// - Empty covering → `Insufficient { target, gap: NoObservation }`.
/// - Affirm only → `Supported { target, supporting }`.
/// - Deny only → `Contradicted { target, contradicting }`.
/// - Both → `Conflicted { target, supporting, contradicting }`
///   (preserved; **never** collapsed).
///
/// Order-independent by construction (REQ-A4S0-011 carries over).
pub fn resolve_subject(
    set: &ObservationSet,
    target: &ObservationTargetRef,
) -> EvidencePosture<ObservationTargetRef> {
    let covering = set.for_subject(target);
    if covering.is_empty() {
        return EvidencePosture::Insufficient {
            target: target.clone(),
            gap: InsufficientGap::NoObservation,
        };
    }

    let mut supporting: Vec<ObservationId> = Vec::new();
    let mut contradicting: Vec<ObservationId> = Vec::new();
    for o in covering {
        match o.stance {
            ObservationStance::Affirms => supporting.push(o.id.clone()),
            ObservationStance::Denies => contradicting.push(o.id.clone()),
        }
    }
    supporting.sort();
    supporting.dedup();
    contradicting.sort();
    contradicting.dedup();

    match (supporting.is_empty(), contradicting.is_empty()) {
        (false, true) => EvidencePosture::Supported {
            target: target.clone(),
            supporting,
        },
        (true, false) => EvidencePosture::Contradicted {
            target: target.clone(),
            contradicting,
        },
        (false, false) => EvidencePosture::Conflicted {
            target: target.clone(),
            supporting,
            contradicting,
        },
        // Cannot happen: `covering` is non-empty, so at least one side has an entry.
        (true, true) => EvidencePosture::Insufficient {
            target: target.clone(),
            gap: InsufficientGap::ObservationsWithoutStance,
        },
    }
}

// ─── Tests: equivalence old/new ──────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;
    use crate::observation::types::{ObservationBasis, ObservationOrigin, SoftwareObservation};
    use crate::observation::{SoftwareEntityRef, SoftwareRelation};
    use crate::semantic_kind::CoreRelationKind;

    fn relation() -> RelationId {
        let r = SoftwareRelation::new(
            SoftwareEntityRef::Unit(crate::architecture_graph::SoftwareUnitRef::new("a")),
            CoreRelationKind::DependsOn,
            SoftwareEntityRef::Unit(crate::architecture_graph::SoftwareUnitRef::new("b")),
        );
        r.id()
    }

    fn basis() -> ObservationBasis {
        ObservationBasis::new(
            "r1",
            crate::knowledge::KnowledgeBasis::empty(crate::knowledge::EventTime(1))
                .basis_hash()
                .clone(),
            "i1",
        )
    }

    fn obs(_relation: RelationId, stance: ObservationStance) -> SoftwareObservation {
        SoftwareObservation::declare(
            super::super::types::ObservationSubject::SoftwareRelation(SoftwareRelation::new(
                SoftwareEntityRef::Unit(crate::architecture_graph::SoftwareUnitRef::new("a")),
                CoreRelationKind::DependsOn,
                SoftwareEntityRef::Unit(crate::architecture_graph::SoftwareUnitRef::new("b")),
            )),
            stance,
            crate::evidence_ref::EvidenceRef::new(
                crate::evidence_ref::EvidenceKind::Governance,
                "loc",
            ),
            ObservationOrigin::DeterministicLocal,
            basis(),
            None,
            "test",
        )
    }

    #[test]
    fn resolve_relation_unchanged_for_empty_set() {
        let set = ObservationSet::new();
        let r = resolve_relation(&set, &relation());
        match r {
            EvidenceResolution::Insufficient { gap, .. } => {
                assert!(gap.contains("no observation covers"), "{gap}");
            }
            _ => panic!("expected Insufficient"),
        }
    }

    #[test]
    fn resolve_subject_relation_target_equivalent_to_resolve_relation_posture_class() {
        // Equivalence: drive the same set through both resolvers and
        // assert posture-class identity (not byte-equality, because the
        // wire shape differs).
        let mut set = ObservationSet::new();
        set.insert(obs(relation(), ObservationStance::Affirms));
        set.insert(obs(relation(), ObservationStance::Denies));

        let old = resolve_relation(&set, &relation());
        let target = ObservationTargetRef::Relation(relation());
        let new = resolve_subject(&set, &target);

        // Same posture class on both sides.
        assert_eq!(old.canonical_tag(), new.posture_class());

        // Conflicted preserves both observation sets.
        match (&old, &new) {
            (
                EvidenceResolution::Conflicted {
                    supporting: s1,
                    contradicting: c1,
                    ..
                },
                EvidencePosture::Conflicted {
                    supporting: s2,
                    contradicting: c2,
                    ..
                },
            ) => {
                assert_eq!(s1, s2);
                assert_eq!(c1, c2);
            }
            _ => panic!("expected Conflicted on both sides"),
        }
    }

    #[test]
    fn resolve_subject_unit_target_no_synthetic_relation() {
        // A4-4bR falsification #1: a Unit(foo) target resolves against
        // ObservationSubject::Unit(foo) observations with NO
        // SoftwareRelation created.
        let unit = crate::architecture_graph::SoftwareUnitRef::new("foo");
        let mut set = ObservationSet::new();
        set.insert(SoftwareObservation::declare(
            super::super::types::ObservationSubject::Unit(unit.clone()),
            ObservationStance::Affirms,
            crate::evidence_ref::EvidenceRef::new(
                crate::evidence_ref::EvidenceKind::Governance,
                "u_loc",
            ),
            ObservationOrigin::DeterministicLocal,
            basis(),
            None,
            "test",
        ));
        let target = ObservationTargetRef::Unit(unit.clone());
        let r = resolve_subject(&set, &target);
        // Target is Unit(foo), not Relation(...).
        match r {
            EvidencePosture::Supported { target: t, .. } => {
                assert_eq!(t, target);
                assert_eq!(t.kind_tag(), "unit");
            }
            _ => panic!("expected Supported(Unit(foo))"),
        }
    }

    #[test]
    fn resolve_subject_unit_conflicted_preserves_both_sets() {
        // A4-4bR falsification #3: Conflicted(Unit) preserves both sets.
        let unit = crate::architecture_graph::SoftwareUnitRef::new("foo");
        let mut set = ObservationSet::new();
        set.insert(SoftwareObservation::declare(
            super::super::types::ObservationSubject::Unit(unit.clone()),
            ObservationStance::Affirms,
            crate::evidence_ref::EvidenceRef::new(
                crate::evidence_ref::EvidenceKind::Governance,
                "u_loc_a",
            ),
            ObservationOrigin::DeterministicLocal,
            basis(),
            None,
            "test",
        ));
        set.insert(SoftwareObservation::declare(
            super::super::types::ObservationSubject::Unit(unit.clone()),
            ObservationStance::Denies,
            crate::evidence_ref::EvidenceRef::new(
                crate::evidence_ref::EvidenceKind::Governance,
                "u_loc_b",
            ),
            ObservationOrigin::DeterministicLocal,
            basis(),
            None,
            "test",
        ));
        let target = ObservationTargetRef::Unit(unit.clone());
        let r = resolve_subject(&set, &target);
        match r {
            EvidencePosture::Conflicted {
                target: t,
                supporting,
                contradicting,
            } => {
                assert_eq!(t.kind_tag(), "unit");
                assert_eq!(supporting.len(), 1);
                assert_eq!(contradicting.len(), 1);
            }
            _ => panic!("expected Conflicted(Unit)"),
        }
    }

    #[test]
    fn resolve_subject_unit_foo_and_relation_foo_foo_distinct() {
        // A4-4bR falsification #1 again: targets of different kinds
        // never collapse.
        let unit = crate::architecture_graph::SoftwareUnitRef::new("foo");
        let target_unit = ObservationTargetRef::Unit(unit);
        let rel_foo_foo = ObservationTargetRef::Relation(RelationId::derive("foo", "X", "foo"));

        // Build a set with a Unit observation only.
        let mut set = ObservationSet::new();
        set.insert(SoftwareObservation::declare(
            super::super::types::ObservationSubject::Unit(
                crate::architecture_graph::SoftwareUnitRef::new("foo"),
            ),
            ObservationStance::Affirms,
            crate::evidence_ref::EvidenceRef::new(
                crate::evidence_ref::EvidenceKind::Governance,
                "u_loc",
            ),
            ObservationOrigin::DeterministicLocal,
            basis(),
            None,
            "test",
        ));

        // Resolve against Unit target: should be Supported.
        let r_unit = resolve_subject(&set, &target_unit);
        // Resolve against the synthetic Relation target: should be Insufficient
        // because the set has no relation observation.
        let r_rel = resolve_subject(&set, &rel_foo_foo);
        assert_eq!(r_unit.posture_class(), "supported");
        assert_eq!(r_rel.posture_class(), "insufficient");
    }
}
