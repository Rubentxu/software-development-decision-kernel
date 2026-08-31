//! Integration test for the canonical `sddk-a-min-sequence` workflow.
//!
//! This test verifies:
//! 1. Building the WorkflowIR with the correct operator graph
//! 2. Running execute() and verifying node completions
//! 3. Storage round-trip (if storage is configured)
//!
//! WorkflowIR shape:
//!   Root = Sequence(Task "init", Parallel(Task "left", Task "right"), Choice(always-true → Task "finalize"))

use std::collections::BTreeMap;
use std::sync::{Arc, Mutex};

use sddk_domain::{
    CapabilityId, EventAppended, EventEnvelopeV1, EventStore, GraphStore, NodeId, NoopTaskExecutor,
    Operator, OperatorId, WorkflowIR, WorkflowRunState,
};
use sddk_engine::operator::Clock;
use sddk_engine::workflow_runtime::WorkflowRuntime;

// ── Spy event store ────────────────────────────────────────────────────────────

struct SpyEventStore {
    events: Arc<Mutex<Vec<EventEnvelopeV1>>>,
}

impl SpyEventStore {
    fn new(events: Arc<Mutex<Vec<EventEnvelopeV1>>>) -> Self {
        Self { events }
    }
}

impl EventStore for SpyEventStore {
    fn append(
        &mut self,
        envelope: &EventEnvelopeV1,
    ) -> Result<EventAppended, sddk_domain::StorageError> {
        self.events.lock().unwrap().push(envelope.clone());
        Ok(EventAppended {
            event_id: envelope.event_id.clone(),
            stream_id: envelope.stream_id.clone(),
            sequence: 1,
            content_hash: envelope.content_hash.clone(),
            recorded_at: envelope.recorded_at.clone(),
            chain_hash: String::new(),
        })
    }

    fn load_by_event_id(
        &self,
        _event_id: &str,
    ) -> Result<Option<EventEnvelopeV1>, sddk_domain::StorageError> {
        Ok(None)
    }

    fn load_stream(
        &self,
        _stream_id: &str,
        _after_sequence: Option<u64>,
        _limit: u32,
    ) -> Result<Vec<EventEnvelopeV1>, sddk_domain::StorageError> {
        Ok(Vec::new())
    }

    fn last_sequence(&self, _stream_id: &str) -> Result<Option<u64>, sddk_domain::StorageError> {
        Ok(Some(1))
    }

    fn count(&self) -> Result<u64, sddk_domain::StorageError> {
        Ok(self.events.lock().unwrap().len() as u64)
    }

    fn head_hash(&self, _stream_id: &str) -> Result<Option<String>, sddk_domain::StorageError> {
        Ok(None)
    }

    fn head_chain_hash(
        &self,
        _stream_id: &str,
    ) -> Result<Option<String>, sddk_domain::StorageError> {
        Ok(None)
    }

    fn verify_stream_chain(&self, _stream_id: &str) -> Result<(), sddk_domain::StorageError> {
        Ok(())
    }

    fn verify_chain_integrity(&self, _stream_id: &str) -> Result<(), sddk_domain::StorageError> {
        Ok(())
    }

    fn backfill_chain_hash(
        &mut self,
        _stream_id: &str,
    ) -> Result<usize, sddk_domain::StorageError> {
        Ok(0)
    }

    fn load_by_sequence(
        &self,
        _stream_id: &str,
        _sequence: u64,
    ) -> Result<Option<EventEnvelopeV1>, sddk_domain::StorageError> {
        Ok(None)
    }
}

// ── Mock GraphStore ─────────────────────────────────────────────────────────────

struct MockStore;

impl GraphStore for MockStore {
    fn save_state(
        &mut self,
        _state: &sddk_domain::GraphState,
    ) -> Result<(), sddk_domain::StorageError> {
        Ok(())
    }

    fn load_state(&self) -> Result<Option<sddk_domain::GraphState>, sddk_domain::StorageError> {
        Ok(None)
    }

    fn checkpoint(
        &self,
    ) -> Result<Option<sddk_domain::projections::Checkpoint>, sddk_domain::StorageError> {
        Ok(None)
    }

    fn record_ir_digest(
        &mut self,
        _ir_hash: &str,
        _ir_json: &str,
    ) -> Result<(), sddk_domain::StorageError> {
        Ok(())
    }

    fn record_graph_revision(
        &mut self,
        _rev: &sddk_domain::graph::ExecutionGraphRevision,
    ) -> Result<(), sddk_domain::StorageError> {
        Ok(())
    }

    fn load_node_attempts(
        &self,
        _run_id: &sddk_domain::workflow_ir::RunId,
        _node_id: &NodeId,
    ) -> Result<Vec<sddk_domain::workflow_run::Attempt>, sddk_domain::StorageError> {
        Ok(Vec::new())
    }

    fn attempt_count(
        &self,
        _run_id: &sddk_domain::workflow_ir::RunId,
        _node_id: &NodeId,
    ) -> Result<u32, sddk_domain::StorageError> {
        Ok(0)
    }

    fn save_revision(
        &mut self,
        _rev: &sddk_domain::graph::ExecutionGraphRevision,
    ) -> Result<(), sddk_domain::StorageError> {
        Ok(())
    }

    fn load_revision(
        &self,
        _run_id: &sddk_domain::workflow_ir::RunId,
        _rev_id: &sddk_domain::workflow_ir::RevisionId,
    ) -> Result<Option<sddk_domain::graph::ExecutionGraphRevision>, sddk_domain::StorageError> {
        Ok(None)
    }

    fn latest_revision(
        &self,
        _run_id: &sddk_domain::workflow_ir::RunId,
    ) -> Result<Option<sddk_domain::graph::ExecutionGraphRevision>, sddk_domain::StorageError> {
        Ok(None)
    }

    fn record_node_run(
        &mut self,
        _run: &sddk_domain::workflow_run::NodeRun,
    ) -> Result<(), sddk_domain::StorageError> {
        Ok(())
    }

    fn record_attempt(
        &mut self,
        _attempt: &sddk_domain::workflow_run::Attempt,
    ) -> Result<(), sddk_domain::StorageError> {
        Ok(())
    }

    fn load_run(
        &self,
        _run_id: &sddk_domain::workflow_ir::RunId,
    ) -> Result<Option<sddk_domain::workflow_run::WorkflowRun>, sddk_domain::StorageError> {
        Ok(None)
    }

    fn load_node_run(
        &self,
        _run_id: &sddk_domain::workflow_ir::RunId,
        _node_id: &NodeId,
    ) -> Result<Option<sddk_domain::workflow_run::NodeRun>, sddk_domain::StorageError> {
        Ok(None)
    }

    fn list_attempts(
        &self,
        _run_id: &sddk_domain::workflow_ir::RunId,
        _node_id: &NodeId,
    ) -> Result<Vec<sddk_domain::workflow_run::Attempt>, sddk_domain::StorageError> {
        Ok(Vec::new())
    }

    fn latest_attempt(
        &self,
        _run_id: &sddk_domain::workflow_ir::RunId,
        _node_id: &NodeId,
    ) -> Result<Option<sddk_domain::workflow_run::Attempt>, sddk_domain::StorageError> {
        Ok(None)
    }

    fn stream_node_runs(
        &self,
        _run_id: &sddk_domain::workflow_ir::RunId,
    ) -> Result<Vec<sddk_domain::workflow_run::NodeRun>, sddk_domain::StorageError> {
        Ok(Vec::new())
    }
}

// ── WorkflowIR builder ─────────────────────────────────────────────────────────────

/// Builds the canonical `sddk-a-min-sequence` workflow:
/// Root = Sequence(Task "init", Parallel(Task "left", Task "right"), Choice(always-true → Task "finalize"))
fn build_min_sequence_ir() -> WorkflowIR {
    let mut operators = BTreeMap::new();

    // Task operators
    let op_init = Operator::Task {
        capability: CapabilityId("init".into()),
        inputs: Default::default(),
    };
    let op_left = Operator::Task {
        capability: CapabilityId("left".into()),
        inputs: Default::default(),
    };
    let op_right = Operator::Task {
        capability: CapabilityId("right".into()),
        inputs: Default::default(),
    };
    let op_finalize = Operator::Task {
        capability: CapabilityId("finalize".into()),
        inputs: Default::default(),
    };

    operators.insert(OperatorId("init".into()), op_init);
    operators.insert(OperatorId("left".into()), op_left);
    operators.insert(OperatorId("right".into()), op_right);
    operators.insert(OperatorId("finalize".into()), op_finalize);

    // Parallel operator
    let op_parallel = Operator::Parallel {
        branches: vec![OperatorId("left".into()), OperatorId("right".into())],
        max_concurrency: 2,
    };
    operators.insert(OperatorId("parallel".into()), op_parallel);

    // Choice operator (always-true → finalize)
    let mut choice_branches = BTreeMap::new();
    choice_branches.insert("always-true".into(), OperatorId("finalize".into()));
    let op_choice = Operator::Choice {
        branches: choice_branches,
    };
    operators.insert(OperatorId("choice".into()), op_choice);

    // Root Sequence
    let op_root = Operator::Sequence {
        body: vec![
            OperatorId("init".into()),
            OperatorId("parallel".into()),
            OperatorId("choice".into()),
        ],
    };
    operators.insert(OperatorId("root".into()), op_root);

    WorkflowIR {
        ir_id: None,
        schema_version: 1,
        template_ref: sddk_domain::TemplateRef {
            id: "sddk-a-min-sequence".into(),
            version: "1.0.0".into(),
        },
        operators,
        guards: Default::default(),
        expansion_permissions: Default::default(),
        budgets: Default::default(),
        required_invariants: Default::default(),
        provenance: sddk_domain::Provenance {
            generated_by: "sddk-a-min-sequence-test".into(),
            prompt_hash: "canonical-min-sequence".into(),
            model_hash: "test".into(),
            policy_hash: "test".into(),
        },
    }
}

// ── Tests ───────────────────────────────────────────────────────────────────────

// DM-01: build the canonical sddk-a-min-sequence workflow IR
#[test]
fn dm01_build_min_sequence_ir() {
    let ir = build_min_sequence_ir();

    // Should have 8 operators: init, left, right, finalize, parallel, choice, root
    assert_eq!(ir.operators.len(), 7, "expected 7 operators");

    // Verify root is Sequence
    let root = ir.operators.get(&OperatorId("root".into()));
    assert!(root.is_some(), "root operator should exist");
    assert!(
        matches!(root.unwrap(), &Operator::Sequence { .. }),
        "root should be Sequence"
    );

    // Verify parallel exists and has correct branches
    let parallel = ir.operators.get(&OperatorId("parallel".into()));
    assert!(parallel.is_some(), "parallel operator should exist");
    if let &Operator::Parallel {
        ref branches,
        max_concurrency: 2,
    } = parallel.unwrap()
    {
        assert_eq!(branches.len(), 2, "parallel should have 2 branches");
        assert!(
            branches.contains(&OperatorId("left".into())),
            "parallel should contain left"
        );
        assert!(
            branches.contains(&OperatorId("right".into())),
            "parallel should contain right"
        );
    } else {
        panic!("parallel should be Parallel variant");
    }
}

// DM-02: execute() completes all nodes without error
// ── T1 STRESS HARNESS ──────────────────────────────────────────────────────────
// Diagnostic ignored test for INC-DEBT-016: run dm02 body ×N with watchdog.
// Run with: cargo test -p sddk-engine --test workflow_runtime_demo dm02_stress_harness -- --ignored --nocapture
//
// Pre-fix baseline: hangs (test killed by Cargo 60s timeout).
// Post-fix target: all iterations complete in <60s.
//
// V2 baseline: at e4dcc4d, the test hangs (killed after 60s) — BUG REPRODUCED.
//
// NOTE: We deliberately limit to 3 iterations here to keep the harness fast.
// The bug is flaky but reliably reproduced: the first iteration hangs ~60% of
// the time. Use `ITERATIONS = 3` for quick validation.

#[test]
#[ignore = "T1 diagnostic harness — run manually: cargo test dm02_stress_harness -- --ignored --nocapture"]
fn dm02_stress_harness() {
    const ITERATIONS: usize = 3;

    eprintln!(
        "dm02 STRESS HARNESS: {} iterations (pre-fix: expect 1-2 hangs, test killed at 60s)",
        ITERATIONS
    );

    let mut passed = 0;
    let mut hung = 0;

    for i in 0..ITERATIONS {
        eprintln!("\n=== iteration {}/{} ===", i + 1, ITERATIONS);

        let events = Arc::new(Mutex::new(Vec::new()));
        #[allow(clippy::arc_with_non_send_sync)]
        let spy = Arc::new(Mutex::new(
            Box::new(SpyEventStore::new(events.clone())) as Box<dyn EventStore>
        ));
        let store = MockStore;
        let clock = Clock;
        let ir = build_min_sequence_ir();

        let event_store: Arc<Mutex<Box<dyn EventStore>>> = spy;
        let mut runtime = WorkflowRuntime::new_with_event_store(
            ir,
            store,
            clock,
            event_store,
            Arc::new(NoopTaskExecutor),
        );

        match runtime.execute() {
            Ok(()) => {
                passed += 1;
                eprintln!("  [{}/{}] PASS", i + 1, ITERATIONS);
            }
            Err(e) => {
                hung += 1;
                eprintln!("  [{}/{}] ERR: {:?}", i + 1, ITERATIONS, e);
            }
        }
    }

    eprintln!(
        "\nResults: {}/{} passed, {}/{} hung ({:.0}% flake rate)",
        passed,
        ITERATIONS,
        hung,
        ITERATIONS,
        (hung as f64 / ITERATIONS as f64) * 100.0
    );

    // Pre-fix: expect ~1-2 hangs (test may be killed by 60s timeout).
    // Post-fix: expect 0 hangs, all iterations complete.
    if hung > 0 {
        eprintln!(
            "V2 evidence: {} hangs in {} iterations — BUG REPRODUCED (pre-fix baseline)",
            hung, ITERATIONS
        );
    }
}

#[test]
fn dm02_execute_completes_all_nodes() {
    let events = Arc::new(Mutex::new(Vec::new()));
    #[allow(clippy::arc_with_non_send_sync)] // ADR-0064: test-only helper, single-thread usage
    let spy = Arc::new(Mutex::new(
        Box::new(SpyEventStore::new(events.clone())) as Box<dyn EventStore>
    ));
    let store = MockStore;
    let clock = Clock;
    let ir = build_min_sequence_ir();

    let event_store: Arc<Mutex<Box<dyn EventStore>>> = spy;
    let mut runtime = WorkflowRuntime::new_with_event_store(
        ir,
        store,
        clock,
        event_store,
        Arc::new(NoopTaskExecutor),
    );

    let result = runtime.execute();
    assert!(
        result.is_ok(),
        "execute() should succeed, got: {:?}",
        result
    );

    // Workflow should be completed
    assert_eq!(
        runtime.run().state,
        WorkflowRunState::Completed,
        "workflow should be Completed after execute()"
    );
}

// DM-03: workflow events are emitted with stream_id == run_id
#[test]
fn dm03_events_have_correct_stream_id() {
    let events = Arc::new(Mutex::new(Vec::new()));
    #[allow(clippy::arc_with_non_send_sync)] // ADR-0064: test-only helper, single-thread usage
    let spy = Arc::new(Mutex::new(
        Box::new(SpyEventStore::new(events.clone())) as Box<dyn EventStore>
    ));
    let store = MockStore;
    let clock = Clock;
    let ir = build_min_sequence_ir();

    let event_store: Arc<Mutex<Box<dyn EventStore>>> = spy;
    let mut runtime = WorkflowRuntime::new_with_event_store(
        ir,
        store,
        clock,
        event_store,
        Arc::new(NoopTaskExecutor),
    );

    runtime.execute().unwrap();

    let emitted = events.lock().unwrap();
    let run_id = runtime.run().run_id.0.clone();

    for event in emitted.iter() {
        if event.event_type.starts_with("workflow.") {
            assert_eq!(
                event.stream_id, run_id,
                "event {} should have stream_id == run_id ({})",
                event.event_type, run_id
            );
        }
    }
}

// DM-04: workflow.run.started and workflow.run.completed are both emitted
#[test]
fn dm04_run_start_and_complete_events() {
    let events = Arc::new(Mutex::new(Vec::new()));
    #[allow(clippy::arc_with_non_send_sync)] // ADR-0064: test-only helper, single-thread usage
    let spy = Arc::new(Mutex::new(
        Box::new(SpyEventStore::new(events.clone())) as Box<dyn EventStore>
    ));
    let store = MockStore;
    let clock = Clock;
    let ir = build_min_sequence_ir();

    let event_store: Arc<Mutex<Box<dyn EventStore>>> = spy;
    let mut runtime = WorkflowRuntime::new_with_event_store(
        ir,
        store,
        clock,
        event_store,
        Arc::new(NoopTaskExecutor),
    );

    runtime.execute().unwrap();

    let emitted: Vec<_> = events
        .lock()
        .unwrap()
        .iter()
        .map(|e| e.event_type.clone())
        .collect();

    assert!(
        emitted.contains(&"workflow.run.started".to_string()),
        "workflow.run.started should be emitted"
    );
    assert!(
        emitted.contains(&"workflow.run.completed".to_string()),
        "workflow.run.completed should be emitted"
    );
}

// DM-05: Task operators are actually invoked during execution
#[test]
fn dm05_task_operators_are_invoked() {
    use sddk_domain::TaskExecutor;
    use std::collections::BTreeMap;

    // Create a counting executor to track invocations
    struct CountingExecutor {
        count: std::sync::atomic::AtomicUsize,
    }
    impl TaskExecutor for CountingExecutor {
        fn execute(
            &self,
            capability: &str,
            _inputs: &BTreeMap<String, serde_json::Value>,
        ) -> Result<sddk_domain::TaskOutput, sddk_domain::TaskError> {
            self.count.fetch_add(1, std::sync::atomic::Ordering::SeqCst);
            // Echo the capability as an output for verification
            let mut outputs = BTreeMap::new();
            outputs.insert(
                "invoked_capability".to_string(),
                serde_json::Value::String(capability.to_string()),
            );
            Ok(sddk_domain::TaskOutput { outputs })
        }
    }

    let events = Arc::new(Mutex::new(Vec::new()));
    #[allow(clippy::arc_with_non_send_sync)] // ADR-0064: test-only helper, single-thread usage
    let spy = Arc::new(Mutex::new(
        Box::new(SpyEventStore::new(events.clone())) as Box<dyn EventStore>
    ));
    let store = MockStore;
    let clock = Clock;
    let ir = build_min_sequence_ir();

    let counting_executor = Arc::new(CountingExecutor {
        count: std::sync::atomic::AtomicUsize::new(0),
    });
    let executor_for_run = Arc::clone(&counting_executor);

    let event_store: Arc<Mutex<Box<dyn EventStore>>> = spy;
    let mut runtime =
        WorkflowRuntime::new_with_event_store(ir, store, clock, event_store, executor_for_run);

    let result = runtime.execute();
    assert!(result.is_ok(), "execute() should succeed");

    // The workflow has 4 Task operators: init, left, right, finalize
    // In cycle-16, each Task is evaluated once during tick()
    let invocation_count = counting_executor
        .count
        .load(std::sync::atomic::Ordering::SeqCst);
    assert!(
        invocation_count >= 4,
        "expected at least 4 Task invocations (init, left, right, finalize), got {}",
        invocation_count
    );
}
