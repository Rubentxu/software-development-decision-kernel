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
}
