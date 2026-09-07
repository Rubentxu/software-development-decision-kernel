//! S6b UAT (DW-RUNTIME-005 exit gate) — end-to-end restart-survival on the real store.
//!
//! Exit gate under test (docs/.../02-roadmap/EXECUTION-SPINE.yaml, DW-RUNTIME-005):
//! "Persisted generated DAG can stop, resume and replay to equivalent state under UAT."
//!
//! Scope landed for S6b:
//!   1. A generated multi-node DAG (Sequence of 5 Task leaves) is compiled through the
//!      real DW-RUNTIME-001 pipeline (`compile_plan_to_revision`) and persisted via
//!      `record_run` on a file-backed `SqliteGraphStore`.
//!   2. The DAG executes to an intermediate state: the first "process" drives the run
//!      to `Running` and completes 2 of 5 sequence children, then dies (runtime dropped,
//!      store closed) leaving the lifecycle log at `Running` plus persisted node runs
//!      and attempts.
//!   3. A NEW runtime (fresh store connection on the same persisted ledger, same
//!      deterministic run_id) restarts the run. The persisted log is non-terminal, so
//!      `execute()` replays the DAG deterministically to completion.
//!   4. The replayed terminal state is equivalent to an uninterrupted golden run of the
//!      same DAG: identical `run.state`, `run.outputs`, and node-state map; operator
//!      invocations are deterministic (the replay performs exactly the golden run's
//!      work); attempt rows stay idempotent at the storage level (no duplicates).
//!   5. A run whose persisted log is already TERMINAL is never re-executed: a restart
//!      over a `Completed` log cuts with `Err(RuntimeError::AlreadyTerminal)` and the
//!      shared operator counter proves no operator ran again.
//!
//! S6b resume semantics (as implemented by slice 1+2, db62c67/15ef8fd): a crashed
//! non-terminal run replays from the persisted start to the equivalent terminal state
//! (node-level checkpoint continuation is a later vertical slice). The guard protects
//! terminal runs; replay is idempotent by construction: completed attempts are
//! re-recorded as safe no-ops (`IdempotencyConflict`) instead of failing the run.

use std::collections::BTreeMap;
use std::path::Path;
use std::sync::Arc;
use std::sync::atomic::{AtomicUsize, Ordering};

use sddk_domain::execution_graph_compiler::compile_plan_to_revision;
use sddk_domain::graph::ExecutionGraphRevision;
use sddk_domain::plan_revision::{
    NormalizedPlanV1, PlanMutation, PlanProvenanceV1, PlanRevisionV1,
};
use sddk_domain::workflow_ir::{Budgets, TemplateRef, WorkflowIR};
use sddk_domain::workflow_run::NodeRunState;
use sddk_domain::{
    CapabilityId, CorrelationId, GraphStore, Operator, OperatorId, RunId, TaskError, TaskExecutor,
    TaskOutput, WorkflowRun, WorkflowRunState,
};
use sddk_engine::RuntimeError;
use sddk_engine::operator::Clock;
use sddk_engine::workflow_runtime::{TickOutcome, WorkflowRuntime};
use sddk_storage::SqliteGraphStore;
use serde_json::Value;

/// Deterministic DAG constants. A `Sequence` of 5 Task leaves evaluates exactly one
/// child per tick, so a crash after 2 ticks leaves a real intermediate frontier.
const SEQ_CHILDREN: usize = 5;
const SEQ_ROOT: &str = "seq-root";
const PLAN_REVISION_ID: &str = "plan-s6b-uat";
const CORRELATION_ID: &str = "corr-s6b-uat";

// ── Deterministic executor ─────────────────────────────────────────────────────

/// Echoes `capability|payload` for every invocation and counts calls through a
/// shared counter. Pure function of its inputs: identical across replays.
#[derive(Debug)]
struct DeterministicExecutor {
    calls: Arc<AtomicUsize>,
}

impl TaskExecutor for DeterministicExecutor {
    fn execute(
        &self,
        capability: &str,
        inputs: &BTreeMap<String, Value>,
    ) -> Result<TaskOutput, TaskError> {
        self.calls.fetch_add(1, Ordering::SeqCst);
        let payload = inputs
            .get("payload")
            .and_then(Value::as_str)
            .unwrap_or("none")
            .to_string();
        let mut outputs = BTreeMap::new();
        outputs.insert(
            "out".into(),
            Value::String(format!("{capability}|{payload}")),
        );
        Ok(TaskOutput { outputs })
    }
}

// ── Generated DAG: Sequence of 5 Task leaves ──────────────────────────────────

fn build_generated_dag() -> (WorkflowIR, ExecutionGraphRevision) {
    let mut operators = BTreeMap::new();
    for i in 0..SEQ_CHILDREN {
        let id = format!("task-{i}");
        let mut inputs = BTreeMap::new();
        inputs.insert("payload".into(), Value::String(format!("seed-{i}")));
        operators.insert(
            OperatorId(id.clone()),
            Operator::Task {
                capability: CapabilityId(format!("cap-{i}")),
                inputs,
            },
        );
    }
    operators.insert(
        OperatorId(SEQ_ROOT.into()),
        Operator::Sequence {
            body: (0..SEQ_CHILDREN)
                .map(|i| OperatorId(format!("task-{i}")))
                .collect(),
        },
    );

    let ir = WorkflowIR {
        ir_id: None,
        schema_version: 1,
        template_ref: TemplateRef {
            id: "s6b-uat-restart-survival".into(),
            version: "0.1.0".into(),
        },
        operators,
        guards: Default::default(),
        expansion_permissions: Default::default(),
        budgets: Budgets {
            max_wall_ms: u64::MAX,
            max_tokens: u64::MAX,
            max_cost_micros: u64::MAX,
            max_depth: 8,
            max_nodes: 16,
            remaining_tokens: None,
            no_progress_threshold: u32::MAX,
        },
        required_invariants: Default::default(),
        provenance: sddk_domain::Provenance {
            generated_by: "s6b-uat".into(),
            prompt_hash: "test".into(),
            model_hash: "test".into(),
            policy_hash: "test".into(),
        },
    };

    // Real generated-DAG pipeline (DW-RUNTIME-001): compile the IR into a frozen
    // ExecutionGraphRevision with deterministic identity before persisting.
    let provenance = PlanProvenanceV1::new("s6b-uat", "1.0.0").expect("valid provenance");
    let normalized = NormalizedPlanV1::from_workflow_ir(&ir);
    let plan = PlanRevisionV1::new(None, PlanMutation::Initial, provenance, normalized)
        .expect("valid initial revision");
    let compiled =
        compile_plan_to_revision(&plan, None, "s6b-uat-v1").expect("compile should succeed");
    (ir, compiled)
}

// ── Persistence + runtime helpers ─────────────────────────────────────────────

/// Opens the store on `dir`, persists the run row + initial revision exactly as a
/// first `execute()` would (REQ-WFR3-PERSIST-001), and returns the derived run_id
/// together with the open store (needed to append lifecycle transitions manually).
fn seed_persisted_run(
    dir: &Path,
    ir: &WorkflowIR,
    compiled: &ExecutionGraphRevision,
) -> (RunId, SqliteGraphStore) {
    let correlation_id = CorrelationId(CORRELATION_ID.into());
    let run_id = RunId::derive(PLAN_REVISION_ID, &correlation_id);
    let mut store = SqliteGraphStore::open(dir).expect("open store");
    let run = WorkflowRun {
        run_id: run_id.clone(),
        template_ref: ir.template_ref.clone(),
        ir_hash: format!("sha256:{}", ir.compute_content_hash()),
        graph_revision: compiled.revision_id.clone(),
        state: WorkflowRunState::Pending,
        inputs: Default::default(),
        outputs: None,
        correlation_id,
        budget: ir.budgets.clone(),
        schema_version: 1,
    };
    store.record_run(&run, compiled).expect("record_run");
    (run_id, store)
}

fn build_runtime(
    dir: &Path,
    ir: WorkflowIR,
    compiled: &ExecutionGraphRevision,
    calls: Arc<AtomicUsize>,
) -> WorkflowRuntime {
    let store = SqliteGraphStore::open(dir).expect("open store");
    let executor: Arc<dyn TaskExecutor> = Arc::new(DeterministicExecutor { calls });
    WorkflowRuntime::from_compiled(
        ir,
        store,
        Clock,
        executor,
        PLAN_REVISION_ID,
        &CorrelationId(CORRELATION_ID.into()),
        compiled,
    )
}

fn assert_completed(rt: &WorkflowRuntime) {
    assert_eq!(
        rt.run().state,
        WorkflowRunState::Completed,
        "run should be Completed"
    );
}

/// Snapshot of the node-state map (operator id -> state). Node attempt HISTORY is
/// intentionally excluded: a replay legitimately re-records attempts (idempotent at
/// the storage level), so equivalence is judged on node states + run.outputs.
fn node_state_snapshot(rt: &WorkflowRuntime) -> BTreeMap<String, NodeRunState> {
    rt.nodes()
        .iter()
        .map(|(op, node)| (op.0.clone(), node.state.clone()))
        .collect()
}

fn outputs_snapshot(rt: &WorkflowRuntime) -> Option<BTreeMap<String, Value>> {
    rt.run().outputs.clone()
}

// ═══════════════════════════════════════════════════════════════════════════════
// Test 1 — stop mid-flight, restart replays to an equivalent terminal state
// ═══════════════════════════════════════════════════════════════════════════════

#[test]
fn e2e_stop_midflight_restart_replays_to_equivalent_terminal_state() {
    let (ir, compiled) = build_generated_dag();
    let correlation_id = CorrelationId(CORRELATION_ID.into());
    let run_id = RunId::derive(PLAN_REVISION_ID, &correlation_id);

    // ── Golden baseline: uninterrupted run on its own ledger ──────────────────
    let golden_dir = tempfile::TempDir::new_in("/tmp").expect("temp dir");
    let (_golden_run_id, _seed_store) = seed_persisted_run(golden_dir.path(), &ir, &compiled);
    let golden_calls = Arc::new(AtomicUsize::new(0));
    let golden_outputs;
    let golden_nodes;
    let golden_call_count;
    {
        let mut golden_rt = build_runtime(
            golden_dir.path(),
            ir.clone(),
            &compiled,
            golden_calls.clone(),
        );
        golden_rt.execute().expect("golden execute should succeed");
        assert_completed(&golden_rt);
        golden_outputs = outputs_snapshot(&golden_rt);
        golden_nodes = node_state_snapshot(&golden_rt);
        golden_call_count = golden_calls.load(Ordering::SeqCst);
    }
    assert!(
        golden_outputs.as_ref().is_some_and(|o| !o.is_empty()),
        "golden run.outputs should aggregate the 5 task outputs"
    );

    // ── Interrupted run: same DAG on a second ledger ───────────────────────────
    let crash_dir = tempfile::TempDir::new_in("/tmp").expect("temp dir");
    let calls = Arc::new(AtomicUsize::new(0));
    let (persisted_run_id, mut seed_store) = seed_persisted_run(crash_dir.path(), &ir, &compiled);
    assert_eq!(persisted_run_id, run_id, "run identity is deterministic");

    let calls_leg1;
    {
        let mut rt1 = build_runtime(crash_dir.path(), ir.clone(), &compiled, calls.clone());
        // Emulate the execute() entry transition (Pending -> Running) that the
        // crashed process had already recorded, then drive 2 of 5 children.
        seed_store
            .record_workflow_run_transition(
                &run_id,
                WorkflowRunState::Pending,
                WorkflowRunState::Running,
                None,
            )
            .expect("record Pending -> Running");
        rt1.start().expect("start");
        let t1 = rt1.tick().expect("tick 1");
        assert!(matches!(t1, TickOutcome::Running), "tick 1: {t1:?}");
        let t2 = rt1.tick().expect("tick 2");
        assert!(matches!(t2, TickOutcome::Running), "tick 2: {t2:?}");
        calls_leg1 = calls.load(Ordering::SeqCst);
        assert!(
            calls_leg1 < golden_call_count,
            "leg 1 must stop mid-flight ({} calls < golden {})",
            calls_leg1,
            golden_call_count
        );
        // Process death: the runtime and its store connection are dropped here.
    }

    // Persisted evidence of the intermediate state, read back from disk.
    let check_store = SqliteGraphStore::open(crash_dir.path()).expect("re-open store");
    assert_eq!(
        check_store
            .latest_workflow_run_state(&run_id)
            .expect("latest_workflow_run_state"),
        Some(WorkflowRunState::Running),
        "event log must report Running (non-terminal) after the crash"
    );
    let seq_node = sddk_domain::NodeId(SEQ_ROOT.into());
    let attempts_midflight = check_store
        .list_attempts(&run_id, &seq_node)
        .expect("list_attempts");
    assert_eq!(
        attempts_midflight.len(),
        2,
        "2 of 5 sequence children persisted before the crash"
    );

    // ── Restart: a NEW runtime over the same persisted ledger ──────────────────
    let mut rt2 = build_runtime(crash_dir.path(), ir.clone(), &compiled, calls.clone());
    rt2.execute()
        .expect("restart replay should drive the run to completion");

    assert_completed(&rt2);
    assert_eq!(
        outputs_snapshot(&rt2),
        golden_outputs,
        "replayed terminal outputs must equal the uninterrupted golden run"
    );
    assert_eq!(
        node_state_snapshot(&rt2),
        golden_nodes,
        "replayed node-state map must equal the uninterrupted golden run"
    );
    assert_eq!(
        check_store
            .latest_workflow_run_state(&run_id)
            .expect("latest after replay"),
        Some(WorkflowRunState::Completed),
        "event log must end terminal after the replay"
    );
    let attempts_replayed = check_store
        .list_attempts(&run_id, &seq_node)
        .expect("list_attempts after replay");
    assert_eq!(
        attempts_replayed.len(),
        SEQ_CHILDREN,
        "attempt rows stay idempotent: exactly one row per child despite the replay"
    );
    let calls_total = calls.load(Ordering::SeqCst);
    assert_eq!(
        calls_total,
        calls_leg1 + golden_call_count,
        "replay must deterministically redo exactly the golden run's work: \
         {calls_leg1} (leg 1) + {golden_call_count} (golden) = {calls_total}"
    );
}

// ═══════════════════════════════════════════════════════════════════════════════
// Test 2 — a terminal run is never re-executed after a restart
// ═══════════════════════════════════════════════════════════════════════════════

#[test]
fn e2e_terminal_run_is_not_reexecuted_after_restart() {
    let (ir, compiled) = build_generated_dag();
    let correlation_id = CorrelationId(CORRELATION_ID.into());
    let run_id = RunId::derive(PLAN_REVISION_ID, &correlation_id);

    let temp_dir = tempfile::TempDir::new_in("/tmp").expect("temp dir");
    let calls = Arc::new(AtomicUsize::new(0));
    let (_persisted_run_id, _seed_store) = seed_persisted_run(temp_dir.path(), &ir, &compiled);

    // First process: full execution to a terminal state.
    let calls_after_first;
    {
        let mut rt1 = build_runtime(temp_dir.path(), ir.clone(), &compiled, calls.clone());
        rt1.execute().expect("first execute should succeed");
        assert_completed(&rt1);
        assert_eq!(
            rt1.run().outputs.as_ref().map(|o| o.len()),
            Some(SEQ_CHILDREN),
            "all 5 task outputs aggregated"
        );
        calls_after_first = calls.load(Ordering::SeqCst);
    }

    let store_check = SqliteGraphStore::open(temp_dir.path()).expect("re-open store");
    assert_eq!(
        store_check
            .latest_workflow_run_state(&run_id)
            .expect("latest_workflow_run_state"),
        Some(WorkflowRunState::Completed),
        "event log must record the terminal Completed state"
    );

    // Restart: the persisted log is terminal, so the guard must cut with
    // AlreadyTerminal and never invoke an operator again.
    let mut rt2 = build_runtime(temp_dir.path(), ir, &compiled, calls.clone());
    let restart_result = rt2.execute();
    match restart_result {
        Err(RuntimeError::AlreadyTerminal { state }) => {
            assert_eq!(
                state,
                WorkflowRunState::Completed,
                "guard reports the persisted terminal state"
            );
        }
        other => panic!("expected Err(AlreadyTerminal), got: {other:?}"),
    }
    assert_completed(&rt2);
    assert_eq!(
        calls.load(Ordering::SeqCst),
        calls_after_first,
        "restart guard must not re-run operators of a terminal run"
    );
}

// ═══════════════════════════════════════════════════════════════════════════════
// Test 3 — crash before start (Pending log) replays the full DAG to completion
// ═══════════════════════════════════════════════════════════════════════════════

#[test]
fn e2e_crash_before_start_pending_run_replays_full_dag() {
    let (ir, compiled) = build_generated_dag();
    let correlation_id = CorrelationId(CORRELATION_ID.into());
    let run_id = RunId::derive(PLAN_REVISION_ID, &correlation_id);

    let temp_dir = tempfile::TempDir::new_in("/tmp").expect("temp dir");
    let calls = Arc::new(AtomicUsize::new(0));
    // Crash before execute(): only the initial Pending event exists.
    let (_persisted_run_id, _seed_store) = seed_persisted_run(temp_dir.path(), &ir, &compiled);

    let mut rt = build_runtime(temp_dir.path(), ir, &compiled, calls.clone());
    rt.execute()
        .expect("replay of a Pending run should succeed");
    assert_completed(&rt);
    assert_eq!(
        rt.run().outputs.as_ref().map(|o| o.len()),
        Some(SEQ_CHILDREN),
        "all 5 task outputs aggregated"
    );

    let store_check = SqliteGraphStore::open(temp_dir.path()).expect("re-open store");
    assert_eq!(
        store_check
            .latest_workflow_run_state(&run_id)
            .expect("latest_workflow_run_state"),
        Some(WorkflowRunState::Completed),
        "execute() appends the terminal transition for a persisted Pending run"
    );
    let seq_node = sddk_domain::NodeId(SEQ_ROOT.into());
    let attempts = store_check
        .list_attempts(&run_id, &seq_node)
        .expect("list_attempts");
    assert_eq!(
        attempts.len(),
        SEQ_CHILDREN,
        "one attempt row per sequence child"
    );
}
