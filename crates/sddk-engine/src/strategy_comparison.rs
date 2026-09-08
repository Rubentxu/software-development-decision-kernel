//! Strategy fork / ablation / promotion evidence.
//!
//! Cycle: LAB-WORKFLOW-002 (H6, order 390, context pack `workflow-lab`).
//!
//! Consumes [`ReplayProof`](crate::replay_proof::ReplayProof) records
//! and [`WorkflowMetricSample`](crate::workflow_metrics::WorkflowMetricSample)
//! series and emits a [`PromotionVerdict`] saying whether a candidate
//! strategy may replace a baseline.
//!
//! ## Design
//!
//! - **Closed-set `PromotionPolicy`.** Three variants: `NotRegressing`,
//!   `MoreMatches`, `MaxErrorRateBps`.
//! - **Pure comparator.** `compare` is a pure function of
//!   `(baseline, candidate, policy)`. No I/O, no clock.
//! - **Promotion evidence.** `compare_replay_proofs` is the public
//!   entry point used by future lab cycles.
//! - **Basis points.** `MaxErrorRateBps` uses basis points
//!   (1/10000) to avoid floating-point error.
//! - **`#[non_exhaustive]`** on every public type.
//!
//! See:
//!
//! - `REQ-StrategyComparison` (spec, accepted)
//! - `ADR-098` (architecture decision, accepted)
//! - `REQ-AdvancedGraphReplay`, `REQ-WorkflowLabMetrics` (dependencies)

use serde::{Deserialize, Serialize};

use crate::replay_proof::ReplayProof;
use crate::workflow_metrics::{
    MetricsRecorder, WorkflowMetricKind, WorkflowMetricSample, observe_replay_proof,
};

// ── PromotionPolicy ─────────────────────────────────────────────────────────

/// Closed-set promotion policies.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[non_exhaustive]
pub enum PromotionPolicy {
    /// Promote the candidate iff every metric value is ≥ the
    /// baseline.
    NotRegressing,
    /// Promote the candidate iff strictly more replays match than
    /// the baseline.
    MoreMatches,
    /// Promote the candidate iff the candidate's `ReplayProofMatches`
    /// is at least `min_match_rate_bps` of `ReplayProofCount`.
    /// 10000 bps = 100%.
    MaxErrorRateBps {
        /// Minimum match-rate threshold in basis points (0–10000).
        min_match_rate_bps: u32,
    },
}

impl PromotionPolicy {
    /// Stable identifier for this policy variant + parameters.
    pub fn id(&self) -> String {
        match self {
            PromotionPolicy::NotRegressing => "not_regressing()".to_string(),
            PromotionPolicy::MoreMatches => "more_matches()".to_string(),
            PromotionPolicy::MaxErrorRateBps { min_match_rate_bps } => {
                format!("max_error_rate_bps(threshold={min_match_rate_bps})")
            }
        }
    }
}

// ── PromotionHoldReason ─────────────────────────────────────────────────────

/// Closed-set reasons for `PromotionVerdict::Hold`.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[non_exhaustive]
pub enum PromotionHoldReason {
    /// A metric regressed (NotRegressing policy).
    MetricRegressed {
        /// Which metric regressed.
        metric: WorkflowMetricKind,
        /// Baseline value.
        baseline: u64,
        /// Candidate value.
        candidate: u64,
    },
    /// Candidate has fewer matches than baseline (MoreMatches policy).
    FewerMatches {
        /// Baseline match count.
        baseline: u64,
        /// Candidate match count.
        candidate: u64,
    },
    /// Candidate's match-rate bps below threshold.
    ErrorRateExceeded {
        /// Candidate's actual match rate in basis points.
        actual_bps: u32,
        /// Required minimum in basis points.
        threshold_bps: u32,
        /// Candidate match count.
        matches: u64,
        /// Candidate total proof count.
        total: u64,
    },
}

// ── PromotionVerdict ────────────────────────────────────────────────────────

/// Outcome of a strategy comparison.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[non_exhaustive]
pub enum PromotionVerdict {
    /// Candidate may replace baseline.
    Promote,
    /// Candidate must hold; carry the reason.
    Hold {
        /// Closed-set reason.
        reason: PromotionHoldReason,
    },
}

// ── StrategyComparator ──────────────────────────────────────────────────────

/// Pure, deterministic comparator.
#[derive(Debug, Clone)]
#[non_exhaustive]
pub struct StrategyComparator {
    baseline_metrics: Vec<WorkflowMetricSample>,
    candidate_metrics: Vec<WorkflowMetricSample>,
    policy: PromotionPolicy,
}

impl StrategyComparator {
    /// Construct a comparator.
    pub fn new(
        baseline_metrics: Vec<WorkflowMetricSample>,
        candidate_metrics: Vec<WorkflowMetricSample>,
        policy: PromotionPolicy,
    ) -> Self {
        Self {
            baseline_metrics,
            candidate_metrics,
            policy,
        }
    }

    /// Policy in use.
    pub fn policy(&self) -> &PromotionPolicy {
        &self.policy
    }

    /// Latest `ReplayProofCount` in the baseline series (0 if absent).
    pub fn baseline_total(&self) -> u64 {
        latest_value(
            &self.baseline_metrics,
            &WorkflowMetricKind::ReplayProofCount,
        )
    }

    /// Latest `ReplayProofCount` in the candidate series (0 if absent).
    pub fn candidate_total(&self) -> u64 {
        latest_value(
            &self.candidate_metrics,
            &WorkflowMetricKind::ReplayProofCount,
        )
    }

    /// Compare the two metric series and emit a verdict.
    pub fn compare(&self) -> PromotionVerdict {
        match &self.policy {
            PromotionPolicy::NotRegressing => self.compare_not_regressing(),
            PromotionPolicy::MoreMatches => self.compare_more_matches(),
            PromotionPolicy::MaxErrorRateBps { min_match_rate_bps } => {
                self.compare_max_error_rate(*min_match_rate_bps)
            }
        }
    }

    // ── policy implementations ────────────────────────────────────────────

    fn compare_not_regressing(&self) -> PromotionVerdict {
        for kind in all_metrics() {
            let baseline = latest_value(&self.baseline_metrics, &kind);
            let candidate = latest_value(&self.candidate_metrics, &kind);
            if candidate < baseline {
                return PromotionVerdict::Hold {
                    reason: PromotionHoldReason::MetricRegressed {
                        metric: kind,
                        baseline,
                        candidate,
                    },
                };
            }
        }
        PromotionVerdict::Promote
    }

    fn compare_more_matches(&self) -> PromotionVerdict {
        let baseline = latest_value(
            &self.baseline_metrics,
            &WorkflowMetricKind::ReplayProofMatches,
        );
        let candidate = latest_value(
            &self.candidate_metrics,
            &WorkflowMetricKind::ReplayProofMatches,
        );
        if candidate > baseline {
            PromotionVerdict::Promote
        } else {
            PromotionVerdict::Hold {
                reason: PromotionHoldReason::FewerMatches {
                    baseline,
                    candidate,
                },
            }
        }
    }

    fn compare_max_error_rate(&self, threshold_bps: u32) -> PromotionVerdict {
        let total = self.candidate_total();
        let matches = latest_value(
            &self.candidate_metrics,
            &WorkflowMetricKind::ReplayProofMatches,
        );
        let actual_bps = if total == 0 {
            0
        } else {
            // Saturating math: matches * 10000 / total, clamped to u32.
            ((matches as u128).saturating_mul(10_000) / (total as u128)) as u32
        };
        if actual_bps >= threshold_bps {
            PromotionVerdict::Promote
        } else {
            PromotionVerdict::Hold {
                reason: PromotionHoldReason::ErrorRateExceeded {
                    actual_bps,
                    threshold_bps,
                    matches,
                    total,
                },
            }
        }
    }
}

/// All metrics iterated by `NotRegressing` in deterministic order.
fn all_metrics() -> Vec<WorkflowMetricKind> {
    vec![
        WorkflowMetricKind::ReplayProofCount,
        WorkflowMetricKind::ReplayProofMatches,
        WorkflowMetricKind::ReplayProofDiverges,
        WorkflowMetricKind::TotalProjections,
        WorkflowMetricKind::JoinVerdictsSucceeded,
        WorkflowMetricKind::JoinVerdictsFailed,
        WorkflowMetricKind::ReducerAggregates,
    ]
}

fn latest_value(series: &[WorkflowMetricSample], kind: &WorkflowMetricKind) -> u64 {
    series
        .iter()
        .rev()
        .find(|s| &s.metric == kind)
        .map(|s| s.value)
        .unwrap_or(0)
}

// ── compare_replay_proofs ───────────────────────────────────────────────────

/// Convenience helper: observe two proofs into their recorders and
/// compare them.
pub fn compare_replay_proofs(
    baseline: &ReplayProof,
    candidate: &ReplayProof,
    recorder_baseline: &mut impl MetricsRecorder,
    recorder_candidate: &mut impl MetricsRecorder,
    policy: PromotionPolicy,
    revision_id: String,
    recorded_at: String,
) -> PromotionVerdict {
    observe_replay_proof(
        recorder_baseline,
        baseline,
        revision_id.clone(),
        recorded_at.clone(),
    );
    observe_replay_proof(recorder_candidate, candidate, revision_id, recorded_at);
    let comparator = StrategyComparator::new(
        recorder_baseline.snapshot(),
        recorder_candidate.snapshot(),
        policy,
    );
    comparator.compare()
}

// ── Audit guard ─────────────────────────────────────────────────────────────

#[allow(unused)]
const PROMOTION_POLICY_VARIANT_LIST: &[PromotionPolicy] = &[
    PromotionPolicy::NotRegressing,
    PromotionPolicy::MoreMatches,
    PromotionPolicy::MaxErrorRateBps {
        min_match_rate_bps: 0,
    },
];

#[allow(unused)]
const PROMOTION_HOLD_REASON_VARIANT_LIST: &[PromotionHoldReason] = &[
    PromotionHoldReason::MetricRegressed {
        metric: WorkflowMetricKind::ReplayProofCount,
        baseline: 0,
        candidate: 0,
    },
    PromotionHoldReason::FewerMatches {
        baseline: 0,
        candidate: 0,
    },
    PromotionHoldReason::ErrorRateExceeded {
        actual_bps: 0,
        threshold_bps: 0,
        matches: 0,
        total: 0,
    },
];

// ── Tests ────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;
    use crate::replay_proof::ReplayEngine;
    use crate::typed_reduce_aggregator::{ReducerKind, TypedReduceAggregator};
    use crate::workflow_metrics::{InMemoryMetricsRecorder, WorkflowMetricSample};
    use sddk_domain::SchemaDialect;
    use std::collections::{BTreeMap, BTreeSet};

    fn node_id(s: &str) -> sddk_domain::workflow_ir::NodeId {
        sddk_domain::workflow_ir::NodeId(s.to_string())
    }

    fn number_schema_for(field: &str) -> sddk_domain::OperatorOutputSchema {
        let doc = serde_json::json!({ "type": "number" });
        let op_schema =
            sddk_domain::OperatorSchema::with_defaults(SchemaDialect::JsonSchemaDraft07, doc);
        let mut sf = BTreeMap::new();
        sf.insert(field.to_string(), op_schema);
        let mut req = BTreeSet::new();
        req.insert(field.to_string());
        sddk_domain::OperatorOutputSchema {
            static_fields: sf,
            required_fields: req,
            accepts_extra_fields: false,
            description: None,
        }
    }

    fn projection_with_int(
        parent: &sddk_domain::workflow_ir::NodeId,
        field: &str,
        n: i64,
    ) -> crate::typed_child_output::TypedChildOutput {
        let mut tf = std::collections::BTreeMap::new();
        tf.insert(
            field.to_string(),
            crate::typed_child_output::TypedFieldValue {
                raw: serde_json::json!(n),
                declared_type: sddk_domain::OperatorSchema::with_defaults(
                    SchemaDialect::JsonSchemaDraft07,
                    serde_json::json!({ "type": "number" }),
                ),
            },
        );
        crate::typed_child_output::TypedChildOutput {
            child_attempt_id: sddk_domain::workflow_run::AttemptId(format!("att-{n}")),
            parent_node_id: parent.clone(),
            child_node_id: node_id("C"),
            declared_schema_id: "x".to_string(),
            typed_fields: tf,
            recorded_at: "t".to_string(),
            schema_version: 1,
        }
    }

    fn build_proof_with_score(parent: &sddk_domain::workflow_ir::NodeId, n: i64) -> ReplayProof {
        let reducer = TypedReduceAggregator::new(
            parent.clone(),
            ReducerKind::Sum {
                field: "score".to_string(),
            },
            number_schema_for("sum_score"),
        );
        let engine = ReplayEngine::new(parent.clone()).with_reducer(reducer);
        let batch = vec![projection_with_int(parent, "score", n)];
        let mut original = std::collections::BTreeMap::new();
        original.insert("sum_score".to_string(), serde_json::json!(n));
        engine
            .build_proof(&batch, &original, "rev-0".to_string(), "t".to_string())
            .unwrap()
    }

    fn sample(metric: WorkflowMetricKind, value: u64) -> WorkflowMetricSample {
        WorkflowMetricSample {
            metric,
            value,
            revision_id: "rev".to_string(),
            recorded_at: "t".to_string(),
        }
    }

    // ── S-1: NotRegressing equal ⇒ Promote ────────────────────────────────

    #[test]
    fn s1_not_regressing_equal_promotes() {
        let samples = vec![
            sample(WorkflowMetricKind::ReplayProofCount, 10),
            sample(WorkflowMetricKind::ReplayProofMatches, 9),
        ];
        let comp =
            StrategyComparator::new(samples.clone(), samples, PromotionPolicy::NotRegressing);
        assert_eq!(comp.compare(), PromotionVerdict::Promote);
    }

    // ── S-2: NotRegressing regressed ⇒ Hold ───────────────────────────────

    #[test]
    fn s2_not_regressing_regressed_holds() {
        let baseline = vec![
            sample(WorkflowMetricKind::ReplayProofCount, 10),
            sample(WorkflowMetricKind::ReplayProofMatches, 10),
        ];
        let candidate = vec![
            sample(WorkflowMetricKind::ReplayProofCount, 10),
            sample(WorkflowMetricKind::ReplayProofMatches, 5),
        ];
        let comp = StrategyComparator::new(baseline, candidate, PromotionPolicy::NotRegressing);
        match comp.compare() {
            PromotionVerdict::Hold {
                reason:
                    PromotionHoldReason::MetricRegressed {
                        metric,
                        baseline,
                        candidate,
                    },
            } => {
                assert_eq!(metric, WorkflowMetricKind::ReplayProofMatches);
                assert_eq!(baseline, 10);
                assert_eq!(candidate, 5);
            }
            other => panic!("expected Hold/MetricRegressed, got {other:?}"),
        }
    }

    // ── S-3: MoreMatches — more ⇒ Promote ─────────────────────────────────

    #[test]
    fn s3_more_matches_promotes() {
        let baseline = vec![sample(WorkflowMetricKind::ReplayProofMatches, 10)];
        let candidate = vec![sample(WorkflowMetricKind::ReplayProofMatches, 12)];
        let comp = StrategyComparator::new(baseline, candidate, PromotionPolicy::MoreMatches);
        assert_eq!(comp.compare(), PromotionVerdict::Promote);
    }

    // ── S-4: MoreMatches — fewer ⇒ Hold ───────────────────────────────────

    #[test]
    fn s4_more_matches_fewer_holds() {
        let baseline = vec![sample(WorkflowMetricKind::ReplayProofMatches, 10)];
        let candidate = vec![sample(WorkflowMetricKind::ReplayProofMatches, 8)];
        let comp = StrategyComparator::new(baseline, candidate, PromotionPolicy::MoreMatches);
        match comp.compare() {
            PromotionVerdict::Hold {
                reason:
                    PromotionHoldReason::FewerMatches {
                        baseline,
                        candidate,
                    },
            } => {
                assert_eq!(baseline, 10);
                assert_eq!(candidate, 8);
            }
            other => panic!("expected Hold/FewerMatches, got {other:?}"),
        }
    }

    // ── S-5: MaxErrorRateBps — 9600 ≥ 9500 ⇒ Promote ─────────────────────

    #[test]
    fn s5_max_error_rate_threshold_met_promotes() {
        let candidate = vec![
            sample(WorkflowMetricKind::ReplayProofCount, 100),
            sample(WorkflowMetricKind::ReplayProofMatches, 96),
        ];
        let comp = StrategyComparator::new(
            vec![],
            candidate,
            PromotionPolicy::MaxErrorRateBps {
                min_match_rate_bps: 9500,
            },
        );
        assert_eq!(comp.compare(), PromotionVerdict::Promote);
    }

    // ── S-6: MaxErrorRateBps — 9000 < 9500 ⇒ Hold ────────────────────────

    #[test]
    fn s6_max_error_rate_threshold_unmet_holds() {
        let candidate = vec![
            sample(WorkflowMetricKind::ReplayProofCount, 100),
            sample(WorkflowMetricKind::ReplayProofMatches, 90),
        ];
        let comp = StrategyComparator::new(
            vec![],
            candidate,
            PromotionPolicy::MaxErrorRateBps {
                min_match_rate_bps: 9500,
            },
        );
        match comp.compare() {
            PromotionVerdict::Hold {
                reason:
                    PromotionHoldReason::ErrorRateExceeded {
                        actual_bps,
                        threshold_bps,
                        matches,
                        total,
                    },
            } => {
                assert_eq!(actual_bps, 9000);
                assert_eq!(threshold_bps, 9500);
                assert_eq!(matches, 90);
                assert_eq!(total, 100);
            }
            other => panic!("expected Hold/ErrorRateExceeded, got {other:?}"),
        }
    }

    // ── S-7: Determinism ──────────────────────────────────────────────────

    #[test]
    fn s7_determinism_byte_equal() {
        let samples = vec![sample(WorkflowMetricKind::ReplayProofMatches, 10)];
        let comp = StrategyComparator::new(samples.clone(), samples, PromotionPolicy::MoreMatches);
        let v1 = comp.compare();
        let v2 = comp.compare();
        assert_eq!(
            serde_json::to_string(&v1).unwrap(),
            serde_json::to_string(&v2).unwrap()
        );
    }

    // ── S-8: Empty candidate ⇒ Hold with ErrorRateExceeded (0 bps) ──────

    #[test]
    fn s8_empty_candidate_holds() {
        let comp = StrategyComparator::new(
            vec![],
            vec![],
            PromotionPolicy::MaxErrorRateBps {
                min_match_rate_bps: 9500,
            },
        );
        match comp.compare() {
            PromotionVerdict::Hold {
                reason:
                    PromotionHoldReason::ErrorRateExceeded {
                        actual_bps, total, ..
                    },
            } => {
                assert_eq!(actual_bps, 0);
                assert_eq!(total, 0);
            }
            other => panic!("expected Hold/ErrorRateExceeded, got {other:?}"),
        }
    }

    // ── Extra: compare_replay_proofs round-trip ───────────────────────────

    #[test]
    fn compare_replay_proofs_round_trip() {
        let p = node_id("P");
        let baseline = build_proof_with_score(&p, 10);
        let candidate = build_proof_with_score(&p, 10);
        let mut rb = InMemoryMetricsRecorder::new();
        let mut rc = InMemoryMetricsRecorder::new();
        let verdict = compare_replay_proofs(
            &baseline,
            &candidate,
            &mut rb,
            &mut rc,
            PromotionPolicy::MoreMatches,
            "rev".to_string(),
            "t".to_string(),
        );
        // Both proofs match ⇒ both have ReplayProofMatches = 1 ⇒ tie ⇒ Hold.
        assert!(matches!(verdict, PromotionVerdict::Hold { .. }));
    }

    // ── Extra: baseline_total / candidate_total ──────────────────────────

    #[test]
    fn totals_are_latest_count() {
        let samples = vec![
            sample(WorkflowMetricKind::ReplayProofCount, 5),
            sample(WorkflowMetricKind::ReplayProofCount, 7),
        ];
        let comp = StrategyComparator::new(samples, vec![], PromotionPolicy::NotRegressing);
        assert_eq!(comp.baseline_total(), 7);
        assert_eq!(comp.candidate_total(), 0);
    }

    // ── Extra: PromotionPolicy::id stability ─────────────────────────────

    #[test]
    fn promotion_policy_ids_stable() {
        assert_eq!(
            PromotionPolicy::NotRegressing.id(),
            "not_regressing()".to_string()
        );
        assert_eq!(
            PromotionPolicy::MoreMatches.id(),
            "more_matches()".to_string()
        );
        assert_eq!(
            PromotionPolicy::MaxErrorRateBps {
                min_match_rate_bps: 9500
            }
            .id(),
            "max_error_rate_bps(threshold=9500)".to_string()
        );
    }
}
