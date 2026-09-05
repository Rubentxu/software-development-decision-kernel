//! Tests for `compute_planning_graph_identity` (AC-PLN2-10).
//!
//! Verifies deterministic ordering, volatile-field exclusion per FIND-PLN-008,
//! and identity stability across replays.

use sddk_domain::planning::{
    DependencyEdgeKind, DependencyEdgeV1, WORK_ITEM_SCHEMA_VERSION, WorkItemStatus, WorkItemV1,
    compute_planning_graph_identity,
};

/// Helper: build a WorkItemV1.
fn work_item(id: &str, cycle: &str, title: &str, status: WorkItemStatus) -> WorkItemV1 {
    WorkItemV1 {
        id: id.to_string(),
        cycle_id: cycle.to_string(),
        title: title.to_string(),
        description: format!("description-{}", id),
        status,
        actor_ref: None,
        created_at: 0,
        schema_version: WORK_ITEM_SCHEMA_VERSION,
    }
}

/// Helper: build a DependencyEdgeV1.
fn edge(from_id: &str, to_id: &str, kind: DependencyEdgeKind) -> DependencyEdgeV1 {
    DependencyEdgeV1::new(from_id.to_string(), to_id.to_string(), kind, None)
}

// ── Deterministic ordering ───────────────────────────────────────────────────────

#[test]
fn same_graph_different_insertion_order_produces_identical_hash() {
    // GIVEN three WorkItems and three edges inserted in different orders
    let items_a = vec![
        work_item("wi-001", "cycle-1", "First", WorkItemStatus::Active),
        work_item("wi-002", "cycle-1", "Second", WorkItemStatus::Draft),
        work_item("wi-003", "cycle-1", "Third", WorkItemStatus::Done),
    ];
    let items_b = vec![
        work_item("wi-003", "cycle-1", "Third", WorkItemStatus::Done),
        work_item("wi-001", "cycle-1", "First", WorkItemStatus::Active),
        work_item("wi-002", "cycle-1", "Second", WorkItemStatus::Draft),
    ];
    let edges_a = vec![
        edge("wi-001", "wi-002", DependencyEdgeKind::Blocks),
        edge("wi-002", "wi-003", DependencyEdgeKind::Blocks),
    ];
    let edges_b = vec![
        edge("wi-002", "wi-003", DependencyEdgeKind::Blocks),
        edge("wi-001", "wi-002", DependencyEdgeKind::Blocks),
    ];
    let evidence: Vec<String> = vec![];
    let decisions: Vec<String> = vec![];

    // WHEN compute_planning_graph_identity is called on each
    let hash_a = compute_planning_graph_identity(&items_a, &edges_a, &evidence, &decisions);
    let hash_b = compute_planning_graph_identity(&items_b, &edges_b, &evidence, &decisions);

    // THEN the SHA-256 is identical
    assert_eq!(hash_a, hash_b);
}

#[test]
fn edges_sorted_by_from_id_then_to_id_for_determinism() {
    // GIVEN edges inserted in non-sorted order
    let items = vec![work_item("wi-001", "cycle-1", "A", WorkItemStatus::Active)];
    let edges_unsorted = vec![
        edge("wi-001", "wi-002", DependencyEdgeKind::Blocks),
        edge("wi-001", "wi-001", DependencyEdgeKind::Blocks),
        edge("wi-000", "wi-001", DependencyEdgeKind::Blocks),
    ];
    let edges_reversed = vec![
        edge("wi-000", "wi-001", DependencyEdgeKind::Blocks),
        edge("wi-001", "wi-001", DependencyEdgeKind::Blocks),
        edge("wi-001", "wi-002", DependencyEdgeKind::Blocks),
    ];
    let evidence: Vec<String> = vec![];
    let decisions: Vec<String> = vec![];

    let hash_unsorted =
        compute_planning_graph_identity(&items, &edges_unsorted, &evidence, &decisions);
    let hash_reversed =
        compute_planning_graph_identity(&items, &edges_reversed, &evidence, &decisions);

    assert_eq!(hash_unsorted, hash_reversed);
}

// ── Volatile-field exclusion (FIND-PLN-008) ───────────────────────────────────

#[test]
fn status_change_does_not_alter_identity() {
    // GIVEN a WorkItem whose status changes (volatile field)
    let item_draft = work_item("wi-001", "cycle-1", "Same title", WorkItemStatus::Draft);
    let item_active = work_item("wi-001", "cycle-1", "Same title", WorkItemStatus::Active);
    let edges: Vec<DependencyEdgeV1> = vec![];
    let evidence: Vec<String> = vec![];
    let decisions: Vec<String> = vec![];

    let hash_draft = compute_planning_graph_identity(&[item_draft], &edges, &evidence, &decisions);
    let hash_active =
        compute_planning_graph_identity(&[item_active], &edges, &evidence, &decisions);

    assert_eq!(
        hash_draft, hash_active,
        "status is volatile and must be excluded"
    );
}

#[test]
fn created_at_change_does_not_alter_identity() {
    // GIVEN two captures of the same WorkItem with different created_at timestamps
    let item_t1 = WorkItemV1 {
        id: "wi-001".to_string(),
        cycle_id: "cycle-1".to_string(),
        title: "Same title".to_string(),
        description: "Same description".to_string(),
        status: WorkItemStatus::Active,
        actor_ref: None,
        created_at: 1000,
        schema_version: WORK_ITEM_SCHEMA_VERSION,
    };
    let item_t2 = WorkItemV1 {
        id: "wi-001".to_string(),
        cycle_id: "cycle-1".to_string(),
        title: "Same title".to_string(),
        description: "Same description".to_string(),
        status: WorkItemStatus::Active,
        actor_ref: None,
        created_at: 9999,
        schema_version: WORK_ITEM_SCHEMA_VERSION,
    };
    let edges: Vec<DependencyEdgeV1> = vec![];
    let evidence: Vec<String> = vec![];
    let decisions: Vec<String> = vec![];

    let hash_t1 = compute_planning_graph_identity(&[item_t1], &edges, &evidence, &decisions);
    let hash_t2 = compute_planning_graph_identity(&[item_t2], &edges, &evidence, &decisions);

    assert_eq!(
        hash_t1, hash_t2,
        "created_at is volatile and must be excluded"
    );
}

// ── Content sensitivity ────────────────────────────────────────────────────────

#[test]
fn title_change_alters_identity() {
    // GIVEN a WorkItem whose title changes
    let item_a = work_item(
        "wi-001",
        "cycle-1",
        "Original title",
        WorkItemStatus::Active,
    );
    let item_b = work_item(
        "wi-001",
        "cycle-1",
        "Modified title",
        WorkItemStatus::Active,
    );
    let edges: Vec<DependencyEdgeV1> = vec![];
    let evidence: Vec<String> = vec![];
    let decisions: Vec<String> = vec![];

    let hash_a = compute_planning_graph_identity(&[item_a], &edges, &evidence, &decisions);
    let hash_b = compute_planning_graph_identity(&[item_b], &edges, &evidence, &decisions);

    assert_ne!(hash_a, hash_b, "title change must alter the hash");
}

#[test]
fn description_change_alters_identity() {
    let item_a = WorkItemV1 {
        id: "wi-001".to_string(),
        cycle_id: "cycle-1".to_string(),
        title: "Same title".to_string(),
        description: "Original description".to_string(),
        status: WorkItemStatus::Active,
        actor_ref: None,
        created_at: 0,
        schema_version: WORK_ITEM_SCHEMA_VERSION,
    };
    let item_b = WorkItemV1 {
        id: "wi-001".to_string(),
        cycle_id: "cycle-1".to_string(),
        title: "Same title".to_string(),
        description: "Modified description".to_string(),
        status: WorkItemStatus::Active,
        actor_ref: None,
        created_at: 0,
        schema_version: WORK_ITEM_SCHEMA_VERSION,
    };
    let edges: Vec<DependencyEdgeV1> = vec![];
    let evidence: Vec<String> = vec![];
    let decisions: Vec<String> = vec![];

    let hash_a = compute_planning_graph_identity(&[item_a], &edges, &evidence, &decisions);
    let hash_b = compute_planning_graph_identity(&[item_b], &edges, &evidence, &decisions);

    assert_ne!(hash_a, hash_b);
}

// ── Evidence and decision refs ─────────────────────────────────────────────────

#[test]
fn evidence_refs_sorted_for_determinism() {
    let items = vec![work_item("wi-001", "cycle-1", "A", WorkItemStatus::Active)];
    let edges: Vec<DependencyEdgeV1> = vec![];
    let evidence_unordered = vec![
        "sha256:ccc".to_string(),
        "sha256:aaa".to_string(),
        "sha256:bbb".to_string(),
    ];
    let evidence_sorted = vec![
        "sha256:aaa".to_string(),
        "sha256:bbb".to_string(),
        "sha256:ccc".to_string(),
    ];
    let decisions: Vec<String> = vec![];

    let hash_unordered =
        compute_planning_graph_identity(&items, &edges, &evidence_unordered, &decisions);
    let hash_sorted = compute_planning_graph_identity(&items, &edges, &evidence_sorted, &decisions);

    assert_eq!(hash_unordered, hash_sorted);
}

#[test]
fn decision_refs_sorted_for_determinism() {
    let items = vec![work_item("wi-001", "cycle-1", "A", WorkItemStatus::Active)];
    let edges: Vec<DependencyEdgeV1> = vec![];
    let evidence: Vec<String> = vec![];
    let decisions_unordered = vec![
        "dec-003".to_string(),
        "dec-001".to_string(),
        "dec-002".to_string(),
    ];
    let decisions_sorted = vec![
        "dec-001".to_string(),
        "dec-002".to_string(),
        "dec-003".to_string(),
    ];

    let hash_unordered =
        compute_planning_graph_identity(&items, &edges, &evidence, &decisions_unordered);
    let hash_sorted = compute_planning_graph_identity(&items, &edges, &evidence, &decisions_sorted);

    assert_eq!(hash_unordered, hash_sorted);
}

// ── Empty cycle ────────────────────────────────────────────────────────────────

#[test]
fn empty_cycle_yields_deterministic_hash() {
    // GIVEN an empty cycle (no WorkItems)
    let items: Vec<WorkItemV1> = vec![];
    let edges: Vec<DependencyEdgeV1> = vec![];
    let evidence: Vec<String> = vec![];
    let decisions: Vec<String> = vec![];

    // WHEN compute_planning_graph_identity is called
    let hash_a = compute_planning_graph_identity(&items, &edges, &evidence, &decisions);
    let hash_b = compute_planning_graph_identity(&items, &edges, &evidence, &decisions);

    // THEN both calls produce identical SHA-256 (empty canonical projection is constant)
    assert_eq!(hash_a, hash_b);
    // Hash should be a valid SHA-256 hex string (64 chars)
    assert_eq!(hash_a.len(), 64);
    assert!(hash_a.chars().all(|c| c.is_ascii_hexdigit()));
}
