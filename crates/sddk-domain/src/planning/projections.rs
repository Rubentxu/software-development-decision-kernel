//! Decision-plane roadmap projections over a `RoadmapSnapshot`.
//!
//! Implements PLN-LEDGER-004 AC-PLN4-05/06/07/08/09 per spec §3.2 Q1-Q6.
//!
//! Five pure projection functions over `RoadmapSnapshot`:
//! - `project_status`: horizon partition + active_item + counts
//! - `project_next`: 8-clause selection rule verbatim
//! - `project_blocked`: Blocked vs PromotionBlocked partition
//! - `project_show`: work-item-id lookup with execution_evidence
//! - `project_graph`: JSON + Dot + Mermaid renderings
//!
//! Kahn's algorithm is used for cycle detection (consistent with
//! `DependencyResolutionService` per ADR-073 §3.2).

use std::collections::{BTreeMap, HashSet};

use serde::{Deserialize, Serialize};

use crate::planning::{CycleId, WorkItemId, WorkItemStatus};
use crate::StorageError;

// ── RoadmapProjectionError ─────────────────────────────────────────────────

/// Errors from roadmap projections.
///
/// Seven variants covering all failure modes from spec AC-PLN4-03/05/06/07/08:
/// - `LedgerNotImported`: no spine imported yet
/// - `MultipleActiveWorkItems`: >1 ACTIVE item (fail-closed per selection_rule clause 2)
/// - `SpineComplete`: all items are terminal
/// - `UnknownWorkItem`: work item id not found
/// - `PromotionBlocked`: PROPOSED item missing exit_gate
/// - `LineStopped`: BLOCKED item blocks the line
/// - `DependencyCycle`: cycle detected in dependency graph
#[derive(Debug, Clone, thiserror::Error)]
pub enum RoadmapProjectionError {
    /// No spine has been imported yet.
    #[error("ledger not imported: run `sddk plan import` first")]
    LedgerNotImported,

    /// More than one ACTIVE work item found (fail-closed per selection_rule clause 2).
    #[error("multiple active work items: {ids:?}")]
    MultipleActiveWorkItems { ids: Vec<WorkItemId> },

    /// All work items are in terminal status (SHIPPED/ABSORBED/SUPERSEDED).
    #[error("spine is complete: all items are terminal")]
    SpineComplete,

    /// Unknown work item id.
    #[error("unknown work item: {id}")]
    UnknownWorkItem { id: WorkItemId },

    /// PROPOSED work item missing exit_gate cannot be promoted.
    #[error("promotion blocked: {item_id} missing {missing}")]
    PromotionBlocked { item_id: WorkItemId, missing: &'static str },

    /// BLOCKED item stops the line.
    #[error("line stopped: {blocker} is blocked")]
    LineStopped { blocker: WorkItemId, reason: &'static str },

    /// Dependency cycle detected in the roadmap graph.
    ///
    /// Path is in canonical form: [A, B, C, A] where A→B→C→A.
    #[error("dependency cycle: {path:?}")]
    DependencyCycle { path: Vec<WorkItemId> },
}

// ── Spine metadata ───────────────────────────────────────────────────────────

/// Spine horizon variants (H0..H12).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum SpineHorizon {
    H0, H1, H2, H3, H4, H5, H6, H7, H8, H9, H10, H11, H12,
}

impl SpineHorizon {
    /// Parses a string like "h0" or "H0" into SpineHorizon.
    pub fn parse(s: &str) -> Option<Self> {
        match s.to_lowercase().as_str() {
            "h0" => Some(SpineHorizon::H0),
            "h1" => Some(SpineHorizon::H1),
            "h2" => Some(SpineHorizon::H2),
            "h3" => Some(SpineHorizon::H3),
            "h4" => Some(SpineHorizon::H4),
            "h5" => Some(SpineHorizon::H5),
            "h6" => Some(SpineHorizon::H6),
            "h7" => Some(SpineHorizon::H7),
            "h8" => Some(SpineHorizon::H8),
            "h9" => Some(SpineHorizon::H9),
            "h10" => Some(SpineHorizon::H10),
            "h11" => Some(SpineHorizon::H11),
            "h12" => Some(SpineHorizon::H12),
            _ => None,
        }
    }
}

/// Terminal status vocabulary (per EXECUTION-SPINE.yaml baseline).
fn is_terminal_status(status: &WorkItemStatus) -> bool {
    matches!(status, WorkItemStatus::Done | WorkItemStatus::Superseded | WorkItemStatus::Cancelled)
}

/// Returns true if the spine status is PROPOSED (promotion check applies).
fn is_proposed_spine_status(spine_status: Option<&str>) -> bool {
    spine_status == Some("proposed")
}

// ── RoadmapSnapshot ────────────────────────────────────────────────────────

/// A read-only snapshot of the planning ledger for projection computation.
///
/// Obtained from `RoadmapGraphRead::snapshot()`.
#[derive(Debug, Clone)]
pub struct RoadmapSnapshot {
    /// All work items indexed by id.
    pub work_items: BTreeMap<WorkItemId, WorkItemSnapshot>,
    /// All dependency edges.
    pub edges: Vec<DependencyEdgeSnapshot>,
    /// Bound cycle IDs from reconciliation (for execution_evidence).
    pub bound_cycles: Vec<CycleId>,
}

/// Single work item snapshot for projections.
#[derive(Debug, Clone)]
pub struct WorkItemSnapshot {
    pub id: WorkItemId,
    pub cycle_id: CycleId,
    pub title: String,
    pub description: String,
    pub status: WorkItemStatus,
    /// Spine metadata (populated from spine import).
    pub spine_order: Option<i32>,
    pub spine_horizon: Option<String>,
    pub spine_status: Option<String>,
    pub exit_gate: Option<String>,
    /// Incoming dependency edges (items that depend on this one).
    pub blocks: Vec<WorkItemId>,
}

/// Single dependency edge snapshot.
#[derive(Debug, Clone)]
pub struct DependencyEdgeSnapshot {
    pub from_id: WorkItemId,
    pub to_id: WorkItemId,
    pub kind: DependencyEdgeKindSnapshot,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum DependencyEdgeKindSnapshot {
    Blocks,
    BlocksOnClosure,
}

// ── Kahn's cycle detection ─────────────────────────────────────────────────

/// Detects cycles using Kahn's algorithm.
///
/// Returns `Ok(())` if the graph is acyclic.
/// Returns `Err(path)` if a cycle is found, where path is the cyclic path
/// in canonical form [A, B, C, A].
///
/// # Arguments
/// * `items` - all work items indexed by id
/// * `edges` - all dependency edges
///
/// # Algorithm
/// Kahn's algorithm: remove all nodes with zero in-degree iteratively.
/// If any nodes remain, they form a cycle.
pub fn detect_cycle(
    items: &BTreeMap<WorkItemId, WorkItemSnapshot>,
    edges: &[DependencyEdgeSnapshot],
) -> Result<(), Vec<WorkItemId>> {
    // Build in-degree map
    let mut in_degree: BTreeMap<WorkItemId, usize> = items
        .keys()
        .map(|id| (id.clone(), 0))
        .collect();

    for edge in edges {
        if edge.kind == DependencyEdgeKindSnapshot::Blocks
            || edge.kind == DependencyEdgeKindSnapshot::BlocksOnClosure
        {
            // edge.from blocks edge.to (to depends on from)
            if let Some(count) = in_degree.get_mut(&edge.to_id) {
                *count += 1;
            }
        }
    }

    // Start with all nodes that have no incoming edges (in_degree = 0)
    let mut queue: Vec<WorkItemId> = in_degree
        .iter()
        .filter(|(_, d)| **d == 0)
        .map(|(id, _)| id.clone())
        .collect();

    let mut removed_count = 0;

    while let Some(node_id) = queue.pop() {
        removed_count += 1;

        // Reduce in-degree of neighbors
        for edge in edges {
            if edge.from_id == node_id {
                if let Some(count) = in_degree.get_mut(&edge.to_id) {
                    *count = count.saturating_sub(1);
                    if *count == 0 {
                        queue.push(edge.to_id.clone());
                    }
                }
            }
        }
    }

    // If not all nodes were removed, there's a cycle
    if removed_count < items.len() {
        // Find the cycle by taking any remaining node and following edges
        let remaining: Vec<WorkItemId> = in_degree
            .iter()
            .filter(|(_, d)| **d > 0)
            .map(|(id, _)| id.clone())
            .collect();

        if let Some(start) = remaining.first() {
            // Extract the cycle path by DFS
            let cycle_path = extract_cycle_path(start, edges);
            return Err(cycle_path);
        }
    }

    Ok(())
}

/// Extracts a cycle path starting from the given node using DFS.
fn extract_cycle_path(start: &WorkItemId, edges: &[DependencyEdgeSnapshot]) -> Vec<WorkItemId> {
    let mut visited: HashSet<WorkItemId> = HashSet::new();
    let mut path: Vec<WorkItemId> = Vec::new();
    let mut stack: Vec<WorkItemId> = vec![start.clone()];

    while let Some(current) = stack.pop() {
        if visited.contains(&current) {
            // Found cycle - extract from first occurrence to current
            if let Some(pos) = path.iter().position(|id| id == &current) {
                let mut cycle: Vec<WorkItemId> = path[pos..].to_vec();
                cycle.push(current.clone());
                return cycle;
            }
            continue;
        }

        visited.insert(current.clone());
        path.push(current.clone());

        // Find neighbors (items that current blocks)
        for edge in edges {
            if edge.from_id == current {
                stack.push(edge.to_id.clone());
            }
        }
    }

    // Fallback: return a simple cycle
    vec![start.clone()]
}

// ── StatusProjection ───────────────────────────────────────────────────────

/// Output of `project_status`.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StatusProjection {
    /// The unique ACTIVE work item, or None.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub active_item: Option<WorkItemId>,
    /// Per-horizon statistics.
    pub per_horizon: BTreeMap<String, HorizonStats>,
    /// Total terminal items.
    pub terminal_count: usize,
    /// Total executable (non-terminal) items.
    pub executable_count: usize,
    /// Total non-executable items.
    pub non_executable_count: usize,
}

/// Statistics for a single horizon.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HorizonStats {
    /// Total items in this horizon.
    pub total: usize,
    /// Terminal items in this horizon.
    pub terminal: usize,
    /// Executable items in this horizon.
    pub executable: usize,
    /// Non-executable items in this horizon.
    pub non_executable: usize,
}

/// Computes the status projection.
///
/// Returns `Err(LedgerNotImported)` if no items exist.
/// Returns `Err(MultipleActiveWorkItems)` if >1 ACTIVE item found (fail-closed).
pub fn project_status(snapshot: &RoadmapSnapshot) -> Result<StatusProjection, RoadmapProjectionError> {
    if snapshot.work_items.is_empty() {
        return Err(RoadmapProjectionError::LedgerNotImported);
    }

    // Find ACTIVE items
    let active_items: Vec<WorkItemId> = snapshot
        .work_items
        .values()
        .filter(|wi| wi.status == WorkItemStatus::Active)
        .map(|wi| wi.id.clone())
        .collect();

    if active_items.len() > 1 {
        return Err(RoadmapProjectionError::MultipleActiveWorkItems {
            ids: active_items,
        });
    }

    let active_item = active_items.into_iter().next();

    // Per-horizon partition
    let mut per_horizon: BTreeMap<String, HorizonStats> = BTreeMap::new();
    let mut terminal_count = 0usize;
    let mut executable_count = 0usize;
    let mut non_executable_count = 0usize;

    for wi in snapshot.work_items.values() {
        let horizon = wi.spine_horizon.clone().unwrap_or_else(|| "unknown".to_string());

        let stats = per_horizon.entry(horizon.clone()).or_insert(HorizonStats {
            total: 0,
            terminal: 0,
            executable: 0,
            non_executable: 0,
        });

        stats.total += 1;

        if is_terminal_status(&wi.status) {
            stats.terminal += 1;
            terminal_count += 1;
        } else if wi.status == WorkItemStatus::Active
            || wi.status == WorkItemStatus::Draft
            || wi.status == WorkItemStatus::Paused
        {
            stats.executable += 1;
            executable_count += 1;
        } else {
            stats.non_executable += 1;
            non_executable_count += 1;
        }
    }

    Ok(StatusProjection {
        active_item,
        per_horizon,
        terminal_count,
        executable_count,
        non_executable_count,
    })
}

// ── NextProjection ──────────────────────────────────────────────────────────

/// Output of `project_next`.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NextProjection {
    pub item_id: WorkItemId,
    pub reason: String,
    pub promotion: NextPromotion,
}

/// Promotion action for `project_next`.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum NextPromotion {
    /// Item is already ACTIVE — no promotion needed.
    Identity,
    /// PROPOSED item with exit_gate → promote to READY.
    PromoteToReady,
    /// No promotion applicable (spine complete or no eligible item).
    NA,
}

/// Computes `project_next` per the 8-clause selection rule.
///
/// Clause 1: exactly one ACTIVE → resume it
/// Clause 2: >1 ACTIVE → fail closed
/// Clause 3: 0 ACTIVE → scan by ascending spine_order
/// Clause 4: first non-terminal whose deps are all terminal
/// Clause 5a: PROPOSED + exit_gate → promote to READY
/// Clause 5b: PROPOSED + missing exit_gate → PromotionBlocked
/// Clause 6: BLOCKED → LineStopped
/// Clause 7+8: binding + completion out of scope
pub fn project_next(snapshot: &RoadmapSnapshot) -> Result<NextProjection, RoadmapProjectionError> {
    if snapshot.work_items.is_empty() {
        return Err(RoadmapProjectionError::LedgerNotImported);
    }

    // Clause 1+2: Check for ACTIVE items
    let active_items: Vec<&WorkItemSnapshot> = snapshot
        .work_items
        .values()
        .filter(|wi| wi.status == WorkItemStatus::Active)
        .collect();

    if active_items.len() > 1 {
        return Err(RoadmapProjectionError::MultipleActiveWorkItems {
            ids: active_items.iter().map(|wi| wi.id.clone()).collect(),
        });
    }

    if let Some(active) = active_items.first() {
        // Clause 1: exactly one ACTIVE → resume it
        return Ok(NextProjection {
            item_id: active.id.clone(),
            reason: "resume_active".to_string(),
            promotion: NextPromotion::Identity,
        });
    }

    // Clause 3: 0 ACTIVE → scan by ascending spine_order
    let mut ordered_items: Vec<&WorkItemSnapshot> = snapshot.work_items.values().collect();
    ordered_items.sort_by_key(|wi| wi.spine_order.unwrap_or(i32::MAX));

    // Build dependency map: item → set of items it depends on
    let deps_map: BTreeMap<WorkItemId, HashSet<WorkItemId>> = {
        let mut map: BTreeMap<WorkItemId, HashSet<WorkItemId>> = BTreeMap::new();
        for wi in snapshot.work_items.values() {
            let deps: HashSet<WorkItemId> = snapshot
                .edges
                .iter()
                .filter(|e| e.to_id == wi.id)
                .map(|e| e.from_id.clone())
                .collect();
            map.insert(wi.id.clone(), deps);
        }
        map
    };

    // Find first non-terminal whose deps are all terminal
    for wi in &ordered_items {
        if is_terminal_status(&wi.status) {
            continue; // Skip terminal items
        }

        // Check if all dependencies are terminal
        if let Some(deps) = deps_map.get(&wi.id) {
            let all_deps_terminal = deps.iter().all(|dep_id| {
                snapshot
                    .work_items
                    .get(dep_id)
                    .map(|dep| is_terminal_status(&dep.status))
                    .unwrap_or(false)
            });

            if !all_deps_terminal {
                continue; // Has non-terminal dependencies
            }
        }

        // This item is eligible — check promotion conditions
        // Clause 5a: PROPOSED + exit_gate → promote to READY
        if wi.status == WorkItemStatus::Draft && is_proposed_spine_status(wi.spine_status.as_deref()) {
            if wi.exit_gate.is_some() && !wi.exit_gate.as_ref().unwrap().is_empty() {
                return Ok(NextProjection {
                    item_id: wi.id.clone(),
                    reason: "first_non_terminal_all_deps_terminal".to_string(),
                    promotion: NextPromotion::PromoteToReady,
                });
            } else {
                // Clause 5b: PROPOSED + missing exit_gate → PromotionBlocked
                return Err(RoadmapProjectionError::PromotionBlocked {
                    item_id: wi.id.clone(),
                    missing: "exit_gate",
                });
            }
        }

        // Clause 6: BLOCKED → LineStopped
        if wi.status == WorkItemStatus::Paused {
            return Err(RoadmapProjectionError::LineStopped {
                blocker: wi.id.clone(),
                reason: "blocked",
            });
        }

        // Default: return this item
        return Ok(NextProjection {
            item_id: wi.id.clone(),
            reason: "first_non_terminal_all_deps_terminal".to_string(),
            promotion: NextPromotion::NA,
        });
    }

    // No eligible item found → spine is complete
    Err(RoadmapProjectionError::SpineComplete)
}

// ── BlockedProjection ───────────────────────────────────────────────────────

/// Output of `project_blocked`.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BlockedProjection {
    /// Items blocked by non-terminal dependencies.
    pub blocked: Vec<BlockedItem>,
    /// Items blocked by missing exit_gate (PROPOSED without acceptance contract).
    pub promotion_blocked: Vec<PromotionBlockedItem>,
}

/// A blocked item with its blocker(s).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BlockedItem {
    pub item_id: WorkItemId,
    pub spine_order: i32,
    pub blockers: Vec<WorkItemId>,
}

/// A promotion-blocked item (missing exit_gate).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PromotionBlockedItem {
    pub item_id: WorkItemId,
    pub spine_order: i32,
    pub missing: String,
}

/// Computes `project_blocked`.
///
/// Kahn's cycle detection runs first.
/// Returns `Err(DependencyCycle)` if a cycle is found.
/// Otherwise returns blocked items sorted by spine_order ASC.
pub fn project_blocked(
    snapshot: &RoadmapSnapshot,
) -> Result<BlockedProjection, RoadmapProjectionError> {
    if snapshot.work_items.is_empty() {
        return Err(RoadmapProjectionError::LedgerNotImported);
    }

    // Kahn's cycle detection first (per spec AC-PLN4-07)
    if let Err(cycle_path) = detect_cycle(&snapshot.work_items, &snapshot.edges) {
        return Err(RoadmapProjectionError::DependencyCycle { path: cycle_path });
    }

    // Build incoming edge map: item → blockers (items that block it)
    let blockers_map: BTreeMap<WorkItemId, Vec<WorkItemId>> = {
        let mut map: BTreeMap<WorkItemId, Vec<WorkItemId>> = BTreeMap::new();
        for edge in &snapshot.edges {
            if edge.kind == DependencyEdgeKindSnapshot::Blocks
                || edge.kind == DependencyEdgeKindSnapshot::BlocksOnClosure
            {
                map.entry(edge.to_id.clone())
                    .or_default()
                    .push(edge.from_id.clone());
            }
        }
        map
    };

    let mut blocked: Vec<BlockedItem> = Vec::new();
    let mut promotion_blocked: Vec<PromotionBlockedItem> = Vec::new();

    for wi in snapshot.work_items.values() {
        if is_terminal_status(&wi.status) {
            continue; // Skip terminal items
        }

        // Check for missing exit_gate on PROPOSED items
        if wi.status == WorkItemStatus::Draft && is_proposed_spine_status(wi.spine_status.as_deref()) {
            if wi.exit_gate.is_none() || wi.exit_gate.as_ref().unwrap().is_empty() {
                promotion_blocked.push(PromotionBlockedItem {
                    item_id: wi.id.clone(),
                    spine_order: wi.spine_order.unwrap_or(0),
                    missing: "exit_gate".to_string(),
                });
                continue;
            }
        }

        // Check for non-terminal blockers
        if let Some(blockers) = blockers_map.get(&wi.id) {
            let non_terminal_blockers: Vec<WorkItemId> = blockers
                .iter()
                .filter(|bid| {
                    snapshot
                        .work_items
                        .get(bid.as_str())
                        .map(|b| !is_terminal_status(&b.status))
                        .unwrap_or(false)
                })
                .cloned()
                .collect();

            if !non_terminal_blockers.is_empty() {
                blocked.push(BlockedItem {
                    item_id: wi.id.clone(),
                    spine_order: wi.spine_order.unwrap_or(0),
                    blockers: non_terminal_blockers,
                });
            }
        }
    }

    // Sort by spine_order ASC
    blocked.sort_by_key(|b| b.spine_order);
    promotion_blocked.sort_by_key(|p| p.spine_order);

    Ok(BlockedProjection {
        blocked,
        promotion_blocked,
    })
}

// ── ShowProjection ─────────────────────────────────────────────────────────

/// Output of `project_show`.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ShowProjection {
    pub id: WorkItemId,
    pub order: i32,
    pub horizon: String,
    pub spine_status: String,
    pub exit_gate: Option<String>,
    pub depends_on: Vec<WorkItemId>,
    pub blocks: Vec<WorkItemId>,
    pub execution_evidence: Vec<CycleId>,
}

/// Computes `project_show` for a work item by id.
///
/// Returns `Err(UnknownWorkItem)` if the id is not found.
/// Returns `Err(UnknownWorkItem)` if a cycle id is passed (cycle IDs are not primary keys).
pub fn project_show(
    snapshot: &RoadmapSnapshot,
    work_item_id: &str,
) -> Result<ShowProjection, RoadmapProjectionError> {
    // Cycle IDs are not work item IDs — reject cycle id lookups
    if work_item_id.starts_with("p-") || work_item_id.contains("/pln-") {
        return Err(RoadmapProjectionError::UnknownWorkItem {
            id: work_item_id.to_string(),
        });
    }

    let wi = snapshot
        .work_items
        .get(work_item_id)
        .ok_or(RoadmapProjectionError::UnknownWorkItem {
            id: work_item_id.to_string(),
        })?;

    // Get outgoing edges (what this item depends on)
    let depends_on: Vec<WorkItemId> = snapshot
        .edges
        .iter()
        .filter(|e| e.to_id == wi.id)
        .map(|e| e.from_id.clone())
        .collect();

    // Get incoming edges (what this item blocks)
    let blocks: Vec<WorkItemId> = snapshot
        .edges
        .iter()
        .filter(|e| e.from_id == wi.id)
        .map(|e| e.to_id.clone())
        .collect();

    // Build execution evidence from bound cycles
    // An item's execution_evidence is the list of cycles that have shipped/closed
    // with this item in the spine
    let execution_evidence: Vec<CycleId> = snapshot
        .bound_cycles
        .iter()
        .filter(|c| c.contains(&wi.id))
        .cloned()
        .collect();

    Ok(ShowProjection {
        id: wi.id.clone(),
        order: wi.spine_order.unwrap_or(0),
        horizon: wi.spine_horizon.clone().unwrap_or_default(),
        spine_status: wi.spine_status.clone().unwrap_or_default(),
        exit_gate: wi.exit_gate.clone(),
        depends_on,
        blocks,
        execution_evidence,
    })
}

// ── GraphProjection ─────────────────────────────────────────────────────────

/// Output format for `project_graph`.
#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum GraphFormat {
    /// Canonical JSON form (determinism-tested).
    Json,
    /// DOT graphviz form.
    Dot,
    /// Mermaid diagram form.
    Mermaid,
}

/// Computes `project_graph` in the requested format.
///
/// Returns `Err(DependencyCycle)` if a cycle is found (graph cannot be emitted).
pub fn project_graph(
    snapshot: &RoadmapSnapshot,
    format: GraphFormat,
) -> Result<String, RoadmapProjectionError> {
    if snapshot.work_items.is_empty() {
        return Err(RoadmapProjectionError::LedgerNotImported);
    }

    // Cycle detection first
    if let Err(cycle_path) = detect_cycle(&snapshot.work_items, &snapshot.edges) {
        return Err(RoadmapProjectionError::DependencyCycle { path: cycle_path });
    }

    match format {
        GraphFormat::Json => graph_to_json(snapshot),
        GraphFormat::Dot => graph_to_dot(snapshot),
        GraphFormat::Mermaid => graph_to_mermaid(snapshot),
    }
}

/// Emits the graph as canonical JSON.
fn graph_to_json(snapshot: &RoadmapSnapshot) -> Result<String, RoadmapProjectionError> {
    // Sort edges by source spine_order for deterministic output
    let mut edges: Vec<&DependencyEdgeSnapshot> = snapshot.edges.iter().collect();
    edges.sort_by_key(|e| {
        snapshot
            .work_items
            .get(&e.from_id)
            .and_then(|wi| wi.spine_order)
            .unwrap_or(i32::MAX)
    });

    #[derive(Serialize)]
    struct GraphJson {
        nodes: Vec<NodeJson>,
        edges: Vec<EdgeJson>,
    }

    #[derive(Serialize)]
    struct NodeJson {
        id: String,
        title: String,
        status: String,
        horizon: String,
        order: i32,
    }

    #[derive(Serialize)]
    struct EdgeJson {
        from: String,
        to: String,
    }

    let nodes: Vec<NodeJson> = snapshot
        .work_items
        .values()
        .map(|wi| NodeJson {
            id: wi.id.clone(),
            title: wi.title.clone(),
            status: serde_json::to_string(&wi.status).unwrap(),
            horizon: wi.spine_horizon.clone().unwrap_or_else(|| "unknown".to_string()),
            order: wi.spine_order.unwrap_or(0),
        })
        .collect();

    let edges: Vec<EdgeJson> = edges
        .iter()
        .map(|e| EdgeJson {
            from: e.from_id.clone(),
            to: e.to_id.clone(),
        })
        .collect();

    let graph = GraphJson { nodes, edges };
    serde_json::to_string_pretty(&graph)
        .map_err(|e| RoadmapProjectionError::LedgerNotImported) // TODO: better error
}

/// Emits the graph as DOT format.
fn graph_to_dot(snapshot: &RoadmapSnapshot) -> Result<String, RoadmapProjectionError> {
    let mut output = String::from("digraph spine {\n");

    // Sort edges by source spine_order for deterministic output
    let mut edges: Vec<&DependencyEdgeSnapshot> = snapshot.edges.iter().collect();
    edges.sort_by_key(|e| {
        snapshot
            .work_items
            .get(&e.from_id)
            .and_then(|wi| wi.spine_order)
            .unwrap_or(i32::MAX)
    });

    for edge in &edges {
        output.push_str(&format!("    {} -> {};\n", edge.from_id, edge.to_id));
    }

    output.push_str("}\n");
    Ok(output)
}

/// Emits the graph as Mermaid format.
fn graph_to_mermaid(snapshot: &RoadmapSnapshot) -> Result<String, RoadmapProjectionError> {
    let mut output = String::from("```mermaid\ngraph TD\n");

    // Sort edges by source spine_order for deterministic output
    let mut edges: Vec<&DependencyEdgeSnapshot> = snapshot.edges.iter().collect();
    edges.sort_by_key(|e| {
        snapshot
            .work_items
            .get(&e.from_id)
            .and_then(|wi| wi.spine_order)
            .unwrap_or(i32::MAX)
    });

    for edge in &edges {
        output.push_str(&format!("    {} --> {}\n", edge.from_id, edge.to_id));
    }

    output.push_str("```\n");
    Ok(output)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn detect_cycle_empty() {
        let items: BTreeMap<WorkItemId, WorkItemSnapshot> = BTreeMap::new();
        let edges: Vec<DependencyEdgeSnapshot> = vec![];
        assert!(detect_cycle(&items, &edges).is_ok());
    }

    #[test]
    fn detect_cycle_linear() {
        let mut items: BTreeMap<WorkItemId, WorkItemSnapshot> = BTreeMap::new();
        items.insert(
            "A".to_string(),
            WorkItemSnapshot {
                id: "A".to_string(),
                cycle_id: "c".to_string(),
                title: "A".to_string(),
                description: "".to_string(),
                status: WorkItemStatus::Done,
                spine_order: Some(1),
                spine_horizon: Some("h1".to_string()),
                spine_status: Some("shipped".to_string()),
                exit_gate: None,
                blocks: vec![],
            },
        );
        items.insert(
            "B".to_string(),
            WorkItemSnapshot {
                id: "B".to_string(),
                cycle_id: "c".to_string(),
                title: "B".to_string(),
                description: "".to_string(),
                status: WorkItemStatus::Draft,
                spine_order: Some(2),
                spine_horizon: Some("h1".to_string()),
                spine_status: Some("proposed".to_string()),
                exit_gate: Some("done".to_string()),
                blocks: vec![],
            },
        );

        let edges = vec![DependencyEdgeSnapshot {
            from_id: "B".to_string(),
            to_id: "A".to_string(),
            kind: DependencyEdgeKindSnapshot::Blocks,
        }];

        assert!(detect_cycle(&items, &edges).is_ok());
    }

    #[test]
    fn detect_cycle_two_node() {
        let mut items: BTreeMap<WorkItemId, WorkItemSnapshot> = BTreeMap::new();
        items.insert(
            "A".to_string(),
            WorkItemSnapshot {
                id: "A".to_string(),
                cycle_id: "c".to_string(),
                title: "A".to_string(),
                description: "".to_string(),
                status: WorkItemStatus::Draft,
                spine_order: Some(1),
                spine_horizon: Some("h1".to_string()),
                spine_status: Some("proposed".to_string()),
                exit_gate: Some("done".to_string()),
                blocks: vec![],
            },
        );
        items.insert(
            "B".to_string(),
            WorkItemSnapshot {
                id: "B".to_string(),
                cycle_id: "c".to_string(),
                title: "B".to_string(),
                description: "".to_string(),
                status: WorkItemStatus::Draft,
                spine_order: Some(2),
                spine_horizon: Some("h1".to_string()),
                spine_status: Some("proposed".to_string()),
                exit_gate: Some("done".to_string()),
                blocks: vec![],
            },
        );

        // A→B and B→A
        let edges = vec![
            DependencyEdgeSnapshot {
                from_id: "A".to_string(),
                to_id: "B".to_string(),
                kind: DependencyEdgeKindSnapshot::Blocks,
            },
            DependencyEdgeSnapshot {
                from_id: "B".to_string(),
                to_id: "A".to_string(),
                kind: DependencyEdgeKindSnapshot::Blocks,
            },
        ];

        let result = detect_cycle(&items, &edges);
        assert!(result.is_err());
        let path = result.unwrap_err();
        assert_eq!(path.len(), 3); // [A, B, A] or [B, A, B]
    }

    #[test]
    fn spine_horizon_parse() {
        assert_eq!(SpineHorizon::parse("h0"), Some(SpineHorizon::H0));
        assert_eq!(SpineHorizon::parse("H1"), Some(SpineHorizon::H1));
        assert_eq!(SpineHorizon::parse("h12"), Some(SpineHorizon::H12));
        assert_eq!(SpineHorizon::parse("invalid"), None);
    }

    #[test]
    fn is_terminal_status_works() {
        assert!(is_terminal_status(&WorkItemStatus::Done));
        assert!(is_terminal_status(&WorkItemStatus::Superseded));
        assert!(is_terminal_status(&WorkItemStatus::Cancelled));
        assert!(!is_terminal_status(&WorkItemStatus::Active));
        assert!(!is_terminal_status(&WorkItemStatus::Draft));
        assert!(!is_terminal_status(&WorkItemStatus::Paused));
    }
}
