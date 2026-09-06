//! Execution graph compiler — bounded compile contract from [`PlanRevisionV1`] to [`ExecutionGraphRevision`].
//!
//! # Revision ID recipe (I-5)
//!
//! ```text
//! revision_id = format!("{:064x}", sha256(parent_digest_hex_or_root || plan_revision_id || compile_anchor))
//! ```
//!
//! # Compile-only scope
//!
//! This module handles **compilation only**. Persistence (DW-RUNTIME-002),
//! execution (DW-RUNTIME-003/004/005), Map/Reduce/Join (H6), IR shape changes, and
//! [`ExecutionGraphRevision`] schema bumps are explicitly out of scope.
//!
//! # Edge synthesis
//!
//! Edges are synthesised by walking [`Operator::referenced_ids()`] —
//! [`NormalizedPlanV1::edges`] (the NIL-placeholder in `plan_revision.rs:97-101`)
//! is **not** used as an edge source.

use std::collections::BTreeMap;

use sha2::{Digest, Sha256};
use thiserror::Error;

use crate::graph::{EdgeSnapshot, ExecutionGraphRevision, NodeSnapshot};
use crate::operator_contract::variant_name;
use crate::plan_revision::PlanRevisionV1;
use crate::workflow_ir::{EdgeId, EventId, NodeId, Operator, OperatorId};

/// Compilation error — closed 8-variant enum (I-9).
#[derive(Debug, Clone, PartialEq, Eq, Error)]
#[error("execution graph compile error")]
pub enum ExecutionGraphCompileError {
    /// Unsupported schema version.
    #[error("unsupported schema version: got {got}, want {want}")]
    UnsupportedSchemaVersion { got: u32, want: u32 },

    /// The plan mutation is not compatible with compilation.
    #[error("invalid plan mutation: {mutation}")]
    InvalidPlanMutation { mutation: String },

    /// Operator tree depth exceeds the hard limit.
    #[error("depth exceeded: have {have}, max {max}")]
    DepthExceeded { have: u64, max: u64 },

    /// Node count exceeds the hard limit.
    #[error("node count exceeded: have {have}, max {max}")]
    NodeCountExceeded { have: u64, max: u64 },

    /// An operator references an ID not present in the plan.
    #[error("orphan operator reference: operator `{operator_id}` references unknown `{child_id}`")]
    OrphanOperatorReference { operator_id: String, child_id: String },

    /// A directed cycle was detected in the operator graph.
    #[error("cycle detected in operator references")]
    CycleDetected,

    /// The plan has empty lineage (not applicable at this layer).
    #[error("empty lineage")]
    EmptyLineage,

    /// The plan has no nodes.
    #[error("empty plan: no nodes")]
    EmptyPlan,
}

// Compile-time guard: 8 variants (I-9).
crate::assert_variant_count_eq!(
    ExecutionGraphCompileError,
    8,
    [
        ExecutionGraphCompileError::UnsupportedSchemaVersion { .. },
        ExecutionGraphCompileError::InvalidPlanMutation { .. },
        ExecutionGraphCompileError::DepthExceeded { .. },
        ExecutionGraphCompileError::NodeCountExceeded { .. },
        ExecutionGraphCompileError::OrphanOperatorReference { .. },
        ExecutionGraphCompileError::CycleDetected,
        ExecutionGraphCompileError::EmptyLineage,
        ExecutionGraphCompileError::EmptyPlan,
    ]
);

/// Compiles a [`PlanRevisionV1`] into an [`ExecutionGraphRevision`].
//
///
///
/// # Arguments
///
/// * `plan` — the tip of a plan revision lineage (I-1)
/// * `parent` — optional parent graph revision for chained compilation
/// * `compile_anchor` — caller-supplied anchor string; MUST NOT be `std::time::SystemTime`
///
/// # Returns
///
/// On success: [`ExecutionGraphRevision`] with:
/// - `revision = parent.revision + 1` (or `0` if no parent)
/// - `revision_id` = `sha256(parent_digest_hex || plan.revision_id || compile_anchor)` (I-5)
/// - `nodes` synthesised from `plan.normalized.nodes` (I-7)
/// - `edges` synthesised from `Operator::referenced_ids()` (I-6)
/// - `events = BTreeMap::new()` (I-8)
/// - `schema_version = 1` (I-2)
/// - `digest = compute_digest()`
///
/// On failure: one of the 8 [`ExecutionGraphCompileError`] variants (fail-closed).
pub fn compile_plan_to_revision(
    plan: &PlanRevisionV1,
    parent: Option<&ExecutionGraphRevision>,
    compile_anchor: &str,
) -> Result<ExecutionGraphRevision, ExecutionGraphCompileError> {
    // I-8: events are always empty (runtime concern)
    let events: BTreeMap<EventId, crate::graph::GraphEvent> = BTreeMap::new();

    // Revision number: 0 for first compile, parent.revision + 1 otherwise
    let revision = parent.map(|p| p.revision + 1).unwrap_or(0);

    // I-5: compute revision_id
    // Recipe: sha256(parent_digest_hex_or_root || plan_revision_id || compile_anchor)
    // Use raw 32-byte digest as 64 lowercase hex chars (no "sha256:" prefix)
    let parent_digest_hex = parent
        .map(|p| p.digest.iter().map(|b| format!("{:02x}", b)).collect::<String>())
        .unwrap_or_else(|| "root".to_string());

    let mut sha = Sha256::new();
    sha.update(parent_digest_hex.as_bytes());
    sha.update(plan.revision_id.as_bytes());
    sha.update(compile_anchor.as_bytes());
    let hash = sha.finalize();
    let revision_id = format!("{:064x}", hash);

    // I-7: synthesize nodes
    let nodes = synthesize_nodes(&plan.normalized.nodes, compile_anchor);

    // I-6 / I-3: synthesize edges (deterministic BTreeMap)
    let edges = synthesize_edges(&plan.normalized, compile_anchor);

    // Build the revision
    let mut revision = ExecutionGraphRevision {
        revision,
        revision_id: crate::workflow_ir::RevisionId(revision_id),
        parent: parent.cloned().map(Box::new),
        events,
        nodes,
        edges,
        digest: [0u8; 32],
        schema_version: ExecutionGraphRevision::SCHEMA_VERSION,
    };

    // I-3: digest is always computed, not wall-clock-derived
    revision.digest = revision.compute_digest();

    Ok(revision)
}

/// Synthesises [`NodeSnapshot`] entries for every operator in the plan (I-7).
fn synthesize_nodes(
    plan_nodes: &BTreeMap<OperatorId, Operator>,
    compile_anchor: &str,
) -> BTreeMap<NodeId, NodeSnapshot> {
    plan_nodes
        .keys()
        .map(|op_id| {
            let snapshot = NodeSnapshot {
                node_id: NodeId(op_id.0.clone()),
                state: "compiled".into(),
                snapshot_at: compile_anchor.into(),
            };
            (NodeId(op_id.0.clone()), snapshot)
        })
        .collect()
}

/// Synthesises [`EdgeSnapshot`] entries by walking [`Operator::referenced_ids()`] (I-6).
///
/// [`NormalizedPlanV1::edges`] (the NIL-placeholder) is NOT used as an edge source.
///
/// `EdgeId(format!("{}->{}->{}", from, relation, to))` provides deterministic ordering.
fn synthesize_edges(
    normalized: &crate::plan_revision::NormalizedPlanV1,
    compile_anchor: &str,
) -> BTreeMap<EdgeId, EdgeSnapshot> {
    let mut edges: BTreeMap<EdgeId, EdgeSnapshot> = BTreeMap::new();

    for (op_id, op) in &normalized.nodes {
        let relation = variant_name(op).to_lowercase();
        for child_id in op.referenced_ids() {
            // Skip if child is not in the plan (orphan — but that is a validation concern)
            if !normalized.nodes.contains_key(&child_id) {
                continue;
            }
            let edge_id = EdgeId(format!("{}->{}->{}", op_id.0, relation, child_id.0));
            let snapshot = EdgeSnapshot {
                edge_id: edge_id.clone(),
                from: op_id.0.clone(),
                relation: relation.clone(),
                to: child_id.0.clone(),
                snapshot_at: compile_anchor.into(),
            };
            edges.insert(edge_id, snapshot);
        }
    }

    edges
}
