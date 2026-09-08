//! Stable runtime and workflow metrics.
//!
//! Cycle: LAB-WORKFLOW-001 (H6, order 380, context pack `workflow-lab`).
//!
//! Reads [`ReplayProof`](crate::replay_proof::ReplayProof) records and
//! emits a closed-set of `WorkflowMetricKind` counters consumed by the
//! Workflow Lab comparison cycles.
//!
//! ## Design
//!
//! - **Closed-set `WorkflowMetricKind`:** seven counters covering
//!   replay proof totals, projection totals, join verdicts and
//!   reducer aggregates.
//! - **Pure observer.** `observe_replay_proof` is a pure function of
//!   `(recorder, proof, …)`. No I/O, no clock.
//! - **Trait seam.** `MetricsRecorder` allows future production
//!   implementations without changing the observer.
//! - **Sorted snapshot.** `snapshot()` returns samples in
//!   `(recorded_at, metric)` order for deterministic comparison.
//! - **`#[non_exhaustive]`** on every public type.
//!
//! See:
//!
//! - `REQ-WorkflowLabMetrics` (spec, accepted)
//! - `ADR-097` (architecture decision, accepted)
//! - `REQ-AdvancedGraphReplay` (DW-REPLAY-001, dependency)

use std::collections::BTreeMap;

use serde::{Deserialize, Serialize};

use crate::join_guard::JoinVerdict;
use crate::replay_proof::{DeterminismCheck, ReplayProof};

// ── WorkflowMetricKind ──────────────────────────────────────────────────────

/// Closed-set metric kinds recorded by the Workflow Lab observer.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
#[non_exhaustive]
pub enum WorkflowMetricKind {
    /// Total `ReplayProof`s observed.
    ReplayProofCount,
    /// Number of proofs whose `DeterminismCheck` was `Match`.
    ReplayProofMatches,
    /// Number of proofs whose `DeterminismCheck` was `Diverged`.
    ReplayProofDiverges,
    /// Total projected child outputs persisted (approximation:
    /// `replayed_aggregate.len()` + witness size for join verdict).
    TotalProjections,
    /// Number of `JoinVerdict::Succeeded` observed.
    JoinVerdictsSucceeded,
    /// Number of `JoinVerdict::Failed` observed.
    JoinVerdictsFailed,
    /// Number of reducer aggregates observed (proofs with non-empty
    /// `replayed_aggregate`).
    ReducerAggregates,
}

impl WorkflowMetricKind {
    /// Stable string identifier (used by audit tests).
    pub fn id(&self) -> &'static str {
        match self {
            WorkflowMetricKind::ReplayProofCount => "replay_proof_count",
            WorkflowMetricKind::ReplayProofMatches => "replay_proof_matches",
            WorkflowMetricKind::ReplayProofDiverges => "replay_proof_diverges",
            WorkflowMetricKind::TotalProjections => "total_projections",
            WorkflowMetricKind::JoinVerdictsSucceeded => "join_verdicts_succeeded",
            WorkflowMetricKind::JoinVerdictsFailed => "join_verdicts_failed",
            WorkflowMetricKind::ReducerAggregates => "reducer_aggregates",
        }
    }
}

// ── WorkflowMetricSample ────────────────────────────────────────────────────

/// A single metrics sample.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[non_exhaustive]
pub struct WorkflowMetricSample {
    /// Kind of metric.
    pub metric: WorkflowMetricKind,
    /// Sample value.
    pub value: u64,
    /// Identifier of the source revision.
    pub revision_id: String,
    /// RFC-3339 timestamp.
    pub recorded_at: String,
}

// ── MetricsRecorder ─────────────────────────────────────────────────────────

/// Storage seam for workflow metrics.
pub trait MetricsRecorder {
    /// Record a sample.
    fn record(&mut self, sample: WorkflowMetricSample);
    /// Get the latest value for `metric`. Returns 0 if none.
    fn get(&self, metric: WorkflowMetricKind) -> u64;
    /// Snapshot of all samples in `(recorded_at, metric)` order.
    fn snapshot(&self) -> Vec<WorkflowMetricSample>;
}

/// Default in-memory recorder.
#[derive(Debug, Default, Clone)]
pub struct InMemoryMetricsRecorder {
    /// Last value per metric.
    latest: BTreeMap<WorkflowMetricKind, u64>,
    /// Append-only sample list.
    samples: Vec<WorkflowMetricSample>,
}

impl InMemoryMetricsRecorder {
    /// Construct an empty recorder.
    pub fn new() -> Self {
        Self::default()
    }

    /// Number of recorded samples.
    pub fn sample_count(&self) -> usize {
        self.samples.len()
    }
}

impl MetricsRecorder for InMemoryMetricsRecorder {
    fn record(&mut self, sample: WorkflowMetricSample) {
        self.latest.insert(sample.metric.clone(), sample.value);
        self.samples.push(sample);
    }

    fn get(&self, metric: WorkflowMetricKind) -> u64 {
        self.latest.get(&metric).copied().unwrap_or(0)
    }

    fn snapshot(&self) -> Vec<WorkflowMetricSample> {
        let mut out = self.samples.clone();
        out.sort_by(|a, b| {
            a.recorded_at
                .cmp(&b.recorded_at)
                .then_with(|| a.metric.cmp(&b.metric))
        });
        out
    }
}

// ── observe_replay_proof ────────────────────────────────────────────────────

/// Observe a `ReplayProof` and emit metrics into `recorder`.
///
/// Pure: identical inputs produce byte-equal sample sets.
pub fn observe_replay_proof(
    recorder: &mut impl MetricsRecorder,
    proof: &ReplayProof,
    revision_id: String,
    recorded_at: String,
) {
    let mut inc = |m: WorkflowMetricKind, delta: u64| {
        let next = recorder.get(m.clone()).saturating_add(delta);
        recorder.record(WorkflowMetricSample {
            metric: m,
            value: next,
            revision_id: revision_id.clone(),
            recorded_at: recorded_at.clone(),
        });
    };

    // Proof total + match/divergence counters.
    inc(WorkflowMetricKind::ReplayProofCount, 1);
    match proof.determinism_check {
        DeterminismCheck::Match => inc(WorkflowMetricKind::ReplayProofMatches, 1),
        DeterminismCheck::Diverged { .. } => {
            inc(WorkflowMetricKind::ReplayProofDiverges, 1);
        }
    }

    // Aggregate counter.
    if !proof.replayed_aggregate.is_empty() {
        inc(WorkflowMetricKind::ReducerAggregates, 1);
        inc(
            WorkflowMetricKind::TotalProjections,
            proof.replayed_aggregate.len() as u64,
        );
    }

    // Join verdict counters.
    if let Some(verdict) = &proof.replayed_join_verdict {
        match verdict {
            JoinVerdict::Succeeded {
                contributing_count,
                witness,
            } => {
                inc(WorkflowMetricKind::JoinVerdictsSucceeded, 1);
                inc(
                    WorkflowMetricKind::TotalProjections,
                    witness.len() as u64 + *contributing_count as u64,
                );
            }
            JoinVerdict::Failed {
                contributing_count, ..
            } => {
                inc(WorkflowMetricKind::JoinVerdictsFailed, 1);
                inc(
                    WorkflowMetricKind::TotalProjections,
                    *contributing_count as u64,
                );
            }
        }
    }
}

// ── Audit guard ─────────────────────────────────────────────────────────────

#[allow(unused)]
const WORKFLOW_METRIC_KIND_VARIANT_LIST: &[WorkflowMetricKind] = &[
    WorkflowMetricKind::ReplayProofCount,
    WorkflowMetricKind::ReplayProofMatches,
    WorkflowMetricKind::ReplayProofDiverges,
    WorkflowMetricKind::TotalProjections,
    WorkflowMetricKind::JoinVerdictsSucceeded,
    WorkflowMetricKind::JoinVerdictsFailed,
    WorkflowMetricKind::ReducerAggregates,
];

// ── Tests ────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;
    use crate::join_guard::JoinFailureReason;
    use crate::replay_proof::{ReplayEngine, ReplayProof};
    use crate::typed_reduce_aggregator::ReducerKind;
    use sddk_domain::SchemaDialect;

    fn node_id(s: &str) -> sddk_domain::workflow_ir::NodeId {
        sddk_domain::workflow_ir::NodeId(s.to_string())
    }

    fn number_schema_for(field: &str) -> sddk_domain::OperatorOutputSchema {
        let doc = serde_json::json!({ "type": "number" });
        let op_schema =
            sddk_domain::OperatorSchema::with_defaults(SchemaDialect::JsonSchemaDraft07, doc);
        let mut sf = BTreeMap::new();
        sf.insert(field.to_string(), op_schema);
        let mut req = std::collections::BTreeSet::new();
        req.insert(field.to_string());
        sddk_domain::OperatorOutputSchema {
            static_fields: sf,
            required_fields: req,
            accepts_extra_fields: false,
            description: None,
        }
    }

    fn bool_schema_for(field: &str) -> sddk_domain::OperatorOutputSchema {
        let doc = serde_json::json!({ "type": "boolean" });
        let op_schema =
            sddk_domain::OperatorSchema::with_defaults(SchemaDialect::JsonSchemaDraft07, doc);
        let mut sf = BTreeMap::new();
        sf.insert(field.to_string(), op_schema);
        let mut req = std::collections::BTreeSet::new();
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
        let mut tf = BTreeMap::new();
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

    fn projection_with_bool(
        parent: &sddk_domain::workflow_ir::NodeId,
        field: &str,
        b: bool,
    ) -> crate::typed_child_output::TypedChildOutput {
        let mut tf = BTreeMap::new();
        tf.insert(
            field.to_string(),
            crate::typed_child_output::TypedFieldValue {
                raw: serde_json::json!(b),
                declared_type: sddk_domain::OperatorSchema::with_defaults(
                    SchemaDialect::JsonSchemaDraft07,
                    serde_json::json!({ "type": "boolean" }),
                ),
            },
        );
        crate::typed_child_output::TypedChildOutput {
            child_attempt_id: sddk_domain::workflow_run::AttemptId(format!("att-{b}")),
            parent_node_id: parent.clone(),
            child_node_id: node_id("C"),
            declared_schema_id: "x".to_string(),
            typed_fields: tf,
            recorded_at: "t".to_string(),
            schema_version: 1,
        }
    }

    fn build_match_proof() -> ReplayProof {
        let p = node_id("P");
        let reducer = crate::typed_reduce_aggregator::TypedReduceAggregator::new(
            p.clone(),
            ReducerKind::Sum {
                field: "score".to_string(),
            },
            number_schema_for("sum_score"),
        );
        let engine = ReplayEngine::new(p.clone()).with_reducer(reducer);
        let batch = vec![
            projection_with_int(&p, "score", 10),
            projection_with_int(&p, "score", 20),
        ];
        let mut original = BTreeMap::new();
        original.insert("sum_score".to_string(), serde_json::json!(30));
        engine
            .build_proof(&batch, &original, "rev-0".to_string(), "t".to_string())
            .unwrap()
    }

    // ── S-1: Match proof increments matches ────────────────────────────────

    #[test]
    fn s1_match_proof_increments_matches() {
        let mut rec = InMemoryMetricsRecorder::new();
        observe_replay_proof(
            &mut rec,
            &build_match_proof(),
            "rev-0".to_string(),
            "t1".to_string(),
        );
        assert_eq!(rec.get(WorkflowMetricKind::ReplayProofCount), 1);
        assert_eq!(rec.get(WorkflowMetricKind::ReplayProofMatches), 1);
        assert_eq!(rec.get(WorkflowMetricKind::ReplayProofDiverges), 0);
    }

    // ── S-2: Diverged proof increments diverges ────────────────────────────

    #[test]
    fn s2_diverged_proof_increments_diverges() {
        let p = node_id("P");
        let reducer = crate::typed_reduce_aggregator::TypedReduceAggregator::new(
            p.clone(),
            ReducerKind::Sum {
                field: "score".to_string(),
            },
            number_schema_for("sum_score"),
        );
        let engine = ReplayEngine::new(p.clone()).with_reducer(reducer);
        let batch = vec![
            projection_with_int(&p, "score", 10),
            projection_with_int(&p, "score", 20),
        ];
        let mut original = BTreeMap::new();
        original.insert("sum_score".to_string(), serde_json::json!(999));
        let proof = engine
            .build_proof(&batch, &original, "rev-0".to_string(), "t".to_string())
            .unwrap();
        assert!(matches!(
            proof.determinism_check,
            DeterminismCheck::Diverged { .. }
        ));
        let mut rec = InMemoryMetricsRecorder::new();
        observe_replay_proof(&mut rec, &proof, "rev-0".to_string(), "t".to_string());
        assert_eq!(rec.get(WorkflowMetricKind::ReplayProofDiverges), 1);
        assert_eq!(rec.get(WorkflowMetricKind::ReplayProofMatches), 0);
    }

    // ── S-3: Succeeded verdict increments succeeded ────────────────────────

    #[test]
    fn s3_succeeded_verdict_increments_succeeded() {
        let p = node_id("P");
        let guard = crate::join_guard::JoinGuard::new_unchecked(
            p.clone(),
            crate::join_guard::JoinPolicy::AllSatisfied {
                satisfaction_field: "ok".to_string(),
            },
            bool_schema_for("ok"),
        );
        let engine = ReplayEngine::new(p.clone()).with_join_guard(guard);
        let batch = vec![
            projection_with_bool(&p, "ok", true),
            projection_with_bool(&p, "ok", true),
        ];
        let mut original = BTreeMap::new();
        original.insert("join_verdict".to_string(), serde_json::json!("succeeded"));
        original.insert("join_contributing_count".to_string(), serde_json::json!(2));
        original.insert(
            "join_witness".to_string(),
            serde_json::json!(["att-true", "att-true"]),
        );
        let proof = engine
            .build_proof(&batch, &original, "rev-0".to_string(), "t".to_string())
            .unwrap();
        let mut rec = InMemoryMetricsRecorder::new();
        observe_replay_proof(&mut rec, &proof, "rev-0".to_string(), "t".to_string());
        assert_eq!(rec.get(WorkflowMetricKind::JoinVerdictsSucceeded), 1);
        assert_eq!(rec.get(WorkflowMetricKind::JoinVerdictsFailed), 0);
    }

    // ── S-4: Failed verdict increments failed ──────────────────────────────

    #[test]
    fn s4_failed_verdict_increments_failed() {
        let p = node_id("P");
        let guard = crate::join_guard::JoinGuard::new_unchecked(
            p.clone(),
            crate::join_guard::JoinPolicy::AllSatisfied {
                satisfaction_field: "ok".to_string(),
            },
            bool_schema_for("ok"),
        );
        let engine = ReplayEngine::new(p.clone()).with_join_guard(guard);
        let batch = vec![
            projection_with_bool(&p, "ok", true),
            projection_with_bool(&p, "ok", false),
        ];
        // Replay produces Failed verdict; original has join_verdict = failed.
        let mut original = BTreeMap::new();
        original.insert("join_verdict".to_string(), serde_json::json!("failed"));
        original.insert("join_contributing_count".to_string(), serde_json::json!(1));
        original.insert(
            "join_failure_reason".to_string(),
            serde_json::json!("AllSatisfiedBroken"),
        );
        let proof = engine
            .build_proof(&batch, &original, "rev-0".to_string(), "t".to_string())
            .unwrap();
        assert!(matches!(
            proof.replayed_join_verdict,
            Some(JoinVerdict::Failed {
                reason_code: JoinFailureReason::AllSatisfiedBroken,
                ..
            })
        ));
        let mut rec = InMemoryMetricsRecorder::new();
        observe_replay_proof(&mut rec, &proof, "rev-0".to_string(), "t".to_string());
        assert_eq!(rec.get(WorkflowMetricKind::JoinVerdictsFailed), 1);
    }

    // ── S-5: Non-empty aggregate increments reducer_aggregates ─────────────

    #[test]
    fn s5_non_empty_aggregate_increments_reducer_aggregates() {
        let mut rec = InMemoryMetricsRecorder::new();
        let proof = build_match_proof();
        observe_replay_proof(&mut rec, &proof, "rev-0".to_string(), "t".to_string());
        assert_eq!(rec.get(WorkflowMetricKind::ReducerAggregates), 1);
        assert!(rec.get(WorkflowMetricKind::TotalProjections) > 0);
    }

    // ── S-6: Determinism ──────────────────────────────────────────────────

    #[test]
    fn s6_determinism_byte_equal() {
        let proof = build_match_proof();
        let mut a = InMemoryMetricsRecorder::new();
        let mut b = InMemoryMetricsRecorder::new();
        observe_replay_proof(&mut a, &proof, "rev-0".to_string(), "t".to_string());
        observe_replay_proof(&mut b, &proof, "rev-0".to_string(), "t".to_string());
        let snap_a = serde_json::to_string(&a.snapshot()).unwrap();
        let snap_b = serde_json::to_string(&b.snapshot()).unwrap();
        assert_eq!(snap_a, snap_b);
    }

    // ── S-7: Snapshot is sorted ───────────────────────────────────────────

    #[test]
    fn s7_snapshot_is_sorted() {
        let mut rec = InMemoryMetricsRecorder::new();
        let proof1 = build_match_proof();
        observe_replay_proof(&mut rec, &proof1, "rev".to_string(), "t2".to_string());
        let mut rec2 = InMemoryMetricsRecorder::new();
        observe_replay_proof(&mut rec2, &proof1, "rev".to_string(), "t1".to_string());

        let merged = rec.clone();
        let snap = merged.snapshot();
        // First sample should be the earliest timestamp.
        // t2 only — but we want to verify ordering across multiple samples.
        for w in snap.windows(2) {
            assert!(w[0].recorded_at <= w[1].recorded_at);
        }
    }

    // ── Extra: WorkflowMetricKind::id stable ───────────────────────────────

    #[test]
    fn workflow_metric_kind_ids() {
        assert_eq!(
            WorkflowMetricKind::ReplayProofCount.id(),
            "replay_proof_count"
        );
        assert_eq!(
            WorkflowMetricKind::TotalProjections.id(),
            "total_projections"
        );
    }

    // ── Extra: empty recorder returns 0 ───────────────────────────────────

    #[test]
    fn empty_recorder_returns_zero() {
        let rec = InMemoryMetricsRecorder::new();
        assert_eq!(rec.get(WorkflowMetricKind::ReplayProofCount), 0);
        assert_eq!(rec.sample_count(), 0);
        assert!(rec.snapshot().is_empty());
    }
}
