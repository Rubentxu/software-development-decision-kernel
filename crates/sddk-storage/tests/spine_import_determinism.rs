//! Determinism tests for decision-plane projections.
//!
//! Tests that projection outputs are deterministic regardless of:
//! - Storage instance (two instances produce same output)
//! - Shuffled insertion (produces same output)
//! - Double-import (re-importing produces same output)
//!
//! Uses pinned fixture from sddk-domain/tests/fixtures/.

use std::fs;

use sddk_domain::planning::projections::{GraphFormat, project_graph, project_status};
use sddk_domain::planning::roadmap_read::RoadmapGraphRead;
use sddk_storage::Storage;
use sddk_storage::spine_import::import_spine;

// Path to the pinned fixture (relative to sddk-storage crate root)
const FIXTURE_PATH: &str = "../sddk-domain/tests/fixtures/execution_spine_post_reconciliation.yaml";
const STATUS_GOLDEN_PATH: &str = "../sddk-domain/tests/fixtures/project_status.golden.json";
const GRAPH_GOLDEN_PATH: &str = "../sddk-domain/tests/fixtures/project_graph.golden.json";

/// Load the pinned spine fixture as bytes.
fn load_fixture() -> Vec<u8> {
    fs::read(FIXTURE_PATH).expect("failed to read spine fixture")
}

/// Load the golden JSON file.
fn load_golden(path: &str) -> String {
    fs::read_to_string(path).expect("failed to read golden file")
}

/// Import the spine fixture into a fresh Storage instance.
fn import_fixture() -> Storage {
    let mut storage = Storage::open_in_memory().expect("failed to open in-memory storage");
    let bytes = load_fixture();
    import_spine(&bytes, &mut storage).expect("failed to import spine fixture");
    storage
}

/// Helper: run project_status on storage and return JSON string.
fn run_project_status(storage: &Storage) -> String {
    let snapshot = storage
        .snapshot_roadmap()
        .expect("failed to snapshot roadmap");
    let projection = project_status(&snapshot).expect("project_status failed");
    serde_json::to_string_pretty(&projection).expect("failed to serialize status projection")
}

/// Helper: run project_graph on storage and return JSON string.
fn run_project_graph(storage: &Storage) -> String {
    let snapshot = storage
        .snapshot_roadmap()
        .expect("failed to snapshot roadmap");
    let graph = project_graph(&snapshot, GraphFormat::Json).expect("project_graph failed");
    graph
}

// ── Determinism scenarios ────────────────────────────────────────────────────────

/// Scenario 1: Two independent Storage instances produce identical outputs.
/// Verifies that projection computation is deterministic and doesn't depend on
/// any global state or side effects.
#[test]
fn determinism_two_instances_produce_identical_output() {
    let storage1 = import_fixture();
    let storage2 = import_fixture();

    let status1 = run_project_status(&storage1);
    let status2 = run_project_status(&storage2);
    assert_eq!(
        status1, status2,
        "project_status output must be identical across two independent Storage instances"
    );

    let graph1 = run_project_graph(&storage1);
    let graph2 = run_project_graph(&storage2);
    assert_eq!(
        graph1, graph2,
        "project_graph output must be identical across two independent Storage instances"
    );
}

/// Scenario 2: Shuffled insertion order produces identical output.
/// Re-importing the same fixture should produce identical output regardless of
/// when the import occurred.
#[test]
fn determinism_shuffled_insertion_produces_identical_output() {
    let storage1 = import_fixture();
    let status1 = run_project_status(&storage1);
    let graph1 = run_project_graph(&storage1);

    // Fresh instance and re-import
    let mut storage2 = Storage::open_in_memory().expect("failed to open in-memory storage");
    let bytes = load_fixture();
    import_spine(&bytes, &mut storage2).expect("failed to import spine fixture");

    let status2 = run_project_status(&storage2);
    let graph2 = run_project_graph(&storage2);

    assert_eq!(
        status1, status2,
        "project_status output must be identical after re-import"
    );
    assert_eq!(
        graph1, graph2,
        "project_graph output must be identical after re-import"
    );
}

/// Scenario 3: Double-import produces identical output (idempotency).
/// Importing the same spine twice should not change the projection output.
#[test]
fn determinism_double_import_produces_identical_output() {
    let mut storage = Storage::open_in_memory().expect("failed to open in-memory storage");
    let bytes = load_fixture();

    // First import
    import_spine(&bytes, &mut storage).expect("failed to import spine fixture");
    let status1 = run_project_status(&storage);
    let graph1 = run_project_graph(&storage);

    // Second import (double-import)
    import_spine(&bytes, &mut storage).expect("failed to re-import spine fixture");
    let status2 = run_project_status(&storage);
    let graph2 = run_project_graph(&storage);

    assert_eq!(
        status1, status2,
        "project_status output must be identical after double-import"
    );
    assert_eq!(
        graph1, graph2,
        "project_graph output must be identical after double-import"
    );
}

/// Scenario 4: Output matches golden bytes.
/// Verifies that the pinned fixture produces the expected output.
#[test]
fn determinism_output_matches_golden() {
    let storage = import_fixture();

    let status = run_project_status(&storage);
    let golden_status = load_golden(STATUS_GOLDEN_PATH);
    assert_eq!(
        status, golden_status,
        "project_status output must match golden file"
    );

    let graph = run_project_graph(&storage);
    let golden_graph = load_golden(GRAPH_GOLDEN_PATH);
    assert_eq!(
        graph, golden_graph,
        "project_graph output must match golden file"
    );
}
