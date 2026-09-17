//! Probes for R1: `WorkflowRunState` survives a store drop + reopen.
//!
//! Three tests that exercise the same scenario through different entry
//! points. All three must hold for cycle-A5-2 to consider R1 closed:
//!
//! - A5-2-R1-public: drive the lifecycle via the public API only
//!   (`record_run`, `record_workflow_run_transition`). RED on main because
//!   `record_workflow_run_transition` does not update
//!   `workflow_runs_v1.state`.
//! - A5-2-R1-direct-event: drive the lifecycle by inserting events
//!   directly into `workflow_run_events_v1` (mirrors what the existing
//!   ignored test does). Asserts that `latest_workflow_run_state`
//!   recovers the new state from the log and that `load_run` reads the
//!   snapshot — exposing the asymmetry between the two reads.
//! - A5-2-R1-chained-transitions: Pending -> Running -> Completed,
//!   assert both `load_run.state` and `latest_workflow_run_state` reflect
//!   Completed after the third commit.
//!
//! Refer to `docs/architecture/a5/A5-2-PLAN.md` (F1, F2, F3).

use sddk_domain::{
    GraphStore,
    execution_graph_compiler::compile_plan_to_revision,
    plan_revision::{NormalizedPlanV1, PlanMutation, PlanProvenanceV1, PlanRevisionV1},
    workflow_ir::{Budgets, CapabilityId, Operator, OperatorId, RunId, TemplateRef, WorkflowIR},
    workflow_run::{CorrelationId, WorkflowRun, WorkflowRunState},
};
use sddk_storage::graph_store::SqliteGraphStore;
use std::collections::BTreeMap;
use tempfile::TempDir;

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

fn compile_test_revision(
    ir: &WorkflowIR,
    anchor: &str,
) -> sddk_domain::graph::ExecutionGraphRevision {
    let provenance = PlanProvenanceV1::new("test-tool", "1.0.0").expect("valid provenance");
    let normalized = NormalizedPlanV1::from_workflow_ir(ir);
    let plan = PlanRevisionV1::new(None, PlanMutation::Initial, provenance, normalized)
        .expect("valid initial revision");
    compile_plan_to_revision(&plan, None, anchor).expect("compile should succeed")
}

fn build_run(
    plan_revision_id: &str,
    correlation: &str,
    ir: &WorkflowIR,
    revision: &sddk_domain::graph::ExecutionGraphRevision,
) -> WorkflowRun {
    let correlation_id = CorrelationId(correlation.into());
    let run_id = RunId::derive(plan_revision_id, &correlation_id);
    WorkflowRun {
        run_id,
        template_ref: ir.template_ref.clone(),
        ir_hash: ir.compute_content_hash(),
        graph_revision: revision.revision_id.clone(),
        state: WorkflowRunState::Pending,
        inputs: BTreeMap::new(),
        outputs: None,
        correlation_id,
        budget: ir.budgets.clone(),
        schema_version: 1,
    }
}

// ───────────────────────────────────────────────────────────────────────
// A5-2-R1-public: lifecycle through the public API only.
// RED on main until `record_workflow_run_transition` writes
// `workflow_runs_v1.state` in the same transaction.
// ───────────────────────────────────────────────────────────────────────
#[test]
fn a5_2_r1_public_state_survives_restart_after_running_transition() {
    let temp_dir = TempDir::new().expect("tempdir");
    let ledger_path = temp_dir.path().to_path_buf();

    let plan_revision_id = "plan-rev-a5-2-r1-public";
    let correlation = "corr-a5-2-r1-public";

    {
        // Phase 1: create the run in Pending.
        let mut store = SqliteGraphStore::open(&ledger_path).expect("open store");
        let ir = make_test_ir();
        let revision = compile_test_revision(&ir, "a5-2-r1-public-v1");
        let run = build_run(plan_revision_id, correlation, &ir, &revision);
        store.record_run(&run, &revision).expect("record_run");

        // Advance lifecycle through the PUBLIC API (not raw SQL).
        store
            .record_workflow_run_transition(
                &run.run_id,
                WorkflowRunState::Pending,
                WorkflowRunState::Running,
                None,
            )
            .expect("transition");
    } // store dropped — process ends here.

    {
        let store = SqliteGraphStore::open(&ledger_path).expect("reopen store");
        // Re-derive run_id from canonical inputs (the protocol works).
        let correlation_id = CorrelationId(correlation.into());
        let run_id = RunId::derive(plan_revision_id, &correlation_id);

        // The log knows.
        let latest = store
            .latest_workflow_run_state(&run_id)
            .expect("latest state from log");
        assert_eq!(
            latest,
            Some(WorkflowRunState::Running),
            "event log should carry the Running transition"
        );

        // The snapshot row must agree. This is the bug on main.
        let loaded = store
            .load_run(&run_id)
            .expect("load_run")
            .expect("row exists");
        assert_eq!(
            loaded.state,
            WorkflowRunState::Running,
            "workflow_runs_v1.state must reflect the latest transition after restart \
             (R1 — see crates/sddk-storage/src/graph_store.rs:571)"
        );
    }
}

// ───────────────────────────────────────────────────────────────────────
// A5-2-R1-direct-event: drive events through raw SQL like the existing
// ignored test does. After F2 the only way `load_run.state` may differ
// from the log's latest is if the call site bypasses
// `record_workflow_run_transition`; here we explicitly do, and pin the
// (still-correct) behaviour: `latest_workflow_run_state` is the
// authoritative read; `load_run.state` falls back to the snapshot.
//
// ───────────────────────────────────────────────────────────────────────
#[test]
fn a5_2_r1_direct_event_load_run_state_falls_back_to_snapshot_after_bypass() {
    let temp_dir = TempDir::new().expect("tempdir");
    let ledger_path = temp_dir.path().to_path_buf();
    let plan_revision_id = "plan-rev-a5-2-r1-direct";
    let correlation = "corr-a5-2-r1-direct";
    let correlation_id = CorrelationId(correlation.into());

    {
        let mut store = SqliteGraphStore::open(&ledger_path).expect("open store");
        let ir = make_test_ir();
        let revision = compile_test_revision(&ir, "a5-2-r1-direct-v1");
        let run = build_run(plan_revision_id, correlation, &ir, &revision);
        store.record_run(&run, &revision).expect("record_run");

        // Raw insertion — exactly what the existing ignored test does to
        // advance the lifecycle. With this path there is NO UPDATE on
        // workflow_runs_v1 (the table is append-only by trigger), so the
        // snapshot stays at Pending.
        store.proj_store_conn_mut().execute(
            r#"INSERT INTO workflow_run_events_v1
               (event_id, run_id, occurred_at, from_state, to_state, actor_kind, actor_id, reason)
               VALUES (?1, ?2, '2030-01-01T00:00:00.000Z', 'pending', 'running', 'system', 'engine', NULL)"#,
            rusqlite::params![format!("evt-{}-to-running", run.run_id.0), run.run_id.0],
        ).expect("raw event insert");
    }

    let run_id = RunId::derive(plan_revision_id, &correlation_id);
    let store = SqliteGraphStore::open(&ledger_path).expect("reopen");

    let latest = store.latest_workflow_run_state(&run_id).expect("latest");
    assert_eq!(latest, Some(WorkflowRunState::Running));

    let loaded = store
        .load_run(&run_id)
        .expect("load_run")
        .expect("row exists");
    // After F2, `load_run.state` reads from the event log via the
    // shared helper, so even raw SQL insertions (e.g. test scaffolding)
    // keep `load_run.state` consistent with `latest_workflow_run_state`.
    assert_eq!(
        loaded.state,
        WorkflowRunState::Running,
        "shared helper: load_run.state is sourced from the event log"
    );
}

// ───────────────────────────────────────────────────────────────────────
// A5-2-R1-chained: Pending -> Running -> Completed through the public
// API. After two transitions and one restart, both reads agree.
// ───────────────────────────────────────────────────────────────────────
#[test]
fn a5_2_r1_chained_public_two_transitions_survive_restart() {
    let temp_dir = TempDir::new().expect("tempdir");
    let ledger_path = temp_dir.path().to_path_buf();
    let plan_revision_id = "plan-rev-a5-2-r1-chain";
    let correlation = "corr-a5-2-r1-chain";
    let run_id = RunId::derive(plan_revision_id, &CorrelationId(correlation.into()));

    {
        let mut store = SqliteGraphStore::open(&ledger_path).expect("open");
        let ir = make_test_ir();
        let revision = compile_test_revision(&ir, "a5-2-r1-chain-v1");
        let run = build_run(plan_revision_id, correlation, &ir, &revision);
        store.record_run(&run, &revision).expect("record_run");
        store
            .record_workflow_run_transition(
                &run_id,
                WorkflowRunState::Pending,
                WorkflowRunState::Running,
                Some("started"),
            )
            .expect("transition 1");
        store
            .record_workflow_run_transition(
                &run_id,
                WorkflowRunState::Running,
                WorkflowRunState::Completed,
                Some("finished"),
            )
            .expect("transition 2");
    }

    let store = SqliteGraphStore::open(&ledger_path).expect("reopen");
    let latest = store.latest_workflow_run_state(&run_id).expect("latest");
    assert_eq!(latest, Some(WorkflowRunState::Completed));
    let loaded = store.load_run(&run_id).expect("load_run").expect("row");
    assert_eq!(
        loaded.state,
        WorkflowRunState::Completed,
        "two transitions, then restart: load_run.state must reach Completed (R1)"
    );
}
