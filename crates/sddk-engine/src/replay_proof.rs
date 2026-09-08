//! Advanced graph replay and recovery proof.
//!
//! Cycle: DW-REPLAY-001 (H6, order 370, context pack `advanced-runtime`).
//!
//! Given a batch of [`TypedChildOutput`](crate::typed_child_output::TypedChildOutput)
//! projections and the original outputs captured at runtime, this
//! module produces a [`ReplayProof`] that demonstrates the
//! `original_outputs` can be reconstructed from the projection store
//! alone. Pure, deterministic, replay-friendly.
//!
//! ## Design
//!
//! - **Pure engine.** `ReplayEngine::build_proof` is a pure function
//!   of `(projections, original_outputs, reducer, guard)`. No I/O,
//!   no clock.
//! - **SHA-256 projection digest.** Pins the input batch so reviewers
//!   can detect drift between revisions.
//! - **Closed-set error type.** `ReplayError` wraps inner reducer /
//!   guard errors.
//! - **Per-field divergence check.** `DeterminismCheck::Diverged`
//!   reports the first diverged field.
//! - **`#[non_exhaustive]`** on every public type.
//!
//! See:
//!
//! - `REQ-AdvancedGraphReplay` (spec, accepted)
//! - `ADR-096` (architecture decision, accepted)
//! - `REQ-TypedChildOutputLineage`, `REQ-DurableMapFanOut`,
//!   `REQ-TypedReduceAggregator`, `REQ-JoinGuard`, `REQ-Map-Cross-Tick-Replay`
//!   (dependencies)

use std::collections::BTreeMap;

use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};

use sddk_domain::workflow_ir::NodeId;

use crate::join_guard::{JoinGuard, JoinVerdict};
use crate::typed_child_output::TypedChildOutput;
use crate::typed_reduce_aggregator::{
    ReduceAggregatorError, TypedAggregateOutput, TypedReduceAggregator,
};

// ── DeterminismCheck ────────────────────────────────────────────────────────

/// Outcome of comparing replayed vs original outputs.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[non_exhaustive]
pub enum DeterminismCheck {
    /// All matched fields agree byte-for-byte.
    Match,
    /// The replayed output diverged from the original at the named
    /// field. The error carries the JSON kind of each side.
    Diverged {
        /// Field name where divergence was detected.
        field: String,
        /// JSON kind of the replayed value.
        replayed_kind: String,
        /// JSON kind of the original value.
        original_kind: String,
    },
}

// ── ReplayProof ─────────────────────────────────────────────────────────────

/// A typed, deterministic replay proof.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[non_exhaustive]
pub struct ReplayProof {
    /// Identifier of the source `ExecutionGraphRevision`.
    pub source_revision_id: String,
    /// Parent node id of the proven aggregate / verdict.
    pub parent_node_id: NodeId,
    /// SHA-256 hex digest of the canonical JSON of the input batch
    /// (pinning the projection to the revision).
    pub projection_digest: String,
    /// Replayed aggregate payload (empty if no reducer).
    pub replayed_aggregate: BTreeMap<String, serde_json::Value>,
    /// Replayed join verdict (`None` if no guard).
    pub replayed_join_verdict: Option<JoinVerdict>,
    /// Original outputs captured at runtime.
    pub original_outputs: BTreeMap<String, serde_json::Value>,
    /// Outcome of the byte-equality check.
    pub determinism_check: DeterminismCheck,
    /// RFC-3339 timestamp supplied by the caller.
    pub generated_at: String,
    /// Schema version of this proof.
    pub schema_version: u32,
}

impl ReplayProof {
    /// Constant schema version.
    pub const SCHEMA_VERSION: u32 = 1;
}

// ── ReplayError ─────────────────────────────────────────────────────────────

/// Errors emitted by [`ReplayEngine::build_proof`].
#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error, Serialize, Deserialize)]
#[non_exhaustive]
pub enum ReplayError {
    /// The configured reducer returned an error.
    #[error("reducer failed: {0}")]
    ReducerFailed(#[from] ReduceAggregatorError),
    /// The configured join guard returned an error.
    #[error("join guard failed: {0}")]
    GuardFailed(#[from] crate::join_guard::JoinGuardError),
    /// The replay produced a key that the original outputs do not
    /// contain.
    #[error("replay produced key {key:?} which is missing from original outputs")]
    MissingReplayedKey {
        /// Key name.
        key: String,
    },
    /// The original outputs contain a key the replay did not
    /// produce.
    #[error("original outputs contain key {key:?} which the replay did not produce")]
    ExtraOriginalKey {
        /// Key name.
        key: String,
    },
}

// ── ReplayEngine ────────────────────────────────────────────────────────────

/// Pure, deterministic replay engine.
#[derive(Debug, Clone)]
#[non_exhaustive]
pub struct ReplayEngine {
    parent_node_id: NodeId,
    reducer: Option<TypedReduceAggregator>,
    join_guard: Option<JoinGuard>,
}

impl ReplayEngine {
    /// Construct an empty engine for `parent_node_id`.
    pub fn new(parent_node_id: NodeId) -> Self {
        Self {
            parent_node_id,
            reducer: None,
            join_guard: None,
        }
    }

    /// Add a reducer.
    pub fn with_reducer(mut self, r: TypedReduceAggregator) -> Self {
        self.reducer = Some(r);
        self
    }

    /// Add a join guard.
    pub fn with_join_guard(mut self, g: JoinGuard) -> Self {
        self.join_guard = Some(g);
        self
    }

    /// Whether the engine has a reducer configured.
    pub fn has_reducer(&self) -> bool {
        self.reducer.is_some()
    }

    /// Whether the engine has a join guard configured.
    pub fn has_join_guard(&self) -> bool {
        self.join_guard.is_some()
    }

    /// Parent node id.
    pub fn parent_node_id(&self) -> &NodeId {
        &self.parent_node_id
    }

    /// Build a replay proof.
    pub fn build_proof(
        &self,
        projections: &[TypedChildOutput],
        original_outputs: &BTreeMap<String, serde_json::Value>,
        source_revision_id: String,
        generated_at: String,
    ) -> Result<ReplayProof, ReplayError> {
        let projection_digest = digest_projections(projections);

        // Run reducer if configured.
        let replayed_aggregate = match &self.reducer {
            Some(r) => {
                let agg: TypedAggregateOutput = r.aggregate(projections)?;
                agg.aggregate_payload
            }
            None => BTreeMap::new(),
        };

        // Run guard if configured.
        let replayed_join_verdict = match &self.join_guard {
            Some(g) => Some(g.decide(projections)?),
            None => None,
        };

        // Build the comparison map: replay's aggregate keys + (if a
        // guard verdict is present) a `join_verdict` JSON tag, plus a
        // count if the guard succeeded.
        let mut replayed_compare: BTreeMap<String, serde_json::Value> = replayed_aggregate.clone();
        if let Some(verdict) = &replayed_join_verdict {
            match verdict {
                JoinVerdict::Succeeded {
                    contributing_count,
                    witness,
                } => {
                    replayed_compare
                        .insert("join_verdict".to_string(), serde_json::json!("succeeded"));
                    replayed_compare.insert(
                        "join_contributing_count".to_string(),
                        serde_json::json!(*contributing_count),
                    );
                    let witness_ids: Vec<String> = witness.iter().map(|a| a.0.clone()).collect();
                    replayed_compare
                        .insert("join_witness".to_string(), serde_json::json!(witness_ids));
                }
                JoinVerdict::Failed {
                    reason_code,
                    contributing_count,
                } => {
                    replayed_compare
                        .insert("join_verdict".to_string(), serde_json::json!("failed"));
                    replayed_compare.insert(
                        "join_contributing_count".to_string(),
                        serde_json::json!(*contributing_count),
                    );
                    replayed_compare.insert(
                        "join_failure_reason".to_string(),
                        serde_json::json!(format!("{reason_code:?}")),
                    );
                }
            }
        }

        // Compare keys: every replayed_compare key MUST exist in
        // original_outputs, and vice versa.
        for k in replayed_compare.keys() {
            if !original_outputs.contains_key(k) {
                return Err(ReplayError::MissingReplayedKey { key: k.clone() });
            }
        }
        for k in original_outputs.keys() {
            if !replayed_compare.contains_key(k) {
                return Err(ReplayError::ExtraOriginalKey { key: k.clone() });
            }
        }

        // Per-field byte-equality check.
        let mut check = DeterminismCheck::Match;
        for (k, replayed_v) in &replayed_compare {
            let original_v = original_outputs.get(k).expect("key present by guard above");
            if !json_equal(replayed_v, original_v) {
                check = DeterminismCheck::Diverged {
                    field: k.clone(),
                    replayed_kind: kind_name(replayed_v),
                    original_kind: kind_name(original_v),
                };
                break;
            }
        }

        Ok(ReplayProof {
            source_revision_id,
            parent_node_id: self.parent_node_id.clone(),
            projection_digest,
            replayed_aggregate,
            replayed_join_verdict,
            original_outputs: original_outputs.clone(),
            determinism_check: check,
            generated_at,
            schema_version: ReplayProof::SCHEMA_VERSION,
        })
    }
}

// ── Helpers ─────────────────────────────────────────────────────────────────

/// SHA-256 of the canonical JSON serialization of the slice.
fn digest_projections(projections: &[TypedChildOutput]) -> String {
    let mut hasher = Sha256::new();
    // BTreeMap fields inside TypedChildOutput keep deterministic
    // ordering; serde_json::to_string on a Vec preserves slice order.
    let json = serde_json::to_string(projections).expect("TypedChildOutput is serializable");
    hasher.update(json.as_bytes());
    let result = hasher.finalize();
    let mut hex = String::with_capacity(64);
    for b in result {
        use std::fmt::Write;
        let _ = write!(hex, "{b:02x}");
    }
    hex
}

/// Byte-equality of two `serde_json::Value` under canonical form.
fn json_equal(a: &serde_json::Value, b: &serde_json::Value) -> bool {
    serde_json::to_string(a).ok() == serde_json::to_string(b).ok()
}

fn kind_name(v: &serde_json::Value) -> String {
    match v {
        serde_json::Value::Null => "null".to_string(),
        serde_json::Value::Bool(_) => "boolean".to_string(),
        serde_json::Value::Number(_) => "number".to_string(),
        serde_json::Value::String(_) => "string".to_string(),
        serde_json::Value::Array(_) => "array".to_string(),
        serde_json::Value::Object(_) => "object".to_string(),
    }
}

// ── Tests ────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;
    use crate::join_guard::{JoinFailureReason, JoinPolicy};
    use crate::typed_reduce_aggregator::ReducerKind;
    use sddk_domain::SchemaDialect;
    use std::collections::BTreeSet;

    fn node_id(s: &str) -> NodeId {
        NodeId(s.to_string())
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

    fn bool_schema_for(field: &str) -> sddk_domain::OperatorOutputSchema {
        let doc = serde_json::json!({ "type": "boolean" });
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

    fn projection_with_int_field(parent: &NodeId, field: &str, n: i64) -> TypedChildOutput {
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
        TypedChildOutput {
            child_attempt_id: sddk_domain::workflow_run::AttemptId(format!("att-{n}")),
            parent_node_id: parent.clone(),
            child_node_id: node_id("C"),
            declared_schema_id: "x".to_string(),
            typed_fields: tf,
            recorded_at: "t".to_string(),
            schema_version: 1,
        }
    }

    fn projection_with_bool_field(parent: &NodeId, field: &str, b: bool) -> TypedChildOutput {
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
        TypedChildOutput {
            child_attempt_id: sddk_domain::workflow_run::AttemptId(format!("att-{b}")),
            parent_node_id: parent.clone(),
            child_node_id: node_id("C"),
            declared_schema_id: "x".to_string(),
            typed_fields: tf,
            recorded_at: "t".to_string(),
            schema_version: 1,
        }
    }

    fn three_int_projections(parent: &NodeId, field: &str) -> Vec<TypedChildOutput> {
        vec![
            projection_with_int_field(parent, field, 10),
            projection_with_int_field(parent, field, 20),
            projection_with_int_field(parent, field, 30),
        ]
    }

    // ── S-1: Match — Sum ───────────────────────────────────────────────────

    #[test]
    fn s1_sum_replay_matches() {
        let p = node_id("P");
        let reducer = TypedReduceAggregator::new(
            p.clone(),
            ReducerKind::Sum {
                field: "score".to_string(),
            },
            number_schema_for("sum_score"),
        );
        let engine = ReplayEngine::new(p.clone()).with_reducer(reducer);
        let batch = three_int_projections(&p, "score");
        let mut original = BTreeMap::new();
        original.insert("sum_score".to_string(), serde_json::json!(60));
        let proof = engine
            .build_proof(&batch, &original, "rev-0".to_string(), "t".to_string())
            .unwrap();
        assert_eq!(proof.replayed_aggregate["sum_score"], serde_json::json!(60));
        assert!(matches!(proof.determinism_check, DeterminismCheck::Match));
        assert_eq!(proof.parent_node_id, p);
    }

    // ── S-2: Diverged ─────────────────────────────────────────────────────

    #[test]
    fn s2_diverged_when_outputs_differ() {
        let p = node_id("P");
        let reducer = TypedReduceAggregator::new(
            p.clone(),
            ReducerKind::Sum {
                field: "score".to_string(),
            },
            number_schema_for("sum_score"),
        );
        let engine = ReplayEngine::new(p.clone()).with_reducer(reducer);
        let batch = three_int_projections(&p, "score");
        let mut original = BTreeMap::new();
        original.insert("sum_score".to_string(), serde_json::json!(100));
        let proof = engine
            .build_proof(&batch, &original, "rev-0".to_string(), "t".to_string())
            .unwrap();
        match proof.determinism_check {
            DeterminismCheck::Diverged { field, .. } => {
                assert_eq!(field, "sum_score");
            }
            other => panic!("expected Diverged, got {other:?}"),
        }
    }

    // ── S-3: Missing replayed key ─────────────────────────────────────────

    #[test]
    fn s3_missing_replayed_key() {
        let p = node_id("P");
        let reducer = TypedReduceAggregator::new(
            p.clone(),
            ReducerKind::Sum {
                field: "score".to_string(),
            },
            number_schema_for("sum_score"),
        );
        let engine = ReplayEngine::new(p.clone()).with_reducer(reducer);
        let batch = three_int_projections(&p, "score");
        let original = BTreeMap::new(); // empty
        let err = engine
            .build_proof(&batch, &original, "rev-0".to_string(), "t".to_string())
            .unwrap_err();
        assert!(matches!(err, ReplayError::MissingReplayedKey { .. }));
    }

    // ── S-4: Extra original key ───────────────────────────────────────────

    #[test]
    fn s4_extra_original_key() {
        let p = node_id("P");
        let reducer = TypedReduceAggregator::new(
            p.clone(),
            ReducerKind::Sum {
                field: "score".to_string(),
            },
            number_schema_for("sum_score"),
        );
        let engine = ReplayEngine::new(p.clone()).with_reducer(reducer);
        let batch = three_int_projections(&p, "score");
        let mut original = BTreeMap::new();
        original.insert("sum_score".to_string(), serde_json::json!(60));
        original.insert("item_results".to_string(), serde_json::json!([]));
        let err = engine
            .build_proof(&batch, &original, "rev-0".to_string(), "t".to_string())
            .unwrap_err();
        assert!(matches!(err, ReplayError::ExtraOriginalKey { .. }));
    }

    // ── S-5: Join guard verdict matches ───────────────────────────────────

    #[test]
    fn s5_join_guard_verdict_matches() {
        let p = node_id("P");
        let guard = JoinGuard::new_unchecked(
            p.clone(),
            JoinPolicy::AllSatisfied {
                satisfaction_field: "ok".to_string(),
            },
            bool_schema_for("ok"),
        );
        let engine = ReplayEngine::new(p.clone()).with_join_guard(guard);
        let batch = vec![
            projection_with_bool_field(&p, "ok", true),
            projection_with_bool_field(&p, "ok", true),
            projection_with_bool_field(&p, "ok", true),
        ];
        let mut original = BTreeMap::new();
        original.insert("join_verdict".to_string(), serde_json::json!("succeeded"));
        original.insert("join_contributing_count".to_string(), serde_json::json!(3));
        original.insert(
            "join_witness".to_string(),
            serde_json::json!(["att-true", "att-true", "att-true"]),
        );
        let proof = engine
            .build_proof(&batch, &original, "rev-0".to_string(), "t".to_string())
            .unwrap();
        assert!(matches!(
            proof.replayed_join_verdict,
            Some(JoinVerdict::Succeeded {
                contributing_count: 3,
                ..
            })
        ));
        assert!(matches!(proof.determinism_check, DeterminismCheck::Match));
    }

    // ── S-6: Reducer failure surfaces ─────────────────────────────────────

    #[test]
    fn s6_reducer_failure_surfaces() {
        let p = node_id("P");
        let reducer = TypedReduceAggregator::new(
            p.clone(),
            ReducerKind::Sum {
                field: "score".to_string(),
            },
            number_schema_for("sum_score"),
        );
        let engine = ReplayEngine::new(p.clone()).with_reducer(reducer);
        // Empty batch + Sum succeeds (returns 0), but to force a real
        // error use Mean on empty:
        let reducer_mean = TypedReduceAggregator::new(
            p.clone(),
            ReducerKind::Mean {
                field: "score".to_string(),
            },
            number_schema_for("mean_score"),
        );
        let engine_mean = ReplayEngine::new(p.clone()).with_reducer(reducer_mean);
        let batch: Vec<TypedChildOutput> = vec![];
        let mut original = BTreeMap::new();
        original.insert("mean_score".to_string(), serde_json::json!(0));
        let err = engine_mean
            .build_proof(&batch, &original, "rev-0".to_string(), "t".to_string())
            .unwrap_err();
        assert!(matches!(err, ReplayError::ReducerFailed(_)));
        // keep `engine` referenced to avoid unused warning.
        let _ = engine;
    }

    // ── S-7: Guard failure surfaces ───────────────────────────────────────

    #[test]
    fn s7_guard_failure_surfaces() {
        let p = node_id("P");
        let guard = JoinGuard::new_unchecked(
            p.clone(),
            JoinPolicy::FirstSuccess {
                success_field: "ok".to_string(),
            },
            bool_schema_for("ok"),
        );
        let engine = ReplayEngine::new(p.clone()).with_join_guard(guard);
        let batch = vec![projection_with_bool_field(&node_id("OTHER"), "ok", true)];
        let original = BTreeMap::new();
        let err = engine
            .build_proof(&batch, &original, "rev-0".to_string(), "t".to_string())
            .unwrap_err();
        assert!(matches!(err, ReplayError::GuardFailed(_)));
    }

    // ── S-8: Determinism ──────────────────────────────────────────────────

    #[test]
    fn s8_determinism_byte_equal() {
        let p = node_id("P");
        let reducer = TypedReduceAggregator::new(
            p.clone(),
            ReducerKind::Sum {
                field: "score".to_string(),
            },
            number_schema_for("sum_score"),
        );
        let engine = ReplayEngine::new(p.clone()).with_reducer(reducer);
        let batch = three_int_projections(&p, "score");
        let mut original = BTreeMap::new();
        original.insert("sum_score".to_string(), serde_json::json!(60));
        let a = engine
            .build_proof(&batch, &original, "rev-0".to_string(), "t".to_string())
            .unwrap();
        let b = engine
            .build_proof(&batch, &original, "rev-0".to_string(), "t".to_string())
            .unwrap();
        assert_eq!(a.projection_digest, b.projection_digest);
        assert_eq!(
            serde_json::to_string(&a.determinism_check).unwrap(),
            serde_json::to_string(&b.determinism_check).unwrap()
        );
    }

    // ── Extra: Projection digest changes when batch changes ──────────────

    #[test]
    fn projection_digest_changes_with_batch() {
        let p = node_id("P");
        let reducer = TypedReduceAggregator::new(
            p.clone(),
            ReducerKind::Sum {
                field: "score".to_string(),
            },
            number_schema_for("sum_score"),
        );
        let engine = ReplayEngine::new(p.clone()).with_reducer(reducer);
        let batch1 = three_int_projections(&p, "score");
        let mut batch2 = batch1.clone();
        batch2.push(projection_with_int_field(&p, "score", 40));
        let mut original = BTreeMap::new();
        original.insert("sum_score".to_string(), serde_json::json!(60));
        let a = engine
            .build_proof(&batch1, &original, "rev-0".to_string(), "t".to_string())
            .unwrap();
        let b = engine
            .build_proof(&batch2, &original, "rev-0".to_string(), "t".to_string())
            .unwrap();
        assert_ne!(a.projection_digest, b.projection_digest);
    }

    // ── Extra: Failed verdict surfaces failure reason ─────────────────────

    #[test]
    fn failed_verdict_records_reason() {
        let p = node_id("P");
        let guard = JoinGuard::new_unchecked(
            p.clone(),
            JoinPolicy::MajorityVote {
                success_field: "ok".to_string(),
            },
            bool_schema_for("ok"),
        );
        let engine = ReplayEngine::new(p.clone()).with_join_guard(guard);
        let batch = vec![
            projection_with_bool_field(&p, "ok", true),
            projection_with_bool_field(&p, "ok", false),
        ];
        let mut original = BTreeMap::new();
        original.insert("join_verdict".to_string(), serde_json::json!("failed"));
        original.insert("join_contributing_count".to_string(), serde_json::json!(2));
        original.insert(
            "join_failure_reason".to_string(),
            serde_json::json!("MajorityTie { for_: 1, against: 1 }"),
        );
        let proof = engine
            .build_proof(&batch, &original, "rev-0".to_string(), "t".to_string())
            .unwrap();
        if let Some(JoinVerdict::Failed {
            reason_code,
            contributing_count,
        }) = &proof.replayed_join_verdict
        {
            assert!(matches!(reason_code, JoinFailureReason::MajorityTie { .. }));
            assert_eq!(*contributing_count, 2);
        } else {
            panic!("expected Failed verdict");
        }
    }

    // ── Extra: schema_version is fixed ────────────────────────────────────

    #[test]
    fn replay_proof_carries_schema_version() {
        let p = node_id("P");
        let engine = ReplayEngine::new(p);
        // No reducer, no guard, empty batch and empty original ⇒
        // nothing to compare, so the build succeeds.
        let proof = engine
            .build_proof(&[], &BTreeMap::new(), "rev".to_string(), "t".to_string())
            .unwrap();
        assert_eq!(proof.schema_version, ReplayProof::SCHEMA_VERSION);
    }
}
