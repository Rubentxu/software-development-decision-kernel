//! Determinism tests for decision-plane projections at the domain layer (AC-PLN4-10).
//!
//! Tests that projection outputs are deterministic regardless of:
//! - Two independent RoadmapSnapshot instances produce same output
//! - Output matches golden bytes from pinned fixture
//! - All five projections succeed on the pinned fixture
//!
//! Uses the pinned post-reconciliation fixture data directly (no Storage dependency).
//! Storage-level determinism is tested in sddk-storage/spine_import_determinism.rs.

use std::fs;

use sddk_domain::planning::projections::{
    project_blocked, project_graph, project_next, project_show, project_status,
    DependencyEdgeKindSnapshot, DependencyEdgeSnapshot, GraphFormat, RoadmapSnapshot,
    WorkItemSnapshot,
};
use sddk_domain::planning::WorkItemStatus;
use sddk_domain::spine::{parse_spine_yaml, SpineStatus};

const FIXTURE_PATH: &str =
    "../sddk-domain/tests/fixtures/execution_spine_post_reconciliation.yaml";
const STATUS_GOLDEN_PATH: &str = "../sddk-domain/tests/fixtures/project_status.golden.json";
const GRAPH_GOLDEN_PATH: &str = "../sddk-domain/tests/fixtures/project_graph.golden.json";

fn load_yaml() -> String {
    fs::read_to_string(FIXTURE_PATH).expect("failed to read spine fixture")
}

fn load_golden(path: &str) -> String {
    fs::read_to_string(path).expect("failed to read golden file")
}

fn parse_yaml_to_snapshot(yaml: &str) -> RoadmapSnapshot {
    let bytes = yaml.as_bytes();
    let spine = parse_spine_yaml(bytes).expect("failed to parse spine YAML");

    let mut work_items = Vec::new();
    let mut edges = Vec::new();

    for item in &spine.items {
        let wi_status = item
            .status
            .to_work_item_status()
            .expect("valid spine status");

        let spine_status_str = match item.status {
            SpineStatus::Proposed => "proposed",
            SpineStatus::Ready => "ready",
            SpineStatus::Active => "active",
            SpineStatus::Partial => "partial",
            SpineStatus::Blocked => "blocked",
            SpineStatus::Shipped => "shipped",
            SpineStatus::Absorbed => "absorbed",
            SpineStatus::Superseded => "superseded",
        };

        let horizon_str = match item.horizon {
            sddk_domain::spine::SpineHorizon::H0 => "h0",
            sddk_domain::spine::SpineHorizon::H1 => "h1",
            sddk_domain::spine::SpineHorizon::H2 => "h2",
            sddk_domain::spine::SpineHorizon::H3 => "h3",
            sddk_domain::spine::SpineHorizon::H4 => "h4",
            sddk_domain::spine::SpineHorizon::H5 => "h5",
            sddk_domain::spine::SpineHorizon::H6 => "h6",
            sddk_domain::spine::SpineHorizon::H7 => "h7",
            sddk_domain::spine::SpineHorizon::H8 => "h8",
            sddk_domain::spine::SpineHorizon::H9 => "h9",
            sddk_domain::spine::SpineHorizon::H10 => "h10",
            sddk_domain::spine::SpineHorizon::H11 => "h11",
            sddk_domain::spine::SpineHorizon::H12 => "h12",
        };

        let blocks: Vec<String> = item.depends_on.clone();

        // Add outgoing edges: from this item TO each item it depends on
        // (this item blocks the items it depends on; they cannot proceed until this item ships)
        for dep in &item.depends_on {
            edges.push(DependencyEdgeSnapshot {
                from_id: item.id.clone(),
                to_id: dep.clone(),
                kind: DependencyEdgeKindSnapshot::Blocks,
            });
        }

        let exit_gate_val = if item.exit_gate.is_empty() {
            None
        } else {
            Some(item.exit_gate.clone())
        };

        work_items.push(WorkItemSnapshot {
            id: item.id.clone(),
            cycle_id: item.id.clone(),
            title: item.id.clone(),
            description: item.objective.clone(),
            status: wi_status,
            spine_order: Some(item.order as i32),
            spine_horizon: Some(horizon_str.to_string()),
            spine_status: Some(spine_status_str.to_string()),
            exit_gate: exit_gate_val,
            blocks,
        });
    }

    let work_items_map: std::collections::BTreeMap<_, _> = work_items
        .into_iter()
        .map(|wi| (wi.id.clone(), wi))
        .collect();

    RoadmapSnapshot {
        work_items: work_items_map,
        edges,
        bound_cycles: vec![],
    }
}

fn snapshot_to_status_json(snapshot: &RoadmapSnapshot) -> String {
    let status = project_status(snapshot).expect("project_status failed");
    serde_json::to_string_pretty(&status).expect("failed to serialize")
}

fn snapshot_to_graph_json(snapshot: &RoadmapSnapshot) -> String {
    project_graph(snapshot, GraphFormat::Json).expect("project_graph failed")
}

// ── Scenario 1: two RoadmapSnapshot instances produce identical output ───────────

#[test]
fn determinism_two_snapshots_identical() {
    let yaml = load_yaml();

    let snap1 = parse_yaml_to_snapshot(&yaml);
    let snap2 = parse_yaml_to_snapshot(&yaml);

    let json1 = snapshot_to_status_json(&snap1);
    let json2 = snapshot_to_status_json(&snap2);
    assert_eq!(
        json1, json2,
        "two parsed snapshots must produce identical status JSON"
    );

    let graph1 = snapshot_to_graph_json(&snap1);
    let graph2 = snapshot_to_graph_json(&snap2);
    assert_eq!(
        graph1, graph2,
        "two parsed snapshots must produce identical graph JSON"
    );
}

// ── Scenario 2: output matches golden bytes ────────────────────────────────

#[test]
fn determinism_matches_golden() {
    let yaml = load_yaml();
    let snap = parse_yaml_to_snapshot(&yaml);

    let status_json = snapshot_to_status_json(&snap);
    let golden_status = load_golden(STATUS_GOLDEN_PATH);
    assert_eq!(
        status_json, golden_status,
        "project_status output must match golden file"
    );

    let graph_json = snapshot_to_graph_json(&snap);
    let golden_graph = load_golden(GRAPH_GOLDEN_PATH);
    assert_eq!(
        graph_json, golden_graph,
        "project_graph output must match golden file"
    );
}

// ── Scenario 3: all five projections succeed on pinned fixture ──────────────

#[test]
fn determinism_all_projections_succeed() {
    let yaml = load_yaml();
    let snap = parse_yaml_to_snapshot(&yaml);

    // project_status
    let _ = project_status(&snap).expect("project_status must succeed");

    // project_next
    let _ = project_next(&snap).expect("project_next must succeed");

    // project_blocked
    let _ = project_blocked(&snap).expect("project_blocked must succeed");

    // project_show for each item
    for item_id in snap.work_items.keys() {
        let _ = project_show(&snap, item_id).expect("project_show must succeed");
    }

    // project_graph all formats
    let _ = project_graph(&snap, GraphFormat::Json).expect("project_graph Json must succeed");
    let _ = project_graph(&snap, GraphFormat::Dot).expect("project_graph Dot must succeed");
    let _ = project_graph(&snap, GraphFormat::Mermaid)
        .expect("project_graph Mermaid must succeed");
}
