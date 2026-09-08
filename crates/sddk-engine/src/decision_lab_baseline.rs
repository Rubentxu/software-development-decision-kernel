//! Decision-Memory branch / fork lookahead + Pareto / Beam / Best-First baseline.
//!
//! Cycle: LAB-DECISION-001 (H6, order 392, context pack `decision-lab`).
//!
//! Pure, deterministic baseline evaluator that reads
//! [`TypedChildOutput`](crate::typed_child_output::TypedChildOutput)
//! projections and emits a [`DecisionLabOutcome`] enumerating the
//! top candidates under a closed-set [`BaselineStrategy`].
//!
//! ## Design
//!
//! - **Closed-set `BaselineStrategy`:** three classical variants
//!   (Pareto, Beam, Best-First). Adding a variant requires an ADR +
//!   audit.
//! - **Pure evaluator.** `evaluate` is a pure function of
//!   `(projections, strategy, declared_schema, score_fields)`. No
//!   I/O, no clock.
//! - **Read-only over canonical HEAD.** The evaluator never mutates
//!   the projection store.
//! - **Construction-time validation.** Required score fields are
//!   validated in `DecisionLabBaseline::new`.
//! - **`#[non_exhaustive]`** on every public type.
//!
//! See:
//!
//! - `REQ-DecisionLabBaseline` (spec, accepted)
//! - `ADR-099` (architecture decision, accepted)
//! - `REQ-AdvancedGraphReplay`, `REQ-WorkflowLabMetrics`,
//!   `REQ-StrategyComparison` (dependencies)

use std::collections::BTreeMap;

use serde::{Deserialize, Serialize};

use sddk_domain::OperatorOutputSchema;
use sddk_domain::workflow_ir::NodeId;
use sddk_domain::workflow_run::AttemptId;

use crate::typed_child_output::TypedChildOutput;

// ── BaselineStrategy ────────────────────────────────────────────────────────

/// Closed-set baseline search strategies.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[non_exhaustive]
pub enum BaselineStrategy {
    /// Pareto front over the score vector. Returns up to `max_keep`
    /// non-dominated branches.
    Pareto {
        /// Maximum number of branches to keep.
        max_keep: usize,
    },
    /// Beam search: keep the top-`beam_width` branches by score
    /// (summed over the score vector) at the current depth.
    Beam {
        /// Width of the beam.
        beam_width: usize,
    },
    /// Best-First: a single branch with the highest summed score.
    BestFirst,
}

impl BaselineStrategy {
    /// Stable identifier.
    pub fn id(&self) -> String {
        match self {
            BaselineStrategy::Pareto { max_keep } => {
                format!("pareto(max_keep={max_keep})")
            }
            BaselineStrategy::Beam { beam_width } => {
                format!("beam(width={beam_width})")
            }
            BaselineStrategy::BestFirst => "best_first()".to_string(),
        }
    }
}

// ── DecisionBranch ──────────────────────────────────────────────────────────

/// A candidate branch produced by the baseline evaluator.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[non_exhaustive]
pub struct DecisionBranch {
    /// Deterministic branch id (`branch-{source_projection.0}`).
    pub branch_id: String,
    /// Parent node that the branch belongs to.
    pub parent_node_id: NodeId,
    /// Per-field score values.
    pub score: BTreeMap<String, f64>,
    /// The chosen fields (the projection's `typed_fields` minus the
    /// score fields).
    pub chosen_fields: BTreeMap<String, serde_json::Value>,
    /// The projection this branch derives from.
    pub source_projection: AttemptId,
    /// Search depth (always 1 for this baseline).
    pub depth: usize,
}

// ── DecisionLabOutcome ──────────────────────────────────────────────────────

/// Result of a baseline evaluation.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[non_exhaustive]
pub struct DecisionLabOutcome {
    /// Strategy used.
    pub strategy: BaselineStrategy,
    /// Selected branches.
    pub branches: Vec<DecisionBranch>,
    /// Number of projections considered.
    pub evaluated_count: usize,
    /// RFC-3339 timestamp supplied by the caller.
    pub generated_at: String,
    /// Schema version of the outcome.
    pub schema_version: u32,
}

impl DecisionLabOutcome {
    /// Constant schema version.
    pub const SCHEMA_VERSION: u32 = 1;
}

// ── DecisionLabError ────────────────────────────────────────────────────────

/// Errors emitted by `DecisionLabBaseline`.
#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error, Serialize, Deserialize)]
#[non_exhaustive]
pub enum DecisionLabError {
    /// No score fields were declared.
    #[error("no score fields declared")]
    NoScoreFields,
    /// A declared score field is missing from the schema.
    #[error("declared score field {field:?} is missing from the schema")]
    SchemaMissingScoreField {
        /// Field name.
        field: String,
    },
    /// A projection's score field had a non-numeric JSON kind.
    #[error("field {field:?} has kind {actual_kind}, expected number")]
    FieldKindMismatch {
        /// Field name.
        field: String,
        /// Actual JSON kind.
        actual_kind: String,
    },
    /// A projection's score value could not be parsed as f64.
    #[error("field {field:?} value {value:?} is not a finite number")]
    FieldNotNumeric {
        /// Field name.
        field: String,
        /// Original value.
        value: String,
    },
}

// ── DecisionLabBaseline ─────────────────────────────────────────────────────

/// Pure baseline evaluator.
#[derive(Debug, Clone)]
#[non_exhaustive]
pub struct DecisionLabBaseline {
    strategy: BaselineStrategy,
    declared_schema: OperatorOutputSchema,
    score_fields: Vec<String>,
}

impl DecisionLabBaseline {
    /// Construct a baseline. Validates that `score_fields` is non-empty
    /// and each field is declared in `declared_schema.static_fields`.
    pub fn new(
        strategy: BaselineStrategy,
        declared_schema: OperatorOutputSchema,
        score_fields: Vec<String>,
    ) -> Result<Self, DecisionLabError> {
        if score_fields.is_empty() {
            return Err(DecisionLabError::NoScoreFields);
        }
        for f in &score_fields {
            if !declared_schema.static_fields.contains_key(f) {
                return Err(DecisionLabError::SchemaMissingScoreField { field: f.clone() });
            }
        }
        Ok(Self {
            strategy,
            declared_schema,
            score_fields,
        })
    }

    /// Construct without validation (used by tests / wiring cycles
    /// that bypass the schema seam).
    pub fn new_unchecked(
        strategy: BaselineStrategy,
        declared_schema: OperatorOutputSchema,
        score_fields: Vec<String>,
    ) -> Self {
        Self {
            strategy,
            declared_schema,
            score_fields,
        }
    }

    /// Strategy in use.
    pub fn strategy(&self) -> &BaselineStrategy {
        &self.strategy
    }

    /// Declared score fields.
    pub fn score_fields(&self) -> &[String] {
        &self.score_fields
    }

    /// Declared schema.
    pub fn declared_schema(&self) -> &OperatorOutputSchema {
        &self.declared_schema
    }

    /// Evaluate the batch.
    pub fn evaluate(
        &self,
        projections: &[TypedChildOutput],
        generated_at: String,
    ) -> Result<DecisionLabOutcome, DecisionLabError> {
        let evaluated_count = projections.len();
        // Score every projection first so we never silently drop a
        // projection due to missing fields.
        let mut scored: Vec<(&TypedChildOutput, BTreeMap<String, f64>, f64)> = Vec::new();
        for p in projections {
            let mut per_field = BTreeMap::new();
            let mut sum = 0.0_f64;
            for f in &self.score_fields {
                let v =
                    p.typed_fields
                        .get(f)
                        .ok_or_else(|| DecisionLabError::FieldKindMismatch {
                            field: f.clone(),
                            actual_kind: "<missing>".to_string(),
                        })?;
                let n = match &v.raw {
                    serde_json::Value::Number(num) => {
                        num.as_f64()
                            .ok_or_else(|| DecisionLabError::FieldNotNumeric {
                                field: f.clone(),
                                value: num.to_string(),
                            })?
                    }
                    other => {
                        return Err(DecisionLabError::FieldKindMismatch {
                            field: f.clone(),
                            actual_kind: kind_name(other),
                        });
                    }
                };
                if !n.is_finite() {
                    return Err(DecisionLabError::FieldNotNumeric {
                        field: f.clone(),
                        value: format!("{n}"),
                    });
                }
                per_field.insert(f.clone(), n);
                sum += n;
            }
            scored.push((p, per_field, sum));
        }

        // Strategy-specific selection.
        let branches: Vec<DecisionBranch> = match &self.strategy {
            BaselineStrategy::BestFirst => scored
                .into_iter()
                .max_by(|a, b| {
                    a.2.partial_cmp(&b.2)
                        .unwrap_or(std::cmp::Ordering::Equal)
                        .then_with(|| a.0.child_attempt_id.0.cmp(&b.0.child_attempt_id.0))
                })
                .into_iter()
                .map(|(p, per_field, _)| make_branch(p, per_field, &self.score_fields))
                .collect(),
            BaselineStrategy::Beam { beam_width } => {
                let mut sorted = scored;
                sorted.sort_by(|a, b| {
                    b.2.partial_cmp(&a.2)
                        .unwrap_or(std::cmp::Ordering::Equal)
                        .then_with(|| a.0.child_attempt_id.0.cmp(&b.0.child_attempt_id.0))
                });
                sorted
                    .into_iter()
                    .take(*beam_width)
                    .map(|(p, per_field, _)| make_branch(p, per_field, &self.score_fields))
                    .collect()
            }
            BaselineStrategy::Pareto { max_keep } => {
                let mut kept: Vec<(&TypedChildOutput, BTreeMap<String, f64>, f64)> = Vec::new();
                for s in scored {
                    let dominated = kept.iter().any(|k| dominates(&k.1, &s.1));
                    if !dominated {
                        kept.retain(|k| !dominates(&s.1, &k.1));
                        kept.push(s);
                    }
                }
                // Final deterministic ordering: by lex score vector,
                // then attempt id.
                kept.sort_by(|a, b| {
                    let mut ax: Vec<(String, f64)> =
                        a.1.iter().map(|(k, v)| (k.clone(), *v)).collect();
                    let mut bx: Vec<(String, f64)> =
                        b.1.iter().map(|(k, v)| (k.clone(), *v)).collect();
                    ax.sort_by(|a, b| a.0.cmp(&b.0));
                    bx.sort_by(|a, b| a.0.cmp(&b.0));
                    let sa: Vec<f64> = ax.iter().map(|(_, v)| *v).collect();
                    let sb: Vec<f64> = bx.iter().map(|(_, v)| *v).collect();
                    for (x, y) in sa.iter().zip(sb.iter()) {
                        match x.partial_cmp(y).unwrap_or(std::cmp::Ordering::Equal) {
                            std::cmp::Ordering::Equal => continue,
                            ord => {
                                return ord.then_with(|| {
                                    a.0.child_attempt_id.0.cmp(&b.0.child_attempt_id.0)
                                });
                            }
                        }
                    }
                    sa.len()
                        .cmp(&sb.len())
                        .then_with(|| a.0.child_attempt_id.0.cmp(&b.0.child_attempt_id.0))
                });
                // Clamp deterministically: keep max_keep highest by
                // summed score, ties broken by attempt id. Best
                // branches first by summed score.
                kept.sort_by(|a, b| {
                    b.2.partial_cmp(&a.2)
                        .unwrap_or(std::cmp::Ordering::Equal)
                        .then_with(|| a.0.child_attempt_id.0.cmp(&b.0.child_attempt_id.0))
                });
                kept.truncate(*max_keep);
                kept.into_iter()
                    .map(|(p, per_field, _)| make_branch(p, per_field, &self.score_fields))
                    .collect()
            }
        };

        // Final deterministic ordering for Best-First (single); the
        // others are already sorted.
        if matches!(
            self.strategy,
            BaselineStrategy::Beam { .. } | BaselineStrategy::Pareto { .. }
        ) {
            // already sorted
        } else {
            // BestFirst: single branch, no-op for sort.
        }

        Ok(DecisionLabOutcome {
            strategy: self.strategy.clone(),
            branches,
            evaluated_count,
            generated_at,
            schema_version: DecisionLabOutcome::SCHEMA_VERSION,
        })
    }
}

// ── Helpers ─────────────────────────────────────────────────────────────────

/// Convert a scored projection into a branch.
fn make_branch(
    p: &TypedChildOutput,
    per_field: BTreeMap<String, f64>,
    score_fields: &[String],
) -> DecisionBranch {
    let mut chosen_fields: BTreeMap<String, serde_json::Value> = BTreeMap::new();
    for (k, tf) in &p.typed_fields {
        if !score_fields.contains(k) {
            chosen_fields.insert(k.clone(), tf.raw.clone());
        }
    }
    DecisionBranch {
        branch_id: format!("branch-{}", p.child_attempt_id.0),
        parent_node_id: p.parent_node_id.clone(),
        score: per_field,
        chosen_fields,
        source_projection: p.child_attempt_id.clone(),
        depth: 1,
    }
}

/// `a` dominates `b` if `a >= b` on every axis and `a > b` on at
/// least one axis.
fn dominates(a: &BTreeMap<String, f64>, b: &BTreeMap<String, f64>) -> bool {
    let mut strictly_greater = false;
    for (k, av) in a {
        let bv = match b.get(k) {
            Some(v) => *v,
            None => return false,
        };
        if av < &bv {
            return false;
        }
        if av > &bv {
            strictly_greater = true;
        }
    }
    strictly_greater
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

// ── Audit guard ─────────────────────────────────────────────────────────────

#[allow(unused)]
const BASELINE_STRATEGY_VARIANT_LIST: &[BaselineStrategy] = &[
    BaselineStrategy::Pareto { max_keep: 0 },
    BaselineStrategy::Beam { beam_width: 0 },
    BaselineStrategy::BestFirst,
];

// ── Tests ────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;
    use sddk_domain::SchemaDialect;

    fn node_id(s: &str) -> NodeId {
        NodeId(s.to_string())
    }

    fn number_schema_for(field: &str) -> OperatorOutputSchema {
        let doc = serde_json::json!({ "type": "number" });
        let op_schema =
            sddk_domain::OperatorSchema::with_defaults(SchemaDialect::JsonSchemaDraft07, doc);
        let mut sf = BTreeMap::new();
        sf.insert(field.to_string(), op_schema);
        let mut req = std::collections::BTreeSet::new();
        req.insert(field.to_string());
        OperatorOutputSchema {
            static_fields: sf,
            required_fields: req,
            accepts_extra_fields: false,
            description: None,
        }
    }

    fn projection_with_score(parent: &NodeId, score: f64, attempt: &str) -> TypedChildOutput {
        let mut tf = BTreeMap::new();
        tf.insert(
            "score".to_string(),
            crate::typed_child_output::TypedFieldValue {
                raw: serde_json::json!(score),
                declared_type: sddk_domain::OperatorSchema::with_defaults(
                    SchemaDialect::JsonSchemaDraft07,
                    serde_json::json!({ "type": "number" }),
                ),
            },
        );
        TypedChildOutput {
            child_attempt_id: AttemptId(attempt.to_string()),
            parent_node_id: parent.clone(),
            child_node_id: node_id("C"),
            declared_schema_id: "x".to_string(),
            typed_fields: tf,
            recorded_at: "t".to_string(),
            schema_version: 1,
        }
    }

    fn projection_with_two_scores(
        parent: &NodeId,
        a: f64,
        b: f64,
        attempt: &str,
    ) -> TypedChildOutput {
        let mut tf = BTreeMap::new();
        tf.insert(
            "a".to_string(),
            crate::typed_child_output::TypedFieldValue {
                raw: serde_json::json!(a),
                declared_type: sddk_domain::OperatorSchema::with_defaults(
                    SchemaDialect::JsonSchemaDraft07,
                    serde_json::json!({ "type": "number" }),
                ),
            },
        );
        tf.insert(
            "b".to_string(),
            crate::typed_child_output::TypedFieldValue {
                raw: serde_json::json!(b),
                declared_type: sddk_domain::OperatorSchema::with_defaults(
                    SchemaDialect::JsonSchemaDraft07,
                    serde_json::json!({ "type": "number" }),
                ),
            },
        );
        let mut schema = number_schema_for("a");
        schema.static_fields.insert(
            "b".to_string(),
            number_schema_for("b").static_fields["b"].clone(),
        );
        schema.required_fields.insert("b".to_string());
        let _ = schema;
        TypedChildOutput {
            child_attempt_id: AttemptId(attempt.to_string()),
            parent_node_id: parent.clone(),
            child_node_id: node_id("C"),
            declared_schema_id: "x".to_string(),
            typed_fields: tf,
            recorded_at: "t".to_string(),
            schema_version: 1,
        }
    }

    fn two_score_schema() -> OperatorOutputSchema {
        let mut schema = number_schema_for("a");
        schema.static_fields.insert(
            "b".to_string(),
            number_schema_for("b").static_fields["b"].clone(),
        );
        schema.required_fields.insert("b".to_string());
        schema
    }

    // ── S-1: Pareto keeps non-dominated branches ──────────────────────────

    #[test]
    fn s1_pareto_keeps_non_dominated() {
        let p = node_id("P");
        let schema = two_score_schema();
        let lab = DecisionLabBaseline::new_unchecked(
            BaselineStrategy::Pareto { max_keep: 10 },
            schema,
            vec!["a".to_string(), "b".to_string()],
        );
        // 4 mutually non-dominated points (each dominates on a
        // different axis).
        let batch = vec![
            projection_with_two_scores(&p, 4.0, 1.0, "p1"),
            projection_with_two_scores(&p, 3.0, 2.0, "p2"),
            projection_with_two_scores(&p, 2.0, 3.0, "p3"),
            projection_with_two_scores(&p, 1.0, 4.0, "p4"),
        ];
        let out = lab.evaluate(&batch, "t".to_string()).unwrap();
        assert_eq!(out.branches.len(), 4);
        assert_eq!(out.evaluated_count, 4);
    }

    // ── S-2: Pareto max_keep clamps ──────────────────────────────────────

    #[test]
    fn s2_pareto_max_keep_clamps() {
        let p = node_id("P");
        let schema = two_score_schema();
        let lab = DecisionLabBaseline::new_unchecked(
            BaselineStrategy::Pareto { max_keep: 3 },
            schema,
            vec!["a".to_string(), "b".to_string()],
        );
        // 6 mutually non-dominated points (single-axis variations).
        let batch = vec![
            projection_with_two_scores(&p, 6.0, 1.0, "p1"),
            projection_with_two_scores(&p, 5.0, 2.0, "p2"),
            projection_with_two_scores(&p, 4.0, 3.0, "p3"),
            projection_with_two_scores(&p, 3.0, 4.0, "p4"),
            projection_with_two_scores(&p, 2.0, 5.0, "p5"),
            projection_with_two_scores(&p, 1.0, 6.0, "p6"),
        ];
        let out = lab.evaluate(&batch, "t".to_string()).unwrap();
        assert_eq!(out.branches.len(), 3);
    }

    // ── S-3: Pareto drops dominated ─────────────────────���────────────────

    #[test]
    fn s3_pareto_drops_dominated() {
        let p = node_id("P");
        let schema = two_score_schema();
        let lab = DecisionLabBaseline::new_unchecked(
            BaselineStrategy::Pareto { max_keep: 10 },
            schema,
            vec!["a".to_string(), "b".to_string()],
        );
        // (1,10), (2,5), (1,5), (0,1)
        // (0,1) dominated by ALL the others.
        // (1,5) NOT dominated by (1,10) on a (tie on a).
        // (2,5) NOT dominated by (1,10) on a (2>1) but dominated on b.
        // Wait — (1,10) >= (2,5) on b (10>5) AND on a (1<2).
        // So (1,10) does NOT dominate (2,5). Both stay.
        // (1,5) dominated by (2,5)? (2,5) >= (1,5) on both but
        // strictly on a. Yes. So (1,5) dropped.
        // Final kept: (1,10), (2,5).
        let batch = vec![
            projection_with_two_scores(&p, 1.0, 10.0, "p1"),
            projection_with_two_scores(&p, 2.0, 5.0, "p2"),
            projection_with_two_scores(&p, 1.0, 5.0, "p3"),
            projection_with_two_scores(&p, 0.0, 1.0, "p4"),
        ];
        let out = lab.evaluate(&batch, "t".to_string()).unwrap();
        let kept_attempts: Vec<&str> = out
            .branches
            .iter()
            .map(|b| b.source_projection.0.as_str())
            .collect();
        assert!(!kept_attempts.contains(&"p4"), "p4 (0,1) must be dropped");
        assert!(!kept_attempts.contains(&"p3"), "p3 (1,5) must be dropped");
        assert_eq!(kept_attempts.len(), 2);
    }

    // ── S-4: Beam keeps top-2 ────────────────────────────────────────────

    #[test]
    fn s4_beam_keeps_top_2() {
        let p = node_id("P");
        let schema = number_schema_for("score");
        let lab = DecisionLabBaseline::new_unchecked(
            BaselineStrategy::Beam { beam_width: 2 },
            schema,
            vec!["score".to_string()],
        );
        let batch = vec![
            projection_with_score(&p, 4.0, "p1"),
            projection_with_score(&p, 3.0, "p2"),
            projection_with_score(&p, 2.0, "p3"),
            projection_with_score(&p, 1.0, "p4"),
        ];
        let out = lab.evaluate(&batch, "t".to_string()).unwrap();
        assert_eq!(out.branches.len(), 2);
        assert_eq!(out.branches[0].source_projection.0, "p1");
        assert_eq!(out.branches[1].source_projection.0, "p2");
    }

    // ── S-5: Best-First returns single branch ────────────────────────────

    #[test]
    fn s5_best_first_returns_single() {
        let p = node_id("P");
        let schema = number_schema_for("score");
        let lab = DecisionLabBaseline::new_unchecked(
            BaselineStrategy::BestFirst,
            schema,
            vec!["score".to_string()],
        );
        let batch = vec![
            projection_with_score(&p, 4.0, "p1"),
            projection_with_score(&p, 3.0, "p2"),
            projection_with_score(&p, 2.0, "p3"),
            projection_with_score(&p, 1.0, "p4"),
        ];
        let out = lab.evaluate(&batch, "t".to_string()).unwrap();
        assert_eq!(out.branches.len(), 1);
        assert_eq!(out.branches[0].source_projection.0, "p1");
    }

    // ── S-6: Schema missing field fails closed ───────────────────────────

    #[test]
    fn s6_schema_missing_field_fails_at_construction() {
        let schema = number_schema_for("other");
        let err = DecisionLabBaseline::new(
            BaselineStrategy::BestFirst,
            schema,
            vec!["score".to_string()],
        )
        .unwrap_err();
        assert!(matches!(
            err,
            DecisionLabError::SchemaMissingScoreField { .. }
        ));
    }

    #[test]
    fn construction_no_score_fields_fails() {
        let schema = number_schema_for("score");
        let err =
            DecisionLabBaseline::new(BaselineStrategy::BestFirst, schema, vec![]).unwrap_err();
        assert!(matches!(err, DecisionLabError::NoScoreFields));
    }

    // ── S-7: Empty batch with Best-First returns empty outcome ──────────

    #[test]
    fn s7_empty_batch_best_first_empty_outcome() {
        let schema = number_schema_for("score");
        let lab = DecisionLabBaseline::new_unchecked(
            BaselineStrategy::BestFirst,
            schema,
            vec!["score".to_string()],
        );
        let out = lab.evaluate(&[], "t".to_string()).unwrap();
        assert!(out.branches.is_empty());
        assert_eq!(out.evaluated_count, 0);
    }

    // ── S-8: Non-numeric score fails closed ──────────────────────────────

    #[test]
    fn s8_non_numeric_score_fails_closed() {
        let p = node_id("P");
        let schema = number_schema_for("score");
        let lab = DecisionLabBaseline::new_unchecked(
            BaselineStrategy::BestFirst,
            schema,
            vec!["score".to_string()],
        );
        // Construct a projection with a string score.
        let mut bad = projection_with_score(&p, 1.0, "p1");
        bad.typed_fields.get_mut("score").unwrap().raw = serde_json::json!("hi");
        let err = lab.evaluate(&[bad], "t".to_string()).unwrap_err();
        assert!(matches!(err, DecisionLabError::FieldKindMismatch { .. }));
    }

    // ── S-9: Determinism ─────────────────────────────────────────────────

    #[test]
    fn s9_determinism_byte_equal() {
        let p = node_id("P");
        let schema = number_schema_for("score");
        let lab = DecisionLabBaseline::new_unchecked(
            BaselineStrategy::BestFirst,
            schema,
            vec!["score".to_string()],
        );
        let batch = vec![
            projection_with_score(&p, 4.0, "p1"),
            projection_with_score(&p, 3.0, "p2"),
        ];
        let a = lab.evaluate(&batch, "t".to_string()).unwrap();
        let b = lab.evaluate(&batch, "t".to_string()).unwrap();
        assert_eq!(
            serde_json::to_string(&a).unwrap(),
            serde_json::to_string(&b).unwrap()
        );
    }

    // ── Extra: branch id stable ──────────────────────────────────────────

    #[test]
    fn branch_id_is_stable() {
        let p = node_id("P");
        let schema = number_schema_for("score");
        let lab = DecisionLabBaseline::new_unchecked(
            BaselineStrategy::BestFirst,
            schema,
            vec!["score".to_string()],
        );
        let batch = vec![projection_with_score(&p, 5.0, "abc")];
        let out = lab.evaluate(&batch, "t".to_string()).unwrap();
        assert_eq!(out.branches[0].branch_id, "branch-abc");
    }

    // ── Extra: score_fields are excluded from chosen_fields ─────────────

    #[test]
    fn chosen_fields_excludes_score_fields() {
        let p = node_id("P");
        let schema = number_schema_for("score");
        let lab = DecisionLabBaseline::new_unchecked(
            BaselineStrategy::BestFirst,
            schema,
            vec!["score".to_string()],
        );
        // Projection with extra field "msg".
        let mut proj = projection_with_score(&p, 5.0, "p1");
        proj.typed_fields.insert(
            "msg".to_string(),
            crate::typed_child_output::TypedFieldValue {
                raw: serde_json::json!("hello"),
                declared_type: sddk_domain::OperatorSchema::with_defaults(
                    SchemaDialect::JsonSchemaDraft07,
                    serde_json::json!({ "type": "string" }),
                ),
            },
        );
        let out = lab.evaluate(&[proj], "t".to_string()).unwrap();
        assert!(!out.branches[0].chosen_fields.contains_key("score"));
        assert_eq!(
            out.branches[0].chosen_fields.get("msg").unwrap(),
            &serde_json::json!("hello")
        );
    }

    // ── Extra: BaselineStrategy::id stability ────────────────────────────

    #[test]
    fn baseline_strategy_ids() {
        assert_eq!(
            BaselineStrategy::Pareto { max_keep: 5 }.id(),
            "pareto(max_keep=5)".to_string()
        );
        assert_eq!(
            BaselineStrategy::Beam { beam_width: 3 }.id(),
            "beam(width=3)".to_string()
        );
        assert_eq!(BaselineStrategy::BestFirst.id(), "best_first()".to_string());
    }
}
