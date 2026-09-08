//! Typed join guard over durable child output projections.
//!
//! Cycle: DW-OPERATORS-004 (H6, order 360, context pack `advanced-runtime`).
//!
//! Decides a [`JoinVerdict`] from a batch of
//! [`TypedChildOutput`](crate::typed_child_output::TypedChildOutput)
//! projections using a closed-set [`JoinPolicy`]. Replay-stable and
//! pure: identical inputs produce byte-equal verdicts.
//!
//! ## Design
//!
//! - **Closed-set `JoinPolicy`:** `FirstSuccess`, `AllSatisfied`,
//!   `Quorum`, `MajorityVote`. Adding a variant requires an ADR +
//!   audit.
//! - **Pure guard.** `decide` is a pure function of `(projections,
//!   policy)`. No I/O, no clock, no shared state.
//! - **Witness preservation.** The `Succeeded` verdict carries the
//!   `AttemptId` of every contributing projection so downstream
//!   `why` / `decision why` queries can read lineage.
//! - **Construction-time validation.** Required fields are validated
//!   in `JoinGuard::new` so misconfigurations surface early.
//! - **`#[non_exhaustive]`** on every public type.
//!
//! See:
//!
//! - `REQ-JoinGuard` (spec, accepted)
//! - `ADR-095` (architecture decision, accepted)
//! - `REQ-TypedChildOutputLineage`, `REQ-DurableMapFanOut`,
//!   `REQ-TypedReduceAggregator` (dependencies)

use serde::{Deserialize, Serialize};

use sddk_domain::OperatorOutputSchema;
use sddk_domain::workflow_ir::NodeId;
use sddk_domain::workflow_run::AttemptId;

use crate::typed_child_output::TypedChildOutput;

// ── JoinPolicy ───────────────────────────────────────────────────────────────

/// Closed-set join policies supported by [`JoinGuard::decide`].
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[non_exhaustive]
pub enum JoinPolicy {
    /// Succeed as soon as any projection emits `success_field = true`.
    /// Fails on empty batch.
    FirstSuccess {
        /// Name of the boolean field per projection.
        success_field: String,
    },
    /// Succeed only when every projection has emitted
    /// `satisfaction_field = true`. Fails on empty batch or any
    /// unsatisfied projection.
    AllSatisfied {
        /// Name of the boolean field per projection.
        satisfaction_field: String,
    },
    /// Succeed when at least `min_count` projections are present
    /// AND every present projection has
    /// `satisfaction_field = true`. Fails on insufficient count or
    /// unsatisfied projection.
    Quorum {
        /// Required minimum count.
        min_count: usize,
        /// Name of the boolean field per projection.
        satisfaction_field: String,
    },
    /// Succeed when more than half of the present projections have
    /// `success_field = true`. Ties and opposition fail.
    MajorityVote {
        /// Name of the boolean field per projection.
        success_field: String,
    },
}

impl JoinPolicy {
    /// Stable identifier for this policy variant + parameters.
    pub fn id(&self) -> String {
        match self {
            JoinPolicy::FirstSuccess { success_field } => {
                format!("first_success(field={success_field})")
            }
            JoinPolicy::AllSatisfied { satisfaction_field } => {
                format!("all_satisfied(field={satisfaction_field})")
            }
            JoinPolicy::Quorum {
                min_count,
                satisfaction_field,
            } => {
                format!("quorum(min={min_count},field={satisfaction_field})")
            }
            JoinPolicy::MajorityVote { success_field } => {
                format!("majority_vote(field={success_field})")
            }
        }
    }
}

// ── JoinVerdict ──────────────────────────────────────────────────────────────

/// Outcome of a join decision.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[non_exhaustive]
pub enum JoinVerdict {
    /// Join succeeded.
    Succeeded {
        /// Number of projections that contributed to the decision.
        contributing_count: usize,
        /// `AttemptId`s of the contributing projections, in the
        /// deterministic order of the input slice.
        witness: Vec<AttemptId>,
    },
    /// Join failed with a stable reason code.
    Failed {
        /// Closed-set reason.
        reason_code: JoinFailureReason,
        /// Number of projections that contributed (regardless of
        /// outcome).
        contributing_count: usize,
    },
}

// ── JoinFailureReason ───────────────────────────────────────────────────────

/// Closed-set reason codes for `JoinVerdict::Failed`.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[non_exhaustive]
pub enum JoinFailureReason {
    /// Batch contained zero projections.
    EmptyBatch,
    /// `Quorum` policy and fewer than `required` projections present.
    QuorumNotMet {
        /// Required minimum count.
        required: usize,
        /// Actual count observed.
        actual: usize,
    },
    /// `AllSatisfied` policy and at least one projection failed.
    AllSatisfiedBroken,
    /// `MajorityVote` and `for == against`.
    MajorityTie {
        /// Count of projections voting for.
        for_: usize,
        /// Count of projections voting against.
        against: usize,
    },
    /// `MajorityVote` and `against > for`.
    MajorityOpposed {
        /// Count of projections voting for.
        for_: usize,
        /// Count of projections voting against.
        against: usize,
    },
}

// ── JoinGuardError ───────────────────────────────────────────────────────────

/// Errors emitted by [`JoinGuard::decide`] and [`JoinGuard::new`].
#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error, Serialize, Deserialize)]
#[non_exhaustive]
pub enum JoinGuardError {
    /// A projection in the batch does not belong to the expected
    /// `parent_node_id`.
    #[error(
        "foreign projection {projection_parent:?} does not belong to parent {declared_parent:?}"
    )]
    ForeignProjection {
        /// The parent the guard was constructed for.
        declared_parent: NodeId,
        /// The parent the offending projection actually references.
        projection_parent: NodeId,
    },
    /// A field required by the policy has the wrong JSON kind.
    #[error("field {field:?} has kind {actual_kind}, expected {expected_kind}")]
    FieldKindMismatch {
        /// Name of the offending field.
        field: String,
        /// Expected JSON kind.
        expected_kind: String,
        /// Actual JSON kind.
        actual_kind: String,
    },
    /// Construction-time: the policy's required field is not in the
    /// declared schema.
    #[error("policy requires field {field:?} but declared schema does not declare it")]
    SchemaMissingField {
        /// Name of the missing field.
        field: String,
    },
}

// ── JoinGuard ────────────────────────────────────────────────────────────────

/// Pure, deterministic join guard.
#[derive(Debug, Clone)]
#[non_exhaustive]
pub struct JoinGuard {
    parent_node_id: NodeId,
    policy: JoinPolicy,
    declared_schema: OperatorOutputSchema,
}

impl JoinGuard {
    /// Construct a guard. Validates that the policy's required field
    /// appears in `declared_schema.static_fields`.
    pub fn new(
        parent_node_id: NodeId,
        policy: JoinPolicy,
        declared_schema: OperatorOutputSchema,
    ) -> Result<Self, JoinGuardError> {
        let required_field = match &policy {
            JoinPolicy::FirstSuccess { success_field }
            | JoinPolicy::MajorityVote { success_field } => success_field.clone(),
            JoinPolicy::AllSatisfied { satisfaction_field }
            | JoinPolicy::Quorum {
                satisfaction_field, ..
            } => satisfaction_field.clone(),
        };
        if !declared_schema.static_fields.contains_key(&required_field) {
            return Err(JoinGuardError::SchemaMissingField {
                field: required_field,
            });
        }
        Ok(Self {
            parent_node_id,
            policy,
            declared_schema,
        })
    }

    /// Construct without schema validation (used by tests / wiring
    /// cycles that bypass the schema seam).
    pub fn new_unchecked(
        parent_node_id: NodeId,
        policy: JoinPolicy,
        declared_schema: OperatorOutputSchema,
    ) -> Self {
        Self {
            parent_node_id,
            policy,
            declared_schema,
        }
    }

    /// Parent node id.
    pub fn parent_node_id(&self) -> &NodeId {
        &self.parent_node_id
    }

    /// Policy in use.
    pub fn policy(&self) -> &JoinPolicy {
        &self.policy
    }

    /// Declared schema.
    pub fn declared_schema(&self) -> &OperatorOutputSchema {
        &self.declared_schema
    }

    /// Stable identifier (policy id).
    pub fn policy_id(&self) -> String {
        self.policy.id()
    }

    /// Decide the join outcome from the batch.
    pub fn decide(&self, projections: &[TypedChildOutput]) -> Result<JoinVerdict, JoinGuardError> {
        // Foreign-projection guard (S-6 in adjacent cycles).
        for p in projections {
            if p.parent_node_id != self.parent_node_id {
                return Err(JoinGuardError::ForeignProjection {
                    declared_parent: self.parent_node_id.clone(),
                    projection_parent: p.parent_node_id.clone(),
                });
            }
        }

        match &self.policy {
            JoinPolicy::FirstSuccess { success_field } => {
                self.decide_first_success(projections, success_field)
            }
            JoinPolicy::AllSatisfied { satisfaction_field } => {
                self.decide_all_satisfied(projections, satisfaction_field)
            }
            JoinPolicy::Quorum {
                min_count,
                satisfaction_field,
            } => self.decide_quorum(projections, *min_count, satisfaction_field),
            JoinPolicy::MajorityVote { success_field } => {
                self.decide_majority_vote(projections, success_field)
            }
        }
    }

    // ── policy implementations ────────────────────────────────────────────

    fn decide_first_success(
        &self,
        projections: &[TypedChildOutput],
        field: &str,
    ) -> Result<JoinVerdict, JoinGuardError> {
        if projections.is_empty() {
            return Ok(JoinVerdict::Failed {
                reason_code: JoinFailureReason::EmptyBatch,
                contributing_count: 0,
            });
        }
        let mut witness: Vec<AttemptId> = Vec::new();
        for p in projections {
            if self.read_bool(p, field)? {
                witness.push(p.child_attempt_id.clone());
                return Ok(JoinVerdict::Succeeded {
                    contributing_count: 1,
                    witness,
                });
            }
        }
        // No projection succeeded.
        Ok(JoinVerdict::Failed {
            reason_code: JoinFailureReason::EmptyBatch, // reuse: no success witness
            contributing_count: projections.len(),
        })
    }

    fn decide_all_satisfied(
        &self,
        projections: &[TypedChildOutput],
        field: &str,
    ) -> Result<JoinVerdict, JoinGuardError> {
        if projections.is_empty() {
            return Ok(JoinVerdict::Failed {
                reason_code: JoinFailureReason::EmptyBatch,
                contributing_count: 0,
            });
        }
        let mut witness: Vec<AttemptId> = Vec::new();
        for p in projections {
            if !self.read_bool(p, field)? {
                return Ok(JoinVerdict::Failed {
                    reason_code: JoinFailureReason::AllSatisfiedBroken,
                    contributing_count: witness.len(),
                });
            }
            witness.push(p.child_attempt_id.clone());
        }
        Ok(JoinVerdict::Succeeded {
            contributing_count: witness.len(),
            witness,
        })
    }

    fn decide_quorum(
        &self,
        projections: &[TypedChildOutput],
        min_count: usize,
        field: &str,
    ) -> Result<JoinVerdict, JoinGuardError> {
        if projections.len() < min_count {
            return Ok(JoinVerdict::Failed {
                reason_code: JoinFailureReason::QuorumNotMet {
                    required: min_count,
                    actual: projections.len(),
                },
                contributing_count: projections.len(),
            });
        }
        let mut witness: Vec<AttemptId> = Vec::new();
        for p in projections {
            if !self.read_bool(p, field)? {
                return Ok(JoinVerdict::Failed {
                    reason_code: JoinFailureReason::AllSatisfiedBroken,
                    contributing_count: witness.len(),
                });
            }
            witness.push(p.child_attempt_id.clone());
        }
        Ok(JoinVerdict::Succeeded {
            contributing_count: witness.len(),
            witness,
        })
    }

    fn decide_majority_vote(
        &self,
        projections: &[TypedChildOutput],
        field: &str,
    ) -> Result<JoinVerdict, JoinGuardError> {
        if projections.is_empty() {
            return Ok(JoinVerdict::Failed {
                reason_code: JoinFailureReason::EmptyBatch,
                contributing_count: 0,
            });
        }
        let mut for_: usize = 0;
        let mut against: usize = 0;
        let mut for_witnesses: Vec<AttemptId> = Vec::new();
        for p in projections {
            if self.read_bool(p, field)? {
                for_ += 1;
                for_witnesses.push(p.child_attempt_id.clone());
            } else {
                against += 1;
            }
        }
        if for_ > against {
            Ok(JoinVerdict::Succeeded {
                contributing_count: for_,
                witness: for_witnesses,
            })
        } else if for_ == against {
            Ok(JoinVerdict::Failed {
                reason_code: JoinFailureReason::MajorityTie { for_, against },
                contributing_count: projections.len(),
            })
        } else {
            Ok(JoinVerdict::Failed {
                reason_code: JoinFailureReason::MajorityOpposed { for_, against },
                contributing_count: projections.len(),
            })
        }
    }

    /// Read a boolean field from a projection, returning
    /// `FieldKindMismatch` if the field is not a JSON boolean.
    fn read_bool(&self, p: &TypedChildOutput, field: &str) -> Result<bool, JoinGuardError> {
        let tf = p
            .typed_fields
            .get(field)
            .ok_or_else(|| JoinGuardError::FieldKindMismatch {
                field: field.to_string(),
                expected_kind: "boolean".to_string(),
                actual_kind: "<missing>".to_string(),
            })?;
        match &tf.raw {
            serde_json::Value::Bool(b) => Ok(*b),
            other => Err(JoinGuardError::FieldKindMismatch {
                field: field.to_string(),
                expected_kind: "boolean".to_string(),
                actual_kind: kind_name(other),
            }),
        }
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

// ── Audit guard ──────────────────────────────────────────────────────────────

#[allow(unused)]
const JOIN_POLICY_VARIANT_LIST: &[JoinPolicy] = &[
    JoinPolicy::FirstSuccess {
        success_field: String::new(),
    },
    JoinPolicy::AllSatisfied {
        satisfaction_field: String::new(),
    },
    JoinPolicy::Quorum {
        min_count: 0,
        satisfaction_field: String::new(),
    },
    JoinPolicy::MajorityVote {
        success_field: String::new(),
    },
];

#[allow(unused)]
const JOIN_FAILURE_REASON_VARIANT_LIST: &[JoinFailureReason] = &[
    JoinFailureReason::EmptyBatch,
    JoinFailureReason::QuorumNotMet {
        required: 0,
        actual: 0,
    },
    JoinFailureReason::AllSatisfiedBroken,
    JoinFailureReason::MajorityTie {
        for_: 0,
        against: 0,
    },
    JoinFailureReason::MajorityOpposed {
        for_: 0,
        against: 0,
    },
];

// ── Tests ────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;
    use sddk_domain::SchemaDialect;
    use std::collections::BTreeMap;

    fn node_id(s: &str) -> NodeId {
        NodeId(s.to_string())
    }

    fn bool_schema_for(field: &str) -> OperatorOutputSchema {
        let doc = serde_json::json!({ "type": "boolean" });
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

    fn projection_with_bool(parent: &NodeId, field: &str, b: bool) -> TypedChildOutput {
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
            child_attempt_id: AttemptId(format!("att-{b}")),
            parent_node_id: parent.clone(),
            child_node_id: node_id("C"),
            declared_schema_id: "x".to_string(),
            typed_fields: tf,
            recorded_at: "t".to_string(),
            schema_version: 1,
        }
    }

    fn projection_with_string_field(parent: &NodeId, field: &str, s: &str) -> TypedChildOutput {
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
            child_attempt_id: AttemptId(format!("att-{s}")),
            parent_node_id: parent.clone(),
            child_node_id: node_id("C"),
            declared_schema_id: "x".to_string(),
            typed_fields: tf,
            recorded_at: "t".to_string(),
            schema_version: 1,
        }
    }

    // ── S-1: AllSatisfied succeed ──────────────────────────────────────────

    #[test]
    fn s1_all_satisfied_succeed() {
        let p = node_id("P");
        let guard = JoinGuard::new_unchecked(
            p.clone(),
            JoinPolicy::AllSatisfied {
                satisfaction_field: "ok".to_string(),
            },
            bool_schema_for("ok"),
        );
        let batch = vec![
            projection_with_bool(&p, "ok", true),
            projection_with_bool(&p, "ok", true),
            projection_with_bool(&p, "ok", true),
        ];
        let v = guard.decide(&batch).unwrap();
        assert!(matches!(
            v,
            JoinVerdict::Succeeded {
                contributing_count: 3,
                ..
            }
        ));
    }

    // ── S-2: AllSatisfied with one failure ────────────────────────────────

    #[test]
    fn s2_all_satisfied_with_one_failure() {
        let p = node_id("P");
        let guard = JoinGuard::new_unchecked(
            p.clone(),
            JoinPolicy::AllSatisfied {
                satisfaction_field: "ok".to_string(),
            },
            bool_schema_for("ok"),
        );
        let batch = vec![
            projection_with_bool(&p, "ok", true),
            projection_with_bool(&p, "ok", false),
            projection_with_bool(&p, "ok", true),
        ];
        let v = guard.decide(&batch).unwrap();
        assert!(matches!(
            v,
            JoinVerdict::Failed {
                reason_code: JoinFailureReason::AllSatisfiedBroken,
                contributing_count: 1,
            }
        ));
    }

    // ── S-3: Quorum sufficient ────────────────────────────────────────────

    #[test]
    fn s3_quorum_sufficient() {
        let p = node_id("P");
        let guard = JoinGuard::new_unchecked(
            p.clone(),
            JoinPolicy::Quorum {
                min_count: 2,
                satisfaction_field: "ok".to_string(),
            },
            bool_schema_for("ok"),
        );
        let batch = vec![
            projection_with_bool(&p, "ok", true),
            projection_with_bool(&p, "ok", true),
            projection_with_bool(&p, "ok", true),
        ];
        let v = guard.decide(&batch).unwrap();
        assert!(matches!(
            v,
            JoinVerdict::Succeeded {
                contributing_count: 3,
                ..
            }
        ));
    }

    // ── S-4: Quorum insufficient ──────────────────────────────────────────

    #[test]
    fn s4_quorum_insufficient() {
        let p = node_id("P");
        let guard = JoinGuard::new_unchecked(
            p.clone(),
            JoinPolicy::Quorum {
                min_count: 3,
                satisfaction_field: "ok".to_string(),
            },
            bool_schema_for("ok"),
        );
        let batch = vec![projection_with_bool(&p, "ok", true)];
        let v = guard.decide(&batch).unwrap();
        match v {
            JoinVerdict::Failed {
                reason_code: JoinFailureReason::QuorumNotMet { required, actual },
                contributing_count,
            } => {
                assert_eq!(required, 3);
                assert_eq!(actual, 1);
                assert_eq!(contributing_count, 1);
            }
            other => panic!("expected QuorumNotMet, got {other:?}"),
        }
    }

    // ── S-5: FirstSuccess ─────────────────────────────────────────────────

    #[test]
    fn s5_first_success() {
        let p = node_id("P");
        let guard = JoinGuard::new_unchecked(
            p.clone(),
            JoinPolicy::FirstSuccess {
                success_field: "success".to_string(),
            },
            bool_schema_for("success"),
        );
        let batch = vec![
            projection_with_bool(&p, "success", false),
            projection_with_bool(&p, "success", true),
            projection_with_bool(&p, "success", true),
        ];
        let v = guard.decide(&batch).unwrap();
        if let JoinVerdict::Succeeded {
            contributing_count,
            witness,
        } = v
        {
            assert_eq!(contributing_count, 1);
            assert_eq!(witness.len(), 1);
            // The first true projection is at index 1.
            assert_eq!(witness[0].0, "att-true");
        } else {
            panic!("expected Succeeded");
        }
    }

    // ── S-6: MajorityVote succeed ─────────────────────────────────────────

    #[test]
    fn s6_majority_vote_succeed() {
        let p = node_id("P");
        let guard = JoinGuard::new_unchecked(
            p.clone(),
            JoinPolicy::MajorityVote {
                success_field: "success".to_string(),
            },
            bool_schema_for("success"),
        );
        let batch = vec![
            projection_with_bool(&p, "success", true),
            projection_with_bool(&p, "success", true),
            projection_with_bool(&p, "success", true),
            projection_with_bool(&p, "success", false),
        ];
        let v = guard.decide(&batch).unwrap();
        if let JoinVerdict::Succeeded {
            contributing_count,
            witness,
        } = v
        {
            assert_eq!(contributing_count, 3);
            assert_eq!(witness.len(), 3);
        } else {
            panic!("expected Succeeded");
        }
    }

    // ── S-7: MajorityVote tie ─────────────────────────────────────────────

    #[test]
    fn s7_majority_vote_tie() {
        let p = node_id("P");
        let guard = JoinGuard::new_unchecked(
            p.clone(),
            JoinPolicy::MajorityVote {
                success_field: "success".to_string(),
            },
            bool_schema_for("success"),
        );
        let batch = vec![
            projection_with_bool(&p, "success", true),
            projection_with_bool(&p, "success", false),
        ];
        let v = guard.decide(&batch).unwrap();
        assert!(matches!(
            v,
            JoinVerdict::Failed {
                reason_code: JoinFailureReason::MajorityTie { .. },
                ..
            }
        ));
    }

    // ── S-8: Empty batch ──────────────────────────────────────────────────

    #[test]
    fn s8_empty_batch() {
        let p = node_id("P");
        let guard = JoinGuard::new_unchecked(
            p.clone(),
            JoinPolicy::FirstSuccess {
                success_field: "success".to_string(),
            },
            bool_schema_for("success"),
        );
        let v = guard.decide(&[]).unwrap();
        assert!(matches!(
            v,
            JoinVerdict::Failed {
                reason_code: JoinFailureReason::EmptyBatch,
                contributing_count: 0,
            }
        ));
    }

    // ── S-9: Field kind mismatch ──────────────────────────────────────────

    #[test]
    fn s9_field_kind_mismatch() {
        let p = node_id("P");
        let guard = JoinGuard::new_unchecked(
            p.clone(),
            JoinPolicy::FirstSuccess {
                success_field: "success".to_string(),
            },
            bool_schema_for("success"),
        );
        let batch = vec![projection_with_string_field(&p, "success", "yes")];
        let err = guard.decide(&batch).unwrap_err();
        assert!(matches!(err, JoinGuardError::FieldKindMismatch { .. }));
    }

    // ── S-10: Determinism ─────────────────────────────────────────────────

    #[test]
    fn s10_determinism() {
        let p = node_id("P");
        let guard = JoinGuard::new_unchecked(
            p.clone(),
            JoinPolicy::AllSatisfied {
                satisfaction_field: "ok".to_string(),
            },
            bool_schema_for("ok"),
        );
        let batch = vec![
            projection_with_bool(&p, "ok", true),
            projection_with_bool(&p, "ok", true),
        ];
        let v1 = guard.decide(&batch).unwrap();
        let v2 = guard.decide(&batch).unwrap();
        assert_eq!(
            serde_json::to_string(&v1).unwrap(),
            serde_json::to_string(&v2).unwrap()
        );
    }

    // ── Construction-time schema validation ───────────────────────────────

    #[test]
    fn construction_validates_required_field() {
        let p = node_id("P");
        let schema = bool_schema_for("other_field");
        let err = JoinGuard::new(
            p,
            JoinPolicy::FirstSuccess {
                success_field: "missing_field".to_string(),
            },
            schema,
        )
        .unwrap_err();
        assert!(matches!(err, JoinGuardError::SchemaMissingField { .. }));
    }

    #[test]
    fn construction_succeeds_when_field_present() {
        let p = node_id("P");
        let schema = bool_schema_for("success");
        let guard = JoinGuard::new(
            p,
            JoinPolicy::FirstSuccess {
                success_field: "success".to_string(),
            },
            schema,
        )
        .unwrap();
        assert_eq!(
            guard.policy_id(),
            "first_success(field=success)".to_string()
        );
    }

    // ── Foreign projection guard ──────────────────────────────────────────

    #[test]
    fn foreign_projection_rejected() {
        let guard = JoinGuard::new_unchecked(
            node_id("P"),
            JoinPolicy::FirstSuccess {
                success_field: "ok".to_string(),
            },
            bool_schema_for("ok"),
        );
        let batch = vec![projection_with_bool(&node_id("OTHER"), "ok", true)];
        let err = guard.decide(&batch).unwrap_err();
        assert!(matches!(err, JoinGuardError::ForeignProjection { .. }));
    }

    // ── MajorityVote opposed ──────────────────────────────────────────────

    #[test]
    fn majority_vote_opposed() {
        let p = node_id("P");
        let guard = JoinGuard::new_unchecked(
            p.clone(),
            JoinPolicy::MajorityVote {
                success_field: "success".to_string(),
            },
            bool_schema_for("success"),
        );
        let batch = vec![
            projection_with_bool(&p, "success", false),
            projection_with_bool(&p, "success", false),
            projection_with_bool(&p, "success", true),
        ];
        let v = guard.decide(&batch).unwrap();
        assert!(matches!(
            v,
            JoinVerdict::Failed {
                reason_code: JoinFailureReason::MajorityOpposed { .. },
                ..
            }
        ));
    }

    // ── ReducerKind::id stability across kinds ────────────────────────────

    #[test]
    fn policy_ids_are_stable() {
        assert_eq!(
            JoinPolicy::FirstSuccess {
                success_field: "x".to_string()
            }
            .id(),
            "first_success(field=x)".to_string()
        );
        assert_eq!(
            JoinPolicy::Quorum {
                min_count: 3,
                satisfaction_field: "y".to_string()
            }
            .id(),
            "quorum(min=3,field=y)".to_string()
        );
    }
}
