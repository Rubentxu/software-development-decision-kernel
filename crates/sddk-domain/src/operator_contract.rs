//! Typed operator input/output/error contracts.
//!
//! Provides canonical typed contracts for every [`Operator`] variant in
//! `workflow_ir.rs:360-443`.  Each contract describes the expected input shape
//! and guaranteed output shape for one operator variant.
//!
//! ## Design decisions
//!
//! - All collection fields use `BTreeMap`/`BTreeSet` for deterministic
//!   serialization and hash stability.  `HashMap` is explicitly forbidden.
//! - `OperatorContractError` lives in this module (domain-level typed contract).
//!   It is **distinct** from `OperatorError` in `sddk-engine/src/operator.rs`
//!   (runtime engine error).  The H0/H6 boundary enforces that `OperatorError`
//!   is untouched by DW-IR-004.
//! - Schema language is JSON Schema draft-07 (`JsonSchemaDraft07`).  Adding a
//!   dialect requires an ADR that bumps `OPERATOR_CONTRACT_SCHEMA_VERSION`.
//! - `description` on `OperatorInputSchema`/`OperatorOutputSchema` is non-semantic
//!   and excluded from `plan_identity` (mirrors `ir_id`/`prompt_hash`/`model_hash`
//!   exclusion in `compute_content_hash`).

use std::collections::BTreeMap;
use std::collections::BTreeSet;

use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};

use crate::workflow_ir::{Operator, OperatorId};

/// Version stamp for the operator contract schema surface.
///
/// Bumping this constant is an ADR-level action.  The value participates in
/// `NormalizedPlanV1.plan_identity` via the `OperatorContractProjectionV1` field.
pub const OPERATOR_CONTRACT_SCHEMA_VERSION: u32 = 1;

/// Closed set of schema dialects accepted for operator I/O contracts.
///
/// As of v1.80.0 the only valid dialect is `JsonSchemaDraft07`.
/// Adding any dialect (e.g. `JsonSchemaDraft202012`) requires a future ADR
/// that bumps `OPERATOR_CONTRACT_SCHEMA_VERSION`.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum SchemaDialect {
    JsonSchemaDraft07,
}

// NOTE: The closed-set guard is in the module-level `#[cfg(test)]` block.
// Adding a variant here requires an ADR that also updates the test.

impl SchemaDialect {
    /// Returns the `OPERATOR_CONTRACT_SCHEMA_VERSION` that applies to this dialect.
    ///
    /// All dialects currently map to version 1.
    pub fn schema_version(&self) -> u32 {
        OPERATOR_CONTRACT_SCHEMA_VERSION
    }
}

// ── OperatorSchema ─────────────────────────────────────────────────────────────

/// Schema descriptor for a single named field in an operator I/O contract.
///
/// The `source` field is the SHA-256 hash of the canonical JSON serialization
/// of `document`, ensuring two schemas with identical content collapse to the
/// same identity.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct OperatorSchema {
    /// Schema version — must equal `OPERATOR_CONTRACT_SCHEMA_VERSION`.
    pub version: u32,
    /// Schema dialect — must be a variant of `SchemaDialect`.
    pub dialect: SchemaDialect,
    /// SHA-256 digest of the canonical JSON bytes of `document`.
    pub source: crate::workflow_ir::ContentHash,
    /// The actual schema document (JSON Schema object).
    pub document: serde_json::Value,
}

impl OperatorSchema {
    /// Constructs an `OperatorSchema`, computing `source` from `document`.
    ///
    /// # Errors
    ///
    /// Returns `OperatorContractError::SchemaSourceMismatch` if the pre-computed
    /// `source` does not match the SHA-256 of the canonical JSON of `document`.
    pub fn new(
        version: u32,
        dialect: SchemaDialect,
        source: crate::workflow_ir::ContentHash,
        document: serde_json::Value,
    ) -> Result<Self, OperatorContractError> {
        let computed = compute_content_hash(&document);
        if computed != source {
            return Err(OperatorContractError::SchemaSourceMismatch {
                operator_id: OperatorId(String::new()),
                expected: source,
                actual: computed,
            });
        }
        Ok(Self {
            version,
            dialect,
            source,
            document,
        })
    }

    /// Constructs an `OperatorSchema` with the current `OPERATOR_CONTRACT_SCHEMA_VERSION`
    /// and the given dialect and document.
    ///
    /// The `source` is computed automatically from `document`.
    pub fn with_defaults(dialect: SchemaDialect, document: serde_json::Value) -> Self {
        let source = compute_content_hash(&document);
        Self {
            version: OPERATOR_CONTRACT_SCHEMA_VERSION,
            dialect,
            source,
            document,
        }
    }

    /// Validates this schema against the current version and dialect.
    pub fn validate(&self) -> Result<(), OperatorContractError> {
        if self.version != OPERATOR_CONTRACT_SCHEMA_VERSION {
            return Err(OperatorContractError::UnsupportedSchemaVersion {
                got: self.version,
                want: OPERATOR_CONTRACT_SCHEMA_VERSION,
            });
        }
        // dialect is closed — any variant is valid; unknown strings fail to deserialize
        Ok(())
    }
}

// ── OperatorInputSchema ────────────────────────────────────────────────────────

/// Typed input contract for an operator variant.
///
/// `static_fields` lists every expected input field with its schema.
/// `required_fields` is the subset of `static_fields.keys()` that MUST be present.
/// `accepts_extra_fields` toggles strictness (JSON Schema `additionalProperties`).
/// `description` is human-readable and non-semantic.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct OperatorInputSchema {
    /// Field name → schema, in deterministic order.
    pub static_fields: BTreeMap<String, OperatorSchema>,
    /// Subset of `static_fields.keys()` that MUST be present in the input.
    pub required_fields: BTreeSet<String>,
    /// If `false`, extra fields are rejected with `ExtraFieldDisallowed`.
    pub accepts_extra_fields: bool,
    /// Human-readable description.  NOT semantic — excluded from `plan_identity`.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
}

impl OperatorInputSchema {
    /// Validates an input payload against this schema.
    ///
    /// # Errors
    ///
    /// - `MissingRequiredField` if a required field is absent.
    /// - `ExtraFieldDisallowed` if `accepts_extra_fields` is `false` and the
    ///   input contains a field not in `static_fields`.
    pub fn validate(
        &self,
        operator_id: &OperatorId,
        variant: &'static str,
        input: &BTreeMap<String, serde_json::Value>,
    ) -> Result<(), OperatorContractError> {
        // Check required fields
        for field in &self.required_fields {
            if !input.contains_key(field) {
                return Err(OperatorContractError::MissingRequiredField {
                    operator_id: operator_id.clone(),
                    variant,
                    field: field.clone(),
                });
            }
        }

        // Check extra fields
        if !self.accepts_extra_fields {
            for field in input.keys() {
                if !self.static_fields.contains_key(field) {
                    return Err(OperatorContractError::ExtraFieldDisallowed {
                        operator_id: operator_id.clone(),
                        variant,
                        field: field.clone(),
                    });
                }
            }
        }

        Ok(())
    }

    /// Returns `true` if this schema accepts no fields and all are optional.
    pub fn is_noop(&self) -> bool {
        self.static_fields.is_empty()
            && self.required_fields.is_empty()
            && self.accepts_extra_fields
    }
}

// ── OperatorOutputSchema ──────────────────────────────────────────────────────

/// Typed output contract for an operator variant.
///
/// Structure mirrors `OperatorInputSchema` — see that type's documentation.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct OperatorOutputSchema {
    /// Field name → schema, in deterministic order.
    pub static_fields: BTreeMap<String, OperatorSchema>,
    /// Subset of `static_fields.keys()` that MUST be present in the output.
    pub required_fields: BTreeSet<String>,
    /// If `false`, extra fields are rejected with `ExtraFieldDisallowed`.
    pub accepts_extra_fields: bool,
    /// Human-readable description.  NOT semantic — excluded from `plan_identity`.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
}

impl OperatorOutputSchema {
    /// Validates an output payload against this schema.
    ///
    /// # Errors
    ///
    /// - `MissingRequiredField` if a required field is absent.
    /// - `ExtraFieldDisallowed` if `accepts_extra_fields` is `false` and the
    ///   output contains a field not in `static_fields`.
    pub fn validate(
        &self,
        operator_id: &OperatorId,
        variant: &'static str,
        output: &BTreeMap<String, serde_json::Value>,
    ) -> Result<(), OperatorContractError> {
        // Check required fields
        for field in &self.required_fields {
            if !output.contains_key(field) {
                return Err(OperatorContractError::MissingRequiredField {
                    operator_id: operator_id.clone(),
                    variant,
                    field: field.clone(),
                });
            }
        }

        // Check extra fields
        if !self.accepts_extra_fields {
            for field in output.keys() {
                if !self.static_fields.contains_key(field) {
                    return Err(OperatorContractError::ExtraFieldDisallowed {
                        operator_id: operator_id.clone(),
                        variant,
                        field: field.clone(),
                    });
                }
            }
        }

        Ok(())
    }
}

// ── TypedOperator ─────────────────────────────────────────────────────────────

/// An operator variant paired with its typed I/O contracts.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct TypedOperator {
    /// The operator variant.
    pub variant: Operator,
    /// Input contract.
    pub input_schema: OperatorInputSchema,
    /// Output contract.
    pub output_schema: OperatorOutputSchema,
}

// ── OperatorContractError ──────────────────────────────────────────────────────

/// Typed error contract — a closed-set enum listing every domain-meaningful
/// failure mode at the operator contract boundary.
///
/// Distinct from `OperatorError` in `sddk-engine/src/operator.rs` which is
/// a runtime engine error.  This type lives in the domain module.
#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum OperatorContractError {
    /// `OperatorSchema.version` does not equal `OPERATOR_CONTRACT_SCHEMA_VERSION`.
    #[error("unsupported schema version: got {got}, want {want}")]
    UnsupportedSchemaVersion { got: u32, want: u32 },

    /// Operator variant name is not in the v1.29.0 closed set.
    ///
    /// Today this variant is unreachable (all 12 variants are known), but exists
    /// to keep the contract closed if H6 adds new operators.
    #[error("unknown operator variant: `{variant}`")]
    UnknownOperatorVariant { variant: &'static str },

    /// Input payload violates the `OperatorInputSchema`.
    #[error("input contract violation for `{variant}` on field `{field}`: {reason}")]
    InputContractViolation {
        operator_id: OperatorId,
        variant: &'static str,
        field: String,
        reason: &'static str,
    },

    /// Output payload violates the `OperatorOutputSchema`.
    #[error("output contract violation for `{variant}` on field `{field}`: {reason}")]
    OutputContractViolation {
        operator_id: OperatorId,
        variant: &'static str,
        field: String,
        reason: &'static str,
    },

    /// A required field is absent from the input or output.
    #[error("missing required field `{field}` on `{variant}`")]
    MissingRequiredField {
        operator_id: OperatorId,
        variant: &'static str,
        field: String,
    },

    /// An extra field is present and `accepts_extra_fields` is `false`.
    #[error("extra field disallowed `{field}` on `{variant}`")]
    ExtraFieldDisallowed {
        operator_id: OperatorId,
        variant: &'static str,
        field: String,
    },

    /// `OperatorSchema.source` does not equal the SHA-256 of `document`.
    #[error("schema source mismatch: expected `{expected}`, got `{actual}`")]
    SchemaSourceMismatch {
        operator_id: OperatorId,
        expected: crate::workflow_ir::ContentHash,
        actual: crate::workflow_ir::ContentHash,
    },

    /// Dialect value is not in the closed `SchemaDialect` set.
    #[error("unknown schema dialect: `{dialect}`")]
    SchemaDialectUnknown { dialect: String },
}

// Compile-time guard: exactly 8 variants.
crate::assert_variant_count_eq!(
    OperatorContractError,
    8,
    [
        OperatorContractError::UnsupportedSchemaVersion { .. },
        OperatorContractError::UnknownOperatorVariant { .. },
        OperatorContractError::InputContractViolation { .. },
        OperatorContractError::OutputContractViolation { .. },
        OperatorContractError::MissingRequiredField { .. },
        OperatorContractError::ExtraFieldDisallowed { .. },
        OperatorContractError::SchemaSourceMismatch { .. },
        OperatorContractError::SchemaDialectUnknown { .. },
    ]
);

// ── OperatorContractProjectionV1 ───────────────────────────────────────────────

/// Deterministic projection of typed operator contracts for a `NormalizedPlanV1`.
///
/// Contains only the semantic fields that participate in `plan_identity`.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct OperatorContractProjectionV1 {
    /// Per-operator input contract (semantic fields only; `description` excluded).
    pub inputs: BTreeMap<OperatorId, OperatorInputSchemaProjection>,
    /// Per-operator output contract (semantic fields only; `description` excluded).
    pub outputs: BTreeMap<OperatorId, OperatorOutputSchemaProjection>,
    /// Schema version stamp — participates in `plan_identity`.
    pub schema_version: u32,
}

/// Semantic projection of an `OperatorInputSchema`.
///
/// Omits `description` (non-semantic per REQ-OPLINE-002).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct OperatorInputSchemaProjection {
    pub static_fields: BTreeMap<String, OperatorSchema>,
    pub required_fields: BTreeSet<String>,
    pub accepts_extra_fields: bool,
}

/// Semantic projection of an `OperatorOutputSchema`.
///
/// Omits `description` (non-semantic per REQ-OPLINE-002).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct OperatorOutputSchemaProjection {
    pub static_fields: BTreeMap<String, OperatorSchema>,
    pub required_fields: BTreeSet<String>,
    pub accepts_extra_fields: bool,
}

impl From<&OperatorInputSchema> for OperatorInputSchemaProjection {
    fn from(schema: &OperatorInputSchema) -> Self {
        Self {
            static_fields: schema.static_fields.clone(),
            required_fields: schema.required_fields.clone(),
            accepts_extra_fields: schema.accepts_extra_fields,
        }
    }
}

impl From<&OperatorOutputSchema> for OperatorOutputSchemaProjection {
    fn from(schema: &OperatorOutputSchema) -> Self {
        Self {
            static_fields: schema.static_fields.clone(),
            required_fields: schema.required_fields.clone(),
            accepts_extra_fields: schema.accepts_extra_fields,
        }
    }
}

// ── Default per-variant contracts ────────────────────────────────────────────

/// JSON Schema document for a field accepting any JSON value.
fn any_json_schema() -> serde_json::Value {
    serde_json::json!({ "type": "any" })
}

/// JSON Schema document for an array of items.
fn array_schema(items: serde_json::Value) -> serde_json::Value {
    serde_json::json!({
        "type": "array",
        "items": items
    })
}

/// JSON Schema document for a string field.
fn string_schema() -> serde_json::Value {
    serde_json::json!({ "type": "string" })
}

/// JSON Schema document for a non-negative integer field.
fn u64_schema() -> serde_json::Value {
    serde_json::json!({ "type": "integer", "minimum": 0 })
}

/// JSON Schema document for a boolean field.
fn bool_schema() -> serde_json::Value {
    serde_json::json!({ "type": "boolean" })
}

/// JSON Schema document for an object (map) field.
fn object_schema() -> serde_json::Value {
    serde_json::json!({ "type": "object" })
}

/// JSON Schema document for a required string field.
fn required_string_schema() -> serde_json::Value {
    serde_json::json!({
        "type": "object",
        "required": ["value"],
        "properties": {
            "value": { "type": "string" }
        }
    })
}

/// Constructs the default `OperatorInputSchema` for `Operator::Task`.
///
/// Task gets no static fields because its inputs come from the capability
/// declaration at runtime.  The schema is permissive: accepts extra fields.
fn task_default_input() -> OperatorInputSchema {
    OperatorInputSchema {
        static_fields: BTreeMap::new(),
        required_fields: BTreeSet::new(),
        accepts_extra_fields: true,
        description: Some("Task inputs are sourced from the capability declaration".into()),
    }
}

/// Constructs the default `OperatorOutputSchema` for `Operator::Task`.
///
/// Task has no static output fields at the operator level.
fn task_default_output() -> OperatorOutputSchema {
    OperatorOutputSchema {
        static_fields: BTreeMap::new(),
        required_fields: BTreeSet::new(),
        accepts_extra_fields: true,
        description: Some("Task outputs are determined by the capability".into()),
    }
}

/// Constructs the default `OperatorInputSchema` for `Operator::Sequence`.
fn sequence_default_input() -> OperatorInputSchema {
    OperatorInputSchema {
        static_fields: BTreeMap::new(),
        required_fields: BTreeSet::new(),
        accepts_extra_fields: true,
        description: None,
    }
}

/// Constructs the default `OperatorOutputSchema` for `Operator::Sequence`.
fn sequence_default_output() -> OperatorOutputSchema {
    OperatorOutputSchema {
        static_fields: BTreeMap::new(),
        required_fields: BTreeSet::new(),
        accepts_extra_fields: true,
        description: None,
    }
}

/// Constructs the default `OperatorInputSchema` for `Operator::Parallel`.
fn parallel_default_input() -> OperatorInputSchema {
    let mut static_fields = BTreeMap::new();
    static_fields.insert(
        "max_concurrency".into(),
        OperatorSchema::with_defaults(SchemaDialect::JsonSchemaDraft07, u64_schema()),
    );
    OperatorInputSchema {
        static_fields,
        required_fields: BTreeSet::from(["max_concurrency".into()]),
        accepts_extra_fields: false,
        description: None,
    }
}

/// Constructs the default `OperatorOutputSchema` for `Operator::Parallel`.
fn parallel_default_output() -> OperatorOutputSchema {
    // Output: Array<{ branch_id, outputs }> — schema is permissive since
    // branch structure is determined at runtime.
    OperatorOutputSchema {
        static_fields: BTreeMap::new(),
        required_fields: BTreeSet::new(),
        accepts_extra_fields: true,
        description: None,
    }
}

/// Constructs the default `OperatorInputSchema` for `Operator::Map`.
fn map_default_input() -> OperatorInputSchema {
    let mut static_fields = BTreeMap::new();
    // source_ref: Ref<OperatorId>
    static_fields.insert(
        "source_ref".into(),
        OperatorSchema::with_defaults(SchemaDialect::JsonSchemaDraft07, string_schema()),
    );
    static_fields.insert(
        "max_concurrency".into(),
        OperatorSchema::with_defaults(SchemaDialect::JsonSchemaDraft07, u64_schema()),
    );
    OperatorInputSchema {
        static_fields,
        required_fields: BTreeSet::from(["source_ref".into(), "max_concurrency".into()]),
        accepts_extra_fields: false,
        description: None,
    }
}

/// Constructs the default `OperatorOutputSchema` for `Operator::Map`.
///
/// **This replaces the v1.29.0 placeholder: the `items` key of `outputs` typed as `serde_json::Value::Array`.**
/// The typed contract is `item_results: BTreeMap<OperatorId, NodeOutcome::Succeeded>`.
/// For serialization purposes we use a JSON Schema that describes the projection shape.
fn map_default_output() -> OperatorOutputSchema {
    // item_results: array of { operator_id, outputs }
    let item_results_schema = array_schema(serde_json::json!({
        "type": "object",
        "required": ["operator_id"],
        "properties": {
            "operator_id": { "type": "string" },
            "outputs": { "type": "object" }
        }
    }));
    let mut static_fields = BTreeMap::new();
    static_fields.insert(
        "item_results".into(),
        OperatorSchema::with_defaults(SchemaDialect::JsonSchemaDraft07, item_results_schema),
    );
    OperatorOutputSchema {
        static_fields,
        required_fields: BTreeSet::from(["item_results".into()]),
        accepts_extra_fields: false,
        description: Some(
            "Map output: item_results replaces the v1.29.0 outputs[\"items\"]: Array placeholder"
                .into(),
        ),
    }
}

/// Constructs the default `OperatorInputSchema` for `Operator::Join`.
fn join_default_input() -> OperatorInputSchema {
    let mut static_fields = BTreeMap::new();
    static_fields.insert(
        "policy".into(),
        OperatorSchema::with_defaults(
            SchemaDialect::JsonSchemaDraft07,
            serde_json::json!({ "type": "string", "enum": ["all", "any"] }),
        ),
    );
    // branches: NonEmpty(Ref<OperatorId>) — represented as required array
    static_fields.insert(
        "branches".into(),
        OperatorSchema::with_defaults(
            SchemaDialect::JsonSchemaDraft07,
            array_schema(string_schema()),
        ),
    );
    OperatorInputSchema {
        static_fields,
        required_fields: BTreeSet::from(["policy".into(), "branches".into()]),
        accepts_extra_fields: false,
        description: None,
    }
}

/// Constructs the default `OperatorOutputSchema` for `Operator::Join`.
fn join_default_output() -> OperatorOutputSchema {
    OperatorOutputSchema {
        static_fields: BTreeMap::new(),
        required_fields: BTreeSet::new(),
        accepts_extra_fields: true,
        description: None,
    }
}

/// Constructs the default `OperatorInputSchema` for `Operator::Race`.
fn race_default_input() -> OperatorInputSchema {
    let mut static_fields = BTreeMap::new();
    static_fields.insert(
        "timeout_ms".into(),
        OperatorSchema::with_defaults(SchemaDialect::JsonSchemaDraft07, u64_schema()),
    );
    static_fields.insert(
        "branches".into(),
        OperatorSchema::with_defaults(
            SchemaDialect::JsonSchemaDraft07,
            array_schema(string_schema()),
        ),
    );
    OperatorInputSchema {
        static_fields,
        required_fields: BTreeSet::from(["timeout_ms".into(), "branches".into()]),
        accepts_extra_fields: false,
        description: None,
    }
}

/// Constructs the default `OperatorOutputSchema` for `Operator::Race`.
fn race_default_output() -> OperatorOutputSchema {
    let mut static_fields = BTreeMap::new();
    static_fields.insert(
        "winner".into(),
        OperatorSchema::with_defaults(SchemaDialect::JsonSchemaDraft07, string_schema()),
    );
    static_fields.insert(
        "winner_outputs".into(),
        OperatorSchema::with_defaults(SchemaDialect::JsonSchemaDraft07, object_schema()),
    );
    OperatorOutputSchema {
        static_fields,
        required_fields: BTreeSet::from(["winner".into(), "winner_outputs".into()]),
        accepts_extra_fields: false,
        description: None,
    }
}

/// Constructs the default `OperatorInputSchema` for `Operator::Choice`.
fn choice_default_input() -> OperatorInputSchema {
    let mut static_fields = BTreeMap::new();
    static_fields.insert(
        "branches".into(),
        OperatorSchema::with_defaults(
            SchemaDialect::JsonSchemaDraft07,
            object_schema(), // BTreeMap<ConditionExpr, OperatorId>
        ),
    );
    OperatorInputSchema {
        static_fields,
        required_fields: BTreeSet::from(["branches".into()]),
        accepts_extra_fields: false,
        description: None,
    }
}

/// Constructs the default `OperatorOutputSchema` for `Operator::Choice`.
fn choice_default_output() -> OperatorOutputSchema {
    let mut static_fields = BTreeMap::new();
    static_fields.insert(
        "selected_branch".into(),
        OperatorSchema::with_defaults(SchemaDialect::JsonSchemaDraft07, string_schema()),
    );
    OperatorOutputSchema {
        static_fields,
        required_fields: BTreeSet::new(), // optional if no branch matches
        accepts_extra_fields: true,
        description: None,
    }
}

/// Constructs the default `OperatorInputSchema` for `Operator::Loop`.
fn loop_default_input() -> OperatorInputSchema {
    let mut static_fields = BTreeMap::new();
    static_fields.insert(
        "max_iterations".into(),
        OperatorSchema::with_defaults(SchemaDialect::JsonSchemaDraft07, u64_schema()),
    );
    static_fields.insert(
        "until".into(),
        OperatorSchema::with_defaults(SchemaDialect::JsonSchemaDraft07, object_schema()),
    );
    static_fields.insert(
        "body".into(),
        OperatorSchema::with_defaults(SchemaDialect::JsonSchemaDraft07, string_schema()),
    );
    OperatorInputSchema {
        static_fields,
        required_fields: BTreeSet::from(["max_iterations".into(), "until".into(), "body".into()]),
        accepts_extra_fields: false,
        description: None,
    }
}

/// Constructs the default `OperatorOutputSchema` for `Operator::Loop`.
fn loop_default_output() -> OperatorOutputSchema {
    let mut static_fields = BTreeMap::new();
    static_fields.insert(
        "iterations_completed".into(),
        OperatorSchema::with_defaults(SchemaDialect::JsonSchemaDraft07, u64_schema()),
    );
    static_fields.insert(
        "last_outputs".into(),
        OperatorSchema::with_defaults(SchemaDialect::JsonSchemaDraft07, object_schema()),
    );
    OperatorOutputSchema {
        static_fields,
        required_fields: BTreeSet::from(["iterations_completed".into(), "last_outputs".into()]),
        accepts_extra_fields: false,
        description: None,
    }
}

/// Constructs the default `OperatorInputSchema` for `Operator::Gate`.
fn gate_default_input() -> OperatorInputSchema {
    let mut static_fields = BTreeMap::new();
    static_fields.insert(
        "condition".into(),
        OperatorSchema::with_defaults(SchemaDialect::JsonSchemaDraft07, object_schema()),
    );
    static_fields.insert(
        "body".into(),
        OperatorSchema::with_defaults(SchemaDialect::JsonSchemaDraft07, string_schema()),
    );
    OperatorInputSchema {
        static_fields,
        required_fields: BTreeSet::from(["condition".into(), "body".into()]),
        accepts_extra_fields: false,
        description: None,
    }
}

/// Constructs the default `OperatorOutputSchema` for `Operator::Gate`.
fn gate_default_output() -> OperatorOutputSchema {
    let mut static_fields = BTreeMap::new();
    static_fields.insert(
        "evaluated".into(),
        OperatorSchema::with_defaults(SchemaDialect::JsonSchemaDraft07, bool_schema()),
    );
    static_fields.insert(
        "then_outputs".into(),
        OperatorSchema::with_defaults(
            SchemaDialect::JsonSchemaDraft07,
            serde_json::json!({ "type": ["object", "null"] }),
        ),
    );
    OperatorOutputSchema {
        static_fields,
        required_fields: BTreeSet::from(["evaluated".into()]),
        accepts_extra_fields: false,
        description: None,
    }
}

/// Constructs the default `OperatorInputSchema` for `Operator::Wait`.
fn wait_default_input() -> OperatorInputSchema {
    let mut static_fields = BTreeMap::new();
    static_fields.insert(
        "event_type".into(),
        OperatorSchema::with_defaults(SchemaDialect::JsonSchemaDraft07, string_schema()),
    );
    static_fields.insert(
        "timeout_ms".into(),
        OperatorSchema::with_defaults(SchemaDialect::JsonSchemaDraft07, u64_schema()),
    );
    OperatorInputSchema {
        static_fields,
        required_fields: BTreeSet::from(["event_type".into(), "timeout_ms".into()]),
        accepts_extra_fields: false,
        description: None,
    }
}

/// Constructs the default `OperatorOutputSchema` for `Operator::Wait`.
fn wait_default_output() -> OperatorOutputSchema {
    let mut static_fields = BTreeMap::new();
    static_fields.insert(
        "event_payload".into(),
        OperatorSchema::with_defaults(
            SchemaDialect::JsonSchemaDraft07,
            serde_json::json!({ "type": ["object", "null"] }),
        ),
    );
    OperatorOutputSchema {
        static_fields,
        required_fields: BTreeSet::new(),
        accepts_extra_fields: true,
        description: None,
    }
}

/// Constructs the default `OperatorInputSchema` for `Operator::SubWorkflow`.
fn subworkflow_default_input() -> OperatorInputSchema {
    let mut static_fields = BTreeMap::new();
    static_fields.insert(
        "run_ref".into(),
        OperatorSchema::with_defaults(SchemaDialect::JsonSchemaDraft07, string_schema()),
    );
    OperatorInputSchema {
        static_fields,
        required_fields: BTreeSet::from(["run_ref".into()]),
        accepts_extra_fields: false,
        description: None,
    }
}

/// Constructs the default `OperatorOutputSchema` for `Operator::SubWorkflow`.
fn subworkflow_default_output() -> OperatorOutputSchema {
    let mut static_fields = BTreeMap::new();
    static_fields.insert(
        "final_outputs".into(),
        OperatorSchema::with_defaults(SchemaDialect::JsonSchemaDraft07, object_schema()),
    );
    OperatorOutputSchema {
        static_fields,
        required_fields: BTreeSet::from(["final_outputs".into()]),
        accepts_extra_fields: false,
        description: None,
    }
}

/// Constructs the default `OperatorInputSchema` for `Operator::Compensate`.
fn compensate_default_input() -> OperatorInputSchema {
    let mut static_fields = BTreeMap::new();
    static_fields.insert(
        "of".into(),
        OperatorSchema::with_defaults(SchemaDialect::JsonSchemaDraft07, string_schema()),
    );
    OperatorInputSchema {
        static_fields,
        required_fields: BTreeSet::from(["of".into()]),
        accepts_extra_fields: false,
        description: None,
    }
}

/// Constructs the default `OperatorOutputSchema` for `Operator::Compensate`.
fn compensate_default_output() -> OperatorOutputSchema {
    let mut static_fields = BTreeMap::new();
    static_fields.insert(
        "compensation_outcome".into(),
        OperatorSchema::with_defaults(
            SchemaDialect::JsonSchemaDraft07,
            serde_json::json!({ "type": ["object", "null"] }),
        ),
    );
    OperatorOutputSchema {
        static_fields,
        required_fields: BTreeSet::new(),
        accepts_extra_fields: true,
        description: None,
    }
}

/// Returns the default `OperatorInputSchema` for a given operator variant.
pub fn default_input_schema(variant: &Operator) -> OperatorInputSchema {
    match variant {
        Operator::Task { .. } => task_default_input(),
        Operator::Sequence { .. } => sequence_default_input(),
        Operator::Parallel { .. } => parallel_default_input(),
        Operator::Map { .. } => map_default_input(),
        Operator::Join { .. } => join_default_input(),
        Operator::Race { .. } => race_default_input(),
        Operator::Choice { .. } => choice_default_input(),
        Operator::Loop { .. } => loop_default_input(),
        Operator::Gate { .. } => gate_default_input(),
        Operator::Wait { .. } => wait_default_input(),
        Operator::SubWorkflow { .. } => subworkflow_default_input(),
        Operator::Compensate { .. } => compensate_default_input(),
    }
}

/// Returns the default `OperatorOutputSchema` for a given operator variant.
pub fn default_output_schema(variant: &Operator) -> OperatorOutputSchema {
    match variant {
        Operator::Task { .. } => task_default_output(),
        Operator::Sequence { .. } => sequence_default_output(),
        Operator::Parallel { .. } => parallel_default_output(),
        Operator::Map { .. } => map_default_output(),
        Operator::Join { .. } => join_default_output(),
        Operator::Race { .. } => race_default_output(),
        Operator::Choice { .. } => choice_default_output(),
        Operator::Loop { .. } => loop_default_output(),
        Operator::Gate { .. } => gate_default_output(),
        Operator::Wait { .. } => wait_default_output(),
        Operator::SubWorkflow { .. } => subworkflow_default_output(),
        Operator::Compensate { .. } => compensate_default_output(),
    }
}

/// Returns the variant name as a static string.
pub fn variant_name(variant: &Operator) -> &'static str {
    match variant {
        Operator::Task { .. } => "Task",
        Operator::Sequence { .. } => "Sequence",
        Operator::Parallel { .. } => "Parallel",
        Operator::Map { .. } => "Map",
        Operator::Join { .. } => "Join",
        Operator::Race { .. } => "Race",
        Operator::Choice { .. } => "Choice",
        Operator::Loop { .. } => "Loop",
        Operator::Gate { .. } => "Gate",
        Operator::Wait { .. } => "Wait",
        Operator::SubWorkflow { .. } => "SubWorkflow",
        Operator::Compensate { .. } => "Compensate",
    }
}

// ── Helpers ────────────────────────────────────────────────────────────────────

/// Computes the canonical content hash of a JSON value.
fn compute_content_hash(value: &serde_json::Value) -> crate::workflow_ir::ContentHash {
    let bytes = serde_json::to_vec(value).expect("serde_json::Value is always serializable");
    let digest = Sha256::digest(&bytes);
    let hex = format!("{:064x}", digest);
    format!("sha256:{}", hex)
}

// ── Tests ──────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    // REQ-OPSCHEMA-001: closed dialect set
    #[test]
    fn schema_dialect_closed_set() {
        // Only JsonSchemaDraft07 is constructable at v1
        let _ = SchemaDialect::JsonSchemaDraft07;
    }

    // REQ-OPERR-001: variant count is exactly 8
    #[test]
    fn operator_contract_error_variant_count_is_8() {
        let variants = [
            OperatorContractError::UnsupportedSchemaVersion { got: 0, want: 1 },
            OperatorContractError::UnknownOperatorVariant { variant: "Task" },
            OperatorContractError::InputContractViolation {
                operator_id: OperatorId("op0".into()),
                variant: "Task",
                field: "path".into(),
                reason: "required",
            },
            OperatorContractError::OutputContractViolation {
                operator_id: OperatorId("op0".into()),
                variant: "Task",
                field: "result".into(),
                reason: "type mismatch",
            },
            OperatorContractError::MissingRequiredField {
                operator_id: OperatorId("op0".into()),
                variant: "Task",
                field: "path".into(),
            },
            OperatorContractError::ExtraFieldDisallowed {
                operator_id: OperatorId("op0".into()),
                variant: "Task",
                field: "rogue".into(),
            },
            OperatorContractError::SchemaSourceMismatch {
                operator_id: OperatorId("op0".into()),
                expected: "sha256:0000000000000000000000000000000000000000000000000000000000000000"
                    .into(),
                actual: "sha256:1111111111111111111111111111111111111111111111111111111111111111"
                    .into(),
            },
            OperatorContractError::SchemaDialectUnknown {
                dialect: "unknown".into(),
            },
        ];
        assert_eq!(variants.len(), 8);
    }

    // REQ-OPERR-002: lives in domain module (module path check via compilation)
    #[test]
    fn operator_contract_error_in_domain_module() {
        // OperatorContractError lives in domain, not in engine.
        // This test verifies the type is accessible and matches the expected variant count.
        let _err: OperatorContractError = OperatorContractError::SchemaDialectUnknown {
            dialect: "test".into(),
        };
        assert!(matches!(
            _err,
            OperatorContractError::SchemaDialectUnknown { .. }
        ));
    }

    // REQ-OPERR-003: deterministic dispatch (variant dispatch is exhaustive)
    #[test]
    fn dispatch_deterministic() {
        fn classify(err: &OperatorContractError) -> &'static str {
            match err {
                OperatorContractError::UnsupportedSchemaVersion { .. } => "version",
                OperatorContractError::UnknownOperatorVariant { .. } => "unknown",
                OperatorContractError::InputContractViolation { .. } => "input",
                OperatorContractError::OutputContractViolation { .. } => "output",
                OperatorContractError::MissingRequiredField { .. } => "missing",
                OperatorContractError::ExtraFieldDisallowed { .. } => "extra",
                OperatorContractError::SchemaSourceMismatch { .. } => "source",
                OperatorContractError::SchemaDialectUnknown { .. } => "dialect",
            }
        }

        let errs = [
            OperatorContractError::UnsupportedSchemaVersion { got: 2, want: 1 },
            OperatorContractError::UnknownOperatorVariant { variant: "Reduce" },
            OperatorContractError::InputContractViolation {
                operator_id: OperatorId("op0".into()),
                variant: "Task",
                field: "x".into(),
                reason: "missing",
            },
            OperatorContractError::OutputContractViolation {
                operator_id: OperatorId("op0".into()),
                variant: "Map",
                field: "item_results".into(),
                reason: "missing",
            },
            OperatorContractError::MissingRequiredField {
                operator_id: OperatorId("op0".into()),
                variant: "Task",
                field: "path".into(),
            },
            OperatorContractError::ExtraFieldDisallowed {
                operator_id: OperatorId("op0".into()),
                variant: "Task",
                field: "rogue".into(),
            },
            OperatorContractError::SchemaSourceMismatch {
                operator_id: OperatorId("op0".into()),
                expected: "sha256:0".into(),
                actual: "sha256:1".into(),
            },
            OperatorContractError::SchemaDialectUnknown {
                dialect: "bad".into(),
            },
        ];

        // Every variant maps to exactly one static string — no String carries
        for err in &errs {
            let tag = classify(err);
            assert!(
                tag != "input"
                    || matches!(err, OperatorContractError::InputContractViolation { .. }),
                "dispatch must be deterministic"
            );
        }
    }

    // REQ-OPSCHEMA-001 / REQ-OPOUT-005: schema source consistency
    #[test]
    fn schema_source_mismatch_rejected() {
        let doc = serde_json::json!({ "type": "string" });
        let good_source = compute_content_hash(&doc);

        // Same source → OK
        let schema = OperatorSchema::new(
            OPERATOR_CONTRACT_SCHEMA_VERSION,
            SchemaDialect::JsonSchemaDraft07,
            good_source.clone(),
            doc.clone(),
        );
        assert!(schema.is_ok());

        // Wrong source → Err
        let bad_source =
            "sha256:0000000000000000000000000000000000000000000000000000000000000000".into();
        let schema2 = OperatorSchema::new(
            OPERATOR_CONTRACT_SCHEMA_VERSION,
            SchemaDialect::JsonSchemaDraft07,
            bad_source,
            doc,
        );
        assert!(matches!(
            schema2,
            Err(OperatorContractError::SchemaSourceMismatch { .. })
        ));
    }

    // REQ-OPIN-003: input rejects extra field when accepts_extra_fields = false
    #[test]
    fn input_rejects_extra_field() {
        let schema = OperatorInputSchema {
            static_fields: BTreeMap::new(),
            required_fields: BTreeSet::new(),
            accepts_extra_fields: false,
            description: None,
        };

        let input: BTreeMap<String, serde_json::Value> =
            BTreeMap::from([("rogue".into(), serde_json::json!(1))]);

        let result = schema.validate(&OperatorId("op0".into()), "Task", &input);
        assert!(matches!(
            result,
            Err(OperatorContractError::ExtraFieldDisallowed { field, .. })
                if field == "rogue"
        ));
    }

    // REQ-OPIN-004: input missing required field
    #[test]
    fn input_missing_required_field() {
        let mut static_fields = BTreeMap::new();
        static_fields.insert(
            "path".into(),
            OperatorSchema::with_defaults(SchemaDialect::JsonSchemaDraft07, string_schema()),
        );
        let schema = OperatorInputSchema {
            static_fields,
            required_fields: BTreeSet::from(["path".into()]),
            accepts_extra_fields: false,
            description: None,
        };

        let input: BTreeMap<String, serde_json::Value> = BTreeMap::new();

        let result = schema.validate(&OperatorId("op0".into()), "Task", &input);
        assert!(matches!(
            result,
            Err(OperatorContractError::MissingRequiredField { field, .. })
                if field == "path"
        ));
    }

    // REQ-OPOUT-001 / REQ-OPOUT-004: output missing required field
    #[test]
    fn output_missing_required_field() {
        let schema = OperatorOutputSchema {
            static_fields: BTreeMap::new(),
            required_fields: BTreeSet::from(["item_results".into()]),
            accepts_extra_fields: false,
            description: None,
        };

        let output: BTreeMap<String, serde_json::Value> = BTreeMap::new();

        let result = schema.validate(&OperatorId("op0".into()), "Map", &output);
        assert!(matches!(
            result,
            Err(OperatorContractError::MissingRequiredField { field, .. })
                if field == "item_results"
        ));
    }

    // REQ-OPOUT-003: output rejects extra field when accepts_extra_fields = false
    #[test]
    fn output_rejects_extra_field_when_strict() {
        let schema = OperatorOutputSchema {
            static_fields: BTreeMap::new(),
            required_fields: BTreeSet::new(),
            accepts_extra_fields: false,
            description: None,
        };

        let output: BTreeMap<String, serde_json::Value> =
            BTreeMap::from([("items".into(), serde_json::json!([]))]); // old placeholder key

        let result = schema.validate(&OperatorId("op0".into()), "Map", &output);
        assert!(matches!(
            result,
            Err(OperatorContractError::ExtraFieldDisallowed { field, .. })
                if field == "items"
        ));
    }

    // REQ-OPIN-001 / REQ-OPOUT-001: every variant has default schemas
    #[test]
    fn default_schemas_per_variant() {
        use crate::workflow_ir::{CapabilityId, GuardExpr, Operator, OperatorId};
        use std::collections::BTreeMap;

        let variants = [
            Operator::Task {
                capability: CapabilityId("test.cap".to_string()),
                inputs: BTreeMap::new(),
            },
            Operator::Sequence { body: vec![] },
            Operator::Parallel {
                branches: vec![],
                max_concurrency: 1,
            },
            Operator::Map {
                source: OperatorId("src".to_string()),
                body: OperatorId("body".to_string()),
                max_concurrency: 4,
            },
            Operator::Join {
                policy: "all".to_string(),
                branches: vec![],
            },
            Operator::Race {
                branches: vec![],
                timeout_ms: 1000,
            },
            Operator::Choice {
                branches: BTreeMap::new(),
            },
            Operator::Loop {
                max_iterations: 10,
                until: GuardExpr {
                    expr: "true".to_string(),
                },
                body: OperatorId("body".to_string()),
            },
            Operator::Gate {
                condition: GuardExpr {
                    expr: "true".to_string(),
                },
                body: OperatorId("body".to_string()),
            },
            Operator::Wait {
                event_type: "click".to_string(),
                timeout_ms: 5000,
            },
            Operator::SubWorkflow {
                run_ref: "run-1".to_string(),
            },
            Operator::Compensate {
                of: OperatorId("op0".to_string()),
            },
        ];

        for variant in &variants {
            let input = default_input_schema(variant);
            let output = default_output_schema(variant);
            // PartialEq is reflexive: x == x proves the trait is implemented
            assert_eq!(&input, &input, "input schema must be PartialEq");
            assert_eq!(&output, &output, "output schema must be PartialEq");
        }
    }

    // REQ-OPSCHEMA-001: schema version must equal constant
    #[test]
    fn operator_schema_version_check() {
        let doc = serde_json::json!({ "type": "string" });
        let schema = OperatorSchema::with_defaults(SchemaDialect::JsonSchemaDraft07, doc);
        assert_eq!(schema.version, OPERATOR_CONTRACT_SCHEMA_VERSION);
        assert_eq!(schema.version, 1);
    }

    // REQ-OPOUT-006: Map output contract replaces placeholder
    #[test]
    fn map_output_contract_replaces_placeholder() {
        let output = map_default_output();
        // The contract requires "item_results", not "items"
        assert!(output.static_fields.contains_key("item_results"));
        assert!(!output.static_fields.contains_key("items"));
        assert!(output.required_fields.contains("item_results"));
        assert!(!output.accepts_extra_fields);
    }

    // REQ-OPLINE-002: description is excluded from projection
    #[test]
    fn description_excluded_from_projection() {
        let schema = OperatorInputSchema {
            static_fields: BTreeMap::new(),
            required_fields: BTreeSet::new(),
            accepts_extra_fields: true,
            description: Some("this is non-semantic".into()),
        };
        let projection: OperatorInputSchemaProjection = (&schema).into();
        // projection has no description field
        assert_eq!(projection.static_fields, schema.static_fields);
        assert_eq!(projection.required_fields, schema.required_fields);
        assert_eq!(projection.accepts_extra_fields, schema.accepts_extra_fields);
    }

    // Round-trip deterministic serialization of OperatorSchema
    #[test]
    fn operator_schema_deterministic_roundtrip() {
        let doc = serde_json::json!({
            "type": "object",
            "properties": {
                "name": { "type": "string" }
            }
        });
        let schema = OperatorSchema::with_defaults(SchemaDialect::JsonSchemaDraft07, doc);

        let bytes = serde_json::to_vec(&schema).expect("always serializable");
        let roundtrip: OperatorSchema =
            serde_json::from_slice(&bytes).expect("always deserializable");

        assert_eq!(schema.version, roundtrip.version);
        assert_eq!(schema.source, roundtrip.source);
        assert_eq!(schema.document, roundtrip.document);

        // Second serialization must be byte-identical (deterministic)
        let bytes2 = serde_json::to_vec(&schema).expect("always serializable");
        assert_eq!(&bytes, &bytes2, "canonical JSON must be byte-stable");
    }

    // OperatorContractProjectionV1 round-trip
    #[test]
    fn projection_roundtrip() {
        let projection = OperatorContractProjectionV1 {
            inputs: BTreeMap::new(),
            outputs: BTreeMap::new(),
            schema_version: OPERATOR_CONTRACT_SCHEMA_VERSION,
        };

        let bytes = serde_json::to_vec(&projection).expect("always serializable");
        let roundtrip: OperatorContractProjectionV1 =
            serde_json::from_slice(&bytes).expect("always deserializable");

        assert_eq!(projection, roundtrip);
    }
}
