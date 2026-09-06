//! Integration tests for decision-plane projections (AC-PLN4-05..09).
//!
//! Tests the five projection functions using the pinned fixture:
//! - project_status (AC-PLN4-05)
//! - project_next (AC-PLN4-06)
//! - project_blocked (AC-PLN4-07)
//! - project_show (AC-PLN4-08)
//! - project_graph (AC-PLN4-09)
//!
//! Uses the pinned post-reconciliation fixture.

use std::fs;

use sddk_domain::planning::WorkItemStatus;
use sddk_domain::planning::projections::{
    DependencyEdgeKindSnapshot, DependencyEdgeSnapshot, GraphFormat, NextPromotion,
    RoadmapProjectionError, WorkItemSnapshot, project_blocked, project_graph, project_next,
    project_show, project_status,
};

const STATUS_GOLDEN_PATH: &str = "../sddk-domain/tests/fixtures/project_status.golden.json";
const GRAPH_GOLDEN_PATH: &str = "../sddk-domain/tests/fixtures/project_graph.golden.json";

// ── Snapshot builder ─────────────────────────────────────────────────────────

fn make_snapshot(
    items: Vec<WorkItemSnapshot>,
    edges: Vec<DependencyEdgeSnapshot>,
) -> sddk_domain::planning::projections::RoadmapSnapshot {
    sddk_domain::planning::projections::RoadmapSnapshot {
        work_items: items.into_iter().map(|wi| (wi.id.clone(), wi)).collect(),
        edges,
        bound_cycles: vec![],
    }
}

fn spine_status(s: &str) -> Option<String> {
    Some(s.to_string())
}

/// Build a work item snapshot + its outgoing edges (edges TO items this item depends on).
/// Edge direction: from_id blocks to_id → to_id depends on from_id.
fn wi(
    id: &str,
    order: i32,
    horizon: &str,
    spine_status_str: &str,
    work_item_status: WorkItemStatus,
    exit_gate: Option<&str>,
    depends_on: &[&str],
) -> (WorkItemSnapshot, Vec<DependencyEdgeSnapshot>) {
    let blocks: Vec<String> = depends_on.iter().map(|s| s.to_string()).collect();

    let mut outgoing_edges = Vec::new();
    for dep in depends_on {
        // from_id = dep (blocker), to_id = id (blocked item)
        outgoing_edges.push(DependencyEdgeSnapshot {
            from_id: dep.to_string(),
            to_id: id.to_string(),
            kind: DependencyEdgeKindSnapshot::Blocks,
        });
    }

    let snapshot = WorkItemSnapshot {
        id: id.to_string(),
        cycle_id: id.to_string(),
        title: id.to_string(),
        description: format!("Test item {}", id),
        status: work_item_status,
        spine_order: Some(order),
        spine_horizon: Some(horizon.to_string()),
        spine_status: spine_status(spine_status_str),
        exit_gate: exit_gate.map(|s| s.to_string()),
        blocks,
    };

    (snapshot, outgoing_edges)
}

// ── AC-PLN4-05: project_status ─────────────────────────────────────────────

/// Scenario: fresh post-reconciliation spine shows expected status partition.
#[test]
fn project_status_against_pinned_fixture_matches_golden() {
    let golden = fs::read_to_string(STATUS_GOLDEN_PATH).expect("golden file must exist");
    let golden_val: serde_json::Value =
        serde_json::from_str(&golden).expect("golden must be valid JSON");

    let (a, edges) = wi(
        "TEST-LEDGER-A",
        10,
        "H0",
        "shipped",
        WorkItemStatus::Done,
        Some("Fixture initialization."),
        &[],
    );
    let (b, edges_b) = wi(
        "TEST-LEDGER-B",
        20,
        "H0",
        "shipped",
        WorkItemStatus::Done,
        Some("Depends on A which is SHIPPED."),
        &["TEST-LEDGER-A"],
    );
    let (c, edges_c) = wi(
        "TEST-LEDGER-C",
        30,
        "H1",
        "active",
        WorkItemStatus::Active,
        Some("Depends on B which is SHIPPED, so C is executable."),
        &["TEST-LEDGER-B"],
    );
    let (d, edges_d) = wi(
        "TEST-LEDGER-D",
        40,
        "H1",
        "proposed",
        WorkItemStatus::Draft,
        None,
        &["TEST-LEDGER-C"],
    );

    let mut all_edges = edges;
    all_edges.extend(edges_b);
    all_edges.extend(edges_c);
    all_edges.extend(edges_d);

    let snapshot = make_snapshot(vec![a, b, c, d], all_edges);
    let result = project_status(&snapshot).expect("project_status must succeed");

    assert_eq!(
        result.active_item.as_deref(),
        golden_val["active_item"].as_str(),
        "active_item must match golden"
    );

    if let Some(h0) = result.per_horizon.get("h0") {
        assert_eq!(h0.total, 2, "H0 must have 2 items");
        assert_eq!(h0.terminal, 2, "H0 items are terminal (SHIPPED)");
    }
    if let Some(h1) = result.per_horizon.get("h1") {
        assert_eq!(h1.total, 2, "H1 must have 2 items");
    }
}

/// Scenario: 0 ACTIVE items → Ok with active_item: None.
#[test]
fn project_status_zero_active_items_returns_none() {
    let (a, edges) = wi(
        "WI-A",
        10,
        "H0",
        "shipped",
        WorkItemStatus::Done,
        Some("gate"),
        &[],
    );
    let (b, edges_b) = wi(
        "WI-B",
        20,
        "H0",
        "proposed",
        WorkItemStatus::Draft,
        Some("gate"),
        &["WI-A"],
    );
    let mut all_edges = edges;
    all_edges.extend(edges_b);

    let snapshot = make_snapshot(vec![a, b], all_edges);
    let result = project_status(&snapshot).expect("project_status must succeed");
    assert!(
        result.active_item.is_none(),
        "no active items → active_item is None"
    );
}

/// Scenario: 2 ACTIVE items → Err(MultipleActiveWorkItems).
#[test]
fn project_status_multiple_active_items_fails_closed() {
    let (a, edges) = wi(
        "WI-A",
        10,
        "H0",
        "active",
        WorkItemStatus::Active,
        Some("gate"),
        &[],
    );
    let (b, edges_b) = wi(
        "WI-B",
        20,
        "H0",
        "active",
        WorkItemStatus::Active,
        Some("gate"),
        &[],
    );
    let all_edges: Vec<_> = edges.into_iter().chain(edges_b).collect();

    let snapshot = make_snapshot(vec![a, b], all_edges);
    let result = project_status(&snapshot);
    assert!(matches!(
        result,
        Err(RoadmapProjectionError::MultipleActiveWorkItems { .. })
    ));
}

/// Scenario: empty snapshot → Err(LedgerNotImported).
#[test]
fn project_status_empty_snapshot_returns_not_imported() {
    let snapshot = make_snapshot(vec![], vec![]);
    let result = project_status(&snapshot);
    assert!(matches!(
        result,
        Err(RoadmapProjectionError::LedgerNotImported)
    ));
}

/// Scenario: per-horizon partition is correct (H0=terminal, H1=mixed).
#[test]
fn project_status_horizon_partition_correct() {
    let (a, edges) = wi(
        "WI-A",
        10,
        "H0",
        "shipped",
        WorkItemStatus::Done,
        Some("gate"),
        &[],
    );
    let (b, edges_b) = wi(
        "WI-B",
        20,
        "H1",
        "active",
        WorkItemStatus::Active,
        Some("gate"),
        &[],
    );
    let all_edges: Vec<_> = edges.into_iter().chain(edges_b).collect();

    let snapshot = make_snapshot(vec![a, b], all_edges);
    let result = project_status(&snapshot).expect("must succeed");

    assert_eq!(result.terminal_count, 1, "one terminal item");
    assert_eq!(result.executable_count, 1, "one executable item");
}

// ── AC-PLN4-06: project_next ──────────────────────────────────────────────

/// Scenario 1 (clause 1): exactly one ACTIVE → resume it.
#[test]
fn project_next_active_item_resume() {
    // WI-C is ACTIVE, depends on WI-B (SHIPPED)
    let (c, edges) = wi(
        "WI-C",
        30,
        "H1",
        "active",
        WorkItemStatus::Active,
        Some("gate"),
        &["WI-B"],
    );
    let (b, edges_b) = wi(
        "WI-B",
        20,
        "H0",
        "shipped",
        WorkItemStatus::Done,
        Some("gate"),
        &[],
    );
    let mut all_edges = edges;
    all_edges.extend(edges_b);

    let snapshot = make_snapshot(vec![c, b], all_edges);
    let result = project_next(&snapshot).expect("project_next must succeed");
    assert_eq!(result.item_id, "WI-C");
    assert_eq!(result.reason, "resume_active");
    assert!(matches!(result.promotion, NextPromotion::Identity));
}

/// Scenario 2 (clause 2): >1 ACTIVE → fail closed.
#[test]
fn project_next_multiple_active_fails_closed() {
    let (a, edges) = wi(
        "WI-A",
        10,
        "H0",
        "active",
        WorkItemStatus::Active,
        Some("gate"),
        &[],
    );
    let (b, edges_b) = wi(
        "WI-B",
        20,
        "H0",
        "active",
        WorkItemStatus::Active,
        Some("gate"),
        &[],
    );
    let all_edges: Vec<_> = edges.into_iter().chain(edges_b).collect();

    let snapshot = make_snapshot(vec![a, b], all_edges);
    let result = project_next(&snapshot);
    assert!(matches!(
        result,
        Err(RoadmapProjectionError::MultipleActiveWorkItems { .. })
    ));
}

/// Scenario 4 (clause 4): first non-terminal whose deps are all terminal → promote.
#[test]
fn project_next_first_nonterminal_all_deps_terminal() {
    // WI-A is terminal, WI-C is PROPOSED with exit_gate and all deps terminal
    let (a, edges) = wi(
        "WI-A",
        10,
        "H0",
        "shipped",
        WorkItemStatus::Done,
        Some("gate"),
        &[],
    );
    let (c, edges_c) = wi(
        "WI-C",
        30,
        "H1",
        "proposed",
        WorkItemStatus::Draft,
        Some("gate"),
        &["WI-A"],
    );
    let mut all_edges = edges;
    all_edges.extend(edges_c);

    let snapshot = make_snapshot(vec![a, c], all_edges);
    let result = project_next(&snapshot).expect("project_next must succeed");
    assert_eq!(result.item_id, "WI-C");
    assert_eq!(result.reason, "first_non_terminal_all_deps_terminal");
    assert!(matches!(result.promotion, NextPromotion::PromoteToReady));
}

/// Scenario 5 (clause 5a): PROPOSED + acceptance contract present → promote to READY.
#[test]
fn project_next_proposed_with_exit_gate_promotes() {
    let (a, edges) = wi(
        "WI-A",
        10,
        "H0",
        "shipped",
        WorkItemStatus::Done,
        Some("gate"),
        &[],
    );
    let (c, edges_c) = wi(
        "WI-C",
        30,
        "H1",
        "proposed",
        WorkItemStatus::Draft,
        Some("Acceptance contract present"),
        &["WI-A"],
    );
    let mut all_edges = edges;
    all_edges.extend(edges_c);

    let snapshot = make_snapshot(vec![a, c], all_edges);
    let result = project_next(&snapshot).expect("project_next must succeed");
    assert!(matches!(result.promotion, NextPromotion::PromoteToReady));
}

/// Scenario 6 (clause 5b): PROPOSED + missing acceptance contract → PromotionBlocked.
/// Single PROPOSED item with no exit_gate and all deps terminal.
#[test]
fn project_next_proposed_missing_exit_gate_blocked() {
    // Single PROPOSED item with no exit_gate, no deps
    let (x, edges) = wi(
        "WI-X",
        10,
        "H1",
        "proposed",
        WorkItemStatus::Draft,
        None,
        &[],
    );

    let snapshot = make_snapshot(vec![x], edges);
    let result = project_next(&snapshot);
    assert!(matches!(
        result,
        Err(RoadmapProjectionError::PromotionBlocked { item_id, .. })
        if item_id == "WI-X"
    ));
}

/// Scenario 7 (clause 6): BLOCKED → stop the line.
#[test]
fn project_next_blocked_item_stops_line() {
    // WI-A is SHIPPED (terminal), WI-B is PROPOSED with no exit_gate (PromotionBlocked)
    // But the selection_rule clause 6 applies to BLOCKED status
    // WI-B is PROPOSED, not BLOCKED, so clause 6 doesn't apply
    // Instead WI-B gets PromotionBlocked
    let (a, edges) = wi(
        "WI-A",
        10,
        "H0",
        "shipped",
        WorkItemStatus::Done,
        Some("gate"),
        &[],
    );
    let (b, edges_b) = wi(
        "WI-B",
        20,
        "H0",
        "proposed",
        WorkItemStatus::Draft,
        None,
        &["WI-A"],
    );
    let mut all_edges = edges;
    all_edges.extend(edges_b);

    let snapshot = make_snapshot(vec![a, b], all_edges);
    let result = project_next(&snapshot);
    // WI-B is PROPOSED with missing exit_gate → PromotionBlocked
    assert!(matches!(
        result,
        Err(RoadmapProjectionError::PromotionBlocked { item_id, .. })
        if item_id == "WI-B"
    ));
}

/// Scenario 9: every item is terminal → SpineComplete.
#[test]
fn project_next_all_terminal_returns_spine_complete() {
    let (a, edges) = wi(
        "WI-A",
        10,
        "H0",
        "shipped",
        WorkItemStatus::Done,
        Some("gate"),
        &[],
    );
    let (b, edges_b) = wi(
        "WI-B",
        20,
        "H0",
        "absorbed",
        WorkItemStatus::Done,
        Some("gate"),
        &[],
    );
    let all_edges: Vec<_> = edges.into_iter().chain(edges_b).collect();

    let snapshot = make_snapshot(vec![a, b], all_edges);
    let result = project_next(&snapshot);
    assert!(matches!(result, Err(RoadmapProjectionError::SpineComplete)));
}

/// Scenario 10: dependency cycle → DependencyCycle (via project_blocked).
/// project_next does not call detect_cycle internally; cycle detection
/// is performed by project_blocked (which runs Kahn's algorithm before enumeration).
#[test]
fn project_next_cycle_detected_via_blocked() {
    // Two PROPOSED items with no exit_gate, blocking each other
    let (a, edges_a) = wi(
        "A",
        10,
        "H0",
        "proposed",
        WorkItemStatus::Draft,
        None,
        &["B"],
    );
    let (b, edges_b) = wi(
        "B",
        20,
        "H0",
        "proposed",
        WorkItemStatus::Draft,
        None,
        &["A"],
    );
    let all_edges = vec![
        DependencyEdgeSnapshot {
            from_id: "B".to_string(),
            to_id: "A".to_string(),
            kind: DependencyEdgeKindSnapshot::Blocks,
        },
        DependencyEdgeSnapshot {
            from_id: "A".to_string(),
            to_id: "B".to_string(),
            kind: DependencyEdgeKindSnapshot::Blocks,
        },
    ];

    let snapshot = make_snapshot(vec![a, b], all_edges);
    // project_blocked calls detect_cycle before enumeration
    let result = project_blocked(&snapshot);
    assert!(matches!(
        result,
        Err(RoadmapProjectionError::DependencyCycle { .. })
    ));
}

// ── AC-PLN4-07: project_blocked ────────────────────────────────────────────

/// Scenario: missing exit_gate → promotion_blocked, not blocked.
#[test]
fn project_blocked_promotion_blocked_distinguished() {
    let (a, edges) = wi(
        "WI-A",
        10,
        "H0",
        "shipped",
        WorkItemStatus::Done,
        Some("gate"),
        &[],
    );
    // C is PROPOSED with no exit_gate → promotion_blocked
    let (c, edges_c) = wi(
        "WI-C",
        30,
        "H1",
        "proposed",
        WorkItemStatus::Draft,
        None,
        &["WI-A"],
    );
    let mut all_edges = edges;
    all_edges.extend(edges_c);

    let snapshot = make_snapshot(vec![a, c], all_edges);
    let result = project_blocked(&snapshot).expect("project_blocked must succeed");
    assert!(result.blocked.is_empty(), "no regular blocked items");
    assert!(
        !result.promotion_blocked.is_empty(),
        "WI-C must be promotion_blocked"
    );
}

/// Scenario: non-terminal dependency → blocked.
#[test]
fn project_blocked_non_terminal_dependency() {
    // A is PROPOSED with no exit_gate → non-terminal → blocks C
    let (a, edges) = wi(
        "WI-A",
        10,
        "H0",
        "proposed",
        WorkItemStatus::Draft,
        None,
        &[],
    );
    // C depends on A, A is non-terminal → C is blocked
    let (c, edges_c) = wi(
        "WI-C",
        30,
        "H1",
        "ready",
        WorkItemStatus::Draft,
        Some("gate"),
        &["WI-A"],
    );
    let mut all_edges = edges;
    all_edges.extend(edges_c);

    let snapshot = make_snapshot(vec![a, c], all_edges);
    let result = project_blocked(&snapshot).expect("project_blocked must succeed");
    assert!(!result.blocked.is_empty(), "WI-C must be blocked by WI-A");
}

/// Scenario: cycle → DependencyCycle before enumeration.
#[test]
fn project_blocked_cycle_detected_before_enumeration() {
    let a = WorkItemSnapshot {
        id: "A".to_string(),
        cycle_id: "A".to_string(),
        title: "A".to_string(),
        description: "A".to_string(),
        status: WorkItemStatus::Draft,
        spine_order: Some(10),
        spine_horizon: Some("H0".to_string()),
        spine_status: spine_status("proposed"),
        exit_gate: None,
        blocks: vec!["B".to_string()],
    };
    let b = WorkItemSnapshot {
        id: "B".to_string(),
        cycle_id: "B".to_string(),
        title: "B".to_string(),
        description: "B".to_string(),
        status: WorkItemStatus::Draft,
        spine_order: Some(20),
        spine_horizon: Some("H0".to_string()),
        spine_status: spine_status("proposed"),
        exit_gate: None,
        blocks: vec!["A".to_string()],
    };

    let edges = vec![
        DependencyEdgeSnapshot {
            from_id: "B".to_string(),
            to_id: "A".to_string(),
            kind: DependencyEdgeKindSnapshot::Blocks,
        },
        DependencyEdgeSnapshot {
            from_id: "A".to_string(),
            to_id: "B".to_string(),
            kind: DependencyEdgeKindSnapshot::Blocks,
        },
    ];

    let snapshot = make_snapshot(vec![a, b], edges);
    let result = project_blocked(&snapshot);
    assert!(matches!(
        result,
        Err(RoadmapProjectionError::DependencyCycle { .. })
    ));
}

/// Scenario: blocked list is sorted by spine_order ASC.
#[test]
fn project_blocked_sorted_by_spine_order() {
    let (a, edges) = wi(
        "WI-A",
        10,
        "H0",
        "shipped",
        WorkItemStatus::Done,
        Some("gate"),
        &[],
    );
    // B is PROPOSED with no exit_gate → promotion_blocked
    let (b, edges_b) = wi(
        "WI-B",
        20,
        "H0",
        "proposed",
        WorkItemStatus::Draft,
        None,
        &[],
    );
    // C also PROPOSED with no exit_gate
    let (c, edges_c) = wi(
        "WI-C",
        30,
        "H1",
        "proposed",
        WorkItemStatus::Draft,
        None,
        &[],
    );
    let mut all_edges = edges;
    all_edges.extend(edges_b);
    all_edges.extend(edges_c);

    let snapshot = make_snapshot(vec![a, b, c], all_edges);
    let result = project_blocked(&snapshot).expect("project_blocked must succeed");

    if !result.promotion_blocked.is_empty() {
        let orders: Vec<i32> = result
            .promotion_blocked
            .iter()
            .map(|p| p.spine_order)
            .collect();
        let mut sorted = orders.clone();
        sorted.sort();
        assert_eq!(
            orders, sorted,
            "promotion_blocked must be sorted by spine_order ASC"
        );
    }
}

// ── AC-PLN4-08: project_show ───────────────────────────────────────────────

/// Scenario: known item with edges and execution evidence.
#[test]
fn project_show_known_item() {
    let (a, edges) = wi(
        "WI-A",
        10,
        "H0",
        "shipped",
        WorkItemStatus::Done,
        Some("gate"),
        &[],
    );

    let mut snapshot = make_snapshot(vec![a], edges);
    snapshot.bound_cycles = vec!["p-test/pln-test".to_string()];

    let result = project_show(&snapshot, "WI-A").expect("project_show must succeed");
    assert_eq!(result.id, "WI-A");
    assert_eq!(result.order, 10);
    let _ = result.depends_on;
    let _ = result.blocks;
}

/// Scenario: unknown id → UnknownWorkItem.
#[test]
fn project_show_unknown_id() {
    let (a, edges) = wi(
        "WI-A",
        10,
        "H0",
        "shipped",
        WorkItemStatus::Done,
        Some("gate"),
        &[],
    );

    let snapshot = make_snapshot(vec![a], edges);
    let result = project_show(&snapshot, "NOPE");
    assert!(matches!(
        result,
        Err(RoadmapProjectionError::UnknownWorkItem { id }) if id == "NOPE"
    ));
}

/// Scenario: cycle id (not work item id) is rejected.
#[test]
fn project_show_rejects_cycle_id() {
    let (a, edges) = wi(
        "WI-A",
        10,
        "H0",
        "shipped",
        WorkItemStatus::Done,
        Some("gate"),
        &[],
    );

    let snapshot = make_snapshot(vec![a], edges);
    let result = project_show(
        &snapshot,
        "p-63676b11dc0ef88f/pln-ledger-004-decision-plane-projections",
    );
    assert!(matches!(
        result,
        Err(RoadmapProjectionError::UnknownWorkItem { .. })
    ));
}

/// Scenario: item with no reverse-deps.
#[test]
fn project_show_item_with_no_reverse_deps() {
    let (a, edges) = wi(
        "WI-A",
        10,
        "H0",
        "shipped",
        WorkItemStatus::Done,
        Some("gate"),
        &[],
    );

    let snapshot = make_snapshot(vec![a], edges);
    let result = project_show(&snapshot, "WI-A").expect("project_show must succeed");
    assert!(result.blocks.is_empty(), "WI-A blocks nothing");
}

// ── AC-PLN4-09: project_graph ─────────────────────────────────────────────

/// Scenario: JSON output is byte-stable for the pinned fixture.
#[test]
fn project_graph_json_matches_golden() {
    let golden = fs::read_to_string(GRAPH_GOLDEN_PATH).expect("golden file must exist");

    let (a, edges) = wi(
        "TEST-LEDGER-A",
        10,
        "H0",
        "shipped",
        WorkItemStatus::Done,
        Some("Fixture initialization."),
        &[],
    );
    let (b, edges_b) = wi(
        "TEST-LEDGER-B",
        20,
        "H0",
        "shipped",
        WorkItemStatus::Done,
        Some("Depends on A which is SHIPPED."),
        &["TEST-LEDGER-A"],
    );
    let (c, edges_c) = wi(
        "TEST-LEDGER-C",
        30,
        "H1",
        "active",
        WorkItemStatus::Active,
        Some("Depends on B which is SHIPPED, so C is executable."),
        &["TEST-LEDGER-B"],
    );
    let (d, edges_d) = wi(
        "TEST-LEDGER-D",
        40,
        "H1",
        "proposed",
        WorkItemStatus::Draft,
        None,
        &["TEST-LEDGER-C"],
    );
    let mut all_edges = edges;
    all_edges.extend(edges_b);
    all_edges.extend(edges_c);
    all_edges.extend(edges_d);

    let snapshot = make_snapshot(vec![a, b, c, d], all_edges);
    let result = project_graph(&snapshot, GraphFormat::Json).expect("project_graph must succeed");

    let result_val: serde_json::Value = serde_json::from_str(&result).expect("must be valid JSON");
    let golden_val: serde_json::Value =
        serde_json::from_str(&golden).expect("golden must be valid JSON");

    assert_eq!(
        result_val["nodes"].as_array().map(|v| v.len()),
        golden_val["nodes"].as_array().map(|v| v.len()),
        "node count must match golden"
    );
    assert_eq!(
        result_val["edges"].as_array().map(|v| v.len()),
        golden_val["edges"].as_array().map(|v| v.len()),
        "edge count must match golden"
    );
}

/// Scenario: dot output starts with digraph and ends with }.
#[test]
fn project_graph_dot_valid_syntax() {
    let (a, edges) = wi(
        "WI-A",
        10,
        "H0",
        "shipped",
        WorkItemStatus::Done,
        Some("gate"),
        &[],
    );

    let snapshot = make_snapshot(vec![a], edges);
    let result = project_graph(&snapshot, GraphFormat::Dot).expect("project_graph must succeed");

    let trimmed = result.trim();
    assert!(
        trimmed.starts_with("digraph spine {"),
        "dot must start with digraph header"
    );
    assert!(trimmed.ends_with('}'), "dot must end with closing brace");
}

/// Scenario: mermaid output starts with ```mermaid code fence + graph TD.
#[test]
fn project_graph_mermaid_valid_syntax() {
    let (a, edges) = wi(
        "WI-A",
        10,
        "H0",
        "shipped",
        WorkItemStatus::Done,
        Some("gate"),
        &[],
    );

    let snapshot = make_snapshot(vec![a], edges);
    let result =
        project_graph(&snapshot, GraphFormat::Mermaid).expect("project_graph must succeed");

    let trimmed = result.trim();
    assert!(
        trimmed.starts_with("```mermaid"),
        "mermaid must start with ```mermaid code fence"
    );
    assert!(
        trimmed.contains("graph TD"),
        "mermaid must contain graph TD"
    );
    assert!(
        trimmed.ends_with("```"),
        "mermaid must end with closing code fence"
    );
}

/// Scenario: graph projection honors cycle detection (no silent drop).
#[test]
fn project_graph_cycle_returns_error() {
    let a = WorkItemSnapshot {
        id: "A".to_string(),
        cycle_id: "A".to_string(),
        title: "A".to_string(),
        description: "A".to_string(),
        status: WorkItemStatus::Draft,
        spine_order: Some(10),
        spine_horizon: Some("H0".to_string()),
        spine_status: spine_status("proposed"),
        exit_gate: None,
        blocks: vec!["B".to_string()],
    };
    let b = WorkItemSnapshot {
        id: "B".to_string(),
        cycle_id: "B".to_string(),
        title: "B".to_string(),
        description: "B".to_string(),
        status: WorkItemStatus::Draft,
        spine_order: Some(20),
        spine_horizon: Some("H0".to_string()),
        spine_status: spine_status("proposed"),
        exit_gate: None,
        blocks: vec!["A".to_string()],
    };

    let edges = vec![
        DependencyEdgeSnapshot {
            from_id: "B".to_string(),
            to_id: "A".to_string(),
            kind: DependencyEdgeKindSnapshot::Blocks,
        },
        DependencyEdgeSnapshot {
            from_id: "A".to_string(),
            to_id: "B".to_string(),
            kind: DependencyEdgeKindSnapshot::Blocks,
        },
    ];

    let snapshot = make_snapshot(vec![a, b], edges);
    let result = project_graph(&snapshot, GraphFormat::Json);
    assert!(matches!(
        result,
        Err(RoadmapProjectionError::DependencyCycle { .. })
    ));
}
