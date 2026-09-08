//! CONVERGE — Adaptive Verification Across a WorkGraph.
//!
//! Cycle: SDD-ADAPTIVE-003 (H8, order 460, context pack
//! `adaptive-sdd`).
//!
//! Pure, deterministic verifier that consumes a `WorkGraph` and
//! per-unit `WorkUnitOutcome`s and emits a typed `ConvergeVerdict`.
//! Stages are gated by a closed-set matrix (Compile / Apply /
//! Verify / Promote must succeed; Integrate / Finalize may be
//! skipped).
//!
//! ## Design
//!
//! - **Closed-set `WorkUnitResult`.** Three values: Succeeded,
//!   Skipped, Failed { reason }.
//! - **Per-stage matrix.** Compile / Apply / Verify / Promote
//!   MUST succeed; Integrate / Finalize may be skipped.
//! - **`ConvergeStatus` closed-set.** AllSucceeded / SkippedSome /
//!   FailedSome / Inconclusive.
//! - **Pure `ConvergeVerifier::verify`.** Deterministic.
//! - **Strict on structural errors.** Duplicate / unknown /
//!   topology mismatch fail closed.
//! - **`#[non_exhaustive]`** everywhere.
//!
//! See:
//!
//! - `REQ-ConvergeVerification` (spec, accepted)
//! - `ADR-107` (architecture decision, accepted)
//! - `REQ-BuildWorkGraph`, `REQ-EngineeringAssuranceProfiles`,
//!   `REQ-EngineeringAssuranceResolvers` (dependencies).

use std::collections::BTreeMap;

use serde::{Deserialize, Serialize};

use crate::build_work_graph::{WorkGraph, WorkUnitStage};

// ── WorkUnitResult ───────────────────────────────────────────────────────

/// Closed-set unit result.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[non_exhaustive]
pub enum WorkUnitResult {
    /// Unit succeeded.
    Succeeded,
    /// Unit was skipped.
    Skipped,
    /// Unit failed with a reason.
    Failed {
        /// Reason.
        reason: String,
    },
}

impl WorkUnitResult {
    /// Stable identifier.
    pub fn id(&self) -> String {
        match self {
            WorkUnitResult::Succeeded => "succeeded".to_string(),
            WorkUnitResult::Skipped => "skipped".to_string(),
            WorkUnitResult::Failed { reason } => {
                format!("failed(reason={reason})")
            }
        }
    }
}

// ── WorkUnitOutcome ──────────────────────────────────────────────────────

/// Per-unit outcome reported by the runner.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[non_exhaustive]
pub struct WorkUnitOutcome {
    /// Unit id.
    pub unit_id: String,
    /// Stage.
    pub stage: WorkUnitStage,
    /// Result.
    pub result: WorkUnitResult,
    /// Detail.
    pub detail: String,
    /// RFC-3339 timestamp supplied by the caller.
    pub observed_at: String,
}

// ── ConvergeStatus ───────────────────────────────────────────────────────

/// Closed-set verdict status.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[non_exhaustive]
pub enum ConvergeStatus {
    /// All units succeeded.
    AllSucceeded,
    /// Some units were skipped.
    SkippedSome {
        /// Skipped unit ids (sorted).
        units: Vec<String>,
    },
    /// Some units failed.
    FailedSome {
        /// Failed unit ids (sorted).
        units: Vec<String>,
    },
    /// Cannot determine.
    Inconclusive {
        /// Reason.
        reason: String,
    },
}

// ── ConvergeVerdict ──────────────────────────────────────────────────────

/// Verdict emitted by `ConvergeVerifier`.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[non_exhaustive]
pub struct ConvergeVerdict {
    /// Graph id.
    pub graph_id: String,
    /// Contract id.
    pub contract_id: String,
    /// Status.
    pub status: ConvergeStatus,
    /// Per-stage result map (unit_id ⇒ result, sorted by unit_id).
    pub stage_results: BTreeMap<String, WorkUnitResult>,
    /// RFC-3339 timestamp supplied by the caller.
    pub evaluated_at: String,
    /// Schema version.
    pub schema_version: u32,
}

impl ConvergeVerdict {
    /// Schema version.
    pub const SCHEMA_VERSION: u32 = 1;
}

// ── ConvergeVerifyError ──────────────────────────────────────────────────

/// Errors emitted by `ConvergeVerifier`.
#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error, Serialize, Deserialize)]
#[non_exhaustive]
pub enum ConvergeVerifyError {
    /// Outcome references an unknown unit.
    #[error("unknown unit in outcomes: {unit_id}")]
    UnknownUnit {
        /// Unit id.
        unit_id: String,
    },
    /// Two outcomes for the same unit.
    #[error("duplicate outcome for unit: {unit_id}")]
    DuplicateOutcome {
        /// Unit id.
        unit_id: String,
    },
    /// Promote stage present but no Integrate.
    #[error("Promote stage present without Integrate")]
    PromoteWithoutIntegrate,
}

// ── ConvergeVerifier ─────────────────────────────────────────────────────

/// Pure verifier.
#[derive(Debug, Clone, Default)]
#[non_exhaustive]
pub struct ConvergeVerifier;

impl ConvergeVerifier {
    /// Verify outcomes against the graph.
    pub fn verify(
        &self,
        graph: &WorkGraph,
        outcomes: &[WorkUnitOutcome],
        evaluated_at: String,
    ) -> Result<ConvergeVerdict, ConvergeVerifyError> {
        // Structural validation.
        let mut stage_results: BTreeMap<String, WorkUnitResult> = BTreeMap::new();
        for o in outcomes {
            // Check unit exists in graph.
            if !graph.units.iter().any(|u| u.unit_id == o.unit_id) {
                return Err(ConvergeVerifyError::UnknownUnit {
                    unit_id: o.unit_id.clone(),
                });
            }
            if stage_results
                .insert(o.unit_id.clone(), o.result.clone())
                .is_some()
            {
                return Err(ConvergeVerifyError::DuplicateOutcome {
                    unit_id: o.unit_id.clone(),
                });
            }
        }

        // Check Promote ⇒ Integrate.
        let has_promote = graph
            .units
            .iter()
            .any(|u| matches!(u.stage, WorkUnitStage::Promote));
        let has_integrate = graph
            .units
            .iter()
            .any(|u| matches!(u.stage, WorkUnitStage::Integrate));
        if has_promote && !has_integrate {
            return Err(ConvergeVerifyError::PromoteWithoutIntegrate);
        }

        // Apply matrix.
        let mut failed: Vec<String> = Vec::new();
        let mut skipped: Vec<String> = Vec::new();
        let mut inconclusive: Option<String> = None;
        for u in &graph.units {
            let required = matches!(
                u.stage,
                WorkUnitStage::Compile
                    | WorkUnitStage::Apply
                    | WorkUnitStage::Verify
                    | WorkUnitStage::Promote
            );
            match stage_results.get(&u.unit_id) {
                Some(WorkUnitResult::Succeeded) => {}
                Some(WorkUnitResult::Skipped) => {
                    if required {
                        failed.push(u.unit_id.clone());
                    } else {
                        skipped.push(u.unit_id.clone());
                    }
                }
                Some(WorkUnitResult::Failed { .. }) => {
                    failed.push(u.unit_id.clone());
                }
                None => {
                    if required {
                        inconclusive.get_or_insert(format!("missing_outcome:{}", u.unit_id));
                    } else {
                        skipped.push(u.unit_id.clone());
                    }
                }
            }
        }

        // Determinism: sort unit lists.
        failed.sort();
        skipped.sort();

        let status = if let Some(reason) = inconclusive {
            ConvergeStatus::Inconclusive { reason }
        } else if !failed.is_empty() {
            ConvergeStatus::FailedSome { units: failed }
        } else if !skipped.is_empty() {
            ConvergeStatus::SkippedSome { units: skipped }
        } else {
            ConvergeStatus::AllSucceeded
        };

        // Determinism on stage_results already guaranteed by
        // BTreeMap ordering (by unit_id).
        Ok(ConvergeVerdict {
            graph_id: graph.graph_id.clone(),
            contract_id: graph.contract_id.clone(),
            status,
            stage_results,
            evaluated_at,
            schema_version: ConvergeVerdict::SCHEMA_VERSION,
        })
    }
}

// ── Audit guards ──────────────────────────────────────────────────────────

#[allow(unused)]
const WORK_UNIT_RESULT_VARIANT_LIST: &[WorkUnitResult] = &[
    WorkUnitResult::Succeeded,
    WorkUnitResult::Skipped,
    WorkUnitResult::Failed {
        reason: String::new(),
    },
];

#[allow(unused)]
const CONVERGE_STATUS_VARIANT_LIST: &[ConvergeStatus] = &[
    ConvergeStatus::AllSucceeded,
    ConvergeStatus::SkippedSome { units: vec![] },
    ConvergeStatus::FailedSome { units: vec![] },
    ConvergeStatus::Inconclusive {
        reason: String::new(),
    },
];

#[allow(unused)]
const CONVERGE_VERIFY_ERROR_VARIANT_LIST: &[ConvergeVerifyError] = &[
    ConvergeVerifyError::UnknownUnit {
        unit_id: String::new(),
    },
    ConvergeVerifyError::DuplicateOutcome {
        unit_id: String::new(),
    },
    ConvergeVerifyError::PromoteWithoutIntegrate,
];

// ── Tests ────────────────────────────────────────────────────────────────

#[cfg(test)]
#[allow(clippy::collapsible_if)]
#[allow(clippy::collapsible_else_if)]
mod tests {
    use super::*;
    use crate::build_work_graph::{WorkGraphBuilder, WorkUnitStage};
    use crate::change_contract::{ShapeDecision, Specialist, SpecialistKind};

    fn graph() -> WorkGraph {
        let decision = ShapeDecision {
            contract_id: "C1".to_string(),
            chosen: Some(Specialist {
                specialist_id: "alice".to_string(),
                kind: SpecialistKind::Human,
                capabilities: vec!["sddk-engine/sc".to_string()],
                admission_criteria: vec![],
                max_concurrent: 1,
            }),
            rejected: vec![],
            rationale: "r".to_string(),
            decided_at: "t".to_string(),
            schema_version: crate::change_contract::ShapeDecision::SCHEMA_VERSION,
        };
        WorkGraphBuilder.build(&decision, "t".to_string()).unwrap()
    }

    fn outcome(unit_id: &str, stage: WorkUnitStage, result: WorkUnitResult) -> WorkUnitOutcome {
        WorkUnitOutcome {
            unit_id: unit_id.to_string(),
            stage,
            result,
            detail: "".to_string(),
            observed_at: "t".to_string(),
        }
    }

    fn all_succeeded(graph: &WorkGraph) -> Vec<WorkUnitOutcome> {
        graph
            .units
            .iter()
            .map(|u| outcome(&u.unit_id, u.stage, WorkUnitResult::Succeeded))
            .collect()
    }

    // ── S-1: All Succeeded ⇒ AllSucceeded ───────────────────────────────

    #[test]
    fn s1_all_succeeded() {
        let g = graph();
        let outcomes = all_succeeded(&g);
        let verifier = ConvergeVerifier;
        let v = verifier.verify(&g, &outcomes, "t".to_string()).unwrap();
        assert_eq!(v.status, ConvergeStatus::AllSucceeded);
    }

    // ── S-2: Finalize Skipped ⇒ SkippedSome ─────────────────────────────

    #[test]
    fn s2_finalize_skipped() {
        let g = graph();
        let mut outcomes = all_succeeded(&g);
        if let Some(f) = g.units.iter().find(|u| u.stage == WorkUnitStage::Finalize) {
            if let Some(o) = outcomes.iter_mut().find(|o| o.unit_id == f.unit_id) {
                o.result = WorkUnitResult::Skipped;
            }
        }
        let verifier = ConvergeVerifier;
        let v = verifier.verify(&g, &outcomes, "t".to_string()).unwrap();
        assert!(matches!(v.status, ConvergeStatus::SkippedSome { .. }));
    }

    // ── S-3: Compile Failed ⇒ FailedSome ────────────────────────────────

    #[test]
    fn s3_compile_failed() {
        let g = graph();
        let mut outcomes = all_succeeded(&g);
        if let Some(c) = g.units.iter().find(|u| u.stage == WorkUnitStage::Compile) {
            if let Some(o) = outcomes.iter_mut().find(|o| o.unit_id == c.unit_id) {
                o.result = WorkUnitResult::Failed {
                    reason: "boom".to_string(),
                };
            }
        }
        let verifier = ConvergeVerifier;
        let v = verifier.verify(&g, &outcomes, "t".to_string()).unwrap();
        if let ConvergeStatus::FailedSome { units } = &v.status {
            assert!(!units.is_empty());
        } else {
            panic!("expected FailedSome");
        }
    }

    // ── S-4: Missing outcome for required stage ⇒ Inconclusive ───────────

    #[test]
    fn s4_missing_required() {
        let g = graph();
        // Drop the Apply outcome.
        let outcomes: Vec<WorkUnitOutcome> = all_succeeded(&g)
            .into_iter()
            .filter(|o| o.stage != WorkUnitStage::Apply)
            .collect();
        let verifier = ConvergeVerifier;
        let v = verifier.verify(&g, &outcomes, "t".to_string()).unwrap();
        assert!(matches!(v.status, ConvergeStatus::Inconclusive { .. }));
    }

    // ── S-5: Promote without Integrate ⇒ error ──────────────────────────

    #[test]
    fn s5_promote_without_integrate() {
        let mut g = graph();
        g.units.retain(|u| u.stage != WorkUnitStage::Integrate);
        // Need a Promote present.
        g.units.push(crate::build_work_graph::WorkUnit {
            unit_id: "x-promote".to_string(),
            stage: WorkUnitStage::Promote,
            surface: "s".to_string(),
            preconditions: vec![],
            payload_kind: "p".to_string(),
            generated_at: "t".to_string(),
        });
        let outcomes = all_succeeded(&g);
        let verifier = ConvergeVerifier;
        let err = verifier.verify(&g, &outcomes, "t".to_string()).unwrap_err();
        assert!(matches!(err, ConvergeVerifyError::PromoteWithoutIntegrate));
    }

    // ── S-6: Duplicate outcome ⇒ error ──────────────────────────────────

    #[test]
    fn s6_duplicate_outcome() {
        let g = graph();
        let mut outcomes = all_succeeded(&g);
        let first = outcomes[0].clone();
        outcomes.push(first);
        let verifier = ConvergeVerifier;
        let err = verifier.verify(&g, &outcomes, "t".to_string()).unwrap_err();
        assert!(matches!(err, ConvergeVerifyError::DuplicateOutcome { .. }));
    }

    // ── S-7: Unknown unit in outcomes ⇒ error ───────────────────────────

    #[test]
    fn s7_unknown_unit() {
        let g = graph();
        let mut outcomes = all_succeeded(&g);
        outcomes.push(outcome(
            "ghost-unit",
            WorkUnitStage::Compile,
            WorkUnitResult::Succeeded,
        ));
        let verifier = ConvergeVerifier;
        let err = verifier.verify(&g, &outcomes, "t".to_string()).unwrap_err();
        assert!(matches!(err, ConvergeVerifyError::UnknownUnit { .. }));
    }

    // ── S-8: Determinism ───────────────────────────────────────────────

    #[test]
    fn s8_determinism_byte_equal() {
        let g = graph();
        let outcomes = all_succeeded(&g);
        let verifier = ConvergeVerifier;
        let a = verifier.verify(&g, &outcomes, "t".to_string()).unwrap();
        let b = verifier.verify(&g, &outcomes, "t".to_string()).unwrap();
        assert_eq!(
            serde_json::to_string(&a).unwrap(),
            serde_json::to_string(&b).unwrap()
        );
    }

    // ── S-9: stage_results sorted by unit_id ───────────────────────────

    #[test]
    fn s9_stage_results_sorted() {
        let g = graph();
        let outcomes = all_succeeded(&g);
        let verifier = ConvergeVerifier;
        let v = verifier.verify(&g, &outcomes, "t".to_string()).unwrap();
        let keys: Vec<&String> = v.stage_results.keys().collect();
        let sorted: Vec<&String> = {
            let mut s = keys.clone();
            s.sort();
            s
        };
        assert_eq!(keys, sorted);
    }

    // ── Extra: WorkUnitResult ids ──────────────────────────────────────

    #[test]
    fn result_ids() {
        assert_eq!(WorkUnitResult::Succeeded.id(), "succeeded".to_string());
        assert_eq!(WorkUnitResult::Skipped.id(), "skipped".to_string());
        assert_eq!(
            WorkUnitResult::Failed {
                reason: "r".to_string()
            }
            .id(),
            "failed(reason=r)".to_string()
        );
    }
}
