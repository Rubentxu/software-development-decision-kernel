//! S6a exit gate (real) — DW-RUNTIME-005.
//!
//! Replaces the cycle-16 placeholder with real assertions that `execute()`
//! finalizes a run reaching `TickOutcome::AllComplete` through a deterministic
//! exit gate:
//!   - REQ-EXIT-001: run transitions to `Completed`; second completion rejected.
//!   - REQ-EXIT-002: final `run.outputs` aggregate completed terminal-node
//!     outputs (non-empty where a node produced outputs), keyed by operator id.

use std::collections::BTreeMap;
use std::sync::Arc;

use sddk_domain::graph::ExecutionGraphRevision;
use sddk_domain::workflow_ir::{Budgets, RevisionId, TemplateRef, WorkflowIR};
use sddk_domain::{
    CapabilityId, CorrelationId, Operator, OperatorId, TaskError, TaskExecutor, TaskOutput,
    WorkflowRunState,
};
use sddk_engine::operator::Clock;
use sddk_engine::workflow_runtime::WorkflowRuntime;
use sddk_storage::SqliteGraphStore;
use serde_json::Value;

// ── Stub executor that yields a deterministic output for every capability ────

#[derive(Debug, Default)]
struct OutputStubExecutor;

impl TaskExecutor for OutputStubExecutor {
    fn execute(
        &self,
        capability: &str,
        _inputs: &BTreeMap<String, Value>,
    ) -> Result<TaskOutput, TaskError> {
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
fn build_single_task_ir() -> WorkflowIR {
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
            id: "exit-gate-test".into(),
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
            generated_by: "exit-gate-test".into(),
            prompt_hash: "test".into(),
            model_hash: "test".into(),
            policy_hash: "test".into(),
        },
    }
}

fn dummy_compiled_revision() -> ExecutionGraphRevision {
    ExecutionGraphRevision {
        revision: 0,
        revision_id: RevisionId("rev-exit-gate".into()),
        parent: None,
        events: BTreeMap::new(),
        nodes: BTreeMap::new(),
        edges: BTreeMap::new(),
        digest: [0u8; 32],
        schema_version: 1,
    }
}

fn run_exit_gate() -> (WorkflowRuntime, tempfile::TempDir) {
    let ir = build_single_task_ir();
    let plan_revision_id = "plan-exit-gate";
    let correlation_id = CorrelationId("corr-exit-gate".into());
    let compiled = dummy_compiled_revision();

    let temp_dir = tempfile::TempDir::new_in("/tmp").expect("temp dir");
    let store = SqliteGraphStore::open(temp_dir.path()).expect("open store");
    let executor: Arc<dyn TaskExecutor> = Arc::new(OutputStubExecutor);
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
    assert!(result.is_ok(), "execute should succeed: {result:?}");
    (runtime, temp_dir)
}

/// REQ-EXIT-001 + REQ-EXIT-002: a terminal Task that produced outputs leaves the
/// run `Completed` with `run.outputs` aggregating that node's outputs.
#[test]
fn exit_gate_aggregates_terminal_outputs() {
    let (runtime, _tmp) = run_exit_gate();

    assert_eq!(
        runtime.run().state,
        WorkflowRunState::Completed,
        "run should be Completed"
    );

    let outputs = runtime
        .run()
        .outputs
        .as_ref()
        .expect("run.outputs should be populated on completion");
    assert!(
        !outputs.is_empty(),
        "REQ-EXIT-002: run.outputs must be non-empty when a node produced outputs"
    );

    let root = outputs
        .get("task-root")
        .expect("aggregated outputs keyed by operator id");
    assert_eq!(
        root.get("greeting"),
        Some(&Value::String("hello echo".into())),
        "terminal node output should be reflected in run.outputs"
    );
}

/// REQ-EXIT-001: a second completion attempt on a terminal run is rejected.
#[test]
fn exit_gate_rejects_double_completion() {
    let (mut runtime, _tmp) = run_exit_gate();
    assert_eq!(runtime.run().state, WorkflowRunState::Completed);

    let second = runtime.complete(Default::default());
    assert!(
        second.is_err(),
        "second completion on a Completed run must be rejected (idempotent)"
    );
}

/// A run with no producing node (Noop-style executor) still completes with an
/// empty-but-present output map (behavior preserved for output-less workflows).
#[test]
fn exit_gate_empty_outputs_when_none_produced() {
    use sddk_domain::workflow_ir::{Budgets, TemplateRef, WorkflowIR};
    use sddk_domain::{
        CapabilityId, CorrelationId, Operator, OperatorId, TaskError, TaskExecutor, TaskOutput,
        WorkflowRunState,
    };

    // NoopTaskExecutor returns empty outputs for every task.
    struct Noop;
    impl TaskExecutor for Noop {
        fn execute(&self, _c: &str, _i: &BTreeMap<String, Value>) -> Result<TaskOutput, TaskError> {
            Ok(TaskOutput {
                outputs: BTreeMap::new(),
            })
        }
    }

    let mut operators = BTreeMap::new();
    operators.insert(
        OperatorId("noop-root".into()),
        Operator::Task {
            capability: CapabilityId("noop".into()),
            inputs: Default::default(),
        },
    );
    let ir = WorkflowIR {
        ir_id: None,
        schema_version: 1,
        template_ref: TemplateRef {
            id: "noop-exit-gate".into(),
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
            generated_by: "noop-exit-gate".into(),
            prompt_hash: "test".into(),
            model_hash: "test".into(),
            policy_hash: "test".into(),
        },
    };

    let temp_dir = tempfile::TempDir::new_in("/tmp").expect("temp dir");
    let store = SqliteGraphStore::open(temp_dir.path()).expect("open store");
    let executor: Arc<dyn TaskExecutor> = Arc::new(Noop);
    let mut runtime = WorkflowRuntime::from_compiled(
        ir,
        store,
        Clock,
        executor,
        "plan-noop",
        &CorrelationId("corr-noop".into()),
        &dummy_compiled_revision(),
    );

    let result = runtime.execute();
    assert!(result.is_ok(), "execute should succeed: {result:?}");
    assert_eq!(runtime.run().state, WorkflowRunState::Completed);
    assert!(
        runtime.run().outputs.is_some(),
        "run.outputs should be present (empty map) even when no node produced outputs"
    );
    assert!(
        runtime.run().outputs.as_ref().unwrap().is_empty(),
        "run.outputs should be empty when no node produced outputs"
    );
}
