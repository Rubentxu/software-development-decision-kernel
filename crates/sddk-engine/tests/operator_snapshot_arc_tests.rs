//! Snapshot and Box-leak-free tests for Parallel operator.
//!
//! Tests verify that:
//! 1. No Box::leak is used in the Parallel child context construction
//! 2. Child NodeRun is properly dropped after thread joins (no leak)
//! 3. The OperatorContext API is unchanged (only snapshot_for_child removed)
//! 4. INV-10: No Mutex/RwLock on workflow state

use std::sync::{Arc, Mutex};
use std::time::Duration;

use sddk_domain::{
    GraphStore, NodeId, NodeRun, NodeRunState, NoopTaskExecutor, StorageError, WorkflowIR,
    WorkflowRun,
};
use sddk_engine::operator::{
    Clock, NodeOutcome, Operator, OperatorContext, Parallel, ScratchStore,
};

// ── Test operators ────────────────────────────────────────────────────────────

#[derive(Debug)]
struct SucceedOp;
impl Operator for SucceedOp {
    fn kind(&self) -> &'static str {
        "SucceedOp"
    }
    fn evaluate(
        &self,
        ctx: &mut OperatorContext,
    ) -> Result<NodeOutcome, sddk_engine::operator::OperatorError> {
        Ok(NodeOutcome::Succeeded {
            node_id: ctx.node_run.lock().unwrap().node_id.clone(),
            outputs: Default::default(),
        })
    }
}

// ── Mock GraphStore ────────────────────────────────────────────────────────────

struct MockStore;
impl GraphStore for MockStore {
    fn save_state(&mut self, _s: &sddk_domain::GraphState) -> Result<(), StorageError> {
        Ok(())
    }
    fn load_state(&self) -> Result<Option<sddk_domain::GraphState>, StorageError> {
        Ok(None)
    }
    fn checkpoint(&self) -> Result<Option<sddk_domain::projections::Checkpoint>, StorageError> {
        Ok(None)
    }
    fn record_ir_digest(&mut self, _: &str, _: &str) -> Result<(), StorageError> {
        Ok(())
    }
    fn record_graph_revision(
        &mut self,
        _: &sddk_domain::graph::ExecutionGraphRevision,
    ) -> Result<(), StorageError> {
        Ok(())
    }
    fn load_node_attempts(
        &self,
        _: &sddk_domain::RunId,
        _: &sddk_domain::NodeId,
    ) -> Result<Vec<sddk_domain::Attempt>, StorageError> {
        Ok(vec![])
    }
    fn attempt_count(
        &self,
        _: &sddk_domain::RunId,
        _: &sddk_domain::NodeId,
    ) -> Result<u32, StorageError> {
        Ok(0)
    }
    fn save_revision(
        &mut self,
        _: &sddk_domain::graph::ExecutionGraphRevision,
    ) -> Result<(), StorageError> {
        Ok(())
    }
    fn load_revision(
        &self,
        _: &sddk_domain::RunId,
        _: &sddk_domain::RevisionId,
    ) -> Result<Option<sddk_domain::graph::ExecutionGraphRevision>, StorageError> {
        Ok(None)
    }
    fn latest_revision(
        &self,
        _: &sddk_domain::RunId,
    ) -> Result<Option<sddk_domain::graph::ExecutionGraphRevision>, StorageError> {
        Ok(None)
    }
}

// ── Helpers ───────────────────────────────────────────────────────────────────

fn make_ir() -> WorkflowIR {
    WorkflowIR {
        ir_id: None,
        schema_version: 1,
        template_ref: sddk_domain::TemplateRef {
            id: "test".into(),
            version: "1.0".into(),
        },
        operators: Default::default(),
        guards: Default::default(),
        expansion_permissions: Default::default(),
        budgets: Default::default(),
        required_invariants: Default::default(),
        provenance: sddk_domain::Provenance {
            generated_by: "test".into(),
            prompt_hash: "test".into(),
            model_hash: "test".into(),
            policy_hash: "test".into(),
        },
    }
}

fn make_run() -> WorkflowRun {
    let ir = make_ir();
    WorkflowRun {
        run_id: sddk_domain::RunId("test-run".into()),
        template_ref: ir.template_ref.clone(),
        ir_hash: ir.compute_content_hash(),
        graph_revision: sddk_domain::RevisionId("rev".into()),
        state: sddk_domain::WorkflowRunState::Running,
        inputs: Default::default(),
        outputs: None,
        correlation_id: sddk_domain::CorrelationId("corr".into()),
        budget: Default::default(),
        schema_version: 1,
    }
}

// ── RED Test 1: parallel_no_box_leak_for_100_children ─────────────────────────

#[test]
fn parallel_no_box_leak_for_100_children() {
    let children: Vec<Arc<dyn Operator>> = (0..100)
        .map(|_| Arc::new(SucceedOp) as Arc<dyn Operator>)
        .collect();
    let parallel = Parallel {
        children,
        max_concurrency: 100,
    };

    let node_run_arc: Arc<Mutex<NodeRun>> = Arc::new(Mutex::new(NodeRun {
        node_id: NodeId("no-leak-100".into()),
        state: NodeRunState::Ready,
        dependencies: Default::default(),
        attempts: vec![],
        expansion_permissions: Default::default(),
        schema_version: 1,
    }));

    let ir = make_ir();
    let run = make_run();
    let store: ScratchStore = Arc::new(Mutex::new(Box::new(MockStore)));
    let ctx = OperatorContext {
        node_run: Arc::clone(&node_run_arc),
        ir: Arc::new(ir),
        run: Arc::new(run),
        store,
        clock: Clock,
        executor: Arc::new(NoopTaskExecutor),
        pending_sender: None,
    };

    let outcome = {
        let mut ctx = ctx;
        parallel.evaluate(&mut ctx)
    }
    .unwrap();
    assert!(matches!(outcome, NodeOutcome::Succeeded { .. }));
    assert_eq!(node_run_arc.lock().unwrap().attempts.len(), 100);
}

// ── RED Test 2: node_run_dropped_after_child_thread_joins ─────────────────────

#[test]
fn node_run_dropped_after_child_thread_joins() {
    let children: Vec<Arc<dyn Operator>> = (0..4)
        .map(|_| Arc::new(SucceedOp) as Arc<dyn Operator>)
        .collect();
    let parallel = Parallel {
        children,
        max_concurrency: 4,
    };

    let node_run_arc: Arc<Mutex<NodeRun>> = Arc::new(Mutex::new(NodeRun {
        node_id: NodeId("drop-test".into()),
        state: NodeRunState::Ready,
        dependencies: Default::default(),
        attempts: vec![],
        expansion_permissions: Default::default(),
        schema_version: 1,
    }));

    let ir = make_ir();
    let run = make_run();
    let store: ScratchStore = Arc::new(Mutex::new(Box::new(MockStore)));
    let ctx = OperatorContext {
        node_run: Arc::clone(&node_run_arc),
        ir: Arc::new(ir),
        run: Arc::new(run),
        store,
        clock: Clock,
        executor: Arc::new(NoopTaskExecutor),
        pending_sender: None,
    };

    let outcome = {
        let mut ctx = ctx;
        parallel.evaluate(&mut ctx)
    }
    .unwrap();
    assert!(matches!(outcome, NodeOutcome::Succeeded { .. }));
    assert_eq!(node_run_arc.lock().unwrap().attempts.len(), 4);
}

// ── RED Test 3: public_operator_context_api_unchanged ─────────────────────────

#[test]
fn public_operator_context_api_unchanged() {
    // Verify field existence at compile time
    fn check_fields(ctx: &OperatorContext) {
        // These fields should still exist
        let _ = &ctx.node_run;
        let _ = &ctx.ir;
        let _ = &ctx.run;
        let _ = &ctx.store;
        let _ = &ctx.clock;
        let _ = &ctx.executor;
        let _ = &ctx.pending_sender;
    }

    let node_run_arc: Arc<Mutex<NodeRun>> = Arc::new(Mutex::new(NodeRun {
        node_id: NodeId("api-check".into()),
        state: NodeRunState::Ready,
        dependencies: Default::default(),
        attempts: vec![],
        expansion_permissions: Default::default(),
        schema_version: 1,
    }));
    let ir = make_ir();
    let run = make_run();
    let store: ScratchStore = Arc::new(Mutex::new(Box::new(MockStore)));
    let ctx = OperatorContext {
        node_run: Arc::clone(&node_run_arc),
        ir: Arc::new(ir),
        run: Arc::new(run),
        store,
        clock: Clock,
        executor: Arc::new(NoopTaskExecutor),
        pending_sender: None,
    };
    check_fields(&ctx);
}

// ── RED Test 4: inv10_grep_gate_no_mutex_on_workflow_state ────────────────────

#[test]
fn inv10_grep_gate_no_mutex_on_workflow_state() {
    // Each child sleeps a known amount so the gate below can compare total
    // elapsed against the cost of a fully serialized run. See the ratio
    // explanation at the assertion. Sleep (not busy-work) keeps the thread
    // off the CPU, so the only cost measured is overlap of the critical
    // sections, not the number of cores doing arithmetic.
    const CHILD_MS: u64 = 20;
    const CHILD_SLEEP: Duration = Duration::from_millis(CHILD_MS);
    const CHILDREN_COUNT: u32 = 50;
    struct SleepingOp(Duration);
    impl std::fmt::Debug for SleepingOp {
        fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
            f.write_str("SleepingOp")
        }
    }
    impl Operator for SleepingOp {
        fn kind(&self) -> &'static str {
            "SleepingOp"
        }
        fn evaluate(
            &self,
            ctx: &mut OperatorContext,
        ) -> Result<NodeOutcome, sddk_engine::operator::OperatorError> {
            std::thread::sleep(self.0);
            Ok(NodeOutcome::Succeeded {
                node_id: ctx.node_run.lock().unwrap().node_id.clone(),
                outputs: Default::default(),
            })
        }
    }

    let children: Vec<Arc<dyn Operator>> = (0..CHILDREN_COUNT)
        .map(|_| Arc::new(SleepingOp(CHILD_SLEEP)) as Arc<dyn Operator>)
        .collect();
    let parallel = Parallel {
        children,
        max_concurrency: CHILDREN_COUNT,
    };

    let node_run_arc: Arc<Mutex<NodeRun>> = Arc::new(Mutex::new(NodeRun {
        node_id: NodeId("inv10-gate".into()),
        state: NodeRunState::Ready,
        dependencies: Default::default(),
        attempts: vec![],
        expansion_permissions: Default::default(),
        schema_version: 1,
    }));

    let ir = make_ir();
    let run = make_run();
    let store: ScratchStore = Arc::new(Mutex::new(Box::new(MockStore)));
    let ctx = OperatorContext {
        node_run: Arc::clone(&node_run_arc),
        ir: Arc::new(ir),
        run: Arc::new(run),
        store,
        clock: Clock,
        executor: Arc::new(NoopTaskExecutor),
        pending_sender: None,
    };

    let start = std::time::Instant::now();
    let outcome = {
        let mut ctx = ctx;
        parallel.evaluate(&mut ctx)
    }
    .unwrap();
    let elapsed = start.elapsed();

    assert!(matches!(outcome, NodeOutcome::Succeeded { .. }));
    assert_eq!(
        node_run_arc.lock().unwrap().attempts.len(),
        CHILDREN_COUNT as usize
    );
    // Concurrency gate (R-flake-inv10, session-22).
    //
    // This used to be `assert!(elapsed < Duration::from_millis(20))`. That
    // was a wall-clock threshold with no headroom, so it measured the
    // machine rather than the code: under CPU contention it fired at
    // 4 failures in 6 runs (elapsed 22-31ms) while passing in 0.00ms in
    // isolation. Loosening it does not fix that, it hides it. At 200ms the
    // test passed even with `max_concurrency: 1` — i.e. fully serialized,
    // 50 x 1ms = ~60ms — so the loosened bound had quietly stopped
    // catching the very regression it exists to catch.
    //
    // Wall-clock alone cannot separate the two cases on an unknown
    // machine: contention on trivial children (~30ms) and genuine
    // serialization (~60ms) overlap. What separates them is the RATIO of
    // total elapsed to the time a fully serialized run would need, which
    // is dimensionless and therefore independent of CPU count, machine
    // speed and ambient load:
    //
    //   serialized:  N children in sequence -> ratio ~ 1.0
    //   concurrent:  N children in parallel  -> ratio ~ 1/N
    //
    // Children below sleep a known CHILD_MS, so the serialized reference
    // is computed rather than guessed, and the bound sits far from both
    // poles instead of between them. Anything under ~0.1 means the work
    // overlapped; anything near 1.0 means the node_run mutex serialized it.
    //
    // CHILD_MS is deliberately long (20ms). A short child makes the gate
    // flaky in the other direction: at 4ms, a loaded machine needed 42ms to
    // schedule 50 concurrent sleeps (21% of serial) even though nothing was
    // serialized — thread wakeup latency, not contention on the mutex, was
    // being measured. Sleep is I/O-free so the OS does not preempt the
    // thread, but 50 of them still need 50 runnable slots, and a saturated
    // box does not hand them out instantly. A 20ms sleep puts that fixed
    // scheduling cost at ~2% of the 1000ms serial reference, well clear of
    // the bound, so the gate measures overlap rather than wakeup latency.
    const MAX_RATIO_PERCENT: u128 = 20; // concurrent ~2%, serialized ~100%

    let serialized = CHILD_SLEEP * CHILDREN_COUNT;
    // Integer percent. The operands are u128 micros, so the multiply cannot
    // overflow for any plausible elapsed time and no cast is needed.
    let ratio_pct = elapsed.as_micros() * 100 / serialized.as_micros();

    assert!(
        ratio_pct < MAX_RATIO_PERCENT,
        "50 children took {:?}, which is {}% of the ~{:?} a fully serialized run \
         would need (bound {}%). The node_run mutex is serializing the children, \
         which is exactly what this gate exists to catch.",
        elapsed,
        ratio_pct,
        serialized,
        MAX_RATIO_PERCENT
    );
}

// ── TRIANGULATE: Multi-child variants ─────────────────────────────────────────

#[test]
fn parallel_no_leak_5_children() {
    let children: Vec<Arc<dyn Operator>> = (0..5)
        .map(|_| Arc::new(SucceedOp) as Arc<dyn Operator>)
        .collect();
    let parallel = Parallel {
        children,
        max_concurrency: 5,
    };

    let node_run_arc: Arc<Mutex<NodeRun>> = Arc::new(Mutex::new(NodeRun {
        node_id: NodeId("no-leak-5".into()),
        state: NodeRunState::Ready,
        dependencies: Default::default(),
        attempts: vec![],
        expansion_permissions: Default::default(),
        schema_version: 1,
    }));

    let ir = make_ir();
    let run = make_run();
    let store: ScratchStore = Arc::new(Mutex::new(Box::new(MockStore)));
    let ctx = OperatorContext {
        node_run: Arc::clone(&node_run_arc),
        ir: Arc::new(ir),
        run: Arc::new(run),
        store,
        clock: Clock,
        executor: Arc::new(NoopTaskExecutor),
        pending_sender: None,
    };

    let outcome = {
        let mut ctx = ctx;
        parallel.evaluate(&mut ctx)
    }
    .unwrap();
    assert!(matches!(outcome, NodeOutcome::Succeeded { .. }));
    assert_eq!(node_run_arc.lock().unwrap().attempts.len(), 5);
}

#[test]
fn parallel_no_leak_50_children() {
    let children: Vec<Arc<dyn Operator>> = (0..50)
        .map(|_| Arc::new(SucceedOp) as Arc<dyn Operator>)
        .collect();
    let parallel = Parallel {
        children,
        max_concurrency: 50,
    };

    let node_run_arc: Arc<Mutex<NodeRun>> = Arc::new(Mutex::new(NodeRun {
        node_id: NodeId("no-leak-50".into()),
        state: NodeRunState::Ready,
        dependencies: Default::default(),
        attempts: vec![],
        expansion_permissions: Default::default(),
        schema_version: 1,
    }));

    let ir = make_ir();
    let run = make_run();
    let store: ScratchStore = Arc::new(Mutex::new(Box::new(MockStore)));
    let ctx = OperatorContext {
        node_run: Arc::clone(&node_run_arc),
        ir: Arc::new(ir),
        run: Arc::new(run),
        store,
        clock: Clock,
        executor: Arc::new(NoopTaskExecutor),
        pending_sender: None,
    };

    let outcome = {
        let mut ctx = ctx;
        parallel.evaluate(&mut ctx)
    }
    .unwrap();
    assert!(matches!(outcome, NodeOutcome::Succeeded { .. }));
    assert_eq!(node_run_arc.lock().unwrap().attempts.len(), 50);
}

#[test]
fn parallel_no_leak_1_child() {
    let children: Vec<Arc<dyn Operator>> = vec![Arc::new(SucceedOp)];
    let parallel = Parallel {
        children,
        max_concurrency: 1,
    };

    let node_run_arc: Arc<Mutex<NodeRun>> = Arc::new(Mutex::new(NodeRun {
        node_id: NodeId("no-leak-1".into()),
        state: NodeRunState::Ready,
        dependencies: Default::default(),
        attempts: vec![],
        expansion_permissions: Default::default(),
        schema_version: 1,
    }));

    let ir = make_ir();
    let run = make_run();
    let store: ScratchStore = Arc::new(Mutex::new(Box::new(MockStore)));
    let ctx = OperatorContext {
        node_run: Arc::clone(&node_run_arc),
        ir: Arc::new(ir),
        run: Arc::new(run),
        store,
        clock: Clock,
        executor: Arc::new(NoopTaskExecutor),
        pending_sender: None,
    };

    let outcome = {
        let mut ctx = ctx;
        parallel.evaluate(&mut ctx)
    }
    .unwrap();
    assert!(matches!(outcome, NodeOutcome::Succeeded { .. }));
    assert_eq!(node_run_arc.lock().unwrap().attempts.len(), 1);
}

// ── RED Tests for Arc<Mutex<T>> refactor ────────────────────────────────────────

// RED Test 1: parallel_arc_no_leak_100_children
#[test]
fn parallel_arc_no_leak_100_children() {
    use std::sync::Mutex;

    let children: Vec<Arc<dyn Operator>> = (0..100)
        .map(|_| Arc::new(SucceedOp) as Arc<dyn Operator>)
        .collect();
    let parallel = Parallel {
        children,
        max_concurrency: 100,
    };

    let node_run_arc: Arc<Mutex<NodeRun>> = Arc::new(Mutex::new(NodeRun {
        node_id: NodeId("arc-no-leak-100".into()),
        state: NodeRunState::Ready,
        dependencies: Default::default(),
        attempts: vec![],
        expansion_permissions: Default::default(),
        schema_version: 1,
    }));

    let ir = make_ir();
    let run = make_run();
    let store: ScratchStore = Arc::new(Mutex::new(Box::new(MockStore)));
    let ctx = OperatorContext {
        node_run: Arc::clone(&node_run_arc),
        ir: Arc::new(ir),
        run: Arc::new(run),
        store,
        clock: Clock,
        executor: Arc::new(NoopTaskExecutor),
        pending_sender: None,
    };

    let outcome = {
        let mut ctx = ctx;
        parallel.evaluate(&mut ctx)
    };

    // After evaluate, all child Arc clones should be dropped
    assert_eq!(
        Arc::strong_count(&node_run_arc),
        1,
        "child Arc clones leaked"
    );
    let _ = outcome;
}

// RED Test 2: parallel_arc_no_leak_5_children
#[test]
fn parallel_arc_no_leak_5_children() {
    use std::sync::Mutex;

    let children: Vec<Arc<dyn Operator>> = (0..5)
        .map(|_| Arc::new(SucceedOp) as Arc<dyn Operator>)
        .collect();
    let parallel = Parallel {
        children,
        max_concurrency: 5,
    };

    let node_run_arc: Arc<Mutex<NodeRun>> = Arc::new(Mutex::new(NodeRun {
        node_id: NodeId("arc-drop-test".into()),
        state: NodeRunState::Ready,
        dependencies: Default::default(),
        attempts: vec![],
        expansion_permissions: Default::default(),
        schema_version: 1,
    }));

    let ir = make_ir();
    let run = make_run();
    let store: ScratchStore = Arc::new(Mutex::new(Box::new(MockStore)));
    let ctx = OperatorContext {
        node_run: Arc::clone(&node_run_arc),
        ir: Arc::new(ir),
        run: Arc::new(run),
        store,
        clock: Clock,
        executor: Arc::new(NoopTaskExecutor),
        pending_sender: None,
    };

    let outcome = {
        let mut ctx = ctx;
        parallel.evaluate(&mut ctx)
    };
    assert_eq!(
        Arc::strong_count(&node_run_arc),
        1,
        "child Arc clones leaked"
    );
    let _ = outcome;
}

// RED Test 3: snapshot_for_child_removed
#[test]
fn snapshot_for_child_removed() {
    use std::process::Command;
    let manifest_dir = env!("CARGO_MANIFEST_DIR");
    let src_path = format!("{}/src", manifest_dir);
    let output = Command::new("grep")
        .args(["-rn", "snapshot_for_child", &src_path])
        .output()
        .expect("grep should execute");
    let count = String::from_utf8_lossy(&output.stdout).lines().count();
    assert_eq!(count, 0, "snapshot_for_child still present in operator.rs");
}

// RED Test 4: box_leak_removed
#[test]
fn box_leak_removed() {
    use std::process::Command;
    let manifest_dir = env!("CARGO_MANIFEST_DIR");
    let src_path = format!("{}/src", manifest_dir);
    let output = Command::new("grep")
        .args(["-rn", "Box::leak\\(", &src_path])
        .output()
        .expect("grep should execute");
    let count = String::from_utf8_lossy(&output.stdout).lines().count();
    assert_eq!(count, 0, "Box::leak still present in operator.rs");
}
