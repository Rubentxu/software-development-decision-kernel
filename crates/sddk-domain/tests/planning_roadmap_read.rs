//! Tests for RoadmapGraphRead trait (AC-PLN4-11).
//!
//! Tests that the trait is object-safe and that a &dyn RoadmapGraphRead
//! can be passed to projection functions.
//!
//! NOTE: The real `impl RoadmapGraphRead for &Storage` lives in sddk-storage.
//! This module tests the trait contract using a fake implementation.

use std::collections::BTreeMap;

use sddk_domain::StorageError;
use sddk_domain::planning::WorkItemStatus;
use sddk_domain::planning::projections::{
    DependencyEdgeKindSnapshot, DependencyEdgeSnapshot, RoadmapSnapshot, WorkItemSnapshot,
};
use sddk_domain::planning::roadmap_read::RoadmapGraphRead;

// ── Fake implementation ────────────────────────────────────────────────────────

/// A fake RoadmapGraphRead for trait contract testing.
#[derive(Clone)]
struct FakeRoadmapGraphRead {
    items: Vec<WorkItemSnapshot>,
    edges: Vec<DependencyEdgeSnapshot>,
}

impl FakeRoadmapGraphRead {
    fn new() -> Self {
        Self {
            items: Vec::new(),
            edges: Vec::new(),
        }
    }

    fn with_items(mut self, items: Vec<WorkItemSnapshot>) -> Self {
        self.items = items;
        self
    }

    fn with_edges(mut self, edges: Vec<DependencyEdgeSnapshot>) -> Self {
        self.edges = edges;
        self
    }
}

impl RoadmapGraphRead for FakeRoadmapGraphRead {
    fn list_work_items_roadmap(&self) -> Result<Vec<WorkItemSnapshot>, StorageError> {
        Ok(self.items.clone())
    }

    fn list_dependency_edges_roadmap(&self) -> Result<Vec<DependencyEdgeSnapshot>, StorageError> {
        Ok(self.edges.clone())
    }

    fn list_incoming_edges_for(
        &self,
        work_item_id: &str,
    ) -> Result<Vec<DependencyEdgeSnapshot>, StorageError> {
        Ok(self
            .edges
            .iter()
            .filter(|e| e.to_id == work_item_id)
            .cloned()
            .collect())
    }

    fn get_work_item_with_spine_metadata(
        &self,
        work_item_id: &str,
    ) -> Result<Option<WorkItemSnapshot>, StorageError> {
        Ok(self.items.iter().find(|i| i.id == work_item_id).cloned())
    }

    fn is_ledger_imported(&self) -> Result<bool, StorageError> {
        Ok(!self.items.is_empty())
    }

    fn snapshot_roadmap(&self) -> Result<RoadmapSnapshot, StorageError> {
        Ok(RoadmapSnapshot {
            work_items: self
                .items
                .iter()
                .map(|wi| (wi.id.clone(), wi.clone()))
                .collect(),
            edges: self.edges.clone(),
            bound_cycles: vec![],
        })
    }
}

// ── Test fixtures ──────────────────────────────────────────────────────────────

fn make_item(
    id: &str,
    order: i32,
    horizon: &str,
    spine_status: &str,
    status: WorkItemStatus,
) -> WorkItemSnapshot {
    WorkItemSnapshot {
        id: id.to_string(),
        cycle_id: id.to_string(),
        title: id.to_string(),
        description: format!("Test item {}", id),
        status,
        spine_order: Some(order),
        spine_horizon: Some(horizon.to_string()),
        spine_status: Some(spine_status.to_string()),
        exit_gate: Some("gate".to_string()),
        blocks: vec![],
    }
}

// ── AC-PLN4-11: RoadmapGraphRead scenarios ─────────────────────────────────

/// Scenario: object-safety holds — &dyn RoadmapGraphRead compiles.
#[test]
fn dyn_roadmap_graph_read_compiles() {
    let fake = FakeRoadmapGraphRead::new();
    // This must compile: proves object-safety
    let _: &dyn RoadmapGraphRead = &fake;
    let snap = fake.snapshot_roadmap().expect("must succeed");
    assert!(
        snap.bound_cycles.is_empty(),
        "documented contract: SQLite adapter (and Fake) return empty bound_cycles; \
         changing this requires a MINOR cycle + ADR per roadmap_read.rs:64"
    );
}

/// Scenario: list_work_items_roadmap returns all items.
#[test]
fn list_work_items_roadmap_returns_all() {
    let items = vec![
        make_item("A", 10, "h0", "shipped", WorkItemStatus::Done),
        make_item("B", 20, "h0", "active", WorkItemStatus::Active),
    ];
    let fake = FakeRoadmapGraphRead::new().with_items(items);

    let result = fake.list_work_items_roadmap().expect("must succeed");
    assert_eq!(result.len(), 2, "should return all items");
}

/// Scenario: list_incoming_edges_for returns reverse edges.
#[test]
fn list_incoming_edges_returns_reverse() {
    // A → B (A blocks B, B depends on A)
    let edges = vec![DependencyEdgeSnapshot {
        from_id: "A".to_string(),
        to_id: "B".to_string(),
        kind: DependencyEdgeKindSnapshot::Blocks,
    }];
    let fake = FakeRoadmapGraphRead::new().with_edges(edges);

    let incoming = fake.list_incoming_edges_for("B").expect("must succeed");
    assert_eq!(incoming.len(), 1, "B should have one incoming edge from A");
    assert_eq!(incoming[0].from_id, "A");
}

/// Scenario: is_ledger_imported returns correct value.
#[test]
fn is_ledger_imported_reflects_items() {
    let empty = FakeRoadmapGraphRead::new();
    assert!(
        !empty.is_ledger_imported().expect("must succeed"),
        "empty roadmap is not imported"
    );

    let with_items = FakeRoadmapGraphRead::new().with_items(vec![make_item(
        "A",
        10,
        "h0",
        "shipped",
        WorkItemStatus::Done,
    )]);
    assert!(
        with_items.is_ledger_imported().expect("must succeed"),
        "non-empty roadmap is imported"
    );
}
