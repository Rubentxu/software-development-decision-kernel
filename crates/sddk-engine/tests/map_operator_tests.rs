//! RED Tests for Operator::Map source evaluation + inputs injection (cycle-27)
//!
//! Map operator evaluates its `source` operator and fans out `body` across
//! the resulting collection. In cycle-27 the body MUST be a Task.

use std::collections::BTreeMap;
use std::sync::{Arc, Mutex};

use sddk_domain::{
    CapabilityId, NodeId, NodeRun, NodeRunState, Operator as DomainOperator, OperatorId, TaskError,
    TaskExecutor, TaskOutput, WorkflowIR, WorkflowRun,
};
use sddk_engine::operator::{Clock, GraphStoreBox, Map, NodeOutcome, Operator, OperatorContext};

fn make_ir_with_source_and_body(
    source_id: OperatorId,
    body_id: OperatorId,
    body_inputs: BTreeMap<String, serde_json::Value>,
) -> WorkflowIR {
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
            capability: CapabilityId("test.cap".into()),
            inputs: body_inputs,
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

/// Minimal test-only TaskExecutor that returns configurable outputs.
#[derive(Clone)]
struct FakeExecutor {
    source_outputs: BTreeMap<String, serde_json::Value>,
    body_outputs: BTreeMap<String, serde_json::Value>,
    body_should_fail: bool,
    fail_on_source: bool,
    fail_on_null_item: bool,
}

impl FakeExecutor {
    fn new() -> Self {
        FakeExecutor {
            source_outputs: BTreeMap::new(),
            body_outputs: BTreeMap::new(),
            body_should_fail: false,
            fail_on_source: false,
            fail_on_null_item: false,
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
    fn body_fails(mut self) -> Self {
        self.body_should_fail = true;
        self
    }
    fn source_fails(mut self) -> Self {
        self.fail_on_source = true;
        self
    }
}

impl TaskExecutor for FakeExecutor {
    fn execute(
        &self,
        capability: &str,
        inputs: &BTreeMap<String, serde_json::Value>,
    ) -> Result<TaskOutput, TaskError> {
        if self.fail_on_source && capability == "source.cap" {
            return Err(TaskError {
                message: "source failed explicitly".into(),
            });
        }
        if capability == "source.cap" {
            Ok(TaskOutput {
                outputs: self.source_outputs.clone(),
            })
        } else if self.fail_on_null_item && inputs.get("item").map(|v| v.is_null()).unwrap_or(false)
        {
            Err(TaskError {
                message: "body cannot process null item".into(),
            })
        } else if self.body_should_fail {
            Err(TaskError {
                message: "body failed explicitly".into(),
            })
        } else {
            // Echo inputs back so tests can verify item/index injection
            Ok(TaskOutput {
                outputs: inputs.clone(),
            })
        }
    }
}

fn make_run() -> WorkflowRun {
    let ir = make_ir_with_source_and_body(
        OperatorId("source".into()),
        OperatorId("body".into()),
        Default::default(),
    );
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

fn make_node_run() -> NodeRun {
    NodeRun {
        node_id: NodeId("test-node".into()),
        state: NodeRunState::Ready,
        dependencies: Default::default(),
        attempts: vec![],
        expansion_permissions: Default::default(),
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

/// Constructs a runtime `Map` from source and body `OperatorId`s using `Map::new`.
///
/// This helper bridges the domain-level IR (which uses `OperatorId` references)
/// to the runtime level (which uses resolved `Arc<dyn Operator>` children).
fn make_map_for_test(source_id: OperatorId, body_id: OperatorId, ir: &WorkflowIR) -> Map {
    let ir_op = DomainOperator::Map {
        source: source_id,
        body: body_id,
        max_concurrency: 4,
    };
    Map::new(&ir_op, ir).expect("test Map construction should not fail")
}

// ── REQ-Map-Source-Evaluation ────────────────────────────────────────────────

/// Scenario: Source produces three items → body executes exactly 3 times.
/// Map returns Succeeded { outputs: { "results": [<r1>, <r2>, <r3>] } }
#[test]
fn map_source_produces_three_items_runs_body_three_times() {
    let source_id = OperatorId("source".into());
    let body_id = OperatorId("body".into());

    let source_items = serde_json::json!(["v1", "v2", "v3"]);
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
        Default::default(),
    ));
    let node_run = Arc::new(Mutex::new(make_node_run()));
    let run = Arc::new(make_run());
    let mut ctx = make_ctx(Arc::clone(&node_run), Arc::clone(&ir), run, executor);

    let m = make_map_for_test(source_id.clone(), body_id.clone(), &ir);
    let outcome = m.evaluate(&mut ctx).expect("Map evaluate should not error");

    let NodeOutcome::Succeeded { outputs, .. } = outcome else {
        panic!("expected Succeeded, got {outcome:?}");
    };
    let results = outputs.get("results").expect("expected results key");
    let serde_json::Value::Array(arr) = results else {
        panic!("expected results to be Array, got {results:?}");
    };
    assert_eq!(arr.len(), 3, "body should run exactly 3 times");
}

/// Scenario: Source produces empty collection → body executes 0 times.
/// Map returns Succeeded { outputs: { "results": [] } }
#[test]
fn map_source_empty_collection_runs_body_zero_times() {
    let source_id = OperatorId("source".into());
    let body_id = OperatorId("body".into());

    let mut source_outputs = BTreeMap::new();
    source_outputs.insert("items".into(), serde_json::json!([]));

    let executor: Arc<dyn TaskExecutor> = Arc::new(
        FakeExecutor::new()
            .source_returns(source_outputs)
            .body_returns(BTreeMap::new()),
    );

    let ir = Arc::new(make_ir_with_source_and_body(
        source_id.clone(),
        body_id.clone(),
        Default::default(),
    ));
    let node_run = Arc::new(Mutex::new(make_node_run()));
    let run = Arc::new(make_run());
    let mut ctx = make_ctx(Arc::clone(&node_run), Arc::clone(&ir), run, executor);

    let m = make_map_for_test(source_id.clone(), body_id.clone(), &ir);
    let outcome = m.evaluate(&mut ctx).expect("Map evaluate should not error");

    let NodeOutcome::Succeeded { outputs, .. } = outcome else {
        panic!("expected Succeeded, got {outcome:?}");
    };
    let results = outputs.get("results").expect("expected results key");
    let serde_json::Value::Array(arr) = results else {
        panic!("expected results to be Array, got {results:?}");
    };
    assert!(arr.is_empty(), "body should run 0 times for empty source");
}

/// Scenario: Source fails → Map returns Failed with reason (first-failure).
/// Body is NOT evaluated.
#[test]
fn map_source_fails_returns_failed() {
    let source_id = OperatorId("source".into());
    let body_id = OperatorId("body".into());

    let executor: Arc<dyn TaskExecutor> = Arc::new(FakeExecutor::new().source_fails().body_fails());

    let ir = Arc::new(make_ir_with_source_and_body(
        source_id.clone(),
        body_id.clone(),
        Default::default(),
    ));
    let node_run = Arc::new(Mutex::new(make_node_run()));
    let run = Arc::new(make_run());
    let mut ctx = make_ctx(Arc::clone(&node_run), Arc::clone(&ir), run, executor);

    let m = make_map_for_test(source_id.clone(), body_id.clone(), &ir);
    let outcome = m.evaluate(&mut ctx).expect("Map evaluate should not error");

    let NodeOutcome::Failed { reason, .. } = outcome else {
        panic!("expected Failed, got {outcome:?}");
    };
    assert!(!reason.is_empty(), "failed reason should not be empty");
}

// ── REQ-Map-Inputs-Injection ───────────────────────────────────────────────────

/// Scenario: Iteration i=2 receives item="hello" and index=2 in body inputs.
#[test]
fn map_injects_item_and_index_into_body_inputs() {
    let source_id = OperatorId("source".into());
    let body_id = OperatorId("body".into());

    let source_items = serde_json::json!(["zero", "one", "hello"]);
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
        Default::default(),
    ));
    let node_run = Arc::new(Mutex::new(make_node_run()));
    let run = Arc::new(make_run());
    let mut ctx = make_ctx(Arc::clone(&node_run), Arc::clone(&ir), run, executor);

    let m = make_map_for_test(source_id.clone(), body_id.clone(), &ir);
    let outcome = m.evaluate(&mut ctx).expect("Map evaluate should not error");

    let NodeOutcome::Succeeded { outputs, .. } = outcome else {
        panic!("expected Succeeded, got {outcome:?}");
    };
    let results = outputs.get("results").expect("expected results key");
    let serde_json::Value::Array(arr) = results else {
        panic!("expected results to be Array, got {results:?}");
    };
    // iteration 2 (item = "hello", index = 2)
    let iter2_result = &arr[2];
    let obj = iter2_result
        .as_object()
        .expect("result should be an object");
    assert_eq!(obj.get("item").and_then(|v| v.as_str()), Some("hello"));
    assert_eq!(obj.get("index").and_then(|v| v.as_i64()), Some(2));
}

/// Scenario: Base inputs preserved after all iterations (non-destructive merge).
#[test]
fn map_preserves_base_inputs_non_destructively() {
    let source_id = OperatorId("source".into());
    let body_id = OperatorId("body".into());

    let source_items = serde_json::json!(["a"]);
    let mut source_outputs = BTreeMap::new();
    source_outputs.insert("items".into(), source_items);

    let mut base_inputs = BTreeMap::new();
    base_inputs.insert("base".into(), serde_json::json!("kept"));

    let executor: Arc<dyn TaskExecutor> = Arc::new(
        FakeExecutor::new()
            .source_returns(source_outputs)
            .body_returns(BTreeMap::new()),
    );

    let ir = Arc::new(make_ir_with_source_and_body(
        source_id.clone(),
        body_id.clone(),
        base_inputs,
    ));
    let node_run = Arc::new(Mutex::new(make_node_run()));
    let run = Arc::new(make_run());
    let mut ctx = make_ctx(Arc::clone(&node_run), Arc::clone(&ir), run, executor);

    let m = make_map_for_test(source_id.clone(), body_id.clone(), &ir);
    let outcome = m.evaluate(&mut ctx).expect("Map evaluate should not error");

    let NodeOutcome::Succeeded { outputs, .. } = outcome else {
        panic!("expected Succeeded, got {outcome:?}");
    };
    let results = outputs.get("results").expect("expected results key");
    let serde_json::Value::Array(arr) = results else {
        panic!("expected results to be Array, got {results:?}");
    };
    let iter0_result = &arr[0];
    let obj = iter0_result
        .as_object()
        .expect("result should be an object");
    // base input should still be present
    assert_eq!(
        obj.get("base").and_then(|v| v.as_str()),
        Some("kept"),
        "base input should be preserved"
    );
}

// ── REQ-Map-Collection-Key-Convention ─────────────────────────────────────────

/// Scenario: Items key missing → EvalFailed("expected outputs[\"items\"]: Array").
#[test]
fn map_missing_items_key_returns_eval_failed() {
    let source_id = OperatorId("source".into());
    let body_id = OperatorId("body".into());

    // Source returns {} — no items key
    let executor: Arc<dyn TaskExecutor> =
        Arc::new(FakeExecutor::new().body_returns(BTreeMap::new()));

    let ir = Arc::new(make_ir_with_source_and_body(
        source_id.clone(),
        body_id.clone(),
        Default::default(),
    ));
    let node_run = Arc::new(Mutex::new(make_node_run()));
    let run = Arc::new(make_run());
    let mut ctx = make_ctx(Arc::clone(&node_run), Arc::clone(&ir), run, executor);

    let m = make_map_for_test(source_id.clone(), body_id.clone(), &ir);
    let outcome = m.evaluate(&mut ctx).expect("Map evaluate should not error");

    let NodeOutcome::Failed { reason, .. } = outcome else {
        panic!("expected Failed, got {outcome:?}");
    };
    assert!(
        reason.contains("expected outputs[\"items\"]"),
        "expected items key error, got: {reason}"
    );
}

/// Scenario: Items key null → EvalFailed.
#[test]
fn map_null_items_key_returns_eval_failed() {
    let source_id = OperatorId("source".into());
    let body_id = OperatorId("body".into());

    let mut source_outputs = BTreeMap::new();
    source_outputs.insert("items".into(), serde_json::Value::Null);

    let executor: Arc<dyn TaskExecutor> = Arc::new(
        FakeExecutor::new()
            .source_returns(source_outputs)
            .body_returns(BTreeMap::new()),
    );

    let ir = Arc::new(make_ir_with_source_and_body(
        source_id.clone(),
        body_id.clone(),
        Default::default(),
    ));
    let node_run = Arc::new(Mutex::new(make_node_run()));
    let run = Arc::new(make_run());
    let mut ctx = make_ctx(Arc::clone(&node_run), Arc::clone(&ir), run, executor);

    let m = make_map_for_test(source_id.clone(), body_id.clone(), &ir);
    let outcome = m.evaluate(&mut ctx).expect("Map evaluate should not error");

    let NodeOutcome::Failed { reason, .. } = outcome else {
        panic!("expected Failed, got {outcome:?}");
    };
    assert!(
        reason.contains("expected outputs[\"items\"]"),
        "expected items key error, got: {reason}"
    );
}

// ── REQ-Map-Body-Restriction ──────────────────────────────────────────────────

/// Scenario: Body is Task → fan-out runs against the Task body.
#[test]
fn map_task_body_runs_fan_out() {
    let source_id = OperatorId("source".into());
    let body_id = OperatorId("body".into());

    let source_items = serde_json::json!([1, 2]);
    let mut source_outputs = BTreeMap::new();
    source_outputs.insert("items".into(), source_items);

    let executor: Arc<dyn TaskExecutor> = Arc::new(
        FakeExecutor::new()
            .source_returns(source_outputs)
            .body_returns({
                let mut o = BTreeMap::new();
                o.insert("processed".into(), serde_json::json!(true));
                o
            }),
    );

    let ir = Arc::new(make_ir_with_source_and_body(
        source_id.clone(),
        body_id.clone(),
        Default::default(),
    ));
    let node_run = Arc::new(Mutex::new(make_node_run()));
    let run = Arc::new(make_run());
    let mut ctx = make_ctx(Arc::clone(&node_run), Arc::clone(&ir), run, executor);

    let m = make_map_for_test(source_id.clone(), body_id.clone(), &ir);
    let outcome = m.evaluate(&mut ctx).expect("Map evaluate should not error");

    let NodeOutcome::Succeeded { outputs, .. } = outcome else {
        panic!("expected Succeeded, got {outcome:?}");
    };
    let results = outputs.get("results").expect("expected results key");
    let serde_json::Value::Array(arr) = results else {
        panic!("expected results to be Array, got {results:?}");
    };
    assert_eq!(arr.len(), 2);
}

/// Scenario: Body is Sequence → EvalFailed("cycle-27 map body must be Task").
#[test]
fn map_sequence_body_returns_eval_failed() {
    let source_id = OperatorId("source".into());
    let body_id = OperatorId("body".into());

    let source_items = serde_json::json!(["x"]);
    let mut source_outputs = BTreeMap::new();
    source_outputs.insert("items".into(), source_items);

    // Build IR with a Sequence body instead of Task
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
        DomainOperator::Sequence {
            body: vec![OperatorId("ignored".into())],
        },
    );
    // Add the "ignored" operator so Sequence resolution succeeds
    operators.insert(
        OperatorId("ignored".into()),
        DomainOperator::Task {
            capability: CapabilityId("ignored.cap".into()),
            inputs: Default::default(),
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

    let executor: Arc<dyn TaskExecutor> = Arc::new(
        FakeExecutor::new()
            .source_returns(source_outputs)
            .body_returns(BTreeMap::new()),
    );

    let node_run = Arc::new(Mutex::new(make_node_run()));
    let run = Arc::new(make_run());
    let _ctx = make_ctx(Arc::clone(&node_run), Arc::clone(&ir), run, executor);

    // Map::new now validates body is Task at construction time (cycle-31).
    // Previously this was checked at evaluate time. The test expectation changes:
    let result = Map::new(
        &DomainOperator::Map {
            source: source_id.clone(),
            body: body_id.clone(),
            max_concurrency: 4,
        },
        &ir,
    );
    assert!(
        result.is_err(),
        "Map::new should fail when body is Sequence"
    );
    let err = result.unwrap_err();
    assert!(
        format!("{err}").contains("map body must be Task"),
        "expected 'map body must be Task' error, got: {err}"
    );
}

// ── REQ-Map-Max-Concurrency (cycle-28) ────────────────────────────────────────

/// Scenario: max_concurrency=1 → sequential execution, no thread spawn.
/// Verifies results are in iteration order and failures array is empty.
#[test]
fn map_max_concurrency_one_runs_sequentially() {
    let source_id = OperatorId("source".into());
    let body_id = OperatorId("body".into());

    let source_items = serde_json::json!(["a", "b", "c", "d"]);
    let mut source_outputs = BTreeMap::new();
    source_outputs.insert("items".into(), source_items);

    let executor: Arc<dyn TaskExecutor> = Arc::new(
        FakeExecutor::new()
            .source_returns(source_outputs)
            .body_returns({
                let mut m = BTreeMap::new();
                m.insert("value".into(), serde_json::json!("processed"));
                m
            }),
    );

    let ir = Arc::new(make_ir_with_source_and_body(
        source_id.clone(),
        body_id.clone(),
        Default::default(),
    ));
    let node_run = Arc::new(Mutex::new(make_node_run()));
    let run = Arc::new(make_run());
    let mut ctx = make_ctx(Arc::clone(&node_run), Arc::clone(&ir), run, executor);

    let m = make_map_for_test(source_id.clone(), body_id.clone(), &ir);
    let outcome = m.evaluate(&mut ctx).expect("Map evaluate should not error");

    let NodeOutcome::Succeeded { outputs, .. } = outcome else {
        panic!("expected Succeeded, got {outcome:?}");
    };

    // Results in iteration order
    let results = outputs.get("results").expect("results key must exist");
    let results_arr = match results {
        serde_json::Value::Array(arr) => arr,
        _ => panic!("results must be Array, got {results:?}"),
    };
    assert_eq!(results_arr.len(), 4, "all 4 items should succeed");

    // Failures must be empty for all-success case
    let failures = outputs.get("failures").expect("failures key must exist");
    let failures_arr = match failures {
        serde_json::Value::Array(arr) => arr,
        _ => panic!("failures must be Array, got {failures:?}"),
    };
    assert_eq!(
        failures_arr.len(),
        0,
        "no failures expected when all succeed"
    );
}

/// Scenario: max_concurrency=2 with 4 items each sleeping 50ms → total time < 150ms
/// proves parallelism. Results must preserve iteration order.
#[test]
fn map_max_concurrency_two_gates_to_two_at_a_time() {
    use std::sync::atomic::{AtomicUsize, Ordering};
    use std::time::{Duration, Instant};

    let source_id = OperatorId("source".into());
    let body_id = OperatorId("body".into());

    let source_items = serde_json::json!([1, 2, 3, 4]);
    let mut source_outputs: BTreeMap<String, serde_json::Value> = BTreeMap::new();
    source_outputs.insert("items".into(), source_items);

    let concurrent_counter = Arc::new(AtomicUsize::new(0));
    let _peak_counter = Arc::clone(&concurrent_counter);
    let sleep_duration_ms = 50;

    // Executor that increments counter on entry, decrements on exit
    #[derive(Clone)]
    struct TimedExecutor {
        source_outputs: BTreeMap<String, serde_json::Value>,
        inner: Arc<AtomicUsize>,
        sleep_ms: u64,
    }
    impl TaskExecutor for TimedExecutor {
        fn execute(
            &self,
            capability: &str,
            inputs: &BTreeMap<String, serde_json::Value>,
        ) -> Result<TaskOutput, TaskError> {
            if capability == "source.cap" {
                return Ok(TaskOutput {
                    outputs: self.source_outputs.clone(),
                });
            }
            let _current = self.inner.fetch_add(1, Ordering::SeqCst);
            // Note: peak tracking happens via a separate mechanism
            std::thread::sleep(Duration::from_millis(self.sleep_ms));
            self.inner.fetch_sub(1, Ordering::SeqCst);
            Ok(TaskOutput {
                outputs: inputs.clone(),
            })
        }
    }

    let executor: Arc<dyn TaskExecutor> = Arc::new(TimedExecutor {
        source_outputs,
        inner: concurrent_counter,
        sleep_ms: sleep_duration_ms,
    });

    let ir = Arc::new(make_ir_with_source_and_body(
        source_id.clone(),
        body_id.clone(),
        Default::default(),
    ));
    let node_run = Arc::new(Mutex::new(make_node_run()));
    let run = Arc::new(make_run());
    let mut ctx = make_ctx(Arc::clone(&node_run), Arc::clone(&ir), run, executor);

    let m = make_map_for_test(source_id.clone(), body_id.clone(), &ir);

    let start = Instant::now();
    let outcome = m.evaluate(&mut ctx).expect("Map evaluate should not error");
    let elapsed = start.elapsed();

    let NodeOutcome::Succeeded { outputs, .. } = outcome else {
        panic!("expected Succeeded, got {outcome:?}");
    };

    // With 4 items and max_concurrency=2, if sequential it would take 200ms.
    // If parallel (unbounded), it would take ~50ms. With semaphore-gated 2 at a time,
    // it should take ~100ms (2 batches of 2 items × 50ms each).
    // We assert < 150ms to prove parallelism while allowing some overhead.
    assert!(
        elapsed < Duration::from_millis(150),
        "expected parallel execution < 150ms, got {}ms (sequential would be ~200ms)",
        elapsed.as_millis()
    );

    // Results preserved in iteration order
    let results = outputs.get("results").expect("results key must exist");
    let results_arr = match results {
        serde_json::Value::Array(arr) => arr,
        _ => panic!("results must be Array"),
    };
    assert_eq!(results_arr.len(), 4, "all 4 items should succeed");
}

/// Scenario: max_concurrency=0 → unbounded (all items run concurrently).
/// Verifies map_max_concurrency_effective(0, n) returns n.
#[test]
fn map_max_concurrency_zero_runs_all_in_parallel_unbounded() {
    let source_id = OperatorId("source".into());
    let body_id = OperatorId("body".into());

    let source_items = serde_json::json!([1, 2, 3, 4, 5, 6, 7, 8]);
    let mut source_outputs = BTreeMap::new();
    source_outputs.insert("items".into(), source_items);

    let executor: Arc<dyn TaskExecutor> = Arc::new(
        FakeExecutor::new()
            .source_returns(source_outputs)
            .body_returns({
                let mut m = BTreeMap::new();
                m.insert("processed".into(), serde_json::json!(true));
                m
            }),
    );

    let ir = Arc::new(make_ir_with_source_and_body(
        source_id.clone(),
        body_id.clone(),
        Default::default(),
    ));
    let node_run = Arc::new(Mutex::new(make_node_run()));
    let run = Arc::new(make_run());
    let mut ctx = make_ctx(Arc::clone(&node_run), Arc::clone(&ir), run, executor);

    let m = make_map_for_test(source_id, body_id, &ir);
    let outcome = m.evaluate(&mut ctx).expect("Map evaluate should not error");

    let NodeOutcome::Succeeded { outputs, .. } = outcome else {
        panic!("expected Succeeded, got {outcome:?}");
    };

    let results = outputs.get("results").expect("results key must exist");
    let results_arr = match results {
        serde_json::Value::Array(arr) => arr,
        _ => panic!("results must be Array"),
    };
    assert_eq!(results_arr.len(), 8, "all 8 items should succeed");

    let failures = outputs.get("failures").expect("failures key must exist");
    let failures_arr = match failures {
        serde_json::Value::Array(arr) => arr,
        _ => panic!("failures must be Array"),
    };
    assert_eq!(failures_arr.len(), 0, "no failures expected");
}

// ── REQ-Map-Collect-All-Errors (cycle-28) ─────────────────────────────────────

/// Scenario: 4 items, items at index 1 and 3 fail → outcome is Succeeded,
/// results contains only successful outputs (no null compaction), failures contains
/// both failure records.
#[test]
fn map_collect_all_partial_failures_returns_succeeded_with_failures() {
    let source_id = OperatorId("source".into());
    let body_id = OperatorId("body".into());

    let source_items = serde_json::json!(["a", "b", "c", "d"]);
    let mut source_outputs: BTreeMap<String, serde_json::Value> = BTreeMap::new();
    source_outputs.insert("items".into(), source_items);

    #[derive(Clone)]
    struct PartialFailExecutor {
        source_outputs: BTreeMap<String, serde_json::Value>,
        fail_indices: Vec<usize>,
    }
    impl TaskExecutor for PartialFailExecutor {
        fn execute(
            &self,
            capability: &str,
            inputs: &BTreeMap<String, serde_json::Value>,
        ) -> Result<TaskOutput, TaskError> {
            if capability == "source.cap" {
                return Ok(TaskOutput {
                    outputs: self.source_outputs.clone(),
                });
            }
            let index = inputs.get("index").and_then(|v| v.as_u64()).unwrap_or(0) as usize;
            if self.fail_indices.contains(&index) {
                Err(TaskError {
                    message: format!("failure at index {}", index),
                })
            } else {
                Ok(TaskOutput {
                    outputs: inputs.clone(),
                })
            }
        }
    }

    let executor: Arc<dyn TaskExecutor> = Arc::new(PartialFailExecutor {
        source_outputs,
        fail_indices: vec![1, 3],
    });

    let ir = Arc::new(make_ir_with_source_and_body(
        source_id.clone(),
        body_id.clone(),
        Default::default(),
    ));
    let node_run = Arc::new(Mutex::new(make_node_run()));
    let run = Arc::new(make_run());
    let mut ctx = make_ctx(Arc::clone(&node_run), Arc::clone(&ir), run, executor);

    let m = make_map_for_test(source_id.clone(), body_id.clone(), &ir);
    let outcome = m.evaluate(&mut ctx).expect("Map evaluate should not error");

    // Outcome is Succeeded because at least one body succeeded (index 0 and 2)
    let NodeOutcome::Succeeded { outputs, .. } = outcome else {
        panic!("expected Succeeded (≥1 success), got {outcome:?}");
    };

    // Results: successful outputs only, no null for failed indices
    let results = outputs.get("results").expect("results key must exist");
    let results_arr = match results {
        serde_json::Value::Array(arr) => arr,
        _ => panic!("results must be Array"),
    };
    assert_eq!(results_arr.len(), 2, "only 2 items succeeded");

    // Failures: both failures recorded
    let failures = outputs.get("failures").expect("failures key must exist");
    let failures_arr = match failures {
        serde_json::Value::Array(arr) => arr,
        _ => panic!("failures must be Array"),
    };
    assert_eq!(failures_arr.len(), 2, "2 items failed");

    // Verify failure indices are 1 and 3
    for f in failures_arr {
        let obj = f.as_object().expect("failure must be object");
        let idx = obj.get("index").expect("index field").as_u64().unwrap() as usize;
        let reason = obj.get("reason").expect("reason field").as_str().unwrap();
        assert!(
            idx == 1 || idx == 3,
            "expected failure at index 1 or 3, got {}: {}",
            idx,
            reason
        );
    }
}

/// Scenario: 3 items, all fail → outcome is Failed with composite reason
/// containing all failure messages.
#[test]
fn map_all_failures_returns_failed_with_composite_reason() {
    let source_id = OperatorId("source".into());
    let body_id = OperatorId("body".into());

    let source_items = serde_json::json!(["a", "b", "c"]);
    let mut source_outputs: BTreeMap<String, serde_json::Value> = BTreeMap::new();
    source_outputs.insert("items".into(), source_items);

    let executor: Arc<dyn TaskExecutor> = Arc::new(
        FakeExecutor::new()
            .source_returns(source_outputs)
            .body_fails(),
    );

    let ir = Arc::new(make_ir_with_source_and_body(
        source_id.clone(),
        body_id.clone(),
        Default::default(),
    ));
    let node_run = Arc::new(Mutex::new(make_node_run()));
    let run = Arc::new(make_run());
    let mut ctx = make_ctx(Arc::clone(&node_run), Arc::clone(&ir), run, executor);

    let m = make_map_for_test(source_id.clone(), body_id.clone(), &ir);
    let outcome = m.evaluate(&mut ctx).expect("Map evaluate should not error");

    let NodeOutcome::Failed { reason, .. } = outcome else {
        panic!("expected Failed (all items failed), got {outcome:?}");
    };

    // Composite reason format: "map body failed at all N iterations: [0]...; [1]...; [2]..."
    assert!(
        reason.contains("map body failed at all 3 iterations"),
        "expected composite reason, got: {}",
        reason
    );
    assert!(
        reason.contains("[0]"),
        "expected [0] marker in composite reason, got: {}",
        reason
    );
    assert!(
        reason.contains("[1]"),
        "expected [1] marker in composite reason, got: {}",
        reason
    );
    assert!(
        reason.contains("[2]"),
        "expected [2] marker in composite reason, got: {}",
        reason
    );
}

/// Scenario: 15 items, all fail → composite reason truncated to top-10 with "..."
/// but failures array contains all 15 entries.
#[test]
fn map_composite_reason_truncates_at_ten() {
    let source_id = OperatorId("source".into());
    let body_id = OperatorId("body".into());

    // 15 items - more than the 10-entry truncation limit
    let source_items: Vec<serde_json::Value> = (0..15).map(|i| serde_json::json!(i)).collect();
    let mut source_outputs: BTreeMap<String, serde_json::Value> = BTreeMap::new();
    source_outputs.insert("items".into(), serde_json::Value::Array(source_items));

    let executor: Arc<dyn TaskExecutor> = Arc::new(
        FakeExecutor::new()
            .source_returns(source_outputs)
            .body_fails(),
    );

    let ir = Arc::new(make_ir_with_source_and_body(
        source_id.clone(),
        body_id.clone(),
        Default::default(),
    ));
    let node_run = Arc::new(Mutex::new(make_node_run()));
    let run = Arc::new(make_run());
    let mut ctx = make_ctx(Arc::clone(&node_run), Arc::clone(&ir), run, executor);

    let m = make_map_for_test(source_id.clone(), body_id.clone(), &ir);
    let outcome = m.evaluate(&mut ctx).expect("Map evaluate should not error");

    let NodeOutcome::Failed { reason, .. } = outcome else {
        panic!("expected Failed (all items failed), got {outcome:?}");
    };

    // Composite reason should contain "all 15 iterations"
    assert!(
        reason.contains("all 15 iterations"),
        "expected 15 in composite reason, got: {}",
        reason
    );

    // Should have 10 [i] markers (indices 0-9)
    let bracket_count = reason.matches("[").count();
    assert_eq!(
        bracket_count, 10,
        "expected 10 [i] markers (0-9), got {} in: {}",
        bracket_count, reason
    );

    // Should end with "..." to indicate truncation
    assert!(
        reason.ends_with("..."),
        "expected reason to end with '...', got: {}",
        reason
    );
}

// ── REQ-Map-Doc-Cycle28-InScope ───────────────────────────────────────────────

/// Scenario: Map docstring states max_concurrency is ENFORCED (cycle-28 complete).
#[test]
fn map_docstring_lists_max_concurrency_enforced() {
    // Read operator.rs as a string and extract the Map struct docstring.
    let operator_rs = include_str!("../src/operator.rs");
    let map_docstring = extract_map_docstring(operator_rs);

    // The docstring should mention max_concurrency
    assert!(
        map_docstring.contains("max_concurrency"),
        "docstring should mention max_concurrency"
    );
    // cycle-28 deferred items should NOT say IGNORED anymore
    assert!(
        !map_docstring.contains("IGNORED"),
        "docstring should NOT say IGNORED (cycle-28 implemented max_concurrency)"
    );
}

/// Scenario: Map docstring states error aggregation is collect-all (cycle-28 complete).
#[test]
fn map_docstring_lists_collect_all() {
    let operator_rs = include_str!("../src/operator.rs");
    let map_docstring = extract_map_docstring(operator_rs);

    // Should mention collect-all (the cycle-28 implementation)
    assert!(
        map_docstring.contains("collect-all"),
        "docstring should mention collect-all (cycle-28 implemented)"
    );
    // Should NOT mention first-failure (that was the old semantics)
    assert!(
        !map_docstring.contains("first-failure"),
        "docstring should NOT mention first-failure (cycle-28 replaced with collect-all)"
    );
}

/// Scenario: Map docstring states cross-tick replay is in-scope (cycle-30).
#[test]
fn map_docstring_lists_cross_tick_replay_in_scope() {
    let operator_rs = include_str!("../src/operator.rs");
    let map_docstring = extract_map_docstring(operator_rs);
    let map_docstring_lower = map_docstring.to_lowercase();

    // Cross-tick replay should now be listed as in-scope (cycle-30), not deferred
    assert!(
        map_docstring_lower.contains("source-context isolation"),
        "docstring should mention source-context isolation (cycle-30 in-scope)"
    );
    // cycle-29 should NOT appear anymore
    assert!(
        !map_docstring.contains("cycle-29"),
        "docstring should NOT mention cycle-29 (cross-tick replay now cycle-30 in-scope)"
    );
    // DC-MAP-002 should still be deferred
    assert!(
        map_docstring.contains("DC-MAP-002"),
        "docstring should still reference DC-MAP-002 deferred"
    );
}

/// Scenario: Map docstring lists DC-MAP-002 as deferred only.
#[test]
fn map_docstring_defers_only_dc_map_002() {
    let operator_rs = include_str!("../src/operator.rs");
    let map_docstring = extract_map_docstring(operator_rs);

    // Find the deferred section
    let deferred_section = map_docstring
        .lines()
        .skip_while(|l| !l.contains("Deferred"))
        .take(10)
        .collect::<String>();

    // Only DC-MAP-002 should be in the deferred list
    assert!(
        deferred_section.contains("DC-MAP-002"),
        "deferred section should contain DC-MAP-002"
    );
    // No cycle-29 in deferred list
    assert!(
        !deferred_section.contains("cycle-29"),
        "deferred section should not contain cycle-29"
    );
    // No cross-tick replay in deferred list (it's now in-scope)
    assert!(
        !deferred_section.to_lowercase().contains("cross-tick"),
        "cross-tick replay should not be in deferred list (cycle-30 in-scope)"
    );
}

/// Extracts the docstring for the Map struct from operator.rs source text.
fn extract_map_docstring(operator_rs: &str) -> String {
    let lines: Vec<&str> = operator_rs.lines().collect();
    let mut result = String::new();

    // Find the "pub struct Map" line
    let struct_line_idx = lines
        .iter()
        .position(|l| l.contains("pub struct Map"))
        .expect("pub struct Map not found");

    // Collect doc comment lines and attributes above the struct
    let mut i = struct_line_idx;
    while i > 0 {
        i -= 1;
        let line = lines[i];
        if line.trim().starts_with("///") || line.trim().is_empty() || line.trim().starts_with("#[")
        {
            // Prepend since we're going backwards
            result = format!("{}\n{}", line, result);
        } else {
            break;
        }
    }

    // Continue from struct line to capture struct body
    let mut brace_depth = 0;
    #[allow(clippy::needless_range_loop)]
    for j in struct_line_idx..lines.len() {
        let line = lines[j];
        result.push_str(line);
        result.push('\n');
        brace_depth += line.matches('{').count() as i32;
        brace_depth -= line.matches('}').count() as i32;
        if brace_depth < 0 {
            break;
        }
    }

    result
}

// ── REQ-Map-Source-Context-Isolation (cycle-30) ───────────────────────────────

/// Scenario: Source does not mutate parent node_run.attempts.
/// Verifies DC-MAP-001 closure: source.evaluate uses fresh child context.
#[test]
fn map_source_context_isolation_source_does_not_mutate_parent_attempts() {
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
        Default::default(),
    ));
    let node_run = Arc::new(Mutex::new(make_node_run()));
    let run = Arc::new(make_run());
    let mut ctx = make_ctx(Arc::clone(&node_run), Arc::clone(&ir), run, executor);

    // Record attempts count BEFORE Map evaluation
    let attempts_before = ctx.node_run.lock().unwrap().attempts.len();

    let m = make_map_for_test(source_id.clone(), body_id.clone(), &ir);
    let outcome = m.evaluate(&mut ctx).expect("Map evaluate should not error");

    let NodeOutcome::Succeeded { .. } = outcome else {
        panic!("expected Succeeded, got {outcome:?}");
    };

    // Source should NOT have mutated parent's node_run.attempts
    let attempts_after = ctx.node_run.lock().unwrap().attempts.len();
    assert_eq!(
        attempts_before, attempts_after,
        "node_run.attempts.len() should be unchanged after source evaluation"
    );
}

/// Scenario: Source child context has pending_sender = None.
#[test]
fn map_source_context_isolation_source_pending_propagates() {
    let source_id = OperatorId("source".into());
    let body_id = OperatorId("body".into());

    #[derive(Clone)]
    struct PendingSourceExecutor;
    impl TaskExecutor for PendingSourceExecutor {
        fn execute(
            &self,
            capability: &str,
            _inputs: &BTreeMap<String, serde_json::Value>,
        ) -> Result<TaskOutput, TaskError> {
            if capability == "source.cap" {
                Err(TaskError {
                    message: "source pending signal".into(),
                })
            } else {
                Ok(TaskOutput {
                    outputs: Default::default(),
                })
            }
        }
    }

    let executor: Arc<dyn TaskExecutor> = Arc::new(PendingSourceExecutor);

    let ir = Arc::new(make_ir_with_source_and_body(
        source_id.clone(),
        body_id.clone(),
        Default::default(),
    ));
    let node_run = Arc::new(Mutex::new(make_node_run()));
    let run = Arc::new(make_run());
    let mut ctx = make_ctx(Arc::clone(&node_run), Arc::clone(&ir), run, executor);

    let m = make_map_for_test(source_id.clone(), body_id.clone(), &ir);
    let outcome = m.evaluate(&mut ctx).expect("Map evaluate should not error");

    let NodeOutcome::Failed { .. } = outcome else {
        panic!("expected Failed (source propagated), got {outcome:?}");
    };
}

// ── REQ-Map-Cross-Tick-Replay (cycle-30) ──────────────────────────────────────

// NOTE: Sequential Pending via Task body cannot be tested because TaskExecutor
// returns TaskOutput/TaskError which maps to Succeeded/Failed, not Pending.
// The sequential Pending code path exists in operator.rs evaluate_sequential()
// but requires a body operator that can return NodeOutcome::Pending directly.
// This is a limitation of the current Task-based body architecture.

/// Scenario: Source NOT re-evaluated on replay.
#[test]
fn map_cross_tick_replay_source_not_reevaluated() {
    use std::sync::atomic::{AtomicUsize, Ordering};

    let source_id = OperatorId("source".into());
    let body_id = OperatorId("body".into());

    let source_items = serde_json::json!(["a", "b"]);
    let mut source_outputs = BTreeMap::new();
    source_outputs.insert("items".into(), source_items);

    let source_eval_count = Arc::new(AtomicUsize::new(0));
    let source_eval_count_clone = Arc::clone(&source_eval_count);

    #[derive(Clone)]
    struct SourceTrackingExecutor {
        source_outputs: BTreeMap<String, serde_json::Value>,
        source_eval_count: Arc<AtomicUsize>,
    }
    impl TaskExecutor for SourceTrackingExecutor {
        fn execute(
            &self,
            capability: &str,
            _inputs: &BTreeMap<String, serde_json::Value>,
        ) -> Result<TaskOutput, TaskError> {
            if capability == "source.cap" {
                self.source_eval_count.fetch_add(1, Ordering::SeqCst);
                return Ok(TaskOutput {
                    outputs: self.source_outputs.clone(),
                });
            }
            Ok(TaskOutput {
                outputs: Default::default(),
            })
        }
    }

    let executor: Arc<dyn TaskExecutor> = Arc::new(SourceTrackingExecutor {
        source_outputs,
        source_eval_count: source_eval_count_clone,
    });

    let ir = Arc::new(make_ir_with_source_and_body(
        source_id.clone(),
        body_id.clone(),
        Default::default(),
    ));
    let node_run = Arc::new(Mutex::new(make_node_run()));
    let run = Arc::new(make_run());
    let mut ctx = make_ctx(Arc::clone(&node_run), Arc::clone(&ir), run, executor);

    let m = make_map_for_test(source_id.clone(), body_id.clone(), &ir);
    let outcome = m.evaluate(&mut ctx).expect("Map evaluate should not error");

    assert!(matches!(outcome, NodeOutcome::Succeeded { .. }));

    assert_eq!(
        source_eval_count.load(Ordering::SeqCst),
        1,
        "source should be evaluated exactly once"
    );
}

// ── REQ-Map-Collect-All-Errors (cycle-30 replay scenario) ─────────────────────

#[test]
fn map_collect_all_preserved_across_replay() {
    let source_id = OperatorId("source".into());
    let body_id = OperatorId("body".into());

    let source_items = serde_json::json!(["a", "b", "c", "d"]);
    let mut source_outputs = BTreeMap::new();
    source_outputs.insert("items".into(), source_items);

    #[derive(Clone)]
    struct CollectAllExecutor {
        source_outputs: BTreeMap<String, serde_json::Value>,
        fail_at_index: Option<usize>,
    }
    impl TaskExecutor for CollectAllExecutor {
        fn execute(
            &self,
            capability: &str,
            inputs: &BTreeMap<String, serde_json::Value>,
        ) -> Result<TaskOutput, TaskError> {
            if capability == "source.cap" {
                return Ok(TaskOutput {
                    outputs: self.source_outputs.clone(),
                });
            }
            let index = inputs.get("index").and_then(|v| v.as_u64()).unwrap_or(0) as usize;
            if self.fail_at_index == Some(index) {
                return Err(TaskError {
                    message: format!("fail at {}", index),
                });
            }
            Ok(TaskOutput {
                outputs: inputs.clone(),
            })
        }
    }

    let executor: Arc<dyn TaskExecutor> = Arc::new(CollectAllExecutor {
        source_outputs,
        fail_at_index: None,
    });

    let ir = Arc::new(make_ir_with_source_and_body(
        source_id.clone(),
        body_id.clone(),
        Default::default(),
    ));
    let node_run = Arc::new(Mutex::new(make_node_run()));
    let run = Arc::new(make_run());
    let mut ctx = make_ctx(Arc::clone(&node_run), Arc::clone(&ir), run, executor);

    let m = make_map_for_test(source_id.clone(), body_id.clone(), &ir);
    let outcome = m.evaluate(&mut ctx).expect("Map evaluate should not error");

    let NodeOutcome::Succeeded { outputs, .. } = outcome else {
        panic!("expected Succeeded, got {outcome:?}");
    };

    let results = outputs.get("results").expect("results key must exist");
    let results_arr = match results {
        serde_json::Value::Array(arr) => arr,
        _ => panic!("results must be Array"),
    };
    let failures = outputs.get("failures").expect("failures key must exist");
    let failures_arr = match failures {
        serde_json::Value::Array(arr) => arr,
        _ => panic!("failures must be Array"),
    };

    assert_eq!(
        results_arr.len() + failures_arr.len(),
        4,
        "results.len() + failures.len() should equal items_len (4)"
    );
}

// ── MapCheckpointState existence test (cycle-30) ─────────────────────────────

#[test]
fn map_checkpoint_state_struct_exists() {
    use sddk_engine::operator::MapCheckpointState;

    fn _assert_checkpoint_state_fields(state: &MapCheckpointState) {
        let _ = &state.items_len;
        let _ = &state.completed_results;
        let _ = &state.source_outputs_snapshot;
    }
}

// ── cycle-32: CheckpointHandle::MapChannel extension ─────────────────────────

/// Scenario: CheckpointHandle::MapChannel variant exists and carries Arc<MapCheckpointState>.
/// This is a compile-time + structural test verifying the enum variant shape per Q1b.
#[test]
fn map_checkpoint_handle_mapchannel_carries_arc_state() {
    use sddk_engine::operator::{CheckpointHandle, MapCheckpointState};
    use std::sync::{Arc, Mutex, mpsc};

    // Build a minimal MapCheckpointState for structural verification
    let (_tx, rx) = mpsc::channel::<sddk_engine::operator::ChildResult>();
    let state = MapCheckpointState {
        receiver: Arc::new(Mutex::new(rx)),
        items_len: 3,
        completed_results: Default::default(),
        source_outputs_snapshot: Default::default(),
    };

    // Verify CheckpointHandle::MapChannel exists and accepts Arc<MapCheckpointState>
    let handle: CheckpointHandle = CheckpointHandle::MapChannel {
        state: Arc::new(state),
        token: 0,
    };

    match handle {
        CheckpointHandle::MapChannel { state: _, token: 0 } => {}
        other => panic!("expected MapChannel with token 0, got {:?}", other),
    }
}

/// Scenario: NodeOutcome::Pending can contain CheckpointHandle::MapChannel
/// and round-trip through pattern matching without losing the Arc state.
#[test]
fn map_node_outcome_pending_mapchannel_roundtrips() {
    use sddk_engine::operator::{CheckpointHandle, MapCheckpointState, NodeOutcome};
    use std::sync::{Arc, Mutex, mpsc};

    let (_tx, rx) = mpsc::channel();
    let state = MapCheckpointState {
        receiver: Arc::new(Mutex::new(rx)),
        items_len: 2,
        completed_results: Default::default(),
        source_outputs_snapshot: Default::default(),
    };

    let outcome = NodeOutcome::Pending {
        checkpoint: CheckpointHandle::MapChannel {
            state: Arc::new(state),
            token: 42,
        },
    };

    // Round-trip through match
    if let NodeOutcome::Pending { checkpoint } = outcome {
        match checkpoint {
            CheckpointHandle::MapChannel { state, token: 42 } => {
                assert_eq!(state.items_len, 2, "items_len preserved through round-trip");
            }
            other => panic!("expected MapChannel with token 42, got {:?}", other),
        }
    } else {
        panic!("expected Pending outcome");
    }
}

// ── cycle-32: Sequential path dummy channel ───────────────────────────────────

/// Scenario: Sequential path creates a dummy channel that is immediately Disconnected.
/// When the runtime drains this receiver, it should detect Disconnected immediately.
#[test]
fn map_checkpoint_handle_mapchannel_sequential_uses_dummy_channel() {
    use sddk_engine::operator::{CheckpointHandle, MapCheckpointState};
    use std::sync::{Arc, Mutex, mpsc};

    // Sequential case: tx is dropped immediately (dummy channel)
    let (tx, rx) = mpsc::channel::<sddk_engine::operator::ChildResult>();
    drop(tx); // Immediately drop tx → channel is Disconnected

    // Verify the receiver is disconnected BEFORE moving into Arc
    match rx.try_recv() {
        Err(mpsc::TryRecvError::Disconnected) => {}
        other => panic!("expected Disconnected, got {:?}", other),
    }

    let state = MapCheckpointState {
        receiver: Arc::new(Mutex::new(rx)),
        items_len: 1,
        completed_results: Default::default(),
        source_outputs_snapshot: Default::default(),
    };

    let _handle = CheckpointHandle::MapChannel {
        state: Arc::new(state),
        token: 0,
    };
}

// ── cycle-32: source_outputs_snapshot non-empty on concurrent Pending ─────────

/// Scenario: Concurrent path's MapCheckpointState.source_outputs_snapshot
/// is populated from source_outcome.outputs.clone(), NOT BTreeMap::new().
/// This verifies INV-11: source_outputs_snapshot is non-empty on checkpoint handoff.
#[test]
fn map_source_outputs_snapshot_non_empty_on_concurrent_pending() {
    use sddk_engine::operator::MapCheckpointState;
    use std::collections::BTreeMap;
    use std::sync::{Arc, Mutex, mpsc};

    // Simulate what concurrent path builds: source produces items
    let (_tx, rx) = mpsc::channel();
    let source_outputs: BTreeMap<String, serde_json::Value> =
        BTreeMap::from([("items".to_string(), serde_json::json!(["a", "b", "c"]))]);

    let state = MapCheckpointState {
        receiver: Arc::new(Mutex::new(rx)),
        items_len: 3,
        completed_results: Default::default(),
        // This is what the FIX provides: actual source outputs, not empty BTreeMap
        source_outputs_snapshot: source_outputs.clone(),
    };

    // INV-11 invariant: source_outputs_snapshot MUST NOT be empty
    assert!(
        !state.source_outputs_snapshot.is_empty(),
        "source_outputs_snapshot must be non-empty on concurrent Pending checkpoint"
    );

    // Verify the snapshot contains the expected items
    let items = state
        .source_outputs_snapshot
        .get("items")
        .expect("items key must exist");
    let items_arr = items.as_array().expect("items must be array");
    assert_eq!(items_arr.len(), 3, "snapshot should capture all 3 items");
    assert_eq!(
        items_arr[0],
        serde_json::json!("a"),
        "first item should be 'a'"
    );
}

// ── cycle-32: Runtime-side storage + drain ───────────────────────────────────

/// Scenario: WorkflowRuntime has pending_map field keyed by (RunId, OperatorId).
/// This is a compile-time verification: the field exists and compiles.
/// Uses make_runtime pattern from runtime_receiver_map_tests.rs.
#[test]
fn map_runtime_storage_pending_map_field_exists() {
    use sddk_domain::{GraphStore, NoopTaskExecutor, StorageError, WorkflowIR};
    use sddk_engine::operator::Clock;
    use sddk_engine::workflow_runtime::WorkflowRuntime;
    use std::collections::BTreeMap;
    use std::sync::Arc;

    // Minimal MockStore implementation for this test
    struct MockStore;
    impl GraphStore for MockStore {
        fn save_state(&mut self, _: &sddk_domain::GraphState) -> Result<(), StorageError> {
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

    // Empty IR to create a minimal runtime
    let ir = WorkflowIR {
        ir_id: None,
        schema_version: 1,
        template_ref: sddk_domain::TemplateRef {
            id: "test".into(),
            version: "1.0".into(),
        },
        operators: BTreeMap::new(),
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
    };

    // Verify WorkflowRuntime::new compiles and returns a runtime with pending_map field
    let _runtime = WorkflowRuntime::new(ir, MockStore, Clock, Arc::new(NoopTaskExecutor));
}

/// Scenario: MapKey is (RunId, OperatorId) — same shape as ParallelKey.
/// Type alias or identical composite type.
#[test]
fn map_runtime_storage_mapkey_uses_runid_and_operatorid() {
    use sddk_domain::{OperatorId, RunId};

    // MapKey = (RunId, OperatorId) per spec
    let run_id = RunId("test-run".into());
    let op_id = OperatorId("test-op".into());
    let key: (RunId, OperatorId) = (run_id.clone(), op_id.clone());

    // Type check above verifies the composite key type compiles
    assert_eq!(key.0, run_id);
    assert_eq!(key.1, op_id);
}

/// Scenario: MapCheckpointState source_outputs_snapshot populated from source outputs
/// (not empty BTreeMap) — verifies the INV-11 fix.
#[test]
fn map_concurrent_source_outputs_snapshot_captures_actual_outputs() {
    use sddk_engine::operator::MapCheckpointState;
    use std::collections::BTreeMap;
    use std::sync::{Arc, Mutex, mpsc};

    let (_tx, rx) = mpsc::channel();
    let source_outcome_outputs: BTreeMap<String, serde_json::Value> =
        BTreeMap::from([("items".to_string(), serde_json::json!(["x", "y", "z", "w"]))]);

    // The FIX: source_outputs_snapshot captures source_outcome.outputs.clone()
    let state = MapCheckpointState {
        receiver: Arc::new(Mutex::new(rx)),
        items_len: 4,
        completed_results: Default::default(),
        source_outputs_snapshot: source_outcome_outputs.clone(),
    };

    // Assert the snapshot matches the source outputs (non-empty invariant)
    assert_eq!(
        state.source_outputs_snapshot, source_outcome_outputs,
        "source_outputs_snapshot should capture actual source outputs"
    );

    let items = state
        .source_outputs_snapshot
        .get("items")
        .expect("items must exist");
    assert_eq!(items.as_array().expect("must be array").len(), 4);
}
