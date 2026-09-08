//! Typed durable child output and lineage.
//!
//! Cycle: DW-OPERATORS-001 (H6, order 330, context pack `advanced-runtime`).
//!
//! This module closes three gaps in the existing operator runtime:
//!
//! 1. **Type binding** — child outputs emitted into
//!    `AttemptOutcome::Succeeded { outputs }` are validated against the
//!    `OperatorOutputSchema` declared on the producing operator. Any
//!    drift is surfaced as a first-class [`LineageError`] instead of
//!    propagating silently.
//! 2. **Durable projection** — every emitted child output is recorded
//!    in a [`ChildOutputStore`] keyed by `AttemptId`. The store is
//!    deterministic (`BTreeMap`-backed) and queryable by parent or by
//!    attempt.
//! 3. **Lineage** — every projection carries an explicit
//!    `parent_node_id` → `child_node_id` → `child_attempt_id` triple
//!    so downstream queries (`why`, `debt why`, `decision why`) can
//!    traverse lineage without reconstructing the graph.
//!
//! ## Design
//!
//! - Pure builder + plain-data validator. No actor shape, no I/O.
//! - `BTreeMap` everywhere → insertion order reproducible across replays.
//! - `#[non_exhaustive]` on every public enum / struct for forward
//!   compatibility (matches v1.94.0 → v1.106.0 conventions).
//! - Determinism: the builder takes `recorded_at` from the caller so
//!   replay can supply a stable timestamp; no `SystemTime::now`.
//!
//! See:
//!
//! - `REQ-TypedChildOutputLineage` (spec, accepted)
//! - `ADR-092` (architecture decision, accepted)
//! - `crates/sddk-domain/src/operator_contract.rs` for the schema types
//!   this module validates against.

use std::collections::BTreeMap;

use serde::{Deserialize, Serialize};

use sddk_domain::workflow_ir::NodeId;
use sddk_domain::workflow_run::{Attempt, AttemptOutcome, NodeRun};
use sddk_domain::{OPERATOR_CONTRACT_SCHEMA_VERSION, OperatorOutputSchema, OperatorSchema};

// ── Schema version of the projection itself ───────────────────────────────────

/// Schema version of `TypedChildOutput`.
///
/// Bump this when the projection's on-disk shape changes. The current
/// version is `1` (initial).
pub const TYPED_CHILD_OUTPUT_SCHEMA_VERSION: u32 = 1;

// ── TypedChildOutput ─────────────────────────────────────────────────────────

/// A typed, durable, lineage-bearing projection of an emitted child
/// output.
///
/// Produced by [`TypedChildOutputBuilder::build`] after validating an
/// `AttemptOutcome::Succeeded { outputs }` against the declared
/// [`OperatorOutputSchema`].
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[non_exhaustive]
pub struct TypedChildOutput {
    /// Identifier of the source `Attempt`. Unique within a run.
    pub child_attempt_id: sddk_domain::workflow_run::AttemptId,
    /// Parent node that owns the child (must appear in `child_node_id`'s
    /// `NodeRun.dependencies`).
    pub parent_node_id: NodeId,
    /// Node that emitted the output.
    pub child_node_id: NodeId,
    /// Stable identity of the declared `OperatorOutputSchema`. For the
    /// initial release this is the schema's first field's
    /// `OperatorSchema.source` content hash, recorded verbatim.
    pub declared_schema_id: String,
    /// Per-field typed payload. Keys MUST be a subset of
    /// `declared_schema.static_fields.keys()`.
    pub typed_fields: BTreeMap<String, TypedFieldValue>,
    /// RFC-3339 timestamp supplied by the caller. Used by replay to
    /// supply a deterministic time.
    pub recorded_at: String,
    /// Schema version of this projection.
    pub schema_version: u32,
}

// ── TypedFieldValue ──────────────────────────────────────────────────────────

/// A single emitted output field paired with its declared schema.
///
/// `raw` MUST conform to `declared_type` (validated by the builder).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[non_exhaustive]
pub struct TypedFieldValue {
    /// Raw JSON value emitted by the runtime.
    pub raw: serde_json::Value,
    /// The declared schema for this field, copied verbatim from
    /// `OperatorOutputSchema.static_fields[name]`.
    pub declared_type: OperatorSchema,
}

// ── ChildOutputStore ─────────────────────────────────────────────────────────

/// Storage seam for [`TypedChildOutput`] projections.
///
/// Implementations MUST be deterministic: insertion order in
/// [`list_for_parent`](Self::list_for_parent) MUST be sorted by
/// `(recorded_at, child_attempt_id)` so two replays with identical
/// inputs produce identical listings.
pub trait ChildOutputStore {
    /// Append a projection. Returns `Err` if a projection for the same
    /// `child_attempt_id` already exists — silent overwrite would mask
    /// retried emission.
    fn append(&mut self, output: &TypedChildOutput) -> Result<(), ChildOutputStoreError>;

    /// List all projections whose `parent_node_id` equals `parent`,
    /// sorted deterministically.
    fn list_for_parent(
        &self,
        parent: &NodeId,
    ) -> Result<Vec<TypedChildOutput>, ChildOutputStoreError>;

    /// Look up a projection by attempt id.
    fn get_by_attempt(
        &self,
        attempt: &sddk_domain::workflow_run::AttemptId,
    ) -> Result<Option<TypedChildOutput>, ChildOutputStoreError>;
}

/// Default in-memory store, backed by a `Vec`.
///
/// `AttemptId` does not implement `Hash` or `Ord`, so this is a linear
/// scan. The store is the canonical projection in `sddk-engine`; for
/// production scale callers should swap in an indexed implementation
/// (e.g. a SQLite-backed one) without changing the trait. Insertion
/// order in [`list_for_parent`](Self::list_for_parent) is enforced
/// deterministically by `(recorded_at, child_attempt_id)` so replays
/// are reproducible regardless of insertion order.
#[derive(Debug, Default, Clone)]
pub struct InMemoryChildOutputStore {
    entries: Vec<TypedChildOutput>,
}

impl InMemoryChildOutputStore {
    /// Construct an empty store.
    pub fn new() -> Self {
        Self::default()
    }

    /// Number of stored projections.
    pub fn len(&self) -> usize {
        self.entries.len()
    }

    /// Whether the store is empty.
    pub fn is_empty(&self) -> bool {
        self.entries.is_empty()
    }
}

impl ChildOutputStore for InMemoryChildOutputStore {
    fn append(&mut self, output: &TypedChildOutput) -> Result<(), ChildOutputStoreError> {
        if self
            .entries
            .iter()
            .any(|e| e.child_attempt_id == output.child_attempt_id)
        {
            return Err(ChildOutputStoreError::AttemptIdAlreadyStored {
                attempt_id: output.child_attempt_id.clone(),
            });
        }
        self.entries.push(output.clone());
        Ok(())
    }

    fn list_for_parent(
        &self,
        parent: &NodeId,
    ) -> Result<Vec<TypedChildOutput>, ChildOutputStoreError> {
        let mut out: Vec<TypedChildOutput> = self
            .entries
            .iter()
            .filter(|o| &o.parent_node_id == parent)
            .cloned()
            .collect();
        out.sort_by(|a, b| {
            a.recorded_at
                .cmp(&b.recorded_at)
                .then_with(|| a.child_attempt_id.0.cmp(&b.child_attempt_id.0))
        });
        Ok(out)
    }

    fn get_by_attempt(
        &self,
        attempt: &sddk_domain::workflow_run::AttemptId,
    ) -> Result<Option<TypedChildOutput>, ChildOutputStoreError> {
        Ok(self
            .entries
            .iter()
            .find(|e| &e.child_attempt_id == attempt)
            .cloned())
    }
}

/// Errors emitted by [`ChildOutputStore`] implementations.
#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error, Serialize, Deserialize)]
#[non_exhaustive]
pub enum ChildOutputStoreError {
    /// A projection for this attempt id was already stored. Retried
    /// emission must use a fresh `AttemptId`.
    #[error("typed child output already stored for attempt {attempt_id:?}")]
    AttemptIdAlreadyStored {
        attempt_id: sddk_domain::workflow_run::AttemptId,
    },
}

// ── LineageError ─────────────────────────────────────────────────────────────

/// Closed-set validation errors raised by [`TypedChildOutputBuilder`].
///
/// Every failure mode is a variant; no `String` payloads escape.
#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error, Serialize, Deserialize)]
#[non_exhaustive]
pub enum LineageError {
    /// The attempt outcome is not `Succeeded` (still in-flight, failed,
    /// timed out, cancelled or pending).
    #[error("attempt {attempt_id:?} is not in terminal Succeeded state")]
    AttemptNotTerminal {
        attempt_id: sddk_domain::workflow_run::AttemptId,
    },
    /// Caller-supplied `parent_node_id` is not present in the child
    /// node's `NodeRun.dependencies` set.
    #[error(
        "parent {declared_parent:?} is not in child {child_node_id:?} dependencies (actual parent: {dependency_parent:?})"
    )]
    ParentMismatchWithDependency {
        declared_parent: NodeId,
        child_node_id: NodeId,
        dependency_parent: Option<NodeId>,
    },
    /// A field declared as required by `OperatorOutputSchema` is missing
    /// from the emitted output.
    #[error("missing required field {field:?} declared by schema")]
    MissingRequiredField { field: String },
    /// A field's JSON kind does not match its declared schema.
    #[error("field {field:?} declared type {expected_kind} but emitted kind {actual_kind}")]
    DeclaredTypeMismatch {
        field: String,
        expected_kind: String,
        actual_kind: String,
    },
    /// Schema version declared on the projection does not match the
    /// current `OPERATOR_CONTRACT_SCHEMA_VERSION`.
    #[error("schema version mismatch: declared {declared}, current {current}")]
    SchemaVersionMismatch { declared: u32, current: u32 },
}

// ── TypedChildOutputBuilder ──────────────────────────────────────────────────

/// Pure, deterministic builder that produces a [`TypedChildOutput`]
/// from a parent node, a child node run, a child attempt and the
/// declared [`OperatorOutputSchema`].
///
/// The builder is intentionally side-effect-free: it does NOT write to
/// a [`ChildOutputStore`]. Callers append the result to a store
/// explicitly, so the projection can be inspected before persistence.
#[derive(Debug, Clone)]
#[non_exhaustive]
pub struct TypedChildOutputBuilder {
    schema_version: u32,
}

impl TypedChildOutputBuilder {
    /// Construct a builder using the current
    /// `OPERATOR_CONTRACT_SCHEMA_VERSION`.
    pub fn new() -> Self {
        Self {
            schema_version: OPERATOR_CONTRACT_SCHEMA_VERSION,
        }
    }

    /// Build a projection. See `REQ-TypedChildOutputBuilder` (S-1..S-7).
    pub fn build(
        &self,
        parent_node_id: &NodeId,
        child_node: &NodeRun,
        child_attempt: &Attempt,
        declared_schema: &OperatorOutputSchema,
        recorded_at: String,
    ) -> Result<TypedChildOutput, LineageError> {
        // S-5: attempt must be terminal & succeeded.
        let outputs = match &child_attempt.outcome {
            Some(AttemptOutcome::Succeeded { outputs }) => outputs,
            _ => {
                return Err(LineageError::AttemptNotTerminal {
                    attempt_id: child_attempt.attempt_id.clone(),
                });
            }
        };

        // S-4: parent must appear in child dependencies.
        if !child_node.dependencies.contains(parent_node_id) {
            let dependency_parent = child_node.dependencies.iter().next().cloned();
            return Err(LineageError::ParentMismatchWithDependency {
                declared_parent: parent_node_id.clone(),
                child_node_id: child_node.node_id.clone(),
                dependency_parent,
            });
        }

        // S-2: missing required fields.
        for required in &declared_schema.required_fields {
            if !outputs.contains_key(required) {
                return Err(LineageError::MissingRequiredField {
                    field: required.clone(),
                });
            }
        }

        // S-3: declared-type conformance per field. Skip extra fields if
        // schema accepts them (mirrors OperatorInputSchema semantics).
        let mut typed_fields: BTreeMap<String, TypedFieldValue> = BTreeMap::new();
        for (field, raw) in outputs {
            let declared = match declared_schema.static_fields.get(field) {
                Some(declared) => declared,
                None => {
                    if declared_schema.accepts_extra_fields {
                        // No declared schema for this field but the
                        // contract allows it. We cannot validate type
                        // conformance without a declared type, so we
                        // skip persisting the field rather than fabricate
                        // one. This is documented fail-closed behaviour
                        // (the field is dropped from the projection).
                        continue;
                    }
                    // Strict schema rejects unknown fields. Fail closed.
                    return Err(LineageError::DeclaredTypeMismatch {
                        field: field.clone(),
                        expected_kind: "<no schema>".to_string(),
                        actual_kind: json_kind_name(raw),
                    });
                }
            };

            let actual_kind = json_kind_name(raw);
            let expected_kind = schema_kind_name(declared);

            if !kind_compatible(declared, raw) {
                return Err(LineageError::DeclaredTypeMismatch {
                    field: field.clone(),
                    expected_kind,
                    actual_kind,
                });
            }

            typed_fields.insert(
                field.clone(),
                TypedFieldValue {
                    raw: raw.clone(),
                    declared_type: declared.clone(),
                },
            );
        }

        // declared_schema_id: stable identity derived from the schema
        // document. Use the first static field's content hash if any,
        // else the static-field-set BTreeMap iteration's joined source.
        let declared_schema_id = derive_schema_id(declared_schema);

        Ok(TypedChildOutput {
            child_attempt_id: child_attempt.attempt_id.clone(),
            parent_node_id: parent_node_id.clone(),
            child_node_id: child_attempt.node_id.clone(),
            declared_schema_id,
            typed_fields,
            recorded_at,
            schema_version: self.schema_version,
        })
    }
}

impl Default for TypedChildOutputBuilder {
    fn default() -> Self {
        Self::new()
    }
}

// ── Helpers ──────────────────────────────────────────────────────────────────

/// Map a `serde_json::Value` to a stable kind string.
fn json_kind_name(v: &serde_json::Value) -> String {
    match v {
        serde_json::Value::Null => "null".to_string(),
        serde_json::Value::Bool(_) => "boolean".to_string(),
        serde_json::Value::Number(_) => "number".to_string(),
        serde_json::Value::String(_) => "string".to_string(),
        serde_json::Value::Array(_) => "array".to_string(),
        serde_json::Value::Object(_) => "object".to_string(),
    }
}

/// Best-effort schema kind name from an `OperatorSchema`. Inspects the
/// JSON-Schema-flavoured `document` for a `type` field; falls back to
/// the schema's content hash if no `type` is declared.
fn schema_kind_name(s: &OperatorSchema) -> String {
    if let Some(t) = s.document.get("type") {
        if let Some(s) = t.as_str() {
            return s.to_string();
        }
        if let Some(arr) = t.as_array() {
            return arr
                .iter()
                .filter_map(|v| v.as_str())
                .collect::<Vec<_>>()
                .join("|");
        }
    }
    format!("schema#{}", s.source)
}

/// Return `true` if `value`'s JSON kind matches `schema`'s declared
/// kind (if any). Conservative: missing declared `type` is permissive
/// (we accept), present declared `type` is strict (must match).
fn kind_compatible(schema: &OperatorSchema, value: &serde_json::Value) -> bool {
    let declared = match schema.document.get("type").and_then(|v| v.as_str()) {
        Some(d) => d,
        None => return true,
    };
    let actual = json_kind_name(value);
    declared == actual
        || (declared == "integer" && actual == "number")
        || (declared == "number" && actual == "integer")
}

/// Stable identity for an `OperatorOutputSchema`.
///
/// Joins each `static_fields` entry's `name:type` pair in BTreeMap
/// iteration order, producing a deterministic string. Sufficient for
/// the projection's `declared_schema_id` field; full cryptographic
/// identity of the schema document is preserved in each
/// [`TypedFieldValue::declared_type`].
fn derive_schema_id(schema: &OperatorOutputSchema) -> String {
    let mut parts: Vec<String> = schema
        .static_fields
        .iter()
        .map(|(name, s)| format!("{}:{}", name, schema_kind_name(s)))
        .collect();
    parts.sort();
    parts.join("|")
}

// ── Tests ────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;
    use sddk_domain::SchemaDialect;
    use sddk_domain::workflow_run::{Attempt, AttemptId, NodeRun, NodeRunState};
    use std::collections::BTreeSet;

    // ── helpers ────────────────────────────────────────────────────────────

    fn node_id(s: &str) -> NodeId {
        NodeId(s.to_string())
    }

    fn attempt_id(s: &str) -> AttemptId {
        AttemptId(s.to_string())
    }

    fn schema_for_string_field(_name: &str) -> OperatorSchema {
        let doc = serde_json::json!({ "type": "string" });
        OperatorSchema::with_defaults(SchemaDialect::JsonSchemaDraft07, doc)
    }

    fn schema_for_number_field(_name: &str) -> OperatorSchema {
        let doc = serde_json::json!({ "type": "number" });
        OperatorSchema::with_defaults(SchemaDialect::JsonSchemaDraft07, doc)
    }

    fn declared_schema_string_only() -> OperatorOutputSchema {
        let mut sf = BTreeMap::new();
        sf.insert("answer".to_string(), schema_for_string_field("answer"));
        let mut req = BTreeSet::new();
        req.insert("answer".to_string());
        OperatorOutputSchema {
            static_fields: sf,
            required_fields: req,
            accepts_extra_fields: false,
            description: None,
        }
    }

    fn declared_schema_two_fields() -> OperatorOutputSchema {
        let mut sf = BTreeMap::new();
        sf.insert("answer".to_string(), schema_for_string_field("answer"));
        sf.insert("score".to_string(), schema_for_number_field("score"));
        let mut req = BTreeSet::new();
        req.insert("answer".to_string());
        req.insert("score".to_string());
        OperatorOutputSchema {
            static_fields: sf,
            required_fields: req,
            accepts_extra_fields: false,
            description: None,
        }
    }

    fn child_node_run(parent: &NodeId) -> NodeRun {
        let mut deps = BTreeSet::new();
        deps.insert(parent.clone());
        NodeRun {
            node_id: node_id("C1"),
            state: NodeRunState::Completed,
            dependencies: deps,
            attempts: Vec::new(),
            expansion_permissions: BTreeSet::new(),
            schema_version: OPERATOR_CONTRACT_SCHEMA_VERSION,
        }
    }

    fn succeeded_attempt(outputs: BTreeMap<String, serde_json::Value>) -> Attempt {
        Attempt {
            attempt_id: attempt_id("att-1"),
            node_id: node_id("C1"),
            route: sddk_domain::Route {
                provider: "p".to_string(),
                model: "m".to_string(),
                host: "h".to_string(),
            },
            started_at: "2026-09-08T00:00:00Z".to_string(),
            ended_at: Some("2026-09-08T00:00:01Z".to_string()),
            outcome: Some(AttemptOutcome::Succeeded { outputs }),
            usage: sddk_domain::Usage {
                tokens_in: 0,
                tokens_out: 0,
                cost_micros: 0,
                wall_ms: 0,
            },
            context_capsule: sddk_domain::ContextCapsuleRef::Inline {
                summary: "s".to_string(),
                sha256: "0".repeat(64),
            },
            idempotency_key: sddk_domain::IdempotencyKey {
                project_id: "p".to_string(),
                run_id: sddk_domain::RunId("r".to_string()),
                node_id: node_id("C1"),
                attempt_seq: 1,
            },
            schema_version: OPERATOR_CONTRACT_SCHEMA_VERSION,
        }
    }

    fn in_flight_attempt() -> Attempt {
        Attempt {
            attempt_id: attempt_id("att-x"),
            node_id: node_id("C1"),
            route: sddk_domain::Route {
                provider: "p".to_string(),
                model: "m".to_string(),
                host: "h".to_string(),
            },
            started_at: "2026-09-08T00:00:00Z".to_string(),
            ended_at: None,
            outcome: None,
            usage: sddk_domain::Usage {
                tokens_in: 0,
                tokens_out: 0,
                cost_micros: 0,
                wall_ms: 0,
            },
            context_capsule: sddk_domain::ContextCapsuleRef::Inline {
                summary: "s".to_string(),
                sha256: "0".repeat(64),
            },
            idempotency_key: sddk_domain::IdempotencyKey {
                project_id: "p".to_string(),
                run_id: sddk_domain::RunId("r".to_string()),
                node_id: node_id("C1"),
                attempt_seq: 1,
            },
            schema_version: OPERATOR_CONTRACT_SCHEMA_VERSION,
        }
    }

    fn child_node_with_no_parent() -> NodeRun {
        NodeRun {
            node_id: node_id("C1"),
            state: NodeRunState::Pending,
            dependencies: BTreeSet::new(),
            attempts: Vec::new(),
            expansion_permissions: BTreeSet::new(),
            schema_version: OPERATOR_CONTRACT_SCHEMA_VERSION,
        }
    }

    fn child_node_with_wrong_parent() -> NodeRun {
        let mut deps = BTreeSet::new();
        deps.insert(node_id("OTHER"));
        NodeRun {
            node_id: node_id("C1"),
            state: NodeRunState::Completed,
            dependencies: deps,
            attempts: Vec::new(),
            expansion_permissions: BTreeSet::new(),
            schema_version: OPERATOR_CONTRACT_SCHEMA_VERSION,
        }
    }

    // ── S-1: happy path ────────────────────────────────────────────────────

    #[test]
    fn s1_happy_path_records_typed_output() {
        let parent = node_id("P");
        let mut outputs = BTreeMap::new();
        outputs.insert("answer".to_string(), serde_json::json!("ok"));
        let attempt = succeeded_attempt(outputs);
        let schema = declared_schema_string_only();

        let built = TypedChildOutputBuilder::new()
            .build(
                &parent,
                &child_node_run(&parent),
                &attempt,
                &schema,
                "t1".to_string(),
            )
            .unwrap();

        assert_eq!(built.parent_node_id, node_id("P"));
        assert_eq!(built.child_node_id, node_id("C1"));
        assert_eq!(built.schema_version, OPERATOR_CONTRACT_SCHEMA_VERSION);
        assert_eq!(built.typed_fields.len(), 1);
        assert_eq!(
            built.typed_fields.get("answer").unwrap().raw,
            serde_json::json!("ok")
        );

        let mut store = InMemoryChildOutputStore::new();
        store.append(&built).unwrap();
        assert_eq!(store.len(), 1);
        let fetched = store.get_by_attempt(&attempt_id("att-1")).unwrap().unwrap();
        assert_eq!(fetched, built);
    }

    // ── S-2: missing required field ───────────────────────────────────────

    #[test]
    fn s2_missing_required_field_fails_closed() {
        let parent = node_id("P");
        let outputs = BTreeMap::new(); // empty; schema requires "answer"
        let attempt = succeeded_attempt(outputs);
        let schema = declared_schema_string_only();

        let err = TypedChildOutputBuilder::new()
            .build(
                &parent,
                &child_node_run(&parent),
                &attempt,
                &schema,
                "t1".to_string(),
            )
            .unwrap_err();
        match err {
            LineageError::MissingRequiredField { field } => assert_eq!(field, "answer"),
            other => panic!("expected MissingRequiredField, got {other:?}"),
        }
    }

    // ── S-3: declared-type mismatch ───────────────────────────────────────

    #[test]
    fn s3_declared_type_mismatch_fails_closed() {
        let parent = node_id("P");
        let mut outputs = BTreeMap::new();
        outputs.insert("answer".to_string(), serde_json::json!(42)); // number, schema says string
        let attempt = succeeded_attempt(outputs);
        let schema = declared_schema_string_only();

        let err = TypedChildOutputBuilder::new()
            .build(
                &parent,
                &child_node_run(&parent),
                &attempt,
                &schema,
                "t1".to_string(),
            )
            .unwrap_err();
        match err {
            LineageError::DeclaredTypeMismatch {
                field,
                expected_kind,
                actual_kind,
            } => {
                assert_eq!(field, "answer");
                assert_eq!(expected_kind, "string");
                assert_eq!(actual_kind, "number");
            }
            other => panic!("expected DeclaredTypeMismatch, got {other:?}"),
        }
    }

    #[test]
    fn s3_two_field_schema_rejects_one_mismatch() {
        let parent = node_id("P");
        let mut outputs = BTreeMap::new();
        outputs.insert("answer".to_string(), serde_json::json!("ok"));
        outputs.insert("score".to_string(), serde_json::json!("not-a-number"));
        let attempt = succeeded_attempt(outputs);
        let schema = declared_schema_two_fields();

        let err = TypedChildOutputBuilder::new()
            .build(
                &parent,
                &child_node_run(&parent),
                &attempt,
                &schema,
                "t1".to_string(),
            )
            .unwrap_err();
        match err {
            LineageError::DeclaredTypeMismatch {
                field,
                expected_kind,
                actual_kind,
            } => {
                assert_eq!(field, "score");
                assert_eq!(expected_kind, "number");
                assert_eq!(actual_kind, "string");
            }
            other => panic!("expected DeclaredTypeMismatch, got {other:?}"),
        }
    }

    // ── S-4: parent not in child dependencies ─────────────────────────────

    #[test]
    fn s4_parent_mismatch_with_dependency_fails_closed() {
        let parent = node_id("P");
        let mut outputs = BTreeMap::new();
        outputs.insert("answer".to_string(), serde_json::json!("ok"));
        let attempt = succeeded_attempt(outputs);
        let schema = declared_schema_string_only();

        let err = TypedChildOutputBuilder::new()
            .build(
                &parent,
                &child_node_with_wrong_parent(),
                &attempt,
                &schema,
                "t1".to_string(),
            )
            .unwrap_err();
        match err {
            LineageError::ParentMismatchWithDependency {
                declared_parent,
                dependency_parent,
                ..
            } => {
                assert_eq!(declared_parent, node_id("P"));
                assert_eq!(dependency_parent, Some(node_id("OTHER")));
            }
            other => panic!("expected ParentMismatchWithDependency, got {other:?}"),
        }
    }

    #[test]
    fn s4_parent_mismatch_with_no_dependency_fails_closed() {
        let parent = node_id("P");
        let mut outputs = BTreeMap::new();
        outputs.insert("answer".to_string(), serde_json::json!("ok"));
        let attempt = succeeded_attempt(outputs);
        let schema = declared_schema_string_only();

        let err = TypedChildOutputBuilder::new()
            .build(
                &parent,
                &child_node_with_no_parent(),
                &attempt,
                &schema,
                "t1".to_string(),
            )
            .unwrap_err();
        match err {
            LineageError::ParentMismatchWithDependency {
                declared_parent,
                dependency_parent,
                ..
            } => {
                assert_eq!(declared_parent, node_id("P"));
                assert!(dependency_parent.is_none());
            }
            other => panic!("expected ParentMismatchWithDependency, got {other:?}"),
        }
    }

    // ── S-5: non-terminal attempt ─────────────────────────────────────────

    #[test]
    fn s5_in_flight_attempt_fails_closed() {
        let parent = node_id("P");
        let attempt = in_flight_attempt();
        let schema = declared_schema_string_only();

        let err = TypedChildOutputBuilder::new()
            .build(
                &parent,
                &child_node_run(&parent),
                &attempt,
                &schema,
                "t1".to_string(),
            )
            .unwrap_err();
        assert!(matches!(err, LineageError::AttemptNotTerminal { .. }));
    }

    // ── S-6: attempt id collision ─────────────────────────────────────────

    #[test]
    fn s6_attempt_id_collision_in_store() {
        let parent = node_id("P");
        let mut outputs = BTreeMap::new();
        outputs.insert("answer".to_string(), serde_json::json!("first"));
        let attempt = succeeded_attempt(outputs);
        let schema = declared_schema_string_only();

        let built1 = TypedChildOutputBuilder::new()
            .build(
                &parent,
                &child_node_run(&parent),
                &attempt,
                &schema,
                "t1".to_string(),
            )
            .unwrap();

        let mut outputs2 = BTreeMap::new();
        outputs2.insert("answer".to_string(), serde_json::json!("second"));
        let attempt2 = Attempt {
            attempt_id: attempt_id("att-1"), // same id, different content
            started_at: "2026-09-08T00:00:00Z".to_string(),
            ended_at: Some("2026-09-08T00:00:02Z".to_string()),
            outcome: Some(AttemptOutcome::Succeeded { outputs: outputs2 }),
            ..attempt.clone()
        };
        let built2 = TypedChildOutputBuilder::new()
            .build(
                &parent,
                &child_node_run(&parent),
                &attempt2,
                &schema,
                "t2".to_string(),
            )
            .unwrap();

        let mut store = InMemoryChildOutputStore::new();
        store.append(&built1).unwrap();
        let err = store.append(&built2).unwrap_err();
        assert!(matches!(
            err,
            ChildOutputStoreError::AttemptIdAlreadyStored { attempt_id: _ }
        ));
    }

    // ── S-7: determinism ──────────────────────────────────────────────────

    #[test]
    fn s7_identical_inputs_produce_byte_equal_outputs() {
        let parent = node_id("P");
        let mut outputs = BTreeMap::new();
        outputs.insert("answer".to_string(), serde_json::json!("ok"));
        outputs.insert("score".to_string(), serde_json::json!(7));
        let attempt = succeeded_attempt(outputs);
        let schema = declared_schema_two_fields();

        let b1 = TypedChildOutputBuilder::new()
            .build(
                &parent,
                &child_node_run(&parent),
                &attempt,
                &schema,
                "t1".to_string(),
            )
            .unwrap();
        let b2 = TypedChildOutputBuilder::new()
            .build(
                &parent,
                &child_node_run(&parent),
                &attempt,
                &schema,
                "t1".to_string(),
            )
            .unwrap();

        let j1 = serde_json::to_string(&b1).unwrap();
        let j2 = serde_json::to_string(&b2).unwrap();
        assert_eq!(j1, j2);
        assert_eq!(b1.declared_schema_id, b2.declared_schema_id);
    }

    #[test]
    fn s7_list_for_parent_is_deterministic() {
        let parent = node_id("P");
        let schema = declared_schema_string_only();

        let mut a = InMemoryChildOutputStore::new();
        // Insert three projections with different recorded_at.
        for (i, ts) in ["t3", "t1", "t2"].iter().enumerate() {
            let mut outputs = BTreeMap::new();
            outputs.insert("answer".to_string(), serde_json::json!(format!("v{i}")));
            let attempt = Attempt {
                attempt_id: attempt_id(&format!("att-{i}")),
                ..succeeded_attempt(outputs)
            };
            let built = TypedChildOutputBuilder::new()
                .build(
                    &parent,
                    &child_node_run(&parent),
                    &attempt,
                    &schema,
                    ts.to_string(),
                )
                .unwrap();
            a.append(&built).unwrap();
        }

        let listed = a.list_for_parent(&node_id("P")).unwrap();
        assert_eq!(listed.len(), 3);
        assert_eq!(listed[0].recorded_at, "t1");
        assert_eq!(listed[1].recorded_at, "t2");
        assert_eq!(listed[2].recorded_at, "t3");
        // Re-querying must give the same order.
        let listed_again = a.list_for_parent(&node_id("P")).unwrap();
        assert_eq!(listed, listed_again);
    }

    // ── Extra: derives schema id stably for two schemas with same fields ──

    #[test]
    fn schema_id_is_stable_for_equal_schemas() {
        let s1 = declared_schema_two_fields();
        let s2 = declared_schema_two_fields();
        assert_eq!(
            derive_schema_id(&s1),
            derive_schema_id(&s2),
            "identical schemas must produce identical declared_schema_id"
        );
    }

    // ── Extra: empty store ─────────────────────────────────────────────────

    #[test]
    fn empty_store_list_is_empty() {
        let a = InMemoryChildOutputStore::new();
        assert!(a.is_empty());
        assert_eq!(a.len(), 0);
        assert!(a.list_for_parent(&node_id("P")).unwrap().is_empty());
        assert!(
            a.get_by_attempt(&attempt_id("att-missing"))
                .unwrap()
                .is_none()
        );
    }

    // ── Extra: schema_version field on the projection is set ───────────────

    #[test]
    fn projection_carries_schema_version() {
        let parent = node_id("P");
        let mut outputs = BTreeMap::new();
        outputs.insert("answer".to_string(), serde_json::json!("ok"));
        let attempt = succeeded_attempt(outputs);
        let schema = declared_schema_string_only();

        let built = TypedChildOutputBuilder::new()
            .build(
                &parent,
                &child_node_run(&parent),
                &attempt,
                &schema,
                "t1".to_string(),
            )
            .unwrap();
        assert_eq!(built.schema_version, OPERATOR_CONTRACT_SCHEMA_VERSION);
        assert_eq!(built.schema_version, TYPED_CHILD_OUTPUT_SCHEMA_VERSION);
    }
}
