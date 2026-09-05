//! `RoadmapGraphRead` trait for decision-plane projections.
//!
//! Implements PLN-LEDGER-004 AC-PLN4-11 per spec §3.2 Q2.
//!
//! Extends `PlanningGraphRead` with roadmap-wide (cross-cycle) read methods.
//! Object-safe: no associated types, no `Self` in generics.

use crate::planning::projections::{DependencyEdgeSnapshot, WorkItemSnapshot};
use crate::StorageError;

// ── RoadmapGraphRead trait ─────────────────────────────────────────────────

/// Read-only port for accessing roadmap-wide (cross-cycle) planning data.
///
/// Used by the five decision-plane projection functions:
/// - `project_status`, `project_next`, `project_blocked`
/// - `project_show`, `project_graph`
///
/// The trait is object-safe (no associated types, no `Self` in generics).
/// Implementations are provided in `sddk-storage` for `Storage`.
pub trait RoadmapGraphRead {
    /// Lists ALL work items across ALL cycles (roadmap-wide enumeration).
    ///
    /// Unlike `PlanningGraphRead::list_work_items_by_cycle` which is cycle-scoped,
    /// this method returns every spine-imported work item in a single call.
    fn list_work_items_roadmap(&self) -> Result<Vec<WorkItemSnapshot>, StorageError>;

    /// Lists ALL dependency edges across ALL cycles (roadmap-wide enumeration).
    fn list_dependency_edges_roadmap(&self) -> Result<Vec<DependencyEdgeSnapshot>, StorageError>;

    /// Lists incoming edges for a work item (items that depend on this one).
    ///
    /// This is the "blocks" direction: if A→B (A blocks B), then B's incoming
    /// edges include A.
    fn list_incoming_edges_for(
        &self,
        work_item_id: &str,
    ) -> Result<Vec<DependencyEdgeSnapshot>, StorageError>;

    /// Returns a work item snapshot with spine metadata populated.
    ///
    /// Includes all four spine columns (spine_order, spine_horizon,
    /// spine_status, exit_gate) from the work_items_v1 row.
    fn get_work_item_with_spine_metadata(
        &self,
        work_item_id: &str,
    ) -> Result<Option<WorkItemSnapshot>, StorageError>;

    /// Returns true if any work items have been imported into the roadmap.
    ///
    /// Used to distinguish "ledger not imported" from "empty roadmap".
    fn is_ledger_imported(&self) -> Result<bool, StorageError>;

    /// Builds a `RoadmapSnapshot` for projection computation.
    ///
    /// Collects all work items and edges roadmap-wide, plus bound cycles
    /// from the reconciliation block.
    fn snapshot_roadmap(&self) -> Result<crate::planning::projections::RoadmapSnapshot, StorageError> {
        let work_items = self.list_work_items_roadmap()?;
        let edges = self.list_dependency_edges_roadmap()?;

        // Collect bound cycles from the reconciliation block.
        // Bound cycles are cycles that have been closed/released with items in the spine.
        let bound_cycles: Vec<String> = Vec::new(); // TODO: populate from reconciliation data

        Ok(crate::planning::projections::RoadmapSnapshot {
            work_items: work_items
                .into_iter()
                .map(|wi| (wi.id.clone(), wi))
                .collect(),
            edges,
            bound_cycles,
        })
    }
}
