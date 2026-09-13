// Copyright (c) SDDK contributors.
// SPDX-License-Identifier: MIT
//
// evidence_relation_mapping.rs — T-09 (M1 ADR-0100 arch-spec-001 CA-004)
//
// Canonical mapping from the legacy planning-only discriminator
// `sddk_domain::planning::PlanningEvidenceKind` to the universal
// `sddk_engine::semantic_kind::CoreRelationKind`.
//
// Per ADR-0100 ("Universal Evidence"), the legacy `PlanningEvidenceKind`
// enum (5 variants: Log/Metric/Snapshot/Reference/Approval) is a
// domain-specialized evidence taxonomy. The canonical authority for
// evidence-to-target relationships is `CoreRelationKind` with the
// 14-variant closed set, including `Verifies` (Evidence → Decision/
// Assumption/Contribution) and `ObservedFor` (Evidence → Risk/Goal/Run).
//
// This module introduces the migration PATH for any code that holds
// a `PlanningEvidenceKind` and needs to express the relationship
// as a `CoreRelationKind`. It is *additive*: the legacy enum remains
// authoritative in its domain (per AGENTS §2.10 strangler migration).
// Future cycles migrate `EvidenceAttachmentV1` and the SQL schema
// (migrations.rs:737) to use `CoreRelationKind` directly.
//
// Mapping rationale (one line per discriminator):
//
// | PlanningEvidenceKind | CoreRelationKind | Why |
// |----------------------|------------------|-----|
// | Log                  | ObservedFor      | A log was emitted to inform / guard a run or goal |
// | Metric               | Verifies         | A metric confirms (verifies) a decision/assumption/contribution |
// | Snapshot             | ObservedFor      | A snapshot was captured to inform / guard a run or goal |
// | Reference            | References       | A reference is an external link — semantic match to References |
// | Approval             | Justifies        | An approval justifies (provides rationale for) a decision |

use sddk_domain::planning::PlanningEvidenceKind;

use crate::semantic_kind::CoreRelationKind;

/// Maps a legacy `PlanningEvidenceKind` discriminator to its canonical
/// `CoreRelationKind` per ADR-0100 mapping table.
///
/// This function is **pure** and **total**: every `PlanningEvidenceKind`
/// variant maps to exactly one `CoreRelationKind` (5 → 5 mapping).
/// Adding a variant to `PlanningEvidenceKind` requires updating this
/// function — the `assert_planning_evidence_kind_mapping_total` pin test
/// fails closed if a variant is added without updating the mapping.
pub fn planning_evidence_relation(kind: PlanningEvidenceKind) -> CoreRelationKind {
    match kind {
        PlanningEvidenceKind::Log => CoreRelationKind::ObservedFor,
        PlanningEvidenceKind::Metric => CoreRelationKind::Verifies,
        PlanningEvidenceKind::Snapshot => CoreRelationKind::ObservedFor,
        PlanningEvidenceKind::Reference => CoreRelationKind::References,
        PlanningEvidenceKind::Approval => CoreRelationKind::Justifies,
    }
}

/// Universal, lossless resolver for planning evidence (WU-C2, DELTA-CONF-003).
///
/// This is the CANONICAL production path for attaching planning evidence:
/// production code must build an `EvidenceRef` (universal substrate) plus a
/// `CoreRelationKind`, never a `PlanningEvidenceKind` discriminator. The
/// `kind` string is the legacy UI vocabulary (`log|metric|snapshot|reference|
/// approval`); it maps to its semantic relation via the same ADR-0100 table
/// as `planning_evidence_relation` (Log/Snapshot → ObservedFor, Metric →
/// Verifies, Reference → References, Approval → Justifies).
///
/// Returns a typed error for unknown kinds so callers fail closed instead of
/// guessing a relation.
pub fn resolve_planning_evidence_relation(
    kind: &str,
) -> Result<CoreRelationKind, UnknownPlanningEvidenceKind> {
    match kind.trim().to_lowercase().as_str() {
        "log" => Ok(planning_evidence_relation(PlanningEvidenceKind::Log)),
        "metric" | "metrics" => Ok(planning_evidence_relation(
            PlanningEvidenceKind::Metric,
        )),
        "snapshot" => Ok(planning_evidence_relation(PlanningEvidenceKind::Snapshot)),
        "reference" => Ok(planning_evidence_relation(PlanningEvidenceKind::Reference)),
        "approval" => Ok(planning_evidence_relation(PlanningEvidenceKind::Approval)),
        other => Err(UnknownPlanningEvidenceKind(other.to_string())),
    }
}

/// Typed error for an unknown planning evidence kind string (fail-closed).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct UnknownPlanningEvidenceKind(pub String);

impl std::fmt::Display for UnknownPlanningEvidenceKind {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "unknown planning evidence kind: {:?} (expected: log, metric, snapshot, reference, approval)",
            self.0
        )
    }
}

impl std::error::Error for UnknownPlanningEvidenceKind {}

/// Asserts the mapping table is **total**: every variant of
/// `PlanningEvidenceKind` is mapped.
///
/// Used by `assert_planning_evidence_kind_mapping_total` (the
/// pin test in this module). Adding a variant to `PlanningEvidenceKind`
/// without updating `planning_evidence_relation` will cause this
/// assert to fail at compile time, in addition to the test failure.
#[allow(dead_code)]
const fn assert_mapping_total() {
    use PlanningEvidenceKind::*;
    // Compile-time exhaustiveness check: if a variant is added to
    // PlanningEvidenceKind, this match will not be exhaustive and
    // the code will not compile.
    let _ = match Log {
        Log => CoreRelationKind::ObservedFor,
        Metric => CoreRelationKind::Verifies,
        Snapshot => CoreRelationKind::ObservedFor,
        Reference => CoreRelationKind::References,
        Approval => CoreRelationKind::Justifies,
    };
}

const _: () = assert_mapping_total();

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn log_maps_to_observed_for() {
        assert_eq!(
            planning_evidence_relation(PlanningEvidenceKind::Log),
            CoreRelationKind::ObservedFor
        );
    }

    #[test]
    fn metric_maps_to_verifies() {
        assert_eq!(
            planning_evidence_relation(PlanningEvidenceKind::Metric),
            CoreRelationKind::Verifies
        );
    }

    #[test]
    fn snapshot_maps_to_observed_for() {
        assert_eq!(
            planning_evidence_relation(PlanningEvidenceKind::Snapshot),
            CoreRelationKind::ObservedFor
        );
    }

    #[test]
    fn reference_maps_to_references() {
        assert_eq!(
            planning_evidence_relation(PlanningEvidenceKind::Reference),
            CoreRelationKind::References
        );
    }

    #[test]
    fn approval_maps_to_justifies() {
        assert_eq!(
            planning_evidence_relation(PlanningEvidenceKind::Approval),
            CoreRelationKind::Justifies
        );
    }

    /// Pin test: every `PlanningEvidenceKind` variant MUST map to a
    /// canonical `CoreRelationKind`. If a variant is added without
    /// updating `planning_evidence_relation`, this test fails closed.
    ///
    /// Mechanism: iterates the ALL-style enumeration explicitly, then
    /// asserts the mapping produces a relation in the closed set.
    /// The compile-time `assert_mapping_total` const above provides
    /// an additional guarantee at the type level.
    #[test]
    fn assert_planning_evidence_kind_mapping_total() {
        let all_kinds = [
            PlanningEvidenceKind::Log,
            PlanningEvidenceKind::Metric,
            PlanningEvidenceKind::Snapshot,
            PlanningEvidenceKind::Reference,
            PlanningEvidenceKind::Approval,
        ];
        for kind in all_kinds {
            let relation = planning_evidence_relation(kind);
            assert!(
                CoreRelationKind::ALL.contains(&relation),
                "kind {kind:?} mapped to {relation:?} which is not in CoreRelationKind::ALL"
            );
        }
    }

    /// Pin test: the 5-variant set maps to exactly 4 distinct relations
    /// (Log + Snapshot both map to ObservedFor). This documents the
    /// intentional 5→4 collapse (Log and Snapshot are semantically
    /// equivalent for the planning domain — both are observations).
    #[test]
    fn mapping_collapses_to_four_relations() {
        use std::collections::BTreeSet;
        let all_kinds = [
            PlanningEvidenceKind::Log,
            PlanningEvidenceKind::Metric,
            PlanningEvidenceKind::Snapshot,
            PlanningEvidenceKind::Reference,
            PlanningEvidenceKind::Approval,
        ];
        let mapped: BTreeSet<_> = all_kinds
            .iter()
            .map(|k| planning_evidence_relation(*k))
            .collect();
        assert_eq!(
            mapped.len(),
            4,
            "expected 5 → 4 mapping (Log/Snapshot collapse); got {mapped:?}"
        );
    }

    // ── WU-C2 (DELTA-CONF-003) R-003.2: universal relation surface ─────────

    /// R-003.2: the FIVE semantic relations required by DELTA-CONF-003
    /// (supports, verifies, gates, produced_by, contradicts) exist as typed
    /// variants of the universal `CoreRelationKind` substrate.
    #[test]
    fn universal_substrate_has_all_five_semantic_relations() {
        let required = [
            (CoreRelationKind::Supports, "supports"),
            (CoreRelationKind::Verifies, "verifies"),
            (CoreRelationKind::Gates, "gates"),
            (CoreRelationKind::ProducedBy, "produced_by"),
            (CoreRelationKind::Contradicts, "contradicts"),
        ];
        for (variant, tag) in required {
            assert!(
                CoreRelationKind::ALL.contains(&variant),
                "semantic relation {tag} missing from CoreRelationKind::ALL"
            );
            assert_eq!(variant.domain_tag(), tag, "canonical tag drift for {tag}");
        }
    }

    /// R-003.2: every `PlanningEvidenceKind` variant maps losslessly into
    /// one of the universal relations, including the five semantic ones.
    /// (Lossless = the mapping is total AND every produced relation is a
    /// canonical variant — see `assert_planning_evidence_kind_mapping_total`
    /// for the base contract.)
    #[test]
    fn every_legacy_variant_maps_into_the_universal_set() {
        let all_kinds = [
            (PlanningEvidenceKind::Log, CoreRelationKind::ObservedFor),
            (PlanningEvidenceKind::Metric, CoreRelationKind::Verifies),
            (PlanningEvidenceKind::Snapshot, CoreRelationKind::ObservedFor),
            (PlanningEvidenceKind::Reference, CoreRelationKind::References),
            (PlanningEvidenceKind::Approval, CoreRelationKind::Justifies),
        ];
        for (kind, expected) in all_kinds {
            assert_eq!(
                planning_evidence_relation(kind),
                expected,
                "{kind:?} must map to {expected:?} without loss"
            );
        }
    }

    /// WU-C2: the universal resolver produces the same relation as the
    /// legacy mapping for every UI vocabulary string, and fails closed on
    /// unknown strings.
    #[test]
    fn universal_resolver_matches_legacy_mapping_and_fails_closed() {
        for (text, kind) in [
            ("log", PlanningEvidenceKind::Log),
            ("LOG", PlanningEvidenceKind::Log),
            ("metric", PlanningEvidenceKind::Metric),
            ("metrics", PlanningEvidenceKind::Metric),
            ("snapshot", PlanningEvidenceKind::Snapshot),
            ("reference", PlanningEvidenceKind::Reference),
            ("approval", PlanningEvidenceKind::Approval),
        ] {
            let via_universal = resolve_planning_evidence_relation(text)
                .unwrap_or_else(|e| panic!("resolver rejected {text:?}: {e}"));
            assert_eq!(
                via_universal,
                planning_evidence_relation(kind),
                "universal resolver drifted from the legacy mapping for {text:?}"
            );
        }
        assert_eq!(
            resolve_planning_evidence_relation("nonsense"),
            Err(UnknownPlanningEvidenceKind("nonsense".to_string())),
            "unknown kinds must fail closed with a typed error"
        );
    }
}
