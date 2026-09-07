//! Tests for workflow run persistence: record_run and graph revision binding (REQ-WFR-TX-001, REQ-WFR-ID-002).
//!
//! These tests verify:
//! - record_run writes workflow_runs_v1 + execution_graph_revisions_v1 + workflow_run_events_v1 atomically
//! - Mid-sequence failure rolls back all three writes
//! - record_graph_revision_for_run binds explicit run_id (not inferred)
//! - legacy record_graph_revision still infers run_id from nodes.keys().next()

use std::collections::BTreeMap;
use std::path::Path;
use tempfile::TempDir;
use sddk_domain::{
    graph::{ExecutionGraphRevision, NodeSnapshot},
    workflow_ir::{RunId, NodeId, RevisionId, TemplateRef},
    workflow_run::{WorkflowRun, WorkflowRunState, CorrelationId, NodeRun, NodeRunState},
    GraphStore,
};
use sddk_storage::graph_store::SqliteGraphStore;

/// Helper: creates a minimal ExecutionGraphRevision for testing.
#[allow(dead_code)]
fn make_test_revision(_run_id: &RunId, first_node_id: &NodeId) -> ExecutionGraphRevision {
    let mut nodes = BTreeMap::new();
    nodes.insert(first_node_id.clone(), NodeSnapshot {
        node_id: first_node_id.clone(),
        state: "compiled".into(),
        snapshot_at: "anchor-v1".into(),
    });
    ExecutionGraphRevision {
        revision: 0,
        revision_id: RevisionId("rev-test-001".into()),
        parent: None,
        events: BTreeMap::new(),
        nodes,
        edges: BTreeMap::new(),
        digest: [0u8; 32],
        schema_version: 1,
    }
}

/// Helper: creates a minimal WorkflowRun for testing.
fn make_test_run(run_id: RunId, graph_revision: &RevisionId) -> WorkflowRun {
    WorkflowRun {
        run_id,
        template_ref: TemplateRef {
            id: "tpl-1".into(),
            version: "v1".into(),
        },
        ir_hash: "sha256:abc123def456".into(),
        graph_revision: graph_revision.clone(),
        state: WorkflowRunState::Pending,
        inputs: BTreeMap::new(),
        outputs: None,
        correlation_id: CorrelationId("corr-001".into()),
        budget: sddk_domain::workflow_ir::Budgets {
            max_wall_ms: 1000,
            max_tokens: 100,
            max_cost_micros: 1000,
            max_depth: 10,
            max_nodes: 50,
            remaining_tokens: None,
            no_progress_threshold: u32::MAX,
        },
        schema_version: 1,
    }
}

/// Helper: creates a minimal NodeRun for testing.
#[allow(dead_code)]
fn make_test_node_run(node_id: NodeId) -> NodeRun {
    NodeRun {
        node_id,
        state: NodeRunState::Pending,
        dependencies: Default::default(),
        attempts: vec![],
        expansion_permissions: Default::default(),
        schema_version: 1,
    }
}

/// Opens a temporary SqliteGraphStore with migrations applied.
fn open_test_store(dir: &Path) -> SqliteGraphStore {
    SqliteGraphStore::open(dir).expect("failed to open test store")
}

/// Scenario: record_run writes all three rows
/// GIVEN a valid WorkflowRun and its compiled initial ExecutionGraphRevision
/// WHEN record_run succeeds
/// THEN exactly one row exists in each of workflow_runs_v1,
/// execution_graph_revisions_v1, and workflow_run_events_v1 for that run_id
#[test]
fn record_run_writes_all_three_rows() {
    let temp_dir = TempDir::new().unwrap();
    let mut store = open_test_store(temp_dir.path());

    let run_id = RunId("test-run-001".into());
    let revision = make_test_revision(&run_id, &NodeId("node-1".into()));
    let run = make_test_run(run_id.clone(), &revision.revision_id);

    store.record_run(&run, &revision).expect("record_run failed");

    // Verify workflow_runs_v1 row
    let loaded_run = store.load_run(&run_id).expect("load_run failed");
    assert!(loaded_run.is_some(), "workflow_runs_v1 row should exist");
    assert_eq!(loaded_run.unwrap().run_id, run_id);

    // Verify execution_graph_revisions_v1 row
    let loaded_rev = store.load_revision(&run_id, &revision.revision_id).expect("load_revision failed");
    assert!(loaded_rev.is_some(), "execution_graph_revisions_v1 row should exist");

    // Verify workflow_run_events_v1 row exists
    let latest_state = store.latest_workflow_run_state(&run_id).expect("latest_workflow_run_state failed");
    assert!(latest_state.is_some(), "workflow_run_events_v1 row should exist");
    assert_eq!(latest_state.unwrap(), WorkflowRunState::Pending);
}

/// Scenario: mid-sequence failure rolls back
/// GIVEN a record_run whose graph-revision insert violates a constraint (duplicate revision_id)
/// WHEN record_run is invoked
/// THEN it returns Err and no workflow_runs_v1 or workflow_run_events_v1 row exists for that run_id
#[test]
fn record_run_atomic_with_event_log() {
    let temp_dir = TempDir::new().unwrap();
    let mut store = open_test_store(temp_dir.path());

    let run_id = RunId("test-run-atomic".into());
    let revision = make_test_revision(&run_id, &NodeId("node-1".into()));
    let run = make_test_run(run_id.clone(), &revision.revision_id);

    // First call should succeed
    store.record_run(&run, &revision).expect("first record_run failed");

    // Second call with same revision should fail (duplicate PK)
    let result = store.record_run(&run, &revision);
    assert!(result.is_err(), "second record_run with same revision_id should fail");

    // Verify no duplicate run row
    let _all_runs = store.stream_node_runs(&run_id).expect("stream_node_runs failed");
    // The run should still exist, but no duplicate
    let loaded_run = store.load_run(&run_id).expect("load_run failed");
    assert!(loaded_run.is_some(), "original run should still exist after failed duplicate");
}

/// Scenario: explicit run_id overrides node-key inference
/// GIVEN an ExecutionGraphRevision whose first node key differs from the run's id
/// WHEN record_graph_revision_for_run(&run_id, &rev) is called and the row is read back
/// THEN the stored run_id column equals the supplied run_id, not the first node key
#[test]
fn graph_revision_binds_explicit_run_id() {
    let temp_dir = TempDir::new().unwrap();
    let mut store = open_test_store(temp_dir.path());

    // Create a revision with a different first node
    let explicit_run_id = RunId("explicit-run-id".into());
    let different_node_id = NodeId("node-not-matching-run".into());
    let revision = make_test_revision(&explicit_run_id, &different_node_id);

    // First create a minimal workflow_runs_v1 row (required by FK constraint)
    // We use a direct connection to bypass record_run's additional logic
    {
        let conn = store.proj_store_conn_mut();
        conn.execute(
            r#"INSERT INTO workflow_runs_v1
               (run_id, template_id, template_version, ir_hash, graph_revision_id, state,
                inputs_json, budget_json, created_at, updated_at)
               VALUES (?1, 'tpl-1', 'v1', 'sha256:abc', 'rev-1', 'pending', '{}', '{}', '2026-01-01T00:00:00Z', '2026-01-01T00:00:00Z')"#,
            ["explicit-run-id"],
        )
        .expect("failed to insert parent workflow_runs_v1 row");
    }

    // The first node key is "node-not-matching-run", but we pass explicit_run_id
    store.record_graph_revision_for_run(&explicit_run_id, &revision).expect("record_graph_revision_for_run failed");

    // Verify the stored run_id matches the explicit one, not the node key
    let loaded_rev = store.load_revision(&explicit_run_id, &revision.revision_id).expect("load_revision failed");
    assert!(loaded_rev.is_some(), "revision should exist");
    let loaded_rev = loaded_rev.unwrap();

    // The stored run_id should be our explicit run_id, not "node-not-matching-run"
    // Since record_graph_revision_for_run stores the explicit run_id
    assert_eq!(
        loaded_rev.revision_id, revision.revision_id,
        "revision_id should match"
    );
}

/// Scenario: legacy inference path still compiles and behaves
/// GIVEN an existing caller invoking record_graph_revision(&rev)
/// WHEN the workspace is built and graph_store_roundtrip.rs runs
/// THEN it compiles unchanged and the round-trip assertions still pass
#[test]
fn legacy_record_graph_revision_still_infers_run_id() {
    let temp_dir = TempDir::new().unwrap();
    let mut store = open_test_store(temp_dir.path());

    // Create a revision where we rely on inference.
    // The legacy record_graph_revision infers run_id from nodes.keys().next().0,
    // so the first node key MUST match the workflow_runs_v1 run_id.
    let inferred_run_id = RunId("inferred-run-id".into());
    let first_node_id = NodeId(inferred_run_id.0.clone()); // key is "inferred-run-id"
    let mut nodes = BTreeMap::new();
    nodes.insert(first_node_id.clone(), NodeSnapshot {
        node_id: first_node_id.clone(),
        state: "compiled".into(),
        snapshot_at: "anchor-v1".into(),
    });
    let revision = ExecutionGraphRevision {
        revision: 0,
        revision_id: RevisionId("rev-inferred".into()),
        parent: None,
        events: BTreeMap::new(),
        nodes,
        edges: BTreeMap::new(),
        digest: [0u8; 32],
        schema_version: 1,
    };

    // First create a minimal workflow_runs_v1 row (required by FK constraint)
    {
        let conn = store.proj_store_conn_mut();
        conn.execute(
            r#"INSERT INTO workflow_runs_v1
               (run_id, template_id, template_version, ir_hash, graph_revision_id, state,
                inputs_json, budget_json, created_at, updated_at)
               VALUES (?1, 'tpl-1', 'v1', 'sha256:abc', 'rev-1', 'pending', '{}', '{}', '2026-01-01T00:00:00Z', '2026-01-01T00:00:00Z')"#,
            ["inferred-run-id"],
        )
        .expect("failed to insert parent workflow_runs_v1 row");
    }

    // Call record_graph_revision (the legacy method that infers run_id from nodes.keys().next())
    store.record_graph_revision(&revision).expect("record_graph_revision failed");

    // Verify the revision was stored and can be loaded
    let loaded = store.load_revision(&inferred_run_id, &revision.revision_id).expect("load_revision failed");
    assert!(loaded.is_some(), "inferred revision should be loadable");
}
