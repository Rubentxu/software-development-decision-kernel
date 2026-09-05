//! Performance soft-budget tests for decision-plane projections (AC-PLN4-16).
//!
//! These tests are SOFT budget tests — they log measurements and emit advisory
//! warnings but NEVER fail the test suite based on elapsed time.
//! Per REQ-PLN4-PRF-001..002 and spec Q10: "soft budget is NOT a gate."
//!
//! Uses the pinned fixture: execution_spine_post_reconciliation.yaml

use std::fs;

use sddk_domain::planning::projections::{
    project_blocked, project_graph, project_next, project_status, GraphFormat,
};
use sddk_domain::planning::roadmap_read::RoadmapGraphRead;
use sddk_storage::spine_import::import_spine;
use sddk_storage::Storage;
use tempfile::TempDir;

const FIXTURE_PATH: &str = "../sddk-domain/tests/fixtures/execution_spine_post_reconciliation.yaml";
const STATUS_GOLDEN_PATH: &str = "../sddk-domain/tests/fixtures/project_status.golden.json";
const GRAPH_GOLDEN_PATH: &str = "../sddk-domain/tests/fixtures/project_graph.golden.json";

fn load_fixture() -> Vec<u8> {
    fs::read(FIXTURE_PATH).expect("failed to read spine fixture")
}

fn load_golden(path: &str) -> String {
    fs::read_to_string(path).expect("failed to read golden file")
}

fn import_fixture() -> Storage {
    let tmp = TempDir::new().unwrap();
    let db_path = tmp.path().join("ledger.sqlite");
    let mut storage = Storage::open(&db_path).expect("failed to open storage");
    let bytes = load_fixture();
    import_spine(&bytes, &mut storage).expect("failed to import fixture");
    storage
}

/// Measure the median wall-time (in nanoseconds) of running `op` `runs` times.
fn median_nanos<F>(runs: usize, mut op: F) -> u128
where
    F: FnMut(),
{
    let mut samples: Vec<u128> = Vec::with_capacity(runs);
    for _ in 0..runs {
        let start = std::time::Instant::now();
        op();
        samples.push(start.elapsed().as_nanos());
    }
    samples.sort();
    samples[runs / 2]
}

// ── Determinism ────────────────────────────────────────────────────────────────

/// Projection output is identical across two independent Storage instances.
#[test]
fn perf_deterministic_across_instances() {
    let storage1 = import_fixture();
    let storage2 = import_fixture();

    let snap1 = storage1.snapshot_roadmap().expect("snapshot1");
    let snap2 = storage2.snapshot_roadmap().expect("snapshot2");

    let status1 = project_status(&snap1).expect("status1");
    let status2 = project_status(&snap2).expect("status2");
    assert_eq!(
        serde_json::to_string(&status1).expect("serialize"),
        serde_json::to_string(&status2).expect("serialize"),
        "project_status must be deterministic across instances"
    );

    let graph1 = project_graph(&snap1, GraphFormat::Json).expect("graph1");
    let graph2 = project_graph(&snap2, GraphFormat::Json).expect("graph2");
    assert_eq!(graph1, graph2, "project_graph must be deterministic");
}

/// Projection output matches golden files (verifies pinned fixture correctness).
#[test]
fn perf_matches_golden() {
    let storage = import_fixture();
    let snap = storage.snapshot_roadmap().expect("snapshot");

    let status = project_status(&snap).expect("status");
    let golden = load_golden(STATUS_GOLDEN_PATH);
    assert_eq!(
        serde_json::to_string_pretty(&status).expect("serialize"),
        golden.trim(),
        "project_status must match golden file"
    );

    let graph = project_graph(&snap, GraphFormat::Json).expect("graph");
    let golden_graph = load_golden(GRAPH_GOLDEN_PATH);
    assert_eq!(graph, golden_graph.trim(), "project_graph must match golden file");
}

// ── Soft budget ─────────────────────────────────────────────────────────────────

/// All 4 projections on the pinned fixture: median-of-5 wall-time, advisory 50ms budget.
/// This is a SOFT budget — it logs a warning but NEVER fails the test.
/// Per REQ-PLN4-PRF-001..002 and spec Q10.
#[test]
#[allow(clippy::unit_cmp)]
fn perf_all_projections_soft_budget() {
    let storage = import_fixture();
    let snap = storage.snapshot_roadmap().expect("snapshot");

    let median_ns = median_nanos(5, || {
        let _status = project_status(&snap).expect("status");
        let _next = project_next(&snap).expect("next");
        let _blocked = project_blocked(&snap).expect("blocked");
        let _graph = project_graph(&snap, GraphFormat::Json).expect("graph");
    });

    let median_ms = median_ns / 1_000_000;
    eprintln!(
        "INFO: projections_perf: median-of-5 wall-time = {}ms (advisory budget: 50ms)",
        median_ms
    );

    if median_ms > 50 {
        eprintln!(
            "WARNING: projections_perf: median-of-5 wall-time {}ms exceeds advisory 50ms budget (hardware-dependent; NOT a test failure)",
            median_ms
        );
    }

    // Logical assertion only — determinism equality, not time
    let snap2 = storage.snapshot_roadmap().expect("snapshot2");
    let status1 = project_status(&snap).expect("status1");
    let status2 = project_status(&snap2).expect("status2");
    assert_eq!(
        serde_json::to_string(&status1).unwrap(),
        serde_json::to_string(&status2).unwrap(),
        "snapshots must be equal (determinism invariant)"
    );
}

/// Double-import: median-of-5 wall-time, advisory 200ms budget.
/// This is a SOFT budget — logs a warning but NEVER fails.
#[test]
fn perf_double_import_soft_budget() {
    let tmp = TempDir::new().unwrap();
    let db_path = tmp.path().join("ledger.sqlite");
    let mut storage = Storage::open(&db_path).expect("failed to open storage");

    let bytes = load_fixture();
    import_spine(&bytes, &mut storage).expect("first import");

    let median_ns = median_nanos(5, || {
        import_spine(&bytes, &mut storage).expect("second import");
    });

    let median_ms = median_ns / 1_000_000;
    eprintln!(
        "INFO: projections_perf: double-import median-of-5 = {}ms (advisory budget: 200ms)",
        median_ms
    );

    if median_ms > 200 {
        eprintln!(
            "WARNING: projections_perf: double-import median-of-5 {}ms exceeds advisory 200ms (hardware-dependent; NOT a test failure)",
            median_ms
        );
    }
}
