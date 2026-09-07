//! S6b restart-survival (real store) — Slice 2 (engine).
//!
//! Verifies the engine half of restart-survival on the real `SqliteGraphStore`:
//!   - `execute()` appends lifecycle transitions to the persisted run's event
//!     log (`record_workflow_run_transition`) as the run really executes:
//!     `Pending -> Running` at start, then `Running -> Completed` / `Running ->
//!     Failed` on the terminal branches.
//!   - The restart guard short-circuits `execute()` when the persisted log
//!     reports a TERMINAL state (`Completed` / `Failed` / `Cancelled`): the run
//!     is NOT re-executed (operators are not invoked again), the in-memory run
//!     is aligned to the persisted terminal state, and
//!     `Err(RuntimeError::AlreadyTerminal)` is returned.
//!   - A persisted but non-terminal run (initial `Pending` event) still
//!     executes normally and records its terminal transition.

use std::collections::BTreeMap;
use std::sync::Arc;
use std::sync::atomic::{AtomicUsize, Ordering};

use sddk_domain::graph::ExecutionGraphRevision;
use sddk_domain::workflow_ir::{Budgets, RevisionId, TemplateRef, WorkflowIR};
use sddk_domain::{
    CapabilityId, CorrelationId, GraphStore, Operator, OperatorId, RunId, TaskError, TaskExecutor,
    TaskOutput, WorkflowRun, WorkflowRunState,
};
use sddk_engine::RuntimeError;
use sddk_engine::operator::Clock;
use sddk_engine::workflow_runtime::WorkflowRuntime;
use sddk_storage::SqliteGraphStore;
use serde_json::Value;

// ── Counting executor (succeeds or fails on demand) ──────────────────────────

/// Executor that counts every invocation through a shared `Arc<AtomicUsize>`.
/// With `fail == true` every task returns `Err`, driving the run to `Failed`.
struct CountingExecutor {
    calls: Arc<AtomicUsize>,
    fail: bool,
}

impl TaskExecutor for CountingExecutor {
    fn execute(
        &self,
        capability: &str,
        _inputs: &BTreeMap<String, Value>,
    ) -> Result<TaskOutput, TaskError> {
        self.calls.fetch_add(1, Ordering::SeqCst);
        if self.fail {
            return Err(TaskError {
                message: format!("capability {capability} failed"),
            });
        }
        let mut outputs = BTreeMap::new();
        outputs.insert(
            "greeting".into(),
            Value::String(format!("hello {capability}")),
        );
        Ok(TaskOutput { outputs })
    }
}

// ── IR + compiled revision helpers ───────────────────────────────────────────

/// Minimal IR with a single top-level Task node (a terminal leaf).
fn build_single_task_ir(template_id: &str) -> WorkflowIR {
    let mut operators = BTreeMap::new();
    operators.insert(
        OperatorId("task-root".into()),
        Operator::Task {
            capability: CapabilityId("echo".into()),
            inputs: Default::default(),
        },
    );
    WorkflowIR {
        ir_id: None,
        schema_version: 1,
        template_ref: TemplateRef {
            id: template_id.into(),
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
            generated_by: "restart-survival".into(),
            prompt_hash: "test".into(),
            model_hash: "test".into(),
            policy_hash: "test".into(),
        },
    }
}

fn dummy_compiled_revision(rev_id: &str) -> ExecutionGraphRevision {
    ExecutionGraphRevision {
        revision: 0,
        revision_id: RevisionId(rev_id.into()),
        parent: None,
        events: BTreeMap::new(),
        nodes: BTreeMap::new(),
        edges: BTreeMap::new(),
        digest: [0u8; 32],
        schema_version: 1,
    }
}

fn assert_already_terminal(
    result: sddk_engine::workflow_runtime::Result<()>,
    expected: WorkflowRunState,
) {
    match result {
        Err(RuntimeError::AlreadyTerminal { state }) => {
            assert_eq!(
                state, expected,
                "guard should report the persisted terminal state"
            );
        }
        other => panic!("expected Err(AlreadyTerminal {{ state: {expected:?} }}), got: {other:?}"),
    }
}

// ═══════════════════════════════════════════════════════════════════════════════
// Recording + Completed restart guard
// ═══════════════════════════════════════════════════════════════════════════════

/// A first `execute()` records the terminal transition and a second `execute()`
/// over the same persisted run_id short-circuits instead of re-running.
#[test]
fn completed_run_records_terminal_state_and_restart_guard_short_circuits() {
    let ir = build_single_task_ir("s6b-completed");
    let plan_revision_id = "plan-s6b-completed";
    let correlation_id = CorrelationId("corr-s6b-completed".into());
    let compiled = dummy_compiled_revision("rev-s6b-completed");
    let run_id = RunId::derive(plan_revision_id, &correlation_id);

    let temp_dir = tempfile::TempDir::new_in("/tmp").expect("temp dir");

    // First process: fresh store, run executes and completes.
    let calls = Arc::new(AtomicUsize::new(0));
    let store1 = SqliteGraphStore::open(temp_dir.path()).expect("open store");
    let executor1: Arc<dyn TaskExecutor> = Arc::new(CountingExecutor {
        calls: calls.clone(),
        fail: false,
    });
    let mut runtime1 = WorkflowRuntime::from_compiled(
        ir.clone(),
        store1,
        Clock,
        executor1,
        plan_revision_id,
        &correlation_id,
        &compiled,
    );
    let result = runtime1.execute();
    assert!(result.is_ok(), "first execute should succeed: {result:?}");
    assert_eq!(runtime1.run().state, WorkflowRunState::Completed);
    assert_eq!(
        calls.load(Ordering::SeqCst),
        1,
        "task should run exactly once"
    );

    // The persisted lifecycle log must now report Completed (recorded via
    // record_workflow_run_transition by execute()).
    let store_check = SqliteGraphStore::open(temp_dir.path()).expect("re-open store");
    let persisted = store_check
        .latest_workflow_run_state(&run_id)
        .expect("latest_workflow_run_state");
    assert_eq!(
        persisted,
        Some(WorkflowRunState::Completed),
        "event log should record the terminal Completed state"
    );

    // "Restart" (process crash simulation): a second runtime over the same
    // run_id must NOT re-run the workflow — the guard cuts with AlreadyTerminal
    // and the shared executor counter proves no operator was invoked again.
    let store2 = SqliteGraphStore::open(temp_dir.path()).expect("re-open store 2");
    let executor2: Arc<dyn TaskExecutor> = Arc::new(CountingExecutor {
        calls: calls.clone(),
        fail: false,
    });
    let mut runtime2 = WorkflowRuntime::from_compiled(
        ir,
        store2,
        Clock,
        executor2,
        plan_revision_id,
        &correlation_id,
        &compiled,
    );
    let restart_result = runtime2.execute();
    assert_already_terminal(restart_result, WorkflowRunState::Completed);
    assert_eq!(
        runtime2.run().state,
        WorkflowRunState::Completed,
        "in-memory run should be aligned to the persisted terminal state"
    );
    assert_eq!(
        calls.load(Ordering::SeqCst),
        1,
        "restart guard must not re-run operators of a terminal run"
    );
}

// ═══════════════════════════════════════════════════════════════════════════════
// Failed restart guard
// ═══════════════════════════════════════════════════════════════════════════════

/// A run that failed records `Running -> Failed`, and a restart over it is
/// short-circuited with the `Failed` terminal state (never re-executed).
#[test]
fn failed_run_records_terminal_state_and_restart_guard_short_circuits() {
    let ir = build_single_task_ir("s6b-failed");
    let plan_revision_id = "plan-s6b-failed";
    let correlation_id = CorrelationId("corr-s6b-failed".into());
    let compiled = dummy_compiled_revision("rev-s6b-failed");
    let run_id = RunId::derive(plan_revision_id, &correlation_id);

    let temp_dir = tempfile::TempDir::new_in("/tmp").expect("temp dir");

    let calls = Arc::new(AtomicUsize::new(0));
    let store1 = SqliteGraphStore::open(temp_dir.path()).expect("open store");
    let executor1: Arc<dyn TaskExecutor> = Arc::new(CountingExecutor {
        calls: calls.clone(),
        fail: true,
    });
    let mut runtime1 = WorkflowRuntime::from_compiled(
        ir.clone(),
        store1,
        Clock,
        executor1,
        plan_revision_id,
        &correlation_id,
        &compiled,
    );
    let result = runtime1.execute();
    assert!(
        result.is_ok(),
        "execute should drive the run to Failed: {result:?}"
    );
    assert_eq!(runtime1.run().state, WorkflowRunState::Failed);
    assert_eq!(
        calls.load(Ordering::SeqCst),
        1,
        "task should fail exactly once"
    );

    let store_check = SqliteGraphStore::open(temp_dir.path()).expect("re-open store");
    let persisted = store_check
        .latest_workflow_run_state(&run_id)
        .expect("latest_workflow_run_state");
    assert_eq!(
        persisted,
        Some(WorkflowRunState::Failed),
        "event log should record the terminal Failed state"
    );

    // Restart: must not re-run a failed workflow.
    let store2 = SqliteGraphStore::open(temp_dir.path()).expect("re-open store 2");
    let executor2: Arc<dyn TaskExecutor> = Arc::new(CountingExecutor {
        calls: calls.clone(),
        fail: true,
    });
    let mut runtime2 = WorkflowRuntime::from_compiled(
        ir,
        store2,
        Clock,
        executor2,
        plan_revision_id,
        &correlation_id,
        &compiled,
    );
    let restart_result = runtime2.execute();
    assert_already_terminal(restart_result, WorkflowRunState::Failed);
    assert_eq!(runtime2.run().state, WorkflowRunState::Failed);
    assert_eq!(
        calls.load(Ordering::SeqCst),
        1,
        "restart guard must not re-run operators of a failed run"
    );
}

// ═══════════════════════════════════════════════════════════════════════════════
// Non-terminal persisted run still executes
// ═══════════════════════════════════════════════════════════════════════════════

/// The restart guard reads the EVENT LOG, not the (stale) `workflow_runs_v1.state`
/// row: a run whose row still says `Pending` but whose log was externally
/// advanced to a terminal state (`Cancelled` here) is NOT re-executed.
#[test]
fn cancelled_persisted_run_guard_reads_event_log_not_row_state() {
    let ir = build_single_task_ir("s6b-cancelled");
    let plan_revision_id = "plan-s6b-cancelled";
    let correlation_id = CorrelationId("corr-s6b-cancelled".into());
    let compiled = dummy_compiled_revision("rev-s6b-cancelled");
    let run_id = RunId::derive(plan_revision_id, &correlation_id);

    let temp_dir = tempfile::TempDir::new_in("/tmp").expect("temp dir");

    // Persist a run row in Pending (as record_run would) and then simulate an
    // external cancellation that advanced the event log to Cancelled while the
    // workflow_runs_v1.state column was left untouched (no UPDATE in Slice 1).
    let mut store = SqliteGraphStore::open(temp_dir.path()).expect("open store");
    let run = WorkflowRun {
        run_id: run_id.clone(),
        template_ref: ir.template_ref.clone(),
        ir_hash: format!("sha256:{}", ir.compute_content_hash()),
        graph_revision: compiled.revision_id.clone(),
        state: WorkflowRunState::Pending,
        inputs: Default::default(),
        outputs: None,
        correlation_id: correlation_id.clone(),
        budget: ir.budgets.clone(),
        schema_version: 1,
    };
    store.record_run(&run, &compiled).expect("record_run");
    store
        .record_workflow_run_transition(
            &run_id,
            WorkflowRunState::Pending,
            WorkflowRunState::Running,
            None,
        )
        .expect("record Pending -> Running");
    store
        .record_workflow_run_transition(
            &run_id,
            WorkflowRunState::Running,
            WorkflowRunState::Cancelled,
            Some("external cancel"),
        )
        .expect("record Running -> Cancelled");

    let calls = Arc::new(AtomicUsize::new(0));
    let executor: Arc<dyn TaskExecutor> = Arc::new(CountingExecutor {
        calls: calls.clone(),
        fail: false,
    });
    let mut runtime = WorkflowRuntime::from_compiled(
        ir,
        store,
        Clock,
        executor,
        plan_revision_id,
        &correlation_id,
        &compiled,
    );
    let result = runtime.execute();
    assert_already_terminal(result, WorkflowRunState::Cancelled);
    assert_eq!(runtime.run().state, WorkflowRunState::Cancelled);
    assert_eq!(
        calls.load(Ordering::SeqCst),
        0,
        "a Cancelled run must never execute its operators"
    );
}

/// A persisted run whose event log reports only the initial `Pending` state is
/// NOT terminal: `execute()` proceeds normally, runs the operators once, and
/// records the terminal transition so a later restart is short-circuited.
#[test]
fn persisted_pending_run_still_executes_and_records_completion() {
    let ir = build_single_task_ir("s6b-pending");
    let plan_revision_id = "plan-s6b-pending";
    let correlation_id = CorrelationId("corr-s6b-pending".into());
    let compiled = dummy_compiled_revision("rev-s6b-pending");
    let run_id = RunId::derive(plan_revision_id, &correlation_id);

    let temp_dir = tempfile::TempDir::new_in("/tmp").expect("temp dir");

    // Simulate a crash BEFORE execute(): the run row + initial Pending event
    // exist (as record_run would have written them), but no transition ran.
    let mut store = SqliteGraphStore::open(temp_dir.path()).expect("open store");
    let run = WorkflowRun {
        run_id: run_id.clone(),
        template_ref: ir.template_ref.clone(),
        ir_hash: format!("sha256:{}", ir.compute_content_hash()),
        graph_revision: compiled.revision_id.clone(),
        state: WorkflowRunState::Pending,
        inputs: Default::default(),
        outputs: None,
        correlation_id: correlation_id.clone(),
        budget: ir.budgets.clone(),
        schema_version: 1,
    };
    store.record_run(&run, &compiled).expect("record_run");

    let calls = Arc::new(AtomicUsize::new(0));
    let executor: Arc<dyn TaskExecutor> = Arc::new(CountingExecutor {
        calls: calls.clone(),
        fail: false,
    });
    let mut runtime = WorkflowRuntime::from_compiled(
        ir,
        store,
        Clock,
        executor,
        plan_revision_id,
        &correlation_id,
        &compiled,
    );
    let result = runtime.execute();
    assert!(
        result.is_ok(),
        "non-terminal persisted run should execute: {result:?}"
    );
    assert_eq!(runtime.run().state, WorkflowRunState::Completed);
    assert_eq!(
        calls.load(Ordering::SeqCst),
        1,
        "task should run exactly once"
    );

    let store_check = SqliteGraphStore::open(temp_dir.path()).expect("re-open store");
    let persisted = store_check
        .latest_workflow_run_state(&run_id)
        .expect("latest_workflow_run_state");
    assert_eq!(
        persisted,
        Some(WorkflowRunState::Completed),
        "execute() should append the terminal transition for a persisted run"
    );
}
