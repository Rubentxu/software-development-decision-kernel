//! Workflow Lab Comparison + Promotion Against A-Full.
//!
//! Cycle: SDD-ADAPTIVE-005 (H8, order 480, context pack
//! `adaptive-sdd`).
//!
//! Pure, deterministic promotion gate that consumes Lab + Baseline
//! outcomes, the comparator verdict (`LAB-WORKFLOW-002`), the
//! integrate parity status (`SDD-ADAPTIVE-004`), and the
//! assurance status (`EA-ASSURANCE-001`).
//!
//! ## Design
//!
//! - **Closed-set `LabPromotionGate`.** Two values: Promote /
//!   Hold { reason }.
//! - **All-clauses gate.** Promotion requires every clause to
//!   hold.
//! - **Pure evaluator.** Deterministic; sorted outputs.
//! - **Reuses existing types.**
//! - **`#[non_exhaustive]`** everywhere.
//!
//! See:
//!
//! - `REQ-LabPromotion` (spec, accepted)
//! - `ADR-109` (architecture decision, accepted)
//! - `LAB-WORKFLOW-002`, `LAB-DECISION-001/002`,
//!   `EA-ASSURANCE-001`, `EA-UAT-001`, `SDD-ADAPTIVE-001..004`
//!   (dependencies).

use serde::{Deserialize, Serialize};

use crate::decision_lab_baseline::DecisionLabOutcome;
use crate::decision_lab_experimental::DecisionLabLabOutcome;
use crate::engineering_assurance::AssuranceStatus;
use crate::integrate_parity::IntegrateStatus;
use crate::strategy_comparison::{PromotionPolicy, PromotionVerdict};

// ── LabPromotionGate ────────────────────────────────────────────────────

/// Closed-set promotion gate.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[non_exhaustive]
pub enum LabPromotionGate {
    /// Promote.
    Promote,
    /// Hold.
    Hold {
        /// Reason.
        reason: String,
    },
}

impl LabPromotionGate {
    /// Stable identifier.
    pub fn id(&self) -> String {
        match self {
            LabPromotionGate::Promote => "promote".to_string(),
            LabPromotionGate::Hold { reason } => format!("hold(reason={reason})"),
        }
    }
}

// ── LabPromotionInput ────────────────────────────────────────────────────

/// Input to the promotion engine.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[non_exhaustive]
pub struct LabPromotionInput {
    /// Cycle id.
    pub cycle_id: String,
    /// Promotion policy.
    pub policy: PromotionPolicy,
    /// Baseline outcome (LAB-DECISION-001).
    pub baseline_outcome: DecisionLabOutcome,
    /// Lab outcome (LAB-DECISION-002).
    pub lab_outcome: DecisionLabLabOutcome,
    /// Comparator status (LAB-WORKFLOW-002).
    pub comparator_status: PromotionVerdict,
    /// Integrate parity status (SDD-ADAPTIVE-004).
    pub integrate_status: IntegrateStatus,
    /// Assurance status (EA-ASSURANCE-001).
    pub assurance_status: AssuranceStatus,
}

// ── LabPromotionVerdict ──────────────────────────────────────────────────

/// Verdict emitted by the engine.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[non_exhaustive]
pub struct LabPromotionVerdict {
    /// Cycle id.
    pub cycle_id: String,
    /// Gate.
    pub gate: LabPromotionGate,
    /// Satisfied clauses.
    pub satisfied_clauses: Vec<String>,
    /// Violated clauses.
    pub violated_clauses: Vec<String>,
    /// RFC-3339 timestamp supplied by the caller.
    pub evaluated_at: String,
    /// Schema version.
    pub schema_version: u32,
}

impl LabPromotionVerdict {
    /// Schema version.
    pub const SCHEMA_VERSION: u32 = 1;
}

// ── LabPromotionEngine ──────────────────────────────────────────────────

/// Pure engine.
#[derive(Debug, Clone, Default)]
#[non_exhaustive]
pub struct LabPromotionEngine;

impl LabPromotionEngine {
    /// Evaluate the input.
    pub fn evaluate(&self, input: &LabPromotionInput, evaluated_at: String) -> LabPromotionVerdict {
        let mut satisfied: Vec<String> = Vec::new();
        let mut violated: Vec<String> = Vec::new();

        // Clause: comparator_promotes
        if matches!(input.comparator_status, PromotionVerdict::Promote) {
            satisfied.push("comparator_promotes".to_string());
        } else {
            violated.push("comparator_promotes".to_string());
        }

        // Clause: integrate_parity
        match input.integrate_status {
            IntegrateStatus::Parity | IntegrateStatus::ParityWithIgnored => {
                satisfied.push("integrate_parity".to_string());
            }
            IntegrateStatus::Diverged => {
                violated.push("integrate_parity".to_string());
            }
        }

        // Clause: assurance_passes
        if matches!(input.assurance_status, AssuranceStatus::Pass) {
            satisfied.push("assurance_passes".to_string());
        } else {
            violated.push("assurance_passes".to_string());
        }

        // Clause: baseline_non_empty
        if !input.baseline_outcome.branches.is_empty() {
            satisfied.push("baseline_non_empty".to_string());
        } else {
            violated.push("baseline_non_empty".to_string());
        }

        // Clause: lab_has_best
        if !input.lab_outcome.best_branch_id.is_empty() {
            satisfied.push("lab_has_best".to_string());
        } else {
            violated.push("lab_has_best".to_string());
        }

        // Clause: match_rate (seam; uses comparator promote as a
        // proxy for "non-inferior" in this iteration).
        let match_rate_bps: u32 = if matches!(input.comparator_status, PromotionVerdict::Promote) {
            10_000
        } else {
            0
        };
        let policy_threshold_bps: u32 = match &input.policy {
            PromotionPolicy::MaxErrorRateBps { min_match_rate_bps } => *min_match_rate_bps,
            // For NotRegressing / MoreMatches the comparator is the
            // canonical gate, so any match rate is acceptable here
            // (we still record the clause as satisfied).
            PromotionPolicy::NotRegressing | PromotionPolicy::MoreMatches => 0,
        };
        if match_rate_bps >= policy_threshold_bps {
            satisfied.push("match_rate".to_string());
        } else {
            violated.push("match_rate".to_string());
        }

        // Determinism.
        satisfied.sort();
        violated.sort();

        let gate = if violated.is_empty() {
            LabPromotionGate::Promote
        } else {
            LabPromotionGate::Hold {
                reason: format!("violated_clauses:{}", violated.join(",")),
            }
        };

        LabPromotionVerdict {
            cycle_id: input.cycle_id.clone(),
            gate,
            satisfied_clauses: satisfied,
            violated_clauses: violated,
            evaluated_at,
            schema_version: LabPromotionVerdict::SCHEMA_VERSION,
        }
    }
}

// ── Audit guards ─────────────────────────────────────────────────────────

#[allow(unused)]
const LAB_PROMOTION_GATE_VARIANT_LIST: &[LabPromotionGate] = &[
    LabPromotionGate::Promote,
    LabPromotionGate::Hold {
        reason: String::new(),
    },
];

// ── Tests ────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;
    use crate::decision_lab_baseline::DecisionLabOutcome;
    use crate::decision_lab_baseline::{BaselineStrategy, DecisionBranch};
    use crate::decision_lab_experimental::{
        DecisionLabLabOutcome, ExperimentalSearchLab, ExperimentalStrategy, LabBranch,
    };
    use crate::strategy_comparison::PromotionHoldReason;
    use sddk_domain::workflow_ir::NodeId;
    use sddk_domain::workflow_run::AttemptId;
    use std::collections::BTreeMap;

    fn baseline_with_branches(n: usize) -> DecisionLabOutcome {
        let mut branches = Vec::new();
        for i in 0..n {
            branches.push(DecisionBranch {
                branch_id: format!("b{i}"),
                parent_node_id: NodeId("p".to_string()),
                score: BTreeMap::new(),
                chosen_fields: BTreeMap::new(),
                source_projection: AttemptId(format!("a{i}")),
                depth: 1,
            });
        }
        DecisionLabOutcome {
            strategy: BaselineStrategy::BestFirst,
            branches,
            evaluated_count: n,
            generated_at: "t".to_string(),
            schema_version: 1,
        }
    }

    fn lab_with_best(best: &str) -> DecisionLabLabOutcome {
        DecisionLabLabOutcome {
            strategy: ExperimentalSearchLab::new_unchecked(
                ExperimentalStrategy::Mcts {
                    rollouts: 1,
                    exploration_c: 1.41,
                    max_depth: 1,
                },
                vec!["score".to_string()],
            )
            .strategy()
            .clone(),
            branches: vec![LabBranch {
                branch_id: best.to_string(),
                parent_node_id: NodeId("p".to_string()),
                path: vec![AttemptId("a".to_string())],
                depth: 1,
                score_per_field: BTreeMap::new(),
                visits: 1,
                evidence_refs: vec![],
                terminal: true,
            }],
            root: NodeId("r".to_string()),
            visited_count: 1,
            best_branch_id: best.to_string(),
            generated_at: "t".to_string(),
            schema_version: 1,
        }
    }

    fn policy_permissive() -> PromotionPolicy {
        PromotionPolicy::NotRegressing
    }

    fn base_input() -> LabPromotionInput {
        LabPromotionInput {
            cycle_id: "C1".to_string(),
            policy: policy_permissive(),
            baseline_outcome: baseline_with_branches(1),
            lab_outcome: lab_with_best("lab-1"),
            comparator_status: PromotionVerdict::Promote,
            integrate_status: IntegrateStatus::Parity,
            assurance_status: AssuranceStatus::Pass,
        }
    }

    // ── S-1: All clauses satisfied ⇒ Promote ─────────────────────────────

    #[test]
    fn s1_all_satisfied_promote() {
        let engine = LabPromotionEngine;
        let input = base_input();
        let v = engine.evaluate(&input, "t".to_string());
        assert_eq!(v.gate, LabPromotionGate::Promote);
        assert!(v.violated_clauses.is_empty());
    }

    // ── S-2: Comparator Hold ⇒ Hold ─────────────────────────────────────

    #[test]
    fn s2_comparator_hold() {
        let engine = LabPromotionEngine;
        let mut input = base_input();
        input.comparator_status = PromotionVerdict::Hold {
            reason: PromotionHoldReason::FewerMatches {
                baseline: 1,
                candidate: 0,
            },
        };
        let v = engine.evaluate(&input, "t".to_string());
        assert!(matches!(v.gate, LabPromotionGate::Hold { .. }));
        assert!(
            v.violated_clauses
                .contains(&"comparator_promotes".to_string())
        );
    }

    // ── S-3: Integrate Diverged ⇒ Hold ─────────────────────────────────

    #[test]
    fn s3_integrate_diverged() {
        let engine = LabPromotionEngine;
        let mut input = base_input();
        input.integrate_status = IntegrateStatus::Diverged;
        let v = engine.evaluate(&input, "t".to_string());
        assert!(matches!(v.gate, LabPromotionGate::Hold { .. }));
        assert!(v.violated_clauses.contains(&"integrate_parity".to_string()));
    }

    // ── S-4: Assurance Degraded ⇒ Hold ─────────────────────────────────

    #[test]
    fn s4_assurance_degraded() {
        let engine = LabPromotionEngine;
        let mut input = base_input();
        input.assurance_status = AssuranceStatus::Degraded {
            reason: "r".to_string(),
        };
        let v = engine.evaluate(&input, "t".to_string());
        assert!(matches!(v.gate, LabPromotionGate::Hold { .. }));
        assert!(v.violated_clauses.contains(&"assurance_passes".to_string()));
    }

    // ── S-5: Lab best_branch missing ⇒ Hold ────────────────────────────

    #[test]
    fn s5_lab_best_missing() {
        let engine = LabPromotionEngine;
        let mut input = base_input();
        input.lab_outcome = lab_with_best("");
        let v = engine.evaluate(&input, "t".to_string());
        assert!(matches!(v.gate, LabPromotionGate::Hold { .. }));
        assert!(v.violated_clauses.contains(&"lab_has_best".to_string()));
    }

    // ── S-6: Determinism ───────────────────────────────────────────────

    #[test]
    fn s6_determinism_byte_equal() {
        let engine = LabPromotionEngine;
        let input = base_input();
        let a = engine.evaluate(&input, "t".to_string());
        let b = engine.evaluate(&input, "t".to_string());
        assert_eq!(
            serde_json::to_string(&a).unwrap(),
            serde_json::to_string(&b).unwrap()
        );
    }

    // ── S-7: violated_clauses sorted ───────────────────────────────────

    #[test]
    fn s7_violated_sorted() {
        let engine = LabPromotionEngine;
        let mut input = base_input();
        input.integrate_status = IntegrateStatus::Diverged;
        input.assurance_status = AssuranceStatus::Fail {
            reason: "x".to_string(),
        };
        let v = engine.evaluate(&input, "t".to_string());
        let mut sorted = v.violated_clauses.clone();
        sorted.sort();
        assert_eq!(v.violated_clauses, sorted);
    }

    // ── Extra: gate ids ────────────────────────────────────────────────

    #[test]
    fn gate_ids() {
        assert_eq!(LabPromotionGate::Promote.id(), "promote".to_string());
        assert_eq!(
            LabPromotionGate::Hold {
                reason: "r".to_string()
            }
            .id(),
            "hold(reason=r)".to_string()
        );
    }
}
