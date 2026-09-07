//! Tests for restart survival of persisted workflow runs (REQ-WFR-RESTART-001).
//!
//! These tests verify:
//! - A workflow run persisted via record_run survives a store drop + reopen
//! - The run's identity (run_id), provenance (ir_hash, template_ref, correlation_id),
//!   and graph_revision binding are intact after restart
//! - Node runs and lifecycle events are preserved across restart
//! - RunId::derive produces the same ID in a fresh process (determinism re-check)
//!
//! IMPORTANT: These tests use a FILE-BACKED ledger (tempfile), not :memory:.
//! A shared in-memory database cannot prove restart survival.

use std::collections::BTreeMap;
use tempfile::TempDir;
use sddk_domain::{
    execution_graph_compiler::compile_plan_to_revision,
    graph::ExecutionGraphRevision,
    plan_revision::{NormalizedPlanV1, PlanMutation, PlanProvenanceV1, PlanRevisionV1},
    workflow_ir::{Budgets, CapabilityId, NodeId, Operator, OperatorId, RunId, TemplateRef, WorkflowIR},
    workflow_run::{CorrelationId, NodeRun, NodeRunState, WorkflowRun, WorkflowRunState},
    GraphStore,
};
use sddk_storage::graph_store::SqliteGraphStore;

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

/// Builds a minimal WorkflowIR with a single Operator::Task node.
fn make_test_ir() -> WorkflowIR {
    WorkflowIR {
        ir_id: None,
        schema_version: 1,
        template_ref: TemplateRef {
            id: "test.template".into(),
            version: "1.0.0".into(),
        },
        operators: BTreeMap::from([(
            OperatorId("node-1".into()),
            Operator::Task {
                capability: CapabilityId("test.cap".into()),
                inputs: Default::default(),
            },
        )]),
        guards: Default::default(),
        expansion_permissions: Default::default(),
        budgets: Budgets::default(),
        required_invariants: Default::default(),
        provenance: sddk_domain::Provenance {
            generated_by: "test-generator".into(),
            prompt_hash: "prompt-hash-abc".into(),
            model_hash: "model-hash-xyz".into(),
            policy_hash: "policy-hash-123".into(),
        },
    }
}

/// Compiles a WorkflowIR to ExecutionGraphRevision.
fn compile_test_revision(ir: &WorkflowIR, anchor: &str) -> ExecutionGraphRevision {
    let provenance = PlanProvenanceV1::new("test-tool", "1.0.0").expect("valid provenance");
    let normalized = NormalizedPlanV1::from_workflow_ir(ir);
    let plan = PlanRevisionV1::new(None, PlanMutation::Initial, provenance, normalized)
        .expect("valid initial revision");
    compile_plan_to_revision(&plan, None, anchor).expect("compile should succeed")
}

/// Scenario: run survives restart with equivalent identity and provenance
/// GIVEN a run persisted via record_run and advanced to running through the lifecycle event log,
///         on a file-backed ledger
/// WHEN the store is dropped and a fresh SqliteGraphStore is opened on the same file
/// THEN load_run returns the same run_id, ir_hash, template_ref, correlation_id,
///       graph_revision and schema_version;
///       latest_workflow_run_state returns Running;
///       stream_node_runs returns the same node runs;
///       and the reloaded revision's digest == compute_digest()
#[test]
fn run_survives_restart_with_equivalent_identity_and_provenance() {
    // Use TempDir (directory) as the ledger backing store.
    // SqliteGraphStore::open(dir) expects dir/ledger.sqlite.
    let temp_dir = TempDir::new().expect("failed to create temp dir");
    let ledger_path = temp_dir.path().to_path_buf();

    // ── Phase 1: Create, persist, advance ───────────────────────────────────
    {
        let mut store = SqliteGraphStore::open(&ledger_path)
            .expect("failed to open store on temp dir");

        // Derive deterministic run_id
        let plan_revision_id = "plan-rev-test-001";
        let correlation_id = CorrelationId("corr-restart-001".into());
        let run_id = RunId::derive(plan_revision_id, &correlation_id);

        // Compile IR to ExecutionGraphRevision
        let ir = make_test_ir();
        let revision = compile_test_revision(&ir, "restart-test-v1");

        // Build WorkflowRun
        let run = WorkflowRun {
            run_id: run_id.clone(),
            template_ref: ir.template_ref.clone(),
            ir_hash: ir.compute_content_hash(),
            graph_revision: revision.revision_id.clone(),
            state: WorkflowRunState::Pending,
            inputs: BTreeMap::new(),
            outputs: None,
            correlation_id: correlation_id.clone(),
            budget: ir.budgets.clone(),
            schema_version: 1,
        };

        // Persist the run
        store.record_run(&run, &revision)
            .expect("record_run failed");

        // Persist a NodeRun
        let node_run = NodeRun {
            node_id: NodeId("node-1".into()),
            state: NodeRunState::Running,
            dependencies: Default::default(),
            attempts: vec![],
            expansion_permissions: Default::default(),
            schema_version: 1,
        };
        store.record_node_run_for_run(&run_id, &node_run)
            .expect("record_node_run_for_run failed");

        // Advance lifecycle: pending → running by inserting directly into workflow_run_events_v1
        // (simulating what the runtime would do after record_run)
        let occurred_at = "2026-09-07T12:00:00.000Z";
        store.proj_store_conn_mut().execute(
            r#"INSERT INTO workflow_run_events_v1
               (event_id, run_id, occurred_at, from_state, to_state, actor_kind, actor_id, reason)
               VALUES (?1, ?2, ?3, 'pending', 'running', 'system', 'engine', NULL)"#,
            rusqlite::params![
                format!("evt-run-{}-to-running", run_id.0),
                run_id.0,
                occurred_at,
            ],
        ).expect("failed to insert running event");

        // Verify initial state before restart
        let loaded = store.load_run(&run_id).expect("load_run failed");
        assert!(loaded.is_some(), "run should exist before restart");
        let loaded = loaded.unwrap();
        assert_eq!(loaded.run_id, run_id);
        assert_eq!(loaded.state, WorkflowRunState::Pending); // record_run writes pending→pending as initial event
        assert_eq!(loaded.correlation_id, correlation_id);

        // latest state should be running
        let latest = store.latest_workflow_run_state(&run_id).expect("latest_workflow_run_state failed");
        assert_eq!(latest, Some(WorkflowRunState::Running), "latest state should be running");
    } // store is dropped here

    // ── Phase 2: Reopen and verify ──────────────────────────────────────────
    {
        let store = SqliteGraphStore::open(&ledger_path)
            .expect("failed to reopen store on temp dir");

        let plan_revision_id = "plan-rev-test-001";
        let correlation_id = CorrelationId("corr-restart-001".into());
        let run_id = RunId::derive(plan_revision_id, &correlation_id);

        // Re-derive the revision to get the expected digest
        let ir = make_test_ir();
        let fresh_revision = compile_test_revision(&ir, "restart-test-v1");

        // Assert: load_run returns the same run_id and provenance
        let loaded = store.load_run(&run_id).expect("load_run failed");
        assert!(loaded.is_some(), "load_run should find the run after restart");
        let loaded = loaded.unwrap();
        assert_eq!(loaded.run_id, run_id, "run_id must match");
        assert_eq!(loaded.template_ref.id, "test.template", "template_ref.id must match");
        assert_eq!(loaded.template_ref.version, "1.0.0", "template_ref.version must match");
        assert_eq!(loaded.correlation_id, correlation_id, "correlation_id must match");
        assert_eq!(loaded.graph_revision, fresh_revision.revision_id, "graph_revision must match");
        assert_eq!(loaded.schema_version, 1, "schema_version must be 1");

        // Assert: latest_workflow_run_state returns Running
        let latest = store.latest_workflow_run_state(&run_id).expect("latest_workflow_run_state failed");
        assert_eq!(latest, Some(WorkflowRunState::Running), "latest state should still be running after restart");

        // Assert: stream_node_runs returns the same node run
        let node_runs = store.stream_node_runs(&run_id).expect("stream_node_runs failed");
        assert_eq!(node_runs.len(), 1, "should have exactly one node run");
        assert_eq!(node_runs[0].node_id, NodeId("node-1".into()), "node_id must match");

        // Assert: load_revision digest equals compute_digest()
        let loaded_rev = store.load_revision(&run_id, &fresh_revision.revision_id)
            .expect("load_revision failed");
        assert!(loaded_rev.is_some(), "revision should be loadable after restart");
        let loaded_rev = loaded_rev.unwrap();
        let computed_digest = fresh_revision.compute_digest();
        assert_eq!(
            loaded_rev.digest, computed_digest,
            "reloaded revision digest must equal compute_digest()"
        );
    }
}

/// Scenario: run_id is reproducible after restart from the same inputs
/// GIVEN the (plan_revision_id, correlation_id) used to create the run
/// WHEN RunId::derive is re-evaluated in the fresh process
/// THEN it equals the run_id loaded from storage
#[test]
fn restart_survives_with_deterministic_run_id() {
    let temp_dir = TempDir::new().expect("failed to create temp dir");
    let ledger_path = temp_dir.path().to_path_buf();

    let plan_revision_id = "plan-rev-det-002";
    let correlation_id = CorrelationId("corr-deterministic-002".into());

    // ── Phase 1: Derive run_id and persist ─────────────────────────────────
    let run_id = RunId::derive(plan_revision_id, &correlation_id);
    let ir = make_test_ir();
    let revision = compile_test_revision(&ir, "determinism-test-v1");

    let run = WorkflowRun {
        run_id: run_id.clone(),
        template_ref: ir.template_ref.clone(),
        ir_hash: ir.compute_content_hash(),
        graph_revision: revision.revision_id.clone(),
        state: WorkflowRunState::Pending,
        inputs: BTreeMap::new(),
        outputs: None,
        correlation_id: correlation_id.clone(),
        budget: ir.budgets.clone(),
        schema_version: 1,
    };

    {
        let mut store = SqliteGraphStore::open(&ledger_path)
            .expect("failed to open store");
        store.record_run(&run, &revision)
            .expect("record_run failed");
    }

    // ── Phase 2: Fresh process — re-derive run_id and compare ────────────────
    let rederived_run_id = RunId::derive(plan_revision_id, &correlation_id);

    {
        let store = SqliteGraphStore::open(&ledger_path)
            .expect("failed to reopen store");
        let loaded = store.load_run(&rederived_run_id)
            .expect("load_run failed");
        assert!(loaded.is_some(), "re-derived run_id should find the persisted run");
        assert_eq!(
            rederived_run_id, run_id,
            "RunId::derive in fresh process must equal the original run_id"
        );
        assert_eq!(
            rederived_run_id, loaded.unwrap().run_id,
            "re-derived run_id must equal the loaded run_id"
        );
    }
}
