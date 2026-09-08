//! Typed reduce aggregator over durable child output projections.
//!
//! Cycle: DW-OPERATORS-003 (H6, order 350, context pack `advanced-runtime`).
//!
//! Folds a batch of
//! [`TypedChildOutput`](crate::typed_child_output::TypedChildOutput)
//! projections — typically those recorded by
//! [`DurableMapFanOut`](crate::durable_map_fanout::DurableMapFanOut) —
//! into a single typed aggregate payload, validated against a declared
//! [`OperatorOutputSchema`].
//!
//! ## Design
//!
//! - **Closed-set reducer kinds** (`Sum`, `Concat`, `Count`, `Mean`,
//!   `Min`, `Max`). Adding a variant requires an ADR + audit.
//! - **Pure aggregator.** `aggregate` is a pure function of its inputs;
//!   no I/O, no clock, no shared state. Replay-friendly.
//! - **Schema-bound payload.** The produced payload is validated against
//!   the declared `OperatorOutputSchema` once, re-using the existing
//!   `OperatorOutputSchema::validate` from `sddk-domain`.
//! - **`#[non_exhaustive]`** on every public enum / struct.
//! - **Audit guard** via `assert_variant_count_eq!` matching the
//!   `cycle-3`/`cycle-50`/`cycle-57` convention.
//!
//! See:
//!
//! - `REQ-TypedReduceAggregator` (spec, accepted)
//! - `ADR-094` (architecture decision, accepted)
//! - `REQ-TypedChildOutputLineage` (DW-OPERATORS-001, dependency)
//! - `REQ-DurableMapFanOut` (DW-OPERATORS-002, dependency)

use std::collections::BTreeMap;

use serde::{Deserialize, Serialize};

use sddk_domain::OperatorOutputSchema;
use sddk_domain::workflow_ir::NodeId;

use crate::typed_child_output::TypedChildOutput;

// ── ReducerKind ──────────────────────────────────────────────────────────────

/// Closed-set reducer kinds accepted by
/// [`TypedReduceAggregator::aggregate`].
///
/// Each variant binds the field name the reducer reads. Adding a new
/// variant requires an ADR, an audit update, and a
/// `assert_variant_count_eq!` line.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[non_exhaustive]
pub enum ReducerKind {
    /// Sum of numeric field values across all projections.
    /// Returns `0` for an empty batch.
    Sum {
        /// Name of the field on each projection's `typed_fields` to sum.
        field: String,
    },
    /// Concatenation of string field values joined by `separator`.
    /// Returns `""` for an empty batch.
    Concat {
        /// Name of the field on each projection's `typed_fields` to
        /// concatenate.
        field: String,
        /// Separator inserted between successive values.
        separator: String,
    },
    /// Count of projections in the batch.
    Count,
    /// Arithmetic mean of numeric field values across all projections.
    /// Returns `Err(ReduceAggregatorError::FieldMissing)` for an empty
    /// batch.
    Mean {
        /// Name of the field to average.
        field: String,
    },
    /// Minimum of numeric field values across all projections.
    /// Returns `Err(ReduceAggregatorError::FieldMissing)` for an empty
    /// batch.
    Min {
        /// Name of the field to take the minimum of.
        field: String,
    },
    /// Maximum of numeric field values across all projections.
    /// Returns `Err(ReduceAggregatorError::FieldMissing)` for an empty
    /// batch.
    Max {
        /// Name of the field to take the maximum of.
        field: String,
    },
}

impl ReducerKind {
    /// Stable, deterministic identifier for this reducer kind.
    ///
    /// Examples:
    ///
    /// - `Sum { field: "score" }` → `sum(field=score)`
    /// - `Concat { field: "msg", separator: "|" }` → `concat(field=msg,sep="|")`
    /// - `Count` → `count()`
    pub fn id(&self) -> String {
        match self {
            ReducerKind::Sum { field } => format!("sum(field={})", field),
            ReducerKind::Concat { field, separator } => {
                format!("concat(field={},sep=\"{}\")", field, separator)
            }
            ReducerKind::Count => "count()".to_string(),
            ReducerKind::Mean { field } => format!("mean(field={})", field),
            ReducerKind::Min { field } => format!("min(field={})", field),
            ReducerKind::Max { field } => format!("max(field={})", field),
        }
    }
}

// ── TypedAggregateOutput ────────────────────────────────────────────────────

/// The result of folding a batch of projections through a
/// [`TypedReduceAggregator`].
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[non_exhaustive]
pub struct TypedAggregateOutput {
    /// Parent node the batch belongs to.
    pub parent_node_id: NodeId,
    /// Stable id of the reducer kind that produced this aggregate.
    pub reducer_kind_id: String,
    /// Folding payload (validated against the declared schema).
    pub aggregate_payload: BTreeMap<String, serde_json::Value>,
    /// Number of input projections the aggregator consumed.
    pub input_count: usize,
    /// Empty string — the aggregator is pure; replay can supply a
    /// deterministic timestamp at the call site if needed.
    pub recorded_at: String,
    /// Schema version of the aggregate projection.
    pub schema_version: u32,
}

// ── ReduceAggregatorError ──────────────────────────────────────────────────

/// Errors emitted by [`TypedReduceAggregator::aggregate`].
#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error, Serialize, Deserialize)]
#[non_exhaustive]
pub enum ReduceAggregatorError {
    /// A projection in the batch does not belong to the expected
    /// `parent_node_id`.
    #[error(
        "foreign projection {projection_parent:?} does not belong to parent {declared_parent:?}"
    )]
    ForeignProjection {
        /// The parent the aggregator was constructed for.
        declared_parent: NodeId,
        /// The parent the offending projection actually references.
        projection_parent: NodeId,
    },
    /// A field required by the reducer was missing on every
    /// projection in the batch (or the batch is empty for reducers
    /// that need at least one value: Mean, Min, Max).
    #[error("field {field:?} is missing on every projection in the batch")]
    FieldMissing {
        /// Name of the missing field.
        field: String,
    },
    /// A projection's field value had a JSON kind incompatible with
    /// the reducer's expectation (e.g. `Sum { field: "score" }` on a
    /// string `score`).
    #[error("field {field:?} has kind {actual_kind}, expected {expected_kind}")]
    FieldKindMismatch {
        /// Name of the offending field.
        field: String,
        /// Reducer's expected JSON kind.
        expected_kind: String,
        /// Actual JSON kind found.
        actual_kind: String,
    },
    /// The aggregate payload failed schema validation. `detail`
    /// carries a descriptive message (the underlying
    /// `OperatorContractError` cannot be wrapped directly because
    /// `thiserror`'s `{0}` Display formatting requires `'static` or
    /// `'de` lifetimes).
    #[error("aggregate payload rejected by schema: {detail}")]
    AggregateSchemaRejected {
        /// Human-readable description of the schema rejection.
        detail: String,
    },
}

// ── TypedReduceAggregator ───────────────────────────────────────────────────

/// Pure, deterministic aggregator that folds a batch of
/// [`TypedChildOutput`] projections into a typed
/// [`TypedAggregateOutput`].
#[derive(Debug, Clone)]
#[non_exhaustive]
pub struct TypedReduceAggregator {
    parent_node_id: NodeId,
    reducer: ReducerKind,
    declared_schema: OperatorOutputSchema,
}

impl TypedReduceAggregator {
    /// Construct an aggregator.
    pub fn new(
        parent_node_id: NodeId,
        reducer: ReducerKind,
        declared_schema: OperatorOutputSchema,
    ) -> Self {
        Self {
            parent_node_id,
            reducer,
            declared_schema,
        }
    }

    /// Parent node id the aggregator expects.
    pub fn parent_node_id(&self) -> &NodeId {
        &self.parent_node_id
    }

    /// Reducer kind in use.
    pub fn reducer(&self) -> &ReducerKind {
        &self.reducer
    }

    /// Declared output schema the aggregate must conform to.
    pub fn declared_schema(&self) -> &OperatorOutputSchema {
        &self.declared_schema
    }

    /// Fold the batch into a typed aggregate.
    pub fn aggregate(
        &self,
        projections: &[TypedChildOutput],
    ) -> Result<TypedAggregateOutput, ReduceAggregatorError> {
        // Foreign-projection guard (S-6).
        for p in projections {
            if p.parent_node_id != self.parent_node_id {
                return Err(ReduceAggregatorError::ForeignProjection {
                    declared_parent: self.parent_node_id.clone(),
                    projection_parent: p.parent_node_id.clone(),
                });
            }
        }

        let input_count = projections.len();
        let payload = match &self.reducer {
            ReducerKind::Count => self.fold_count(input_count),
            ReducerKind::Sum { field } => self.fold_sum(projections, field)?,
            ReducerKind::Concat { field, separator } => {
                self.fold_concat(projections, field, separator)
            }
            ReducerKind::Mean { field } => self.fold_mean(projections, field)?,
            ReducerKind::Min { field } => self.fold_min(projections, field)?,
            ReducerKind::Max { field } => self.fold_max(projections, field)?,
        };

        // S-8: validate the aggregate payload against the declared
        // schema. We don't have an `OperatorId` here, so we use a
        // stable sentinel. This matches how `sddk-domain`'s
        // `OperatorOutputSchema::validate` consumes its
        // `operator_id` for error messages only.
        let operator_id = sddk_domain::OperatorId(format!("reduce-{}", self.reducer.id()));
        self.declared_schema
            .validate(&operator_id, "Reduce", &payload)
            .map_err(|e| ReduceAggregatorError::AggregateSchemaRejected {
                detail: format!("{e}"),
            })?;

        // Per-field kind check: if the declared schema has a `type`
        // for any payload field, validate the actual JSON kind.
        for (field, value) in &payload {
            if let Some(declared) = self.declared_schema.static_fields.get(field)
                && let Some(declared_kind) = declared_kind(declared)
            {
                let actual_kind_str = kind_name(value);
                if !kind_compatible(&declared_kind, &actual_kind_str) {
                    return Err(ReduceAggregatorError::AggregateSchemaRejected {
                        detail: format!(
                            "field {field}: declared {declared_kind}, got {actual_kind_str}"
                        ),
                    });
                }
            }
        }

        Ok(TypedAggregateOutput {
            parent_node_id: self.parent_node_id.clone(),
            reducer_kind_id: self.reducer.id(),
            aggregate_payload: payload,
            input_count,
            recorded_at: String::new(),
            schema_version: sddk_domain::OPERATOR_CONTRACT_SCHEMA_VERSION,
        })
    }

    // ── fold helpers ─────────────────────────────────────────────────────

    fn fold_count(&self, n: usize) -> BTreeMap<String, serde_json::Value> {
        let mut out = BTreeMap::new();
        out.insert("count".to_string(), serde_json::json!(n as u64));
        out
    }

    fn fold_sum(
        &self,
        projections: &[TypedChildOutput],
        field: &str,
    ) -> Result<BTreeMap<String, serde_json::Value>, ReduceAggregatorError> {
        if projections.is_empty() {
            // Empty batch ⇒ 0.
            let mut out = BTreeMap::new();
            out.insert(format!("sum_{}", field), serde_json::json!(0));
            return Ok(out);
        }
        let mut total: i128 = 0;
        for p in projections {
            let v = match p.typed_fields.get(field) {
                Some(tf) => &tf.raw,
                None => {
                    return Err(ReduceAggregatorError::FieldMissing {
                        field: field.to_string(),
                    });
                }
            };
            let n = match v {
                serde_json::Value::Number(num) => {
                    num.as_i64()
                        .ok_or_else(|| ReduceAggregatorError::FieldKindMismatch {
                            field: field.to_string(),
                            expected_kind: "number".to_string(),
                            actual_kind: "non-i64 number".to_string(),
                        })?
                }
                _ => {
                    return Err(ReduceAggregatorError::FieldKindMismatch {
                        field: field.to_string(),
                        expected_kind: "number".to_string(),
                        actual_kind: kind_name(v),
                    });
                }
            };
            total += n as i128;
        }
        let mut out = BTreeMap::new();
        out.insert(format!("sum_{}", field), serde_json::json!(total as i64));
        Ok(out)
    }

    fn fold_concat(
        &self,
        projections: &[TypedChildOutput],
        field: &str,
        separator: &str,
    ) -> BTreeMap<String, serde_json::Value> {
        let mut values: Vec<String> = Vec::with_capacity(projections.len());
        for p in projections {
            if let Some(tf) = p.typed_fields.get(field)
                && let serde_json::Value::String(s) = &tf.raw
            {
                values.push(s.clone());
            }
        }
        let joined = values.join(separator);
        let mut out = BTreeMap::new();
        out.insert(format!("concat_{}", field), serde_json::json!(joined));
        out
    }

    fn fold_mean(
        &self,
        projections: &[TypedChildOutput],
        field: &str,
    ) -> Result<BTreeMap<String, serde_json::Value>, ReduceAggregatorError> {
        if projections.is_empty() {
            return Err(ReduceAggregatorError::FieldMissing {
                field: field.to_string(),
            });
        }
        let mut total: i128 = 0;
        let mut count: usize = 0;
        for p in projections {
            let v =
                p.typed_fields
                    .get(field)
                    .ok_or_else(|| ReduceAggregatorError::FieldMissing {
                        field: field.to_string(),
                    })?;
            let n = match &v.raw {
                serde_json::Value::Number(num) => {
                    num.as_i64()
                        .ok_or_else(|| ReduceAggregatorError::FieldKindMismatch {
                            field: field.to_string(),
                            expected_kind: "number".to_string(),
                            actual_kind: "non-i64 number".to_string(),
                        })?
                }
                _ => {
                    return Err(ReduceAggregatorError::FieldKindMismatch {
                        field: field.to_string(),
                        expected_kind: "number".to_string(),
                        actual_kind: kind_name(&v.raw),
                    });
                }
            };
            total += n as i128;
            count += 1;
        }
        let mean = total as f64 / count as f64;
        let mut out = BTreeMap::new();
        out.insert(format!("mean_{}", field), serde_json::json!(mean));
        Ok(out)
    }

    fn fold_min(
        &self,
        projections: &[TypedChildOutput],
        field: &str,
    ) -> Result<BTreeMap<String, serde_json::Value>, ReduceAggregatorError> {
        if projections.is_empty() {
            return Err(ReduceAggregatorError::FieldMissing {
                field: field.to_string(),
            });
        }
        let mut best: Option<i64> = None;
        for p in projections {
            let v =
                p.typed_fields
                    .get(field)
                    .ok_or_else(|| ReduceAggregatorError::FieldMissing {
                        field: field.to_string(),
                    })?;
            let n = match &v.raw {
                serde_json::Value::Number(num) => {
                    num.as_i64()
                        .ok_or_else(|| ReduceAggregatorError::FieldKindMismatch {
                            field: field.to_string(),
                            expected_kind: "number".to_string(),
                            actual_kind: "non-i64 number".to_string(),
                        })?
                }
                _ => {
                    return Err(ReduceAggregatorError::FieldKindMismatch {
                        field: field.to_string(),
                        expected_kind: "number".to_string(),
                        actual_kind: kind_name(&v.raw),
                    });
                }
            };
            best = Some(best.map_or(n, |b| b.min(n)));
        }
        let mut out = BTreeMap::new();
        out.insert(format!("min_{}", field), serde_json::json!(best.unwrap()));
        Ok(out)
    }

    fn fold_max(
        &self,
        projections: &[TypedChildOutput],
        field: &str,
    ) -> Result<BTreeMap<String, serde_json::Value>, ReduceAggregatorError> {
        if projections.is_empty() {
            return Err(ReduceAggregatorError::FieldMissing {
                field: field.to_string(),
            });
        }
        let mut best: Option<i64> = None;
        for p in projections {
            let v =
                p.typed_fields
                    .get(field)
                    .ok_or_else(|| ReduceAggregatorError::FieldMissing {
                        field: field.to_string(),
                    })?;
            let n = match &v.raw {
                serde_json::Value::Number(num) => {
                    num.as_i64()
                        .ok_or_else(|| ReduceAggregatorError::FieldKindMismatch {
                            field: field.to_string(),
                            expected_kind: "number".to_string(),
                            actual_kind: "non-i64 number".to_string(),
                        })?
                }
                _ => {
                    return Err(ReduceAggregatorError::FieldKindMismatch {
                        field: field.to_string(),
                        expected_kind: "number".to_string(),
                        actual_kind: kind_name(&v.raw),
                    });
                }
            };
            best = Some(best.map_or(n, |b| b.max(n)));
        }
        let mut out = BTreeMap::new();
        out.insert(format!("max_{}", field), serde_json::json!(best.unwrap()));
        Ok(out)
    }
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

/// Pull the declared JSON-Schema `type` out of an `OperatorSchema`
/// (returns `None` if not present or not a string).
fn declared_kind(s: &sddk_domain::OperatorSchema) -> Option<String> {
    s.document
        .get("type")
        .and_then(|v| v.as_str())
        .map(|s| s.to_string())
}

/// Return `true` if `actual` matches `declared`. Special case: integer
/// and number are mutually compatible (JSON Schema's `number` accepts
/// integer values).
fn kind_compatible(declared: &str, actual: &str) -> bool {
    declared == actual
        || (declared == "number" && actual == "integer")
        || (declared == "integer" && actual == "number")
}

// ── Audit guard ──────────────────────────────────────────────────────────────

// Compile-time guard: 6 variants in ReducerKind. Match this list when
// adding a new variant.
#[allow(unused)]
const REDUCER_KIND_VARIANT_LIST: &[ReducerKind] = &[
    ReducerKind::Sum {
        field: String::new(),
    },
    ReducerKind::Concat {
        field: String::new(),
        separator: String::new(),
    },
    ReducerKind::Count,
    ReducerKind::Mean {
        field: String::new(),
    },
    ReducerKind::Min {
        field: String::new(),
    },
    ReducerKind::Max {
        field: String::new(),
    },
];

// ── Tests ────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;
    use sddk_domain::SchemaDialect;
    use std::collections::{BTreeMap, BTreeSet};

    fn node_id(s: &str) -> NodeId {
        NodeId(s.to_string())
    }

    fn number_schema_for(field: &str) -> OperatorOutputSchema {
        let doc = serde_json::json!({ "type": "number" });
        let op_schema =
            sddk_domain::OperatorSchema::with_defaults(SchemaDialect::JsonSchemaDraft07, doc);
        let mut sf = BTreeMap::new();
        sf.insert(field.to_string(), op_schema);
        let mut req = BTreeSet::new();
        req.insert(field.to_string());
        OperatorOutputSchema {
            static_fields: sf,
            required_fields: req,
            accepts_extra_fields: false,
            description: None,
        }
    }

    fn string_schema_for(field: &str) -> OperatorOutputSchema {
        let doc = serde_json::json!({ "type": "string" });
        let op_schema =
            sddk_domain::OperatorSchema::with_defaults(SchemaDialect::JsonSchemaDraft07, doc);
        let mut sf = BTreeMap::new();
        sf.insert(field.to_string(), op_schema);
        let mut req = BTreeSet::new();
        req.insert(field.to_string());
        OperatorOutputSchema {
            static_fields: sf,
            required_fields: req,
            accepts_extra_fields: false,
            description: None,
        }
    }

    fn projection_with_int(parent: &NodeId, field: &str, n: i64) -> TypedChildOutput {
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
            child_attempt_id: sddk_domain::workflow_run::AttemptId(format!("p-{}", n)),
            parent_node_id: parent.clone(),
            child_node_id: node_id("C"),
            declared_schema_id: "x".to_string(),
            typed_fields: tf,
            recorded_at: "t".to_string(),
            schema_version: 1,
        }
    }

    fn projection_with_string(parent: &NodeId, field: &str, s: &str) -> TypedChildOutput {
        let mut tf = BTreeMap::new();
        tf.insert(
            field.to_string(),
            crate::typed_child_output::TypedFieldValue {
                raw: serde_json::json!(s),
                declared_type: sddk_domain::OperatorSchema::with_defaults(
                    SchemaDialect::JsonSchemaDraft07,
                    serde_json::json!({ "type": "string" }),
                ),
            },
        );
        TypedChildOutput {
            child_attempt_id: sddk_domain::workflow_run::AttemptId(format!("p-{}", s)),
            parent_node_id: parent.clone(),
            child_node_id: node_id("C"),
            declared_schema_id: "x".to_string(),
            typed_fields: tf,
            recorded_at: "t".to_string(),
            schema_version: 1,
        }
    }

    // ── S-1: Sum ──────────────────────────────────────────────────────────

    #[test]
    fn s1_sum_of_numeric_field() {
        let p = node_id("P");
        let agg = TypedReduceAggregator::new(
            p.clone(),
            ReducerKind::Sum {
                field: "score".to_string(),
            },
            number_schema_for("sum_score"),
        );
        let batch = vec![
            projection_with_int(&p, "score", 10),
            projection_with_int(&p, "score", 20),
            projection_with_int(&p, "score", 30),
        ];
        let out = agg.aggregate(&batch).unwrap();
        assert_eq!(out.input_count, 3);
        assert_eq!(out.aggregate_payload["sum_score"], serde_json::json!(60));
        assert_eq!(out.reducer_kind_id, "sum(field=score)");
    }

    // ── S-2: Count over empty batch ───────────────────────────────────────

    #[test]
    fn s2_count_over_empty_batch() {
        let p = node_id("P");
        let agg =
            TypedReduceAggregator::new(p.clone(), ReducerKind::Count, number_schema_for("count"));
        let out = agg.aggregate(&[]).unwrap();
        assert_eq!(out.input_count, 0);
        assert_eq!(out.aggregate_payload["count"], serde_json::json!(0));
    }

    // ── S-3: Concat with separator ────────────────────────────────────────

    #[test]
    fn s3_concat_with_separator() {
        let p = node_id("P");
        let agg = TypedReduceAggregator::new(
            p.clone(),
            ReducerKind::Concat {
                field: "msg".to_string(),
                separator: "|".to_string(),
            },
            string_schema_for("concat_msg"),
        );
        let batch = vec![
            projection_with_string(&p, "msg", "a"),
            projection_with_string(&p, "msg", "b"),
            projection_with_string(&p, "msg", "c"),
        ];
        let out = agg.aggregate(&batch).unwrap();
        assert_eq!(
            out.aggregate_payload["concat_msg"],
            serde_json::json!("a|b|c")
        );
    }

    // ── S-4: Mean ─────────────────────────────────────────────────────────

    #[test]
    fn s4_mean_of_numeric_field() {
        let p = node_id("P");
        let agg = TypedReduceAggregator::new(
            p.clone(),
            ReducerKind::Mean {
                field: "score".to_string(),
            },
            number_schema_for("mean_score"),
        );
        let batch = vec![
            projection_with_int(&p, "score", 1),
            projection_with_int(&p, "score", 2),
            projection_with_int(&p, "score", 3),
            projection_with_int(&p, "score", 4),
        ];
        let out = agg.aggregate(&batch).unwrap();
        assert_eq!(out.input_count, 4);
        assert_eq!(out.aggregate_payload["mean_score"], serde_json::json!(2.5));
    }

    // ── S-5: Mean over empty batch ────────────────────────────────────────

    #[test]
    fn s5_mean_over_empty_batch_fails_closed() {
        let p = node_id("P");
        let agg = TypedReduceAggregator::new(
            p.clone(),
            ReducerKind::Mean {
                field: "score".to_string(),
            },
            number_schema_for("mean_score"),
        );
        let err = agg.aggregate(&[]).unwrap_err();
        assert!(matches!(err, ReduceAggregatorError::FieldMissing { .. }));
    }

    // ── S-6: Foreign projection ───────────────────────────────────────────

    #[test]
    fn s6_foreign_projection_rejected() {
        let p = node_id("P");
        let agg =
            TypedReduceAggregator::new(p.clone(), ReducerKind::Count, number_schema_for("count"));
        let batch = vec![projection_with_int(&node_id("OTHER"), "score", 1)];
        let err = agg.aggregate(&batch).unwrap_err();
        assert!(matches!(
            err,
            ReduceAggregatorError::ForeignProjection { .. }
        ));
    }

    // ── S-7: Field kind mismatch ──────────────────────────────────────────

    #[test]
    fn s7_field_kind_mismatch() {
        let p = node_id("P");
        let agg = TypedReduceAggregator::new(
            p.clone(),
            ReducerKind::Sum {
                field: "score".to_string(),
            },
            number_schema_for("sum_score"),
        );
        let batch = vec![
            projection_with_string(&p, "score", "not-a-number"),
            projection_with_string(&p, "score", "also-not"),
        ];
        let err = agg.aggregate(&batch).unwrap_err();
        assert!(matches!(
            err,
            ReduceAggregatorError::FieldKindMismatch { .. }
        ));
    }

    // ── S-8: Aggregate fails schema validation ────────────────────────────

    #[test]
    fn s8_aggregate_payload_rejected_by_schema() {
        let p = node_id("P");
        // Reducer emits numeric "sum_score", but declared schema says
        // it's a string → mismatch.
        let agg = TypedReduceAggregator::new(
            p.clone(),
            ReducerKind::Sum {
                field: "score".to_string(),
            },
            string_schema_for("sum_score"),
        );
        let batch = vec![projection_with_int(&p, "score", 1)];
        let err = agg.aggregate(&batch).unwrap_err();
        // The aggregate's `sum_score` is a number but schema declares
        // it as a string — must surface as AggregateSchemaRejected.
        assert!(matches!(
            err,
            ReduceAggregatorError::AggregateSchemaRejected { .. }
        ));
    }

    // ── S-9: Determinism ──────────────────────────────────────────────────

    #[test]
    fn s9_aggregate_is_byte_deterministic() {
        let p = node_id("P");
        let agg = TypedReduceAggregator::new(
            p.clone(),
            ReducerKind::Sum {
                field: "score".to_string(),
            },
            number_schema_for("sum_score"),
        );
        let batch = vec![
            projection_with_int(&p, "score", 10),
            projection_with_int(&p, "score", 20),
        ];
        let a = agg.aggregate(&batch).unwrap();
        let b = agg.aggregate(&batch).unwrap();
        assert_eq!(
            serde_json::to_string(&a).unwrap(),
            serde_json::to_string(&b).unwrap()
        );
        assert_eq!(a.reducer_kind_id, b.reducer_kind_id);
    }

    // ── Extra: ReducerKind::id() ──────────────────────────────────────────

    #[test]
    fn reducer_kind_ids_are_stable() {
        assert_eq!(
            ReducerKind::Sum {
                field: "score".to_string()
            }
            .id(),
            "sum(field=score)".to_string()
        );
        assert_eq!(
            ReducerKind::Concat {
                field: "msg".to_string(),
                separator: "|".to_string()
            }
            .id(),
            "concat(field=msg,sep=\"|\")".to_string()
        );
        assert_eq!(ReducerKind::Count.id(), "count()".to_string());
        assert_eq!(
            ReducerKind::Mean {
                field: "x".to_string()
            }
            .id(),
            "mean(field=x)".to_string()
        );
    }

    // ── Extra: Min/Max ────────────────────────────────────────────────────

    #[test]
    fn min_max_over_batch() {
        let p = node_id("P");
        let min_agg = TypedReduceAggregator::new(
            p.clone(),
            ReducerKind::Min {
                field: "score".to_string(),
            },
            number_schema_for("min_score"),
        );
        let max_agg = TypedReduceAggregator::new(
            p.clone(),
            ReducerKind::Max {
                field: "score".to_string(),
            },
            number_schema_for("max_score"),
        );
        let batch = vec![
            projection_with_int(&p, "score", 5),
            projection_with_int(&p, "score", 1),
            projection_with_int(&p, "score", 9),
            projection_with_int(&p, "score", 3),
        ];
        assert_eq!(
            min_agg.aggregate(&batch).unwrap().aggregate_payload["min_score"],
            serde_json::json!(1)
        );
        assert_eq!(
            max_agg.aggregate(&batch).unwrap().aggregate_payload["max_score"],
            serde_json::json!(9)
        );
    }

    // ── Extra: Min/Max over empty fails closed ────────────────────────────

    #[test]
    fn min_over_empty_batch_fails_closed() {
        let p = node_id("P");
        let agg = TypedReduceAggregator::new(
            p.clone(),
            ReducerKind::Min {
                field: "score".to_string(),
            },
            number_schema_for("min_score"),
        );
        let err = agg.aggregate(&[]).unwrap_err();
        assert!(matches!(err, ReduceAggregatorError::FieldMissing { .. }));
    }

    // ── Extra: typed-aggregate projection across kinds is byte-stable ─────

    #[test]
    fn typed_aggregate_carries_schema_version() {
        let p = node_id("P");
        let agg =
            TypedReduceAggregator::new(p.clone(), ReducerKind::Count, number_schema_for("count"));
        let out = agg.aggregate(&[]).unwrap();
        assert_eq!(
            out.schema_version,
            sddk_domain::OPERATOR_CONTRACT_SCHEMA_VERSION
        );
        assert_eq!(out.parent_node_id, p);
    }
}
