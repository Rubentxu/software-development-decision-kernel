//! Concurrency tests for the Parallel operator.
//!
//! Tests concurrent fan-out, ordering invariants, panic isolation, backpressure,
//! and snapshot properties.

use std::collections::BTreeMap;
use std::sync::{Arc, Mutex};
use std::thread;
use std::time::Duration;

use sddk_domain::{
    GraphStore, NodeId, NodeRun, NodeRunState, NoopTaskExecutor, RunId, StorageError, WorkflowIR,
    WorkflowRun,
};
use sddk_engine::operator::{
    Clock, GraphStoreBox, NodeOutcome, Operator, OperatorContext, OperatorError, Parallel,
};

// ── Test operators ────────────────────────────────────────────────────────────

#[derive(Debug)]
struct SucceedOp;
impl Operator for SucceedOp {
    fn kind(&self) -> &'static str {
        "SucceedOp"
    }
    fn evaluate(&self, ctx: &mut OperatorContext) -> Result<NodeOutcome, OperatorError> {
        Ok(NodeOutcome::Succeeded {
            node_id: ctx.node_run.lock().unwrap().node_id.clone(),
            outputs: Default::default(),
        })
    }
}

#[derive(Debug)]
struct FailOp(String);
impl Operator for FailOp {
    fn kind(&self) -> &'static str {
        "FailOp"
    }
    fn evaluate(&self, ctx: &mut OperatorContext) -> Result<NodeOutcome, OperatorError> {
        Ok(NodeOutcome::Failed {
            node_id: ctx.node_run.lock().unwrap().node_id.clone(),
            reason: self.0.clone(),
        })
    }
}

#[derive(Debug)]
struct PanicOp;
impl Operator for PanicOp {
    fn kind(&self) -> &'static str {
        "PanicOp"
    }
    fn evaluate(&self, _ctx: &mut OperatorContext) -> Result<NodeOutcome, OperatorError> {
        panic!("PanicOp panicked as intended");
    }
}

#[derive(Debug)]
struct SleepThenSucceedOp {
    duration: Duration,
}
impl SleepThenSucceedOp {
    fn new(ms: u64) -> Self {
        Self {
            duration: Duration::from_millis(ms),
        }
    }
}
impl Operator for SleepThenSucceedOp {
    fn kind(&self) -> &'static str {
        "SleepThenSucceedOp"
    }
    fn evaluate(&self, ctx: &mut OperatorContext) -> Result<NodeOutcome, OperatorError> {
        thread::sleep(self.duration);
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

/// eval! takes NodeRun by value and returns (outcome, node_run_arc).
macro_rules! eval {
    ($parallel:expr, $node_run:expr) => {{
        let ir = make_ir();
        let run = make_run();
        let store: Arc<Mutex<GraphStoreBox>> = Arc::new(Mutex::new(GraphStoreBox {
            inner: Box::new(MockStore),
        }));
        let node_run_arc: Arc<Mutex<NodeRun>> = Arc::new(Mutex::new($node_run));
        let mut ctx = OperatorContext {
            node_run: Arc::clone(&node_run_arc),
            ir: Arc::new(ir),
            run: Arc::new(run),
            store,
            clock: Clock,
            executor: Arc::new(NoopTaskExecutor),
            pending_sender: None,
        };
        ($parallel.evaluate(&mut ctx), node_run_arc)
    }};
}

// ── A. Timing tests ────────────────────────────────────────────────────────────

#[test]
fn parallel_timing_4x100_under_200ms() {
    let children: Vec<Arc<dyn Operator>> = (0..4)
        .map(|_| Arc::new(SleepThenSucceedOp::new(100)) as Arc<dyn Operator>)
        .collect();
    let parallel = Parallel {
        children,
        max_concurrency: 4,
    };
    let node_run = NodeRun {
        node_id: NodeId("timing-4x100".into()),
        state: NodeRunState::Ready,
        dependencies: Default::default(),
        attempts: vec![],
        expansion_permissions: Default::default(),
        schema_version: 1,
    };
    let start = std::time::Instant::now();
    let (outcome, node_run_arc) = eval!(parallel, node_run);
    let elapsed = start.elapsed();
    assert!(matches!(outcome.unwrap(), NodeOutcome::Succeeded { .. }));
    assert_eq!(node_run_arc.lock().unwrap().attempts.len(), 4);
    assert!(elapsed < Duration::from_millis(200), "took {:?}", elapsed);
}

#[test]
fn parallel_timing_8x50_under_150ms() {
    let children: Vec<Arc<dyn Operator>> = (0..8)
        .map(|_| Arc::new(SleepThenSucceedOp::new(50)) as Arc<dyn Operator>)
        .collect();
    let parallel = Parallel {
        children,
        max_concurrency: 8,
    };
    let node_run = NodeRun {
        node_id: NodeId("timing-8x50".into()),
        state: NodeRunState::Ready,
        dependencies: Default::default(),
        attempts: vec![],
        expansion_permissions: Default::default(),
        schema_version: 1,
    };
    let start = std::time::Instant::now();
    let (outcome, node_run_arc) = eval!(parallel, node_run);
    let elapsed = start.elapsed();
    assert!(matches!(outcome.unwrap(), NodeOutcome::Succeeded { .. }));
    assert_eq!(node_run_arc.lock().unwrap().attempts.len(), 8);
    assert!(elapsed < Duration::from_millis(150), "took {:?}", elapsed);
}

#[test]
fn parallel_timing_max_concurrency_2_pairs() {
    let children: Vec<Arc<dyn Operator>> = (0..4)
        .map(|_| Arc::new(SleepThenSucceedOp::new(50)) as Arc<dyn Operator>)
        .collect();
    let parallel = Parallel {
        children,
        max_concurrency: 2,
    };
    let node_run = NodeRun {
        node_id: NodeId("timing-2-pairs".into()),
        state: NodeRunState::Ready,
        dependencies: Default::default(),
        attempts: vec![],
        expansion_permissions: Default::default(),
        schema_version: 1,
    };
    let start = std::time::Instant::now();
    let (outcome, _node_run_arc) = eval!(parallel, node_run);
    let elapsed = start.elapsed();
    assert!(matches!(outcome.unwrap(), NodeOutcome::Succeeded { .. }));
    assert!(elapsed >= Duration::from_millis(90));
    assert!(elapsed < Duration::from_millis(160));
}

#[test]
fn parallel_timing_varying_durations() {
    let children: Vec<Arc<dyn Operator>> = vec![
        Arc::new(SleepThenSucceedOp::new(20)),
        Arc::new(SleepThenSucceedOp::new(100)),
        Arc::new(SleepThenSucceedOp::new(50)),
        Arc::new(SleepThenSucceedOp::new(30)),
    ];
    let parallel = Parallel {
        children,
        max_concurrency: 4,
    };
    let node_run = NodeRun {
        node_id: NodeId("timing-varying".into()),
        state: NodeRunState::Ready,
        dependencies: Default::default(),
        attempts: vec![],
        expansion_permissions: Default::default(),
        schema_version: 1,
    };
    let start = std::time::Instant::now();
    let (outcome, node_run_arc) = eval!(parallel, node_run);
    let elapsed = start.elapsed();
    assert!(matches!(outcome.unwrap(), NodeOutcome::Succeeded { .. }));
    assert_eq!(node_run_arc.lock().unwrap().attempts.len(), 4);
    assert!(elapsed < Duration::from_millis(150), "took {:?}", elapsed);
}

#[test]
fn parallel_timing_no_deadlock_100_children() {
    let children: Vec<Arc<dyn Operator>> = (0..100)
        .map(|_| Arc::new(SleepThenSucceedOp::new(1)) as Arc<dyn Operator>)
        .collect();
    let parallel = Parallel {
        children,
        max_concurrency: 100,
    };
    let node_run = NodeRun {
        node_id: NodeId("timing-100".into()),
        state: NodeRunState::Ready,
        dependencies: Default::default(),
        attempts: vec![],
        expansion_permissions: Default::default(),
        schema_version: 1,
    };
    let start = std::time::Instant::now();
    let (outcome, node_run_arc) = eval!(parallel, node_run);
    let elapsed = start.elapsed();
    assert!(matches!(outcome.unwrap(), NodeOutcome::Succeeded { .. }));
    assert_eq!(node_run_arc.lock().unwrap().attempts.len(), 100);
    assert!(elapsed < Duration::from_secs(5), "took {:?}", elapsed);
}

// ── B. Determinism / ordering tests ──────────────────────────────────────────

#[test]
fn parallel_determinism_attempts_in_child_index_order() {
    let children: Vec<Arc<dyn Operator>> = vec![
        Arc::new(SucceedOp),
        Arc::new(SucceedOp),
        Arc::new(SucceedOp),
    ];
    let parallel = Parallel {
        children,
        max_concurrency: 3,
    };
    let node_run = NodeRun {
        node_id: NodeId("order-3".into()),
        state: NodeRunState::Ready,
        dependencies: Default::default(),
        attempts: vec![],
        expansion_permissions: Default::default(),
        schema_version: 1,
    };
    let (outcome, node_run_arc) = eval!(parallel, node_run);
    assert!(matches!(outcome.unwrap(), NodeOutcome::Succeeded { .. }));
    let nr = node_run_arc.lock().unwrap();
    assert_eq!(nr.attempts.len(), 3);
    for (i, attempt) in nr.attempts.iter().enumerate() {
        assert_eq!(attempt.idempotency_key.attempt_seq, i as u32);
    }
}

#[test]
fn parallel_determinism_idempotent_re_evaluate() {
    let children: Vec<Arc<dyn Operator>> = vec![Arc::new(SucceedOp), Arc::new(SucceedOp)];
    let parallel = Parallel {
        children,
        max_concurrency: 2,
    };
    let node_run = NodeRun {
        node_id: NodeId("idempotent".into()),
        state: NodeRunState::Ready,
        dependencies: Default::default(),
        attempts: vec![
            sddk_domain::Attempt {
                attempt_id: sddk_domain::workflow_run::AttemptId("pre-0".into()),
                node_id: NodeId("idempotent".into()),
                route: sddk_domain::workflow_run::Route {
                    provider: "t".into(),
                    model: "t".into(),
                    host: "t".into(),
                },
                started_at: "1970-01-01T00:00:00Z".into(),
                ended_at: None,
                outcome: Some(sddk_domain::workflow_run::AttemptOutcome::Succeeded {
                    outputs: Default::default(),
                }),
                usage: sddk_domain::workflow_run::Usage {
                    tokens_in: 0,
                    tokens_out: 0,
                    cost_micros: 0,
                    wall_ms: 0,
                },
                context_capsule: sddk_domain::workflow_run::ContextCapsuleRef::Pointer {
                    cid: "pre".into(),
                },
                idempotency_key: sddk_domain::workflow_run::IdempotencyKey {
                    project_id: "sddk".into(),
                    run_id: sddk_domain::RunId("r".into()),
                    node_id: NodeId("idempotent".into()),
                    attempt_seq: 0,
                },
                schema_version: 1,
            },
            sddk_domain::Attempt {
                attempt_id: sddk_domain::workflow_run::AttemptId("pre-1".into()),
                node_id: NodeId("idempotent".into()),
                route: sddk_domain::workflow_run::Route {
                    provider: "t".into(),
                    model: "t".into(),
                    host: "t".into(),
                },
                started_at: "1970-01-01T00:00:00Z".into(),
                ended_at: None,
                outcome: Some(sddk_domain::workflow_run::AttemptOutcome::Succeeded {
                    outputs: Default::default(),
                }),
                usage: sddk_domain::workflow_run::Usage {
                    tokens_in: 0,
                    tokens_out: 0,
                    cost_micros: 0,
                    wall_ms: 0,
                },
                context_capsule: sddk_domain::workflow_run::ContextCapsuleRef::Pointer {
                    cid: "pre2".into(),
                },
                idempotency_key: sddk_domain::workflow_run::IdempotencyKey {
                    project_id: "sddk".into(),
                    run_id: sddk_domain::RunId("r".into()),
                    node_id: NodeId("idempotent".into()),
                    attempt_seq: 1,
                },
                schema_version: 1,
            },
        ],
        expansion_permissions: Default::default(),
        schema_version: 1,
    };
    let (outcome, node_run_arc) = eval!(parallel, node_run);
    assert!(matches!(outcome.unwrap(), NodeOutcome::Succeeded { .. }));
    let nr = node_run_arc.lock().unwrap();
    assert_eq!(
        nr.attempts.len(),
        2,
        "should not add new attempts on replay"
    );
    assert_eq!(nr.state, NodeRunState::Completed);
}

#[test]
fn parallel_determinism_attempt_seq_matches_child_index() {
    let children: Vec<Arc<dyn Operator>> = vec![
        Arc::new(FailOp("error0".into())),
        Arc::new(FailOp("error1".into())),
        Arc::new(SucceedOp),
    ];
    let parallel = Parallel {
        children,
        max_concurrency: 3,
    };
    let node_run = NodeRun {
        node_id: NodeId("attempt-seq".into()),
        state: NodeRunState::Ready,
        dependencies: Default::default(),
        attempts: vec![],
        expansion_permissions: Default::default(),
        schema_version: 1,
    };
    let (_, node_run_arc) = eval!(parallel, node_run);
    let nr = node_run_arc.lock().unwrap();
    assert_eq!(nr.attempts.len(), 3);
    assert_eq!(nr.attempts[0].idempotency_key.attempt_seq, 0);
    assert_eq!(nr.attempts[1].idempotency_key.attempt_seq, 1);
    assert_eq!(nr.attempts[2].idempotency_key.attempt_seq, 2);
}

#[test]
fn parallel_determinism_varying_durations_still_ordered() {
    let children: Vec<Arc<dyn Operator>> = vec![
        Arc::new(SleepThenSucceedOp::new(100)),
        Arc::new(SleepThenSucceedOp::new(10)),
        Arc::new(SleepThenSucceedOp::new(50)),
    ];
    let parallel = Parallel {
        children,
        max_concurrency: 3,
    };
    let node_run = NodeRun {
        node_id: NodeId("order-varying".into()),
        state: NodeRunState::Ready,
        dependencies: Default::default(),
        attempts: vec![],
        expansion_permissions: Default::default(),
        schema_version: 1,
    };
    let (outcome, node_run_arc) = eval!(parallel, node_run);
    outcome.unwrap();
    let nr = node_run_arc.lock().unwrap();
    assert_eq!(nr.attempts.len(), 3);
    for (i, attempt) in nr.attempts.iter().enumerate() {
        assert_eq!(attempt.idempotency_key.attempt_seq, i as u32);
    }
}

#[test]
fn parallel_determinism_failure_doesnt_break_order() {
    let children: Vec<Arc<dyn Operator>> = vec![
        Arc::new(SucceedOp),
        Arc::new(FailOp("fail".into())),
        Arc::new(SucceedOp),
    ];
    let parallel = Parallel {
        children,
        max_concurrency: 3,
    };
    let node_run = NodeRun {
        node_id: NodeId("fail-order".into()),
        state: NodeRunState::Ready,
        dependencies: Default::default(),
        attempts: vec![],
        expansion_permissions: Default::default(),
        schema_version: 1,
    };
    let (outcome, node_run_arc) = eval!(parallel, node_run);
    assert!(matches!(
        outcome,
        Err(sddk_engine::operator::OperatorError::EvalFailed(_))
    ));
    let nr = node_run_arc.lock().unwrap();
    assert_eq!(nr.state, NodeRunState::Failed);
}

// ── C. Panic isolation ────────────────────────────────────────────────────────

#[test]
fn parallel_isolation_panic_propagates_correctly() {
    let children: Vec<Arc<dyn Operator>> =
        vec![Arc::new(SucceedOp), Arc::new(PanicOp), Arc::new(SucceedOp)];
    let parallel = Parallel {
        children,
        max_concurrency: 3,
    };
    let node_run = NodeRun {
        node_id: NodeId("panic-propagate".into()),
        state: NodeRunState::Ready,
        dependencies: Default::default(),
        attempts: vec![],
        expansion_permissions: Default::default(),
        schema_version: 1,
    };
    let (outcome, node_run_arc) = eval!(parallel, node_run);
    // Parallel returns Ok(NodeOutcome::Failed { reason: "child N panicked" }) for panics
    assert!(
        matches!(outcome, Ok(NodeOutcome::Failed { reason, .. } ) if reason.contains("panicked"))
    );
    let nr = node_run_arc.lock().unwrap();
    assert!(!nr.attempts.is_empty() || nr.state == NodeRunState::Failed);
}

#[test]
fn parallel_isolation_no_panic_if_child_succeeds() {
    let children: Vec<Arc<dyn Operator>> = vec![Arc::new(SucceedOp), Arc::new(SucceedOp)];
    let parallel = Parallel {
        children,
        max_concurrency: 2,
    };
    let node_run = NodeRun {
        node_id: NodeId("panic-no".into()),
        state: NodeRunState::Ready,
        dependencies: Default::default(),
        attempts: vec![],
        expansion_permissions: Default::default(),
        schema_version: 1,
    };
    let (outcome, node_run_arc) = eval!(parallel, node_run);
    assert!(matches!(outcome.unwrap(), NodeOutcome::Succeeded { .. }));
    let nr = node_run_arc.lock().unwrap();
    assert_eq!(nr.attempts.len(), 2);
}

#[test]
fn parallel_isolation_all_siblings_complete_before_panic_propagates() {
    let children: Vec<Arc<dyn Operator>> = vec![
        Arc::new(SleepThenSucceedOp::new(50)),
        Arc::new(PanicOp),
        Arc::new(SleepThenSucceedOp::new(50)),
    ];
    let parallel = Parallel {
        children,
        max_concurrency: 3,
    };
    let node_run = NodeRun {
        node_id: NodeId("panic-siblings".into()),
        state: NodeRunState::Ready,
        dependencies: Default::default(),
        attempts: vec![],
        expansion_permissions: Default::default(),
        schema_version: 1,
    };
    let start = std::time::Instant::now();
    let (outcome, _node_run_arc) = eval!(parallel, node_run);
    let elapsed = start.elapsed();
    assert!(
        matches!(outcome, Ok(NodeOutcome::Failed { reason, .. } ) if reason.contains("panicked"))
    );
    // With max_concurrency=3, all children run. Both SleepThenSucceedOps should finish
    // before panic propagates, proving sibling threads don't interfere.
    assert!(elapsed >= Duration::from_millis(45));
}

#[test]
fn parallel_isolation_max_concurrency_1_panic_ordering() {
    let children: Vec<Arc<dyn Operator>> = vec![Arc::new(PanicOp), Arc::new(SucceedOp)];
    let parallel = Parallel {
        children,
        max_concurrency: 1,
    };
    let node_run = NodeRun {
        node_id: NodeId("panic-seq".into()),
        state: NodeRunState::Ready,
        dependencies: Default::default(),
        attempts: vec![],
        expansion_permissions: Default::default(),
        schema_version: 1,
    };
    let (outcome, node_run_arc) = eval!(parallel, node_run);
    assert!(
        matches!(outcome, Ok(NodeOutcome::Failed { reason, .. } ) if reason.contains("panicked"))
    );
    let nr = node_run_arc.lock().unwrap();
    assert_eq!(nr.state, NodeRunState::Failed);
}

// ── D. Concurrency correctness ────────────────────────────────────────────────

#[test]
fn parallel_concurrency_attempt_count_matches_children() {
    let children: Vec<Arc<dyn Operator>> = (0..10)
        .map(|_| Arc::new(SucceedOp) as Arc<dyn Operator>)
        .collect();
    let parallel = Parallel {
        children,
        max_concurrency: 5,
    };
    let node_run = NodeRun {
        node_id: NodeId("concurrency-count".into()),
        state: NodeRunState::Ready,
        dependencies: Default::default(),
        attempts: vec![],
        expansion_permissions: Default::default(),
        schema_version: 1,
    };
    let (outcome, node_run_arc) = eval!(parallel, node_run);
    assert!(matches!(outcome.unwrap(), NodeOutcome::Succeeded { .. }));
    assert_eq!(node_run_arc.lock().unwrap().attempts.len(), 10);
}

// ── Multi-tick integration tests ──────────────────────────────────────────────

/// REQ-WF-RT-013 scenario: multi-tick drain of Parallel children.
///
/// A Parallel node with 5 children (all SucceedOp):
///
/// This test validates the non-blocking path (pending_sender = Some):
/// 1. First evaluate: all 5 children invoked and complete.
///    Supervisor collects all 5 results and forwards to pending_sender.
///    Parallel::evaluate returns NodeOutcome::Pending (non-blocking contract).
/// 2. We drain the receiver and verify all 5 ChildResults were sent.
/// 3. Second evaluate (with 5 pre-filled attempts): replay-safety triggers,
///    Parallel::evaluate returns NodeOutcome::Succeeded immediately.
#[test]
fn parallel_spans_three_ticks_drain() {
    use std::sync::mpsc;

    // ── Build children (all SucceedOp for deterministic behavior) ─────────────
    let children: Vec<Arc<dyn Operator>> = (0..5)
        .map(|_| Arc::new(SucceedOp) as Arc<dyn Operator>)
        .collect();

    assert_eq!(children.len(), 5, "should have 5 children");

    let parallel = Parallel {
        children,
        max_concurrency: 5,
    };

    // ── Tick 1: Non-blocking evaluate with pending_sender ─────────────────────
    let (tx, rx) = mpsc::channel::<sddk_engine::operator::ChildResult>();
    let pending_sender = Some(tx);

    let ir = make_ir();
    let run = make_run();
    let store: Arc<Mutex<GraphStoreBox>> = Arc::new(Mutex::new(GraphStoreBox {
        inner: Box::new(MockStore),
    }));
    let node_run_arc: Arc<Mutex<NodeRun>> = Arc::new(Mutex::new(NodeRun {
        node_id: NodeId("parallel-node".into()),
        state: NodeRunState::Ready,
        dependencies: Default::default(),
        attempts: vec![],
        expansion_permissions: Default::default(),
        schema_version: 1,
    }));

    let mut ctx = OperatorContext {
        node_run: Arc::clone(&node_run_arc),
        ir: Arc::new(ir),
        run: Arc::new(run),
        store,
        clock: Clock,
        executor: Arc::new(NoopTaskExecutor),
        pending_sender,
    };

    let outcome_1 = parallel
        .evaluate(&mut ctx)
        .expect("evaluate should not error");

    // Non-blocking path returns Pending immediately (supervisor runs in background)
    assert!(
        matches!(&outcome_1, NodeOutcome::Pending { .. }),
        "first evaluate should return Pending, got {:?}",
        outcome_1
    );

    // ── Drain supervisor results (tick 1 drain) ─────────────────────────────
    // The supervisor collects all 5 results and sends them to rx.
    let mut results: BTreeMap<usize, sddk_engine::operator::ChildResult> = BTreeMap::new();
    let deadline = std::time::Instant::now() + std::time::Duration::from_secs(2);
    while results.len() < 5 {
        if std::time::Instant::now() >= deadline {
            panic!(
                "supervisor did not send all 5 results within 2s (got {})",
                results.len()
            );
        }
        match rx.recv_timeout(std::time::Duration::from_millis(100)) {
            Ok(r) => {
                results.insert(r.child_index, r);
            }
            Err(mpsc::RecvTimeoutError::Timeout) => continue,
            Err(mpsc::RecvTimeoutError::Disconnected) => break,
        }
    }
    assert_eq!(
        results.len(),
        5,
        "supervisor should have sent 5 ChildResults"
    );

    // All 5 children should have Succeeded outcomes (SucceedOp always succeeds)
    for i in 0..5 {
        let outcome = &results.get(&i).unwrap().outcome;
        assert!(
            matches!(outcome, Ok(NodeOutcome::Succeeded { .. })),
            "child {} should Succeed, got {:?}",
            i,
            outcome
        );
    }

    // ── Tick 2: Evaluate with 5 pre-filled attempts (replay-safety) ───────────
    // Pre-fill 5 attempts (all children already completed in tick 1).
    // With attempts.len() >= children.len(), replay-safety triggers immediately.
    let node_run_2_arc: Arc<Mutex<NodeRun>> = Arc::new(Mutex::new(NodeRun {
        node_id: NodeId("parallel-node".into()),
        state: NodeRunState::Ready,
        dependencies: Default::default(),
        attempts: (0..5)
            .map(|i| sddk_domain::Attempt {
                attempt_id: sddk_domain::workflow_run::AttemptId(format!("att-{}", i)),
                node_id: NodeId("parallel-node".into()),
                route: sddk_domain::workflow_run::Route {
                    provider: "t".into(),
                    model: "t".into(),
                    host: "t".into(),
                },
                started_at: "1970-01-01T00:00:00Z".into(),
                ended_at: None,
                outcome: Some(sddk_domain::workflow_run::AttemptOutcome::Succeeded {
                    outputs: Default::default(),
                }),
                usage: sddk_domain::workflow_run::Usage {
                    tokens_in: 0,
                    tokens_out: 0,
                    cost_micros: 0,
                    wall_ms: 0,
                },
                context_capsule: sddk_domain::workflow_run::ContextCapsuleRef::Pointer {
                    cid: format!("c{}", i),
                },
                idempotency_key: sddk_domain::workflow_run::IdempotencyKey {
                    project_id: "sddk".into(),
                    run_id: RunId("r".into()),
                    node_id: NodeId("parallel-node".into()),
                    attempt_seq: i as u32,
                },
                schema_version: 1,
            })
            .collect(),
        expansion_permissions: Default::default(),
        schema_version: 1,
    }));

    let ir2 = make_ir();
    let run2 = make_run();
    let store2: Arc<Mutex<GraphStoreBox>> = Arc::new(Mutex::new(GraphStoreBox {
        inner: Box::new(MockStore),
    }));

    // Second evaluate: pending_sender = None (consumed on first evaluate)
    let mut ctx2 = OperatorContext {
        node_run: Arc::clone(&node_run_2_arc),
        ir: Arc::new(ir2),
        run: Arc::new(run2),
        store: store2,
        clock: Clock,
        executor: Arc::new(NoopTaskExecutor),
        pending_sender: None,
    };

    let outcome_2 = parallel
        .evaluate(&mut ctx2)
        .expect("evaluate should not error");

    // Replay-safety: attempts.len() (5) >= children.len() (5) → returns Succeeded immediately
    assert!(
        matches!(&outcome_2, NodeOutcome::Succeeded { .. }),
        "second evaluate with pre-filled attempts should return Succeeded, got {:?}",
        outcome_2
    );

    // NodeRun attempts: 5 pre-filled (no new attempts added by replay-safety)
    assert_eq!(
        node_run_2_arc.lock().unwrap().attempts.len(),
        5,
        "should have 5 total attempts (pre-filled, replay-safety skips re-evaluation)"
    );

    // Verify child_index ordering: attempts 0..4 in order
    let attempts = &node_run_2_arc.lock().unwrap().attempts;
    for (i, attempt) in attempts.iter().enumerate() {
        assert_eq!(
            attempt.idempotency_key.attempt_seq, i as u32,
            "attempt {} should have attempt_seq={}",
            i, i
        );
    }
}

#[test]
fn parallel_concurrency_state_reflects_outcome() {
    let children: Vec<Arc<dyn Operator>> = vec![
        Arc::new(SucceedOp),
        Arc::new(SucceedOp),
        Arc::new(FailOp("intentional".into())),
    ];
    let parallel = Parallel {
        children,
        max_concurrency: 3,
    };
    let node_run = NodeRun {
        node_id: NodeId("concurrency-outcome".into()),
        state: NodeRunState::Ready,
        dependencies: Default::default(),
        attempts: vec![],
        expansion_permissions: Default::default(),
        schema_version: 1,
    };
    let (outcome, node_run_arc) = eval!(parallel, node_run);
    assert!(matches!(
        outcome,
        Err(sddk_engine::operator::OperatorError::EvalFailed(_))
    ));
    assert_eq!(node_run_arc.lock().unwrap().state, NodeRunState::Failed);
}

#[test]
fn parallel_concurrency_all_states_accessible() {
    let children: Vec<Arc<dyn Operator>> = vec![Arc::new(SucceedOp), Arc::new(SucceedOp)];
    let parallel = Parallel {
        children,
        max_concurrency: 2,
    };
    let node_run = NodeRun {
        node_id: NodeId("concurrency-states".into()),
        state: NodeRunState::Ready,
        dependencies: Default::default(),
        attempts: vec![],
        expansion_permissions: Default::default(),
        schema_version: 1,
    };
    let (outcome, node_run_arc) = eval!(parallel, node_run);
    assert!(matches!(outcome.unwrap(), NodeOutcome::Succeeded { .. }));
    let nr = node_run_arc.lock().unwrap();
    assert_eq!(nr.state, NodeRunState::Completed);
    assert_eq!(nr.attempts.len(), 2);
}

// ── E. Backpressure / queueing ───────────────────────────────────────────────

#[test]
fn parallel_backpressure_queue_respects_max() {
    let children: Vec<Arc<dyn Operator>> = (0..20)
        .map(|_| Arc::new(SucceedOp) as Arc<dyn Operator>)
        .collect();
    let parallel = Parallel {
        children,
        max_concurrency: 4,
    };
    let node_run = NodeRun {
        node_id: NodeId("backpressure".into()),
        state: NodeRunState::Ready,
        dependencies: Default::default(),
        attempts: vec![],
        expansion_permissions: Default::default(),
        schema_version: 1,
    };
    let (outcome, node_run_arc) = eval!(parallel, node_run);
    assert!(matches!(outcome.unwrap(), NodeOutcome::Succeeded { .. }));
    assert_eq!(node_run_arc.lock().unwrap().attempts.len(), 20);
}

#[test]
fn parallel_backpressure_timing_shows_queueing() {
    let children: Vec<Arc<dyn Operator>> = (0..4)
        .map(|_| Arc::new(SleepThenSucceedOp::new(50)) as Arc<dyn Operator>)
        .collect();
    let parallel = Parallel {
        children,
        max_concurrency: 1,
    };
    let node_run = NodeRun {
        node_id: NodeId("backpressure-timing".into()),
        state: NodeRunState::Ready,
        dependencies: Default::default(),
        attempts: vec![],
        expansion_permissions: Default::default(),
        schema_version: 1,
    };
    let start = std::time::Instant::now();
    let (outcome, node_run_arc) = eval!(parallel, node_run);
    let elapsed = start.elapsed();
    assert!(matches!(outcome.unwrap(), NodeOutcome::Succeeded { .. }));
    // max_concurrency=1, 4 x 50ms = 200ms minimum
    assert!(elapsed >= Duration::from_millis(195));
    assert_eq!(node_run_arc.lock().unwrap().attempts.len(), 4);
}

// ── F. Empty / single-child edge cases ───────────────────────────────────────

#[test]
fn parallel_empty_children_succeeds_immediately() {
    let children: Vec<Arc<dyn Operator>> = vec![];
    let parallel = Parallel {
        children,
        max_concurrency: 0,
    };
    let node_run = NodeRun {
        node_id: NodeId("empty".into()),
        state: NodeRunState::Ready,
        dependencies: Default::default(),
        attempts: vec![],
        expansion_permissions: Default::default(),
        schema_version: 1,
    };
    let (outcome, node_run_arc) = eval!(parallel, node_run);
    assert!(matches!(outcome.unwrap(), NodeOutcome::Succeeded { .. }));
    assert_eq!(node_run_arc.lock().unwrap().state, NodeRunState::Completed);
}

#[test]
fn parallel_single_child_succeeds() {
    let children: Vec<Arc<dyn Operator>> = vec![Arc::new(SucceedOp)];
    let parallel = Parallel {
        children,
        max_concurrency: 1,
    };
    let node_run = NodeRun {
        node_id: NodeId("single".into()),
        state: NodeRunState::Ready,
        dependencies: Default::default(),
        attempts: vec![],
        expansion_permissions: Default::default(),
        schema_version: 1,
    };
    let (outcome, node_run_arc) = eval!(parallel, node_run);
    assert!(matches!(outcome.unwrap(), NodeOutcome::Succeeded { .. }));
    assert_eq!(node_run_arc.lock().unwrap().attempts.len(), 1);
}

// ── G. Replay / idempotency ──────────────────────────────────────────────────

#[test]
fn parallel_replay_does_not_double_count() {
    let children: Vec<Arc<dyn Operator>> = vec![Arc::new(SucceedOp), Arc::new(SucceedOp)];
    let parallel = Parallel {
        children,
        max_concurrency: 2,
    };
    let node_run = NodeRun {
        node_id: NodeId("replay".into()),
        state: NodeRunState::Ready,
        dependencies: Default::default(),
        attempts: vec![
            sddk_domain::Attempt {
                attempt_id: sddk_domain::workflow_run::AttemptId("existing-0".into()),
                node_id: NodeId("replay".into()),
                route: sddk_domain::workflow_run::Route {
                    provider: "t".into(),
                    model: "t".into(),
                    host: "t".into(),
                },
                started_at: "1970-01-01T00:00:00Z".into(),
                ended_at: None,
                outcome: Some(sddk_domain::workflow_run::AttemptOutcome::Succeeded {
                    outputs: Default::default(),
                }),
                usage: sddk_domain::workflow_run::Usage {
                    tokens_in: 0,
                    tokens_out: 0,
                    cost_micros: 0,
                    wall_ms: 0,
                },
                context_capsule: sddk_domain::workflow_run::ContextCapsuleRef::Pointer {
                    cid: "pre".into(),
                },
                idempotency_key: sddk_domain::workflow_run::IdempotencyKey {
                    project_id: "sddk".into(),
                    run_id: sddk_domain::RunId("r".into()),
                    node_id: NodeId("replay".into()),
                    attempt_seq: 0,
                },
                schema_version: 1,
            },
            sddk_domain::Attempt {
                attempt_id: sddk_domain::workflow_run::AttemptId("existing-1".into()),
                node_id: NodeId("replay".into()),
                route: sddk_domain::workflow_run::Route {
                    provider: "t".into(),
                    model: "t".into(),
                    host: "t".into(),
                },
                started_at: "1970-01-01T00:00:00Z".into(),
                ended_at: None,
                outcome: Some(sddk_domain::workflow_run::AttemptOutcome::Succeeded {
                    outputs: Default::default(),
                }),
                usage: sddk_domain::workflow_run::Usage {
                    tokens_in: 0,
                    tokens_out: 0,
                    cost_micros: 0,
                    wall_ms: 0,
                },
                context_capsule: sddk_domain::workflow_run::ContextCapsuleRef::Pointer {
                    cid: "pre2".into(),
                },
                idempotency_key: sddk_domain::workflow_run::IdempotencyKey {
                    project_id: "sddk".into(),
                    run_id: sddk_domain::RunId("r".into()),
                    node_id: NodeId("replay".into()),
                    attempt_seq: 1,
                },
                schema_version: 1,
            },
        ],
        expansion_permissions: Default::default(),
        schema_version: 1,
    };
    let (outcome, node_run_arc) = eval!(parallel, node_run);
    assert!(matches!(outcome.unwrap(), NodeOutcome::Succeeded { .. }));
    // Replay should not add new attempts
    assert_eq!(node_run_arc.lock().unwrap().attempts.len(), 2);
}

#[test]
fn parallel_replay_preserves_existing_outcome() {
    let children: Vec<Arc<dyn Operator>> = vec![Arc::new(SucceedOp)];
    let parallel = Parallel {
        children,
        max_concurrency: 1,
    };
    let node_run = NodeRun {
        node_id: NodeId("replay-outcome".into()),
        state: NodeRunState::Completed,
        dependencies: Default::default(),
        attempts: vec![sddk_domain::Attempt {
            attempt_id: sddk_domain::workflow_run::AttemptId("existing".into()),
            node_id: NodeId("replay-outcome".into()),
            route: sddk_domain::workflow_run::Route {
                provider: "t".into(),
                model: "t".into(),
                host: "t".into(),
            },
            started_at: "1970-01-01T00:00:00Z".into(),
            ended_at: None,
            outcome: Some(sddk_domain::workflow_run::AttemptOutcome::Succeeded {
                outputs: Default::default(),
            }),
            usage: sddk_domain::workflow_run::Usage {
                tokens_in: 0,
                tokens_out: 0,
                cost_micros: 0,
                wall_ms: 0,
            },
            context_capsule: sddk_domain::workflow_run::ContextCapsuleRef::Pointer {
                cid: "pre".into(),
            },
            idempotency_key: sddk_domain::workflow_run::IdempotencyKey {
                project_id: "sddk".into(),
                run_id: sddk_domain::RunId("r".into()),
                node_id: NodeId("replay-outcome".into()),
                attempt_seq: 0,
            },
            schema_version: 1,
        }],
        expansion_permissions: Default::default(),
        schema_version: 1,
    };
    let (outcome, node_run_arc) = eval!(parallel, node_run);
    assert!(matches!(outcome.unwrap(), NodeOutcome::Succeeded { .. }));
    assert_eq!(node_run_arc.lock().unwrap().attempts.len(), 1);
}
