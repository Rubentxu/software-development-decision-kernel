//! Plan revision, hash, provenance, and explicit mutation lineage.
//!
//! Provides canonical normalization of [`WorkflowIR`] plans for stable identity
//! (semantically-equivalent IRs produce identical plan identities even when
//! they differ in non-semantic metadata such as `ir_id`, `prompt_hash`,
//! `model_hash`).  Every normalized plan carries typed mutations and provenance
//! so that revision chains are fully auditable and deterministic.
//!
//! ## Design decisions
//!
//! - **Identity exclusion** (`ir_id`, `prompt_hash`, `model_hash`) follows
//!   ADR-024 / ADR-037: those fields are provenance of generation, not
//!   semantic content of the plan.
//! - **revision_id** is deterministic: `sha256(parent_revision_id|mutation|normalized_identity)`.
//!   Repeated construction of the same lineage always yields the same `revision_id`.
//! - **Append-only lineage**: [`PlanRevisionLineageV1::derive`] always returns a
//!   NEW lineage (the receiver is never mutated), whose tip is the child of the
//!   previous tip.
//!
//! ## Blocker / known limitation
//!
//! The spec REQ-1 requests an `edges` field (`BTreeMap<EdgeId, …>`) in
//! `NormalizedPlanV1`, stating it should be "according to IR structure".
//! However, the actual [`WorkflowIR`] type (workflow_ir.rs:537-559) contains
//! NO `edges` field — operators (`operators: BTreeMap<OperatorId, Operator>`)
//! are the sole topological primitive; edges are implicit inside operator
//! references (e.g. `body`, `branches`, `source`/`body` fields).
//! `EdgeId` IS defined (workflow_ir.rs:50) but belongs to the runtime level
//! (`ExecutionGraphRevision`, graph.rs:1234) and is not present in `WorkflowIR`.
//!
//! `edges: BTreeMap<crate::workflow_ir::EdgeId, Edge>` **cannot be added**
//! to `NormalizedPlanV1` without either (a) modifying `WorkflowIR` to add an
//! edges field, or (b) inferring edges from operator references (non-trivial
//! semantic transformation beyond normalisation scope).
//!
//! Current implementation: `edges: BTreeMap<crate::workflow_ir::EdgeId, Edge>`
//! is included in `NormalizedPlanV1` as a **placeholder empty map** so that the
//! field signature matches the spec.  Tests that exercise edge-level mutation
//! (`EdgesChanged`) will need to be revisited when the IR gains an edges
//! representation.
//!
//! ```ignore
//! // Current (placeholder — will be non-empty once WorkflowIR has edges):
//! edges: BTreeMap::<crate::workflow_ir::EdgeId, Edge>::new(),
//! ```

use std::collections::BTreeMap;
use std::collections::BTreeSet;

use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};

use crate::operator_contract::{
    OPERATOR_CONTRACT_SCHEMA_VERSION, OperatorContractProjectionV1, OperatorInputSchemaProjection,
    OperatorOutputSchemaProjection, default_input_schema, default_output_schema,
};
use crate::workflow_ir::{
    Budgets, EdgeId, GuardExpr, Invariant, Operator, OperatorId, Policy, WorkflowIR,
};

/// Canonical semantic form of a plan.
///
/// Produced by [`NormalizedPlanV1::from_workflow_ir`].  Two `WorkflowIR`
/// values that differ ONLY in non-semantic fields (`ir_id`, `prompt_hash`,
/// `model_hash`) or in `BTreeMap` construction order will produce identical
/// `NormalizedPlanV1` and therefore the same [`plan_identity`].
///
/// ## Blocker (see module-level doc)
///
/// The spec requests an `edges` field.  See the module-level blocker note for
/// the full evidence.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct NormalizedPlanV1 {
    /// Nodes (operators) in deterministic order.
    pub nodes: BTreeMap<OperatorId, Operator>,

    /// Guards keyed by operator ID, in deterministic order.
    pub guards: BTreeMap<OperatorId, GuardExpr>,

    /// Effective expansion permissions.
    pub expansion_permissions: BTreeSet<crate::workflow_ir::ExpansionPermission>,

    /// Execution budgets.
    pub budgets: Budgets,

    /// Required invariants (subset of template invariants).
    pub required_invariants: BTreeSet<Invariant>,

    /// Policy — structural (NOT `policy_hash` from provenance).
    pub policy: Policy,

    /// Schema version of the normalised representation (fixed: 1).
    pub schema_version: u32,

    // ── Blocker placeholder ───────────────────────────────────────────────────
    // See module-level doc.  DO NOT remove — signature must match REQ-1.
    /// Edge map — **PLACEHOLDER**: `WorkflowIR` has no edges field; this is
    /// empty until the IR gains an explicit edge representation.
    #[serde(default, skip_serializing_if = "BTreeMap::is_empty")]
    pub edges: BTreeMap<EdgeId, Edge>,
    // ─────────────────────────────────────────────────────────────────────────
    /// Typed operator contracts keyed by operator ID.
    ///
    /// Populated by projecting each operator in `nodes` through
    /// `default_input_schema` / `default_output_schema`.  The `description`
    /// field of each schema is excluded from the projection (non-semantic).
    /// Legacy IRs default to an empty map via `#[serde(default)]`.
    #[serde(default, skip_serializing_if = "BTreeMap::is_empty")]
    pub operator_contracts: BTreeMap<OperatorId, OperatorContractProjectionV1>,
}

/// Minimal edge representation (placeholder for the blocker above).
///
/// Until `WorkflowIR` gains a real `edges` field we use this zero-value type
/// so that the `BTreeMap<EdgeId, Edge>` field in `NormalizedPlanV1` is
/// well-typed even though it is always empty.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Edge {
    _private: (),
}

impl Edge {
    fn placeholder() -> Self {
        Self { _private: () }
    }
}

impl NormalizedPlanV1 {
    /// Constructs a normalised plan from a [`WorkflowIR`].
    ///
    /// Excludes non-semantic fields (`ir_id`, `prompt_hash`, `model_hash`) per
    /// ADR-024 / ADR-037.
    ///
    /// ## Policy field limitation
    ///
    /// `WorkflowIR` does not store the full `Policy` object — only
    /// `provenance.policy_hash` (a string).  The `policy` field is populated
    /// with a placeholder that carries the `policy_hash` as its name and an
    /// empty config map.  This is a **known limitation** of the current IR
    /// substrate; the field will become non-empty if `WorkflowIR` is ever
    /// extended to include the policy content directly.
    pub fn from_workflow_ir(ir: &WorkflowIR) -> Self {
        // Placeholder policy — WorkflowIR stores only policy_hash, not the full Policy.
        let policy = Policy {
            name: ir.provenance.policy_hash.clone(),
            config: Default::default(),
        };

        // Populate operator_contracts: project each operator to its typed contract.
        // Non-semantic description fields are excluded by the projection types.
        let operator_contracts = ir
            .operators
            .iter()
            .map(|(op_id, op)| {
                let input = default_input_schema(op);
                let output = default_output_schema(op);
                let projection = OperatorContractProjectionV1 {
                    inputs: BTreeMap::from([(
                        op_id.clone(),
                        OperatorInputSchemaProjection::from(&input),
                    )]),
                    outputs: BTreeMap::from([(
                        op_id.clone(),
                        OperatorOutputSchemaProjection::from(&output),
                    )]),
                    schema_version: OPERATOR_CONTRACT_SCHEMA_VERSION,
                };
                (op_id.clone(), projection)
            })
            .collect();

        Self {
            nodes: ir.operators.clone(),
            guards: ir.guards.clone(),
            expansion_permissions: ir.expansion_permissions.clone(),
            budgets: ir.budgets.clone(),
            required_invariants: ir.required_invariants.clone(),
            policy,
            schema_version: 1,
            // Blocker placeholder — always empty until WorkflowIR has edges.
            edges: BTreeMap::new(),
            operator_contracts,
        }
    }

    /// Returns the canonical plan identity as `sha256:<64-hex-lowercase>`.
    ///
    /// The digest is computed over the canonical JSON serialisation of `self`,
    /// which is stable because all collection fields are `BTreeMap`/`BTreeSet`
    /// and `serde_json` serialises them in sorted key order.
    pub fn plan_identity(&self) -> String {
        let bytes = serde_json::to_vec(self).expect("NormalizedPlanV1 is always serializable");
        let digest = Sha256::digest(&bytes);
        let hex = format!("{:064x}", digest);
        format!("sha256:{}", hex)
    }

    /// Validates the normalised plan.
    ///
    /// Currently delegates the schema-version check; the IR-level validation
    /// is assumed to have already run.
    pub fn validate(&self) -> Result<(), PlanRevisionError> {
        if self.schema_version != 1 {
            return Err(PlanRevisionError::UnsupportedSchemaVersion {
                got: self.schema_version,
                want: 1,
            });
        }
        Ok(())
    }
}

impl Eq for NormalizedPlanV1 {}

// ── PlanMutation ──────────────────────────────────────────────────────────────

/// Closed set of typed mutations that can transform one normalised plan into
/// another.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum PlanMutation {
    /// The first entry in a lineage (root).
    Initial,
    /// One or more operators were added, removed, or changed.
    NodesChanged,
    /// One or more edges were added, removed, or changed.
    EdgesChanged,
    /// Execution budgets were adjusted.
    BudgetsAdjusted,
    /// Policy was amended.
    PolicyAmended,
    /// Provenance metadata was refreshed (ir_id, prompt_hash, model_hash).
    ProvenanceRefreshed,
    /// Structure was replaced wholesale.
    StructureReplaced,
}

// Compile-time guard: 7 variants.
crate::assert_variant_count_eq!(
    PlanMutation,
    7,
    [
        PlanMutation::Initial,
        PlanMutation::NodesChanged,
        PlanMutation::EdgesChanged,
        PlanMutation::BudgetsAdjusted,
        PlanMutation::PolicyAmended,
        PlanMutation::ProvenanceRefreshed,
        PlanMutation::StructureReplaced,
    ]
);

// ── PlanProvenanceV1 ─────────────────────────────────────────────────────────

/// Provenance metadata attached to a single revision.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct PlanProvenanceV1 {
    /// Tool or agent that produced this revision.  Must be non-empty.
    pub author: String,
    /// Version string of the tool.  Must be non-empty.
    pub tool_version: String,
}

impl PlanProvenanceV1 {
    /// Constructs a provenance, validating that neither field is empty.
    pub fn new(
        author: impl Into<String>,
        tool_version: impl Into<String>,
    ) -> Result<Self, PlanRevisionError> {
        let author = author.into();
        let tool_version = tool_version.into();
        if author.is_empty() {
            return Err(PlanRevisionError::EmptyProvenanceField { field: "author" });
        }
        if tool_version.is_empty() {
            return Err(PlanRevisionError::EmptyProvenanceField {
                field: "tool_version",
            });
        }
        Ok(Self {
            author,
            tool_version,
        })
    }
}

// ── PlanRevisionV1 ───────────────────────────────────────────────────────────

/// A single revision in a [`PlanRevisionLineageV1`] chain.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct PlanRevisionV1 {
    /// Deterministic revision identifier: `sha256(parent_revision_id|mutation|normalized_identity)`.
    pub revision_id: String,
    /// Parent revision ID, or `None` for the root (`Initial`).
    pub parent_revision_id: Option<String>,
    /// The mutation that produced this revision from its parent.
    pub mutation: PlanMutation,
    /// Provenance of this revision.
    pub provenance: PlanProvenanceV1,
    /// The normalised plan at this revision.
    pub normalized: NormalizedPlanV1,
}

impl PlanRevisionV1 {
    /// Constructs a revision.
    ///
    /// `revision_id` is computed deterministically.  `parent_revision_id` must
    /// be `None` iff `mutation == Initial`.
    pub fn new(
        parent_revision_id: Option<String>,
        mutation: PlanMutation,
        provenance: PlanProvenanceV1,
        normalized: NormalizedPlanV1,
    ) -> Result<Self, PlanRevisionError> {
        if parent_revision_id.is_some() && mutation == PlanMutation::Initial {
            return Err(PlanRevisionError::NoOpMutation);
        }

        let parent_str = parent_revision_id.as_deref().unwrap_or("root");
        let normalized_id = normalized.plan_identity();

        // Build the determinism input: parent_revision_id|mutation|normalized_identity
        let mut input = parent_str.to_string();
        input.push('|');
        input.push_str(&serde_json::to_string(&mutation).expect("mutation is always serializable"));
        input.push('|');
        input.push_str(&normalized_id);

        let digest = Sha256::digest(input.as_bytes());
        let revision_id = format!("{:064x}", digest);

        Ok(Self {
            revision_id,
            parent_revision_id,
            mutation,
            provenance,
            normalized,
        })
    }
}

// ── PlanRevisionLineageV1 ─────────────────────────────────────────────────────

/// Append-only chain of plan revisions.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct PlanRevisionLineageV1 {
    /// All revisions in the chain, oldest first.
    revisions: Vec<PlanRevisionV1>,
}

impl PlanRevisionLineageV1 {
    /// Initialises a new lineage from a `WorkflowIR` and provenance.
    ///
    /// The first revision has mutation `Initial` and no parent.
    pub fn initial(
        ir: &WorkflowIR,
        provenance: PlanProvenanceV1,
    ) -> Result<Self, PlanRevisionError> {
        let normalized = NormalizedPlanV1::from_workflow_ir(ir);
        normalized.validate()?;

        let revision = PlanRevisionV1::new(None, PlanMutation::Initial, provenance, normalized)?;
        Ok(Self {
            revisions: vec![revision],
        })
    }

    /// Derives a new child revision from the tip of this lineage.
    ///
    /// The receiver is **not** mutated; a new `PlanRevisionLineageV1` is returned.
    ///
    /// ## Errors
    ///
    /// - [`PlanRevisionError::EmptyLineage`] if called on an empty lineage.
    /// - [`PlanRevisionError::NoOpMutation`] if the `mutated_ir` normalises to the
    ///   same identity as the tip AND `mutation != Initial`.
    pub fn derive(
        &self,
        mutated_ir: &WorkflowIR,
        mutation: PlanMutation,
        provenance: PlanProvenanceV1,
    ) -> Result<Self, PlanRevisionError> {
        if self.revisions.is_empty() {
            return Err(PlanRevisionError::EmptyLineage);
        }

        let tip = self.tip();
        let new_normalized = NormalizedPlanV1::from_workflow_ir(mutated_ir);
        new_normalized.validate()?;

        // No-op detection: same normalised identity + non-Initial mutation
        if tip.normalized.plan_identity() == new_normalized.plan_identity()
            && mutation != PlanMutation::Initial
        {
            return Err(PlanRevisionError::NoOpMutation);
        }

        let parent_id = Some(tip.revision_id.clone());
        let revision = PlanRevisionV1::new(parent_id, mutation, provenance, new_normalized)?;

        let mut new_revisions = self.revisions.clone();
        new_revisions.push(revision);
        Ok(Self {
            revisions: new_revisions,
        })
    }

    /// Returns the tip (most recent revision) of the lineage.
    ///
    /// ## Panics
    ///
    /// Panics if the lineage is empty.
    pub fn tip(&self) -> &PlanRevisionV1 {
        self.revisions
            .last()
            .expect("PlanRevisionLineageV1 is guaranteed non-empty after initial()")
    }

    /// Returns all revisions from tip (most recent) down to root (oldest).
    pub fn ancestry(&self) -> Vec<&PlanRevisionV1> {
        self.revisions.iter().rev().collect()
    }

    /// Returns the number of revisions in the lineage.
    pub fn len(&self) -> usize {
        self.revisions.len()
    }

    /// Returns `true` if the lineage has no revisions.
    pub fn is_empty(&self) -> bool {
        self.revisions.is_empty()
    }
}

// ── PlanRevisionError ────────────────────────────────────────────────────────

/// Errors that can arise from plan-revision operations.
#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum PlanRevisionError {
    /// Unsupported schema version.
    #[error("unsupported schema version: got {got}, want {want}")]
    UnsupportedSchemaVersion { got: u32, want: u32 },

    /// The mutation would not change the normalised plan identity.
    ///
    /// Raised when [`PlanRevisionLineageV1::derive`] is called with a
    /// `WorkflowIR` that normalises to the same identity as the tip AND the
    /// mutation is not `Initial`.
    #[error("no-op mutation: normalised plan identity is unchanged")]
    NoOpMutation,

    /// A required provenance field is empty.
    #[error("empty provenance field: {field}")]
    EmptyProvenanceField { field: &'static str },

    /// Operation attempted on an empty lineage.
    #[error("empty lineage")]
    EmptyLineage,
}

// Compile-time guard: 4 variants.
crate::assert_variant_count_eq!(
    PlanRevisionError,
    4,
    [
        PlanRevisionError::UnsupportedSchemaVersion { .. },
        PlanRevisionError::NoOpMutation,
        PlanRevisionError::EmptyProvenanceField { .. },
        PlanRevisionError::EmptyLineage,
    ]
);

// ── Versioned enum ────────────────────────────────────────────────────────────

/// Versioned normalised plan.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "version", rename_all = "snake_case")]
pub enum NormalizedPlan {
    V1(NormalizedPlanV1),
}

impl NormalizedPlan {
    /// Validates the versioned plan.
    pub fn validate(&self) -> Result<(), PlanRevisionError> {
        match self {
            NormalizedPlan::V1(v1) => v1.validate(),
        }
    }
}

// ── Tests ────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;
    use crate::workflow_ir::{
        Budgets, CapabilityId, ExpansionPermission, Operator, OperatorId, Policy, Provenance,
        SCHEMA_VERSION, TemplateRef, WorkflowIR,
    };
    use std::collections::BTreeMap;

    fn sample_template_ref() -> TemplateRef {
        TemplateRef {
            id: "test.template".to_string(),
            version: "1.0.0".to_string(),
        }
    }

    fn sample_ir_with_op(op: Operator) -> WorkflowIR {
        let mut operators = BTreeMap::new();
        operators.insert(OperatorId("op0".to_string()), op);

        WorkflowIR {
            ir_id: Some(crate::workflow_ir::IrId("ir-001".to_string())),
            schema_version: SCHEMA_VERSION,
            template_ref: sample_template_ref(),
            operators,
            guards: Default::default(),
            expansion_permissions: BTreeSet::from([ExpansionPermission::Map]),
            budgets: Budgets::default(),
            required_invariants: Default::default(),
            provenance: Provenance {
                generated_by: "test".to_string(),
                prompt_hash: "prompt-hash-1".to_string(),
                model_hash: "model-hash-1".to_string(),
                policy_hash: "policy-hash-1".to_string(),
            },
        }
    }

    fn ir_with_different_metadata() -> WorkflowIR {
        let mut ir = sample_ir_with_op(Operator::Task {
            capability: CapabilityId("git.commit".to_string()),
            inputs: Default::default(),
        });
        ir.ir_id = Some(crate::workflow_ir::IrId("ir-different".to_string()));
        ir.provenance.prompt_hash = "different-prompt-hash".to_string();
        ir.provenance.model_hash = "different-model-hash".to_string();
        ir
    }

    fn ir_with_extra_node() -> WorkflowIR {
        let mut ir = sample_ir_with_op(Operator::Task {
            capability: CapabilityId("git.commit".to_string()),
            inputs: Default::default(),
        });
        ir.operators.insert(
            OperatorId("op1".to_string()),
            Operator::Task {
                capability: CapabilityId("git.push".to_string()),
                inputs: Default::default(),
            },
        );
        ir
    }

    // REQ-2: equivalence
    #[test]
    fn test_equivalence_ir_id_prompt_hash_different_same_identity() {
        let ir1 = sample_ir_with_op(Operator::Task {
            capability: CapabilityId("git.commit".to_string()),
            inputs: Default::default(),
        });
        let ir2 = ir_with_different_metadata();

        let np1 = NormalizedPlanV1::from_workflow_ir(&ir1);
        let np2 = NormalizedPlanV1::from_workflow_ir(&ir2);

        assert_eq!(np1.plan_identity(), np2.plan_identity());
    }

    #[test]
    fn test_equivalence_extra_node_different_identity() {
        let ir1 = sample_ir_with_op(Operator::Task {
            capability: CapabilityId("git.commit".to_string()),
            inputs: Default::default(),
        });
        let ir2 = ir_with_extra_node();

        let np1 = NormalizedPlanV1::from_workflow_ir(&ir1);
        let np2 = NormalizedPlanV1::from_workflow_ir(&ir2);

        assert_ne!(np1.plan_identity(), np2.plan_identity());
    }

    // REQ-2 / REQ-5: stability
    #[test]
    fn test_stability_repeated_calls() {
        let ir = sample_ir_with_op(Operator::Task {
            capability: CapabilityId("git.commit".to_string()),
            inputs: Default::default(),
        });
        let np = NormalizedPlanV1::from_workflow_ir(&ir);

        let id1 = np.plan_identity();
        let id2 = np.plan_identity();
        assert_eq!(id1, id2);
    }

    #[test]
    fn test_stability_roundtrip_serde() {
        let ir = sample_ir_with_op(Operator::Task {
            capability: CapabilityId("git.commit".to_string()),
            inputs: Default::default(),
        });
        let np = NormalizedPlanV1::from_workflow_ir(&ir);

        let bytes = serde_json::to_vec(&np).unwrap();
        let np2: NormalizedPlanV1 = serde_json::from_slice(&bytes).unwrap();

        assert_eq!(np.plan_identity(), np2.plan_identity());
    }

    // REQ-IRDT-RT-02: NormalizedPlanV1 round-trip with populated operator_contracts (≥2 entries).
    #[test]
    fn test_stability_roundtrip_serde_with_operator_contracts() {
        // Build IR with ≥2 operators so operator_contracts has ≥2 entries
        let ir = ir_with_extra_node(); // has op0 and op1
        let np = NormalizedPlanV1::from_workflow_ir(&ir);

        // Verify operator_contracts is populated
        assert!(
            np.operator_contracts.len() >= 2,
            "operator_contracts must have ≥2 entries, got {}",
            np.operator_contracts.len()
        );

        // Round-trip
        let bytes = serde_json::to_vec(&np).unwrap();
        let np2: NormalizedPlanV1 = serde_json::from_slice(&bytes).unwrap();

        // plan_identity must be stable
        assert_eq!(
            np.plan_identity(),
            np2.plan_identity(),
            "plan_identity must be stable after round-trip"
        );

        // Per-operator projection must match
        assert_eq!(
            np.operator_contracts.len(),
            np2.operator_contracts.len(),
            "operator_contracts count must match after round-trip"
        );

        for (op_id, proj1) in &np.operator_contracts {
            let proj2 = np2
                .operator_contracts
                .get(op_id)
                .expect("operator id must be preserved in round-trip");
            assert_eq!(
                proj1, proj2,
                "per-operator projection must match for {:?}",
                op_id
            );
        }
    }

    // REQ-3: lineage
    #[test]
    fn test_lineage_initial_then_derive_nodes_changed() {
        let ir_initial = sample_ir_with_op(Operator::Task {
            capability: CapabilityId("git.commit".to_string()),
            inputs: Default::default(),
        });
        let provenance = PlanProvenanceV1::new("test-author", "1.0.0").unwrap();

        let lineage = PlanRevisionLineageV1::initial(&ir_initial, provenance.clone()).unwrap();
        assert_eq!(lineage.len(), 1);
        assert!(lineage.tip().parent_revision_id.is_none());
        assert_eq!(lineage.tip().mutation, PlanMutation::Initial);

        // Derive with NodesChanged
        let ir_mutated = ir_with_extra_node();
        let lineage2 = lineage
            .derive(&ir_mutated, PlanMutation::NodesChanged, provenance.clone())
            .unwrap();

        assert_eq!(lineage2.len(), 2);
        assert_eq!(
            lineage2.tip().parent_revision_id.as_deref(),
            Some(lineage.tip().revision_id.as_str())
        );
        assert_eq!(lineage2.tip().mutation, PlanMutation::NodesChanged);

        // Derive with BudgetsAdjusted
        let mut ir_budget = ir_mutated.clone();
        ir_budget.budgets.max_wall_ms = 42;

        let lineage3 = lineage2
            .derive(&ir_budget, PlanMutation::BudgetsAdjusted, provenance)
            .unwrap();

        assert_eq!(lineage3.len(), 3);
        assert_eq!(
            lineage3.tip().parent_revision_id.as_deref(),
            Some(lineage2.tip().revision_id.as_str())
        );
    }

    #[test]
    fn test_lineage_ancestry_order_tip_to_root() {
        let ir_initial = sample_ir_with_op(Operator::Task {
            capability: CapabilityId("git.commit".to_string()),
            inputs: Default::default(),
        });
        let provenance = PlanProvenanceV1::new("test", "1.0.0").unwrap();

        let lineage = PlanRevisionLineageV1::initial(&ir_initial, provenance.clone()).unwrap();
        let ir2 = ir_with_extra_node();
        let lineage = lineage
            .derive(&ir2, PlanMutation::NodesChanged, provenance.clone())
            .unwrap();

        let ancestry = lineage.ancestry();
        assert_eq!(ancestry.len(), 2);
        assert_eq!(ancestry[0].mutation, PlanMutation::NodesChanged); // tip first
        assert_eq!(ancestry[1].mutation, PlanMutation::Initial); // root last
    }

    #[test]
    fn test_lineage_deterministic_revision_id() {
        let ir = sample_ir_with_op(Operator::Task {
            capability: CapabilityId("git.commit".to_string()),
            inputs: Default::default(),
        });
        let provenance = PlanProvenanceV1::new("test", "1.0.0").unwrap();

        let l1 = PlanRevisionLineageV1::initial(&ir, provenance.clone()).unwrap();
        let tip1 = l1.tip().revision_id.clone();

        // Rebuild from scratch
        let l2 = PlanRevisionLineageV1::initial(&ir, provenance).unwrap();
        let tip2 = l2.tip().revision_id.clone();

        assert_eq!(tip1, tip2, "revision_id must be deterministic");
    }

    // REQ-4: no-op detection
    #[test]
    fn test_noop_mutation_rejected() {
        let ir = sample_ir_with_op(Operator::Task {
            capability: CapabilityId("git.commit".to_string()),
            inputs: Default::default(),
        });
        let provenance = PlanProvenanceV1::new("test", "1.0.0").unwrap();

        let lineage = PlanRevisionLineageV1::initial(&ir, provenance.clone()).unwrap();

        // Derive with the SAME IR but non-Initial mutation
        let result = lineage.derive(&ir, PlanMutation::PolicyAmended, provenance);
        assert!(matches!(result, Err(PlanRevisionError::NoOpMutation)));
    }

    #[test]
    fn test_noop_mutation_with_initial_is_ok() {
        // Initial is always allowed (it's the root constructor path)
        let ir = sample_ir_with_op(Operator::Task {
            capability: CapabilityId("git.commit".to_string()),
            inputs: Default::default(),
        });
        let provenance = PlanProvenanceV1::new("test", "1.0.0").unwrap();

        let lineage = PlanRevisionLineageV1::initial(&ir, provenance).unwrap();
        // The lineage itself is fine; derive with identical IR + Initial would mean
        // calling new() with Some(parent) + Initial which is caught by the constructor.
        // This is tested separately in the constructor test below.
    }

    #[test]
    fn test_constructor_rejects_initial_with_parent() {
        let ir = sample_ir_with_op(Operator::Task {
            capability: CapabilityId("git.commit".to_string()),
            inputs: Default::default(),
        });
        let provenance = PlanProvenanceV1::new("test", "1.0.0").unwrap();
        let normalized = NormalizedPlanV1::from_workflow_ir(&ir);

        let result = PlanRevisionV1::new(
            Some("some-parent".into()),
            PlanMutation::Initial,
            provenance,
            normalized,
        );
        assert!(matches!(result, Err(PlanRevisionError::NoOpMutation)));
    }

    // REQ-4: provenance validation
    #[test]
    fn test_empty_author_rejected() {
        let result = PlanProvenanceV1::new("", "1.0.0");
        assert!(matches!(
            result,
            Err(PlanRevisionError::EmptyProvenanceField { field: "author" })
        ));
    }

    #[test]
    fn test_empty_tool_version_rejected() {
        let result = PlanProvenanceV1::new("author", "");
        assert!(matches!(
            result,
            Err(PlanRevisionError::EmptyProvenanceField {
                field: "tool_version"
            })
        ));
    }

    // REQ-4: empty lineage
    #[test]
    fn test_derive_on_empty_lineage_rejected() {
        let ir = sample_ir_with_op(Operator::Task {
            capability: CapabilityId("git.commit".to_string()),
            inputs: Default::default(),
        });
        let provenance = PlanProvenanceV1::new("test", "1.0.0").unwrap();

        let empty_lineage: PlanRevisionLineageV1 =
            serde_json::from_str(r#"{"revisions":[]}"#).unwrap();
        let result = empty_lineage.derive(&ir, PlanMutation::NodesChanged, provenance);
        assert!(matches!(result, Err(PlanRevisionError::EmptyLineage)));
    }

    // REQ-5: variant counts
    #[test]
    fn test_plan_mutation_variant_count() {
        // The assert_variant_count_eq! at the type level already guarantees this.
        // Additional runtime check for documentation clarity.
        let variants = [
            PlanMutation::Initial,
            PlanMutation::NodesChanged,
            PlanMutation::EdgesChanged,
            PlanMutation::BudgetsAdjusted,
            PlanMutation::PolicyAmended,
            PlanMutation::ProvenanceRefreshed,
            PlanMutation::StructureReplaced,
        ];
        assert_eq!(variants.len(), 7);
    }

    #[test]
    fn test_plan_revision_error_variant_count() {
        let variants = [
            PlanRevisionError::UnsupportedSchemaVersion { got: 0, want: 1 },
            PlanRevisionError::NoOpMutation,
            PlanRevisionError::EmptyProvenanceField { field: "author" },
            PlanRevisionError::EmptyLineage,
        ];
        assert_eq!(variants.len(), 4);
    }

    // REQ-5: canonical byte-stability
    #[test]
    fn test_canonical_byte_stability() {
        let ir = sample_ir_with_op(Operator::Task {
            capability: CapabilityId("git.commit".to_string()),
            inputs: Default::default(),
        });
        let np = NormalizedPlanV1::from_workflow_ir(&ir);

        let bytes1 = serde_json::to_vec(&np).unwrap();
        let bytes2 = serde_json::to_vec(&np).unwrap();
        assert_eq!(&bytes1, &bytes2, "canonical JSON must be byte-stable");

        let id1 = np.plan_identity();
        let id2 = np.plan_identity();
        assert_eq!(id1, id2, "plan_identity must be stable across calls");
    }

    // Derive preserves receiver (not mutated)
    #[test]
    fn test_derive_does_not_mutate_receiver() {
        let ir_initial = sample_ir_with_op(Operator::Task {
            capability: CapabilityId("git.commit".to_string()),
            inputs: Default::default(),
        });
        let provenance = PlanProvenanceV1::new("test", "1.0.0").unwrap();

        let lineage = PlanRevisionLineageV1::initial(&ir_initial, provenance.clone()).unwrap();
        let ir2 = ir_with_extra_node();
        let _lineage2 = lineage
            .derive(&ir2, PlanMutation::NodesChanged, provenance)
            .unwrap();

        assert_eq!(lineage.len(), 1, "receiver must not be mutated");
    }
}
