//! Unit tests: Map/Sequence/Parallel evaluate does NOT resolve via ctx.ir.operators.
//!
//! These tests prove the core invariant of cycle-31: Operator::evaluate must NOT
//! call ctx.ir.operators.get(...) for children it already owns as Arc<dyn Operator>.
//!
//! RED phase: all tests fail because Map::evaluate still calls ctx.ir.operators.get().
//! GREEN phase: all tests pass after Map::evaluate is refactored to use resolved slots.

use std::collections::BTreeMap;
use std::sync::{Arc, Mutex};

use sddk_domain::{
    CapabilityId, NodeId, NodeRun, NodeRunState, Operator as DomainOperator, OperatorId, TaskError,
    TaskExecutor, TaskOutput, WorkflowIR, WorkflowRun,
};
use sddk_engine::operator::build_operator;
use sddk_engine::operator::{Clock, GraphStoreBox, NodeOutcome, OperatorContext};

/// Fake executor that returns configurable outputs.
#[derive(Clone)]
struct FakeExecutor {
    source_outputs: BTreeMap<String, serde_json::Value>,
    body_outputs: BTreeMap<String, serde_json::Value>,
    body_should_fail: bool,
}

impl FakeExecutor {
    fn new() -> Self {
        Self {
            source_outputs: BTreeMap::new(),
            body_outputs: BTreeMap::new(),
            body_should_fail: false,
        }
    }
    fn source_returns(mut self, outputs: BTreeMap<String, serde_json::Value>) -> Self {
        self.source_outputs = outputs;
        self
    }
    fn body_returns(mut self, outputs: BTreeMap<String, serde_json::Value>) -> Self {
        self.body_outputs = outputs;
        self
    }
}

impl TaskExecutor for FakeExecutor {
    fn execute(
        &self,
        capability: &str,
        _inputs: &BTreeMap<String, serde_json::Value>,
    ) -> Result<TaskOutput, TaskError> {
        if capability == "source.cap" {
            Ok(TaskOutput {
                outputs: self.source_outputs.clone(),
            })
        } else if self.body_should_fail {
            Err(TaskError {
                message: "body failed explicitly".into(),
            })
        } else {
            Ok(TaskOutput {
                outputs: self.body_outputs.clone(),
            })
        }
    }
}

fn make_ir_with_source_and_body(source_id: OperatorId, body_id: OperatorId) -> WorkflowIR {
    let mut operators = BTreeMap::new();
    operators.insert(
        source_id.clone(),
        DomainOperator::Task {
            capability: CapabilityId("source.cap".into()),
            inputs: Default::default(),
        },
    );
    operators.insert(
        body_id.clone(),
        DomainOperator::Task {
            capability: CapabilityId("body.cap".into()),
            inputs: Default::default(),
        },
    );
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
    }
}

fn make_node_run() -> Arc<Mutex<NodeRun>> {
    Arc::new(Mutex::new(NodeRun {
        node_id: NodeId("test-node".into()),
        state: NodeRunState::Ready,
        dependencies: Default::default(),
        attempts: vec![],
        expansion_permissions: Default::default(),
        schema_version: 1,
    }))
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

fn make_ctx(
    node_run: Arc<Mutex<NodeRun>>,
    ir: Arc<WorkflowIR>,
    run: Arc<WorkflowRun>,
    executor: Arc<dyn TaskExecutor>,
) -> OperatorContext {
    let store: Arc<Mutex<GraphStoreBox>> = Arc::new(Mutex::new(GraphStoreBox {
        inner: Box::new(sddk_engine::operator::ScratchGraphStore),
    }));
    OperatorContext {
        node_run,
        ir,
        run,
        store,
        clock: Clock,
        executor,
        pending_sender: None,
    }
}

// ── Map tests ─────────────────────────────────────────────────────────────────

/// Scenario: Map evaluate does NOT call ctx.ir.operators.get for source or body.
/// RED: fails — Map::evaluate still calls ctx.ir.operators.get().
/// GREEN: passes after Map::evaluate uses self.source and self.body slots.
#[test]
fn map_evaluate_does_not_resolve_source_after_construction() {
    let source_id = OperatorId("source".into());
    let body_id = OperatorId("body".into());

    let source_items = serde_json::json!(["a", "b"]);
    let mut source_outputs = BTreeMap::new();
    source_outputs.insert("items".into(), source_items);

    let executor: Arc<dyn TaskExecutor> = Arc::new(
        FakeExecutor::new()
            .source_returns(source_outputs)
            .body_returns(BTreeMap::new()),
    );

    let ir = Arc::new(make_ir_with_source_and_body(
        source_id.clone(),
        body_id.clone(),
    ));
    let node_run = make_node_run();
    let run = make_run();

    // Build the Map operator via build_operator (Arc<dyn Operator>)
    let map_domain_op = DomainOperator::Map {
        source: source_id.clone(),
        body: body_id.clone(),
        max_concurrency: 4,
    };
    let runtime_map =
        build_operator(&map_domain_op, &ir).expect("build_operator should succeed for Map");

    // Remove source and body from the IR AFTER construction
    // If evaluate tries to resolve via ctx.ir.operators.get, it will fail
    let mut ir_mut = (*ir).clone();
    ir_mut.operators.remove(&source_id);
    ir_mut.operators.remove(&body_id);
    let ir_without_children = Arc::new(ir_mut);

    let mut ctx = make_ctx(
        Arc::clone(&node_run),
        ir_without_children,
        Arc::new(run),
        executor,
    );

    // Evaluate — should succeed using resolved Arc<dyn Operator> slots
    let outcome = runtime_map.evaluate(&mut ctx);
    assert!(
        outcome.is_ok(),
        "Map::evaluate should succeed even after source/body removed from IR: {:?}",
        outcome
    );
    match outcome.unwrap() {
        NodeOutcome::Succeeded {
            outputs,
            node_id: _,
        } => {
            let results = outputs
                .get("item_results")
                .expect("expected item_results key");
            let arr = results.as_array().expect("results should be array");
            assert_eq!(arr.len(), 2, "body should run 2 times");
        }
        other => panic!("Expected Succeeded, got {:?}", other),
    }
}

/// Scenario: Removing source/body from IR after Map construction does NOT break evaluate.
/// RED: fails — Map::evaluate still calls ctx.ir.operators.get().
/// GREEN: passes after refactor.
#[test]
fn map_evaluate_works_after_source_body_removed_from_ir() {
    let source_id = OperatorId("src".into());
    let body_id = OperatorId("bod".into());

    let source_items = serde_json::json!(["x"]);
    let mut source_outputs = BTreeMap::new();
    source_outputs.insert("items".into(), source_items);

    let executor: Arc<dyn TaskExecutor> = Arc::new(
        FakeExecutor::new()
            .source_returns(source_outputs)
            .body_returns(BTreeMap::new()),
    );

    let ir = Arc::new(make_ir_with_source_and_body(
        source_id.clone(),
        body_id.clone(),
    ));
    let node_run = make_node_run();
    let run = make_run();

    // Build Map via build_operator
    let map_domain_op = DomainOperator::Map {
        source: source_id.clone(),
        body: body_id.clone(),
        max_concurrency: 2,
    };
    let runtime_map = build_operator(&map_domain_op, &ir).expect("build_operator should succeed");

    // Remove source/body from IR
    let mut ir_mut = (*ir).clone();
    ir_mut.operators.remove(&source_id);
    ir_mut.operators.remove(&body_id);
    let ir_without_children = Arc::new(ir_mut);

    let mut ctx = make_ctx(
        Arc::clone(&node_run),
        ir_without_children,
        Arc::new(run),
        executor,
    );

    // Should NOT raise "map source not found" or "map body not found"
    let outcome = runtime_map.evaluate(&mut ctx);
    assert!(
        outcome.is_ok(),
        "Map::evaluate should not fail after source/body removed: {:?}",
        outcome
    );
}

// ── Sequence tests ────────────────────────────────────────────────────────────

/// Scenario: Sequence evaluate does NOT resolve children via ctx.ir.
/// RED: fails — Sequence may still use ctx.ir.operators.get.
/// GREEN: passes after Sequence is refactored to use stored Arc<dyn Operator> children.
#[test]
fn sequence_evaluate_does_not_resolve_children_after_construction() {
    let t1 = DomainOperator::Task {
        capability: CapabilityId("t1.cap".into()),
        inputs: Default::default(),
    };
    let t2 = DomainOperator::Task {
        capability: CapabilityId("t2.cap".into()),
        inputs: Default::default(),
    };

    let mut operators = BTreeMap::new();
    operators.insert(OperatorId("t1".into()), t1);
    operators.insert(OperatorId("t2".into()), t2);
    operators.insert(
        OperatorId("seq".into()),
        DomainOperator::Sequence {
            body: vec![OperatorId("t1".into()), OperatorId("t2".into())],
        },
    );

    let ir = Arc::new(WorkflowIR {
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
    });

    let executor: Arc<dyn TaskExecutor> = Arc::new(FakeExecutor::new());
    let node_run = make_node_run();
    let run = make_run();

    // Build Sequence via build_operator
    let seq_domain_op = ir.operators.get(&OperatorId("seq".into())).unwrap();
    let runtime_seq =
        build_operator(seq_domain_op, &ir).expect("build_operator should succeed for Sequence");

    // Remove children from IR
    let mut ir_mut = (*ir).clone();
    ir_mut.operators.remove(&OperatorId("t1".into()));
    ir_mut.operators.remove(&OperatorId("t2".into()));
    let ir_without_children = Arc::new(ir_mut);

    let mut ctx = make_ctx(
        Arc::clone(&node_run),
        ir_without_children,
        Arc::new(run),
        executor,
    );

    // Should NOT call ctx.ir.operators.get for t1/t2
    let outcome = runtime_seq.evaluate(&mut ctx);
    assert!(
        outcome.is_ok(),
        "Sequence::evaluate should succeed after children removed from IR: {:?}",
        outcome
    );
}

// ── Parallel tests ────────────────────────────────────────────────────────────

/// Scenario: Parallel evaluate does NOT resolve branches via ctx.ir.
/// RED: fails — Parallel may still use ctx.ir.operators.get.
/// GREEN: passes after Parallel is refactored.
#[test]
fn parallel_evaluate_does_not_resolve_branches_after_construction() {
    let p1 = DomainOperator::Task {
        capability: CapabilityId("p1.cap".into()),
        inputs: Default::default(),
    };
    let p2 = DomainOperator::Task {
        capability: CapabilityId("p2.cap".into()),
        inputs: Default::default(),
    };

    let mut operators = BTreeMap::new();
    operators.insert(OperatorId("p1".into()), p1);
    operators.insert(OperatorId("p2".into()), p2);
    operators.insert(
        OperatorId("par".into()),
        DomainOperator::Parallel {
            branches: vec![OperatorId("p1".into()), OperatorId("p2".into())],
            max_concurrency: 2,
        },
    );

    let ir = Arc::new(WorkflowIR {
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
    });

    let executor: Arc<dyn TaskExecutor> = Arc::new(FakeExecutor::new());
    let node_run = make_node_run();
    let run = make_run();

    // Build Parallel via build_operator
    let par_domain_op = ir.operators.get(&OperatorId("par".into())).unwrap();
    let runtime_par =
        build_operator(par_domain_op, &ir).expect("build_operator should succeed for Parallel");

    // Remove branches from IR
    let mut ir_mut = (*ir).clone();
    ir_mut.operators.remove(&OperatorId("p1".into()));
    ir_mut.operators.remove(&OperatorId("p2".into()));
    let ir_without_branches = Arc::new(ir_mut);

    let mut ctx = make_ctx(
        Arc::clone(&node_run),
        ir_without_branches,
        Arc::new(run),
        executor,
    );

    // Should NOT call ctx.ir.operators.get for p1/p2
    let outcome = runtime_par.evaluate(&mut ctx);
    assert!(
        outcome.is_ok(),
        "Parallel::evaluate should succeed after branches removed from IR: {:?}",
        outcome
    );
}
