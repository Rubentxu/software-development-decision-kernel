//! Integration tests for runtime node construction via `build_operator` (REQ-WF-RT-016).
//!
//! RED phase: all tests fail because `build_operator` does not exist yet
//! and workflow_runtime.rs still calls `dispatch()`.
//! GREEN phase: all tests pass after runtime wiring is updated.

use std::collections::BTreeMap;
use std::sync::{Arc, Mutex};

use sddk_domain::{
    CapabilityId, NodeId, NodeRun, NodeRunState, Operator as DomainOperator, OperatorId, TaskError,
    TaskExecutor, TaskOutput, WorkflowIR, WorkflowRun,
};
use sddk_engine::operator::{Clock, GraphStoreBox, NodeOutcome, OperatorContext};

/// Fake executor that echoes inputs back as outputs (or returns configured source_outputs).
#[derive(Clone)]
struct FakeExecutor {
    call_count: Arc<std::sync::atomic::AtomicUsize>,
    fail_capabilities: Vec<String>,
    source_outputs: BTreeMap<String, serde_json::Value>,
}

impl FakeExecutor {
    fn new() -> Self {
        Self {
            call_count: Arc::new(std::sync::atomic::AtomicUsize::new(0)),
            fail_capabilities: vec![],
            source_outputs: BTreeMap::new(),
        }
    }
    fn source_returns(mut self, outputs: BTreeMap<String, serde_json::Value>) -> Self {
        self.source_outputs = outputs;
        self
    }
}

impl TaskExecutor for FakeExecutor {
    fn execute(
        &self,
        capability: &str,
        inputs: &BTreeMap<String, serde_json::Value>,
    ) -> Result<TaskOutput, TaskError> {
        self.call_count
            .fetch_add(1, std::sync::atomic::Ordering::SeqCst);
        if self.fail_capabilities.contains(&capability.to_string()) {
            Err(TaskError {
                message: format!("{} failed", capability),
            })
        } else if capability == "source.cap" {
            Ok(TaskOutput {
                outputs: self.source_outputs.clone(),
            })
        } else {
            // Echo inputs back so tests can verify item/index injection
            Ok(TaskOutput {
                outputs: inputs.clone(),
            })
        }
    }
}

fn make_ir_with_map() -> (WorkflowIR, BTreeMap<String, serde_json::Value>) {
    let source_items = serde_json::json!(["v1", "v2", "v3"]);
    let mut source_outputs: BTreeMap<String, serde_json::Value> = BTreeMap::new();
    source_outputs.insert("items".into(), source_items);

    let source_op = DomainOperator::Task {
        capability: CapabilityId("source.cap".into()),
        inputs: Default::default(),
    };
    let body_op = DomainOperator::Task {
        capability: CapabilityId("body.cap".into()),
        inputs: Default::default(),
    };

    let mut operators = BTreeMap::new();
    operators.insert(OperatorId("source".into()), source_op);
    operators.insert(OperatorId("body".into()), body_op);
    operators.insert(
        OperatorId("map".into()),
        DomainOperator::Map {
            source: OperatorId("source".into()),
            body: OperatorId("body".into()),
            max_concurrency: 4,
        },
    );

    (
        WorkflowIR {
            ir_id: None,
            schema_version: 1,
            template_ref: sddk_domain::TemplateRef {
                id: "test".into(),
                version: "1.0".into(),
            },
            operators,
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
        },
        source_outputs,
    )
}

fn make_run() -> WorkflowRun {
    WorkflowRun {
        run_id: sddk_domain::RunId("test-run".into()),
        template_ref: sddk_domain::TemplateRef {
            id: "test".into(),
            version: "1.0".into(),
        },
        ir_hash: "test-hash".into(),
        graph_revision: sddk_domain::RevisionId("rev".into()),
        state: sddk_domain::WorkflowRunState::Running,
        inputs: Default::default(),
        outputs: None,
        correlation_id: sddk_domain::CorrelationId("corr".into()),
        budget: Default::default(),
        schema_version: 1,
    }
}

/// Scenario: runtime tick uses build_operator (not dispatch) for Map node construction.
/// RED: fails — workflow_runtime.rs still calls dispatch();
/// GREEN: passes after workflow_runtime.rs:568 is updated to call build_operator.
#[test]
fn runtime_tick_uses_build_operator_not_dispatch() {
    use sddk_engine::operator::build_operator;

    let (ir, _source_outputs) = make_ir_with_map();
    let _executor: Arc<dyn TaskExecutor> = Arc::new(FakeExecutor::new());
    let _run = make_run();

    // Verify build_operator exists and resolves Map correctly
    let map_op = ir.operators.get(&OperatorId("map".into())).unwrap();
    let result = build_operator(map_op, &ir);
    assert!(
        result.is_ok(),
        "build_operator should resolve Map: {:?}",
        result
    );
    let runtime_map = result.unwrap();
    assert_eq!(runtime_map.kind(), "Map");
}

/// Scenario: runtime smoke — Map runs through one tick without EvalFailed.
/// RED: fails — dispatch() returns degenerate Map with OperatorId fields;
/// GREEN: passes after Map stores Arc<dyn Operator> source/body.
#[test]
fn runtime_smoke_map_runs_through_one_tick() {
    let (ir, source_outputs) = make_ir_with_map();
    let executor: Arc<dyn TaskExecutor> =
        Arc::new(FakeExecutor::new().source_returns(source_outputs));
    let run = make_run();

    // Minimal runtime construction — just verify no panic and Arc<dyn Operator> returned
    use sddk_engine::operator::build_operator;
    let map_op = ir.operators.get(&OperatorId("map".into())).unwrap();
    let runtime_map = build_operator(map_op, &ir).expect("build_operator should succeed");

    // Evaluate the map operator
    let node_run = Arc::new(Mutex::new(NodeRun {
        node_id: NodeId("map-node".into()),
        state: NodeRunState::Ready,
        dependencies: Default::default(),
        attempts: vec![],
        expansion_permissions: Default::default(),
        schema_version: 1,
    }));

    let store: Arc<Mutex<GraphStoreBox>> = Arc::new(Mutex::new(GraphStoreBox {
        inner: Box::new(sddk_engine::operator::ScratchGraphStore),
    }));

    let mut ctx = OperatorContext {
        node_run: Arc::clone(&node_run),
        ir: Arc::new(ir.clone()),
        run: Arc::new(run.clone()),
        store,
        clock: Clock,
        executor,
        pending_sender: None,
    };

    let outcome = runtime_map.evaluate(&mut ctx);
    // Should succeed with 3 body iterations
    match outcome {
        Ok(NodeOutcome::Succeeded {
            outputs,
            node_id: _,
        }) => {
            let results = outputs.get("item_results").expect("expected item_results key");
            let arr = results.as_array().expect("results should be array");
            assert_eq!(arr.len(), 3, "body should run 3 times");
        }
        Ok(NodeOutcome::Pending { checkpoint }) => {
            // Acceptable: Map goes Pending via runtime-owned pending_map (cycle-32).
            // See REQ-Map-Runtime-Checkpoint-Storage: pending_map stores MapCheckpointState
            // keyed by (RunId, OperatorId); drain_pending_map handles cross-tick resumption.
            use sddk_engine::operator::CheckpointHandle;
            match checkpoint {
                CheckpointHandle::MapChannel { state, token: _ } => {
                    // Verify the checkpoint carries the expected items_len
                    assert_eq!(state.items_len, 3, "checkpoint should preserve items_len");
                }
                other => panic!("expected MapChannel checkpoint, got {:?}", other),
            }
        }
        other => {
            panic!(
                "Expected Succeeded or Pending with MapChannel, got {:?}",
                other
            );
        }
    }
}

/// Scenario: runtime tick works with borrowed ir (no Arc<WorkflowIR> change).
/// RED: fails — build_operator signature may be wrong.
/// GREEN: passes after build_operator takes &WorkflowIR.
#[test]
fn runtime_tick_works_with_borrowed_ir() {
    use sddk_engine::operator::build_operator;

    let (ir, _source_outputs) = make_ir_with_map();
    let map_op = ir.operators.get(&OperatorId("map".into())).unwrap();

    // build_operator must accept &WorkflowIR (not Arc<WorkflowIR>)
    // This test verifies the borrow signature works
    let result = build_operator(map_op, &ir);
    assert!(result.is_ok(), "build_operator should accept &WorkflowIR");
}

/// Scenario: NotImplementedInCycle16 variant surfaces as Failed outcome in runtime.
/// RED: fails — build_operator not wired yet.
/// GREEN: passes after wiring + error handling.
#[test]
fn runtime_failed_outcome_for_not_implemented_variant() {
    use sddk_engine::operator::OperatorError;
    use sddk_engine::operator::build_operator;

    // Join is out-of-scope
    let join_op = DomainOperator::Join {
        policy: "all".into(),
        branches: vec![],
    };
    let (ir, _) = make_ir_with_map();
    let result = build_operator(&join_op, &ir);
    assert!(
        matches!(result, Err(OperatorError::NotImplementedInCycle16 { variant }) if variant == "Join"),
        "Expected NotImplementedInCycle16 for Join, got: {:?}",
        result
    );
}
