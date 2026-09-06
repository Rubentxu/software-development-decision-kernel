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
use std::collections::BTreeSet;

use sha2::{Digest, Sha256};
use thiserror::Error;

use crate::graph::{EdgeSnapshot, ExecutionGraphRevision, NodeSnapshot};
use crate::operator_contract::variant_name;
use crate::plan_revision::{PlanMutation, PlanRevisionV1};
use crate::workflow_ir::{EdgeId, EventId, NodeId, Operator, OperatorId};

/// Compilation error — closed 8-variant enum (I-9).
#[derive(Debug, Clone, PartialEq, Eq, Error)]
#[error("execution graph compile error")]
pub enum ExecutionGraphCompileError {
    /// Unsupported schema version.
    #[error("unsupported schema version: got {got}, want {want}")]
    UnsupportedSchemaVersion {
        /// Schema version found.
        got: u32,
        /// Expected schema version.
        want: u32,
    },

    /// The plan mutation is not compatible with compilation.
    #[error("invalid plan mutation: {mutation}")]
    InvalidPlanMutation {
        /// The mutation that was rejected.
        mutation: String,
    },

    /// Operator tree depth exceeds the hard limit.
    #[error("depth exceeded: have {have}, max {max}")]
    DepthExceeded {
        /// Current depth.
        have: u64,
        /// Maximum allowed depth.
        max: u64,
    },

    /// Node count exceeds the hard limit.
    #[error("node count exceeded: have {have}, max {max}")]
    NodeCountExceeded {
        /// Current node count.
        have: u64,
        /// Maximum allowed nodes.
        max: u64,
    },

    /// An operator references an ID not present in the plan.
    #[error("orphan operator reference: operator `{operator_id}` references unknown `{child_id}`")]
    OrphanOperatorReference {
        /// The operator making the reference.
        operator_id: String,
        /// The referenced ID that does not exist.
        child_id: String,
    },

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

// ── Validation helpers (fail-closed) ─────────────────────────────────────────

/// S1: schema version must be 1.
fn validate_schema_version(plan: &PlanRevisionV1) -> Result<(), ExecutionGraphCompileError> {
    if plan.normalized.schema_version != 1 {
        return Err(ExecutionGraphCompileError::UnsupportedSchemaVersion {
            got: plan.normalized.schema_version,
            want: 1,
        });
    }
    Ok(())
}

/// S2: mutation must be compatible with compilation.
///
/// `StructureReplaced` is not applicable at the compile layer.
fn validate_mutation(plan: &PlanRevisionV1) -> Result<(), ExecutionGraphCompileError> {
    match plan.mutation {
        PlanMutation::Initial
        | PlanMutation::NodesChanged
        | PlanMutation::EdgesChanged
        | PlanMutation::BudgetsAdjusted
        | PlanMutation::PolicyAmended
        | PlanMutation::ProvenanceRefreshed => Ok(()),
        PlanMutation::StructureReplaced => Err(ExecutionGraphCompileError::InvalidPlanMutation {
            mutation: "StructureReplaced".to_string(),
        }),
    }
}

/// S3: plan must not be empty.
fn validate_non_empty(plan: &PlanRevisionV1) -> Result<(), ExecutionGraphCompileError> {
    if plan.normalized.nodes.is_empty() {
        return Err(ExecutionGraphCompileError::EmptyPlan);
    }
    Ok(())
}

/// S4: check budget limits (node count, depth).
fn validate_budgets(plan: &PlanRevisionV1) -> Result<(), ExecutionGraphCompileError> {
    let node_count = plan.normalized.nodes.len() as u64;
    let max_nodes = plan.normalized.budgets.max_nodes;

    if node_count > max_nodes {
        return Err(ExecutionGraphCompileError::NodeCountExceeded {
            have: node_count,
            max: max_nodes,
        });
    }

    // Depth check: walk the operator graph to measure depth
    let max_depth = plan.normalized.budgets.max_depth;
    if max_depth < u64::MAX {
        let depth = compute_max_depth(&plan.normalized.nodes);
        if depth > max_depth {
            return Err(ExecutionGraphCompileError::DepthExceeded {
                have: depth,
                max: max_depth,
            });
        }
    }

    Ok(())
}

/// Computes maximum operator graph depth via DFS from root nodes.
fn compute_max_depth(nodes: &BTreeMap<OperatorId, Operator>) -> u64 {
    if nodes.is_empty() {
        return 0;
    }

    // Find root nodes (operators never referenced by any other)
    let all_ids: BTreeSet<OperatorId> = nodes.keys().cloned().collect();
    let referenced: BTreeSet<OperatorId> = nodes
        .values()
        .flat_map(|op| op.referenced_ids())
        .collect();
    let roots: Vec<OperatorId> = all_ids.difference(&referenced).cloned().collect();

    let mut max_depth_found = 0u64;

    fn dfs_depth(
        nodes: &BTreeMap<OperatorId, Operator>,
        current: &OperatorId,
        visited: &mut BTreeSet<OperatorId>,
        current_depth: u64,
        max_depth_found: &mut u64,
    ) {
        if visited.contains(current) {
            return;
        }
        visited.insert(current.clone());
        if current_depth > *max_depth_found {
            *max_depth_found = current_depth;
        }
        if let Some(op) = nodes.get(current) {
            for child_id in op.referenced_ids() {
                if nodes.contains_key(&child_id) {
                    dfs_depth(nodes, &child_id, visited, current_depth + 1, max_depth_found);
                }
            }
        }
        visited.remove(current);
    }

    let mut visited = BTreeSet::new();
    for root in roots {
        dfs_depth(nodes, &root, &mut visited, 1, &mut max_depth_found);
    }

    max_depth_found
}

/// S5: no orphan references.
fn validate_no_orphans(plan: &PlanRevisionV1) -> Result<(), ExecutionGraphCompileError> {
    for (op_id, op) in &plan.normalized.nodes {
        for child_id in op.referenced_ids() {
            if !plan.normalized.nodes.contains_key(&child_id) {
                return Err(ExecutionGraphCompileError::OrphanOperatorReference {
                    operator_id: op_id.0.clone(),
                    child_id: child_id.0.clone(),
                });
            }
        }
    }
    Ok(())
}

/// S6: detect cycles using iterative DFS (white/grey/black coloring).
fn detect_cycles(plan: &PlanRevisionV1) -> Result<(), ExecutionGraphCompileError> {
    if plan.normalized.nodes.is_empty() {
        return Ok(());
    }

    // Find roots: operators never referenced by any other operator
    let all_ids: BTreeSet<OperatorId> = plan.normalized.nodes.keys().cloned().collect();
    let referenced_ids: BTreeSet<OperatorId> = plan
        .normalized
        .nodes
        .values()
        .flat_map(|op| op.referenced_ids())
        .collect();
    let roots: Vec<OperatorId> = all_ids.difference(&referenced_ids).cloned().collect();

    // If every operator is referenced (no roots) and there are operators, it's a cycle
    if roots.is_empty() && !plan.normalized.nodes.is_empty() {
        return Err(ExecutionGraphCompileError::CycleDetected);
    }

    // DFS with color marking: white=0, grey=1, black=2
    let mut color: BTreeMap<OperatorId, u8> = BTreeMap::new();
    let mut stack: Vec<OperatorId> = roots;

    while let Some(current) = stack.pop() {
        let c = color.entry(current.clone()).or_insert(0);
        if *c == 2 {
            continue;
        }
        if *c == 1 {
            return Err(ExecutionGraphCompileError::CycleDetected);
        }
        *c = 1;

        if let Some(op) = plan.normalized.nodes.get(&current) {
            for child_id in op.referenced_ids() {
                if plan.normalized.nodes.contains_key(&child_id) {
                    stack.push(child_id);
                }
            }
        }

        *color.get_mut(&current).unwrap() = 2;
    }

    Ok(())
}

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
    // ── Validation pipeline (fail-closed) ────────────────────────────────────
    // Run ALL checks before synthesising any state.

    // S1: check schema version
    validate_schema_version(plan)?;

    // S2: check mutation compatibility
    validate_mutation(plan)?;

    // S3: check non-empty (covers both EmptyPlan and EmptyLineage)
    validate_non_empty(plan)?;

    // S4: check budget limits
    validate_budgets(plan)?;

    // S5: check orphan references
    validate_no_orphans(plan)?;

    // S6: detect cycles
    detect_cycles(plan)?;

    // ── Happy path: synthesise ──────────────────────────────────────────────

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
            // Skip if child is not in the plan (orphan — validation handles this)
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
