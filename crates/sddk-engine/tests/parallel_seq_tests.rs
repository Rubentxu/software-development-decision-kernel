//! Adapted tests for Parallel operator with concurrent semantics.

use std::sync::{Arc, Mutex};

use sddk_domain::{CapabilityId, NodeId, NodeRun, NodeRunState, WorkflowIR, WorkflowRun};
use sddk_engine::operator::{NodeOutcome, Operator, OperatorContext, OperatorError, Parallel};

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
struct SimpleTask {
    capability: CapabilityId,
}
impl Operator for SimpleTask {
    fn kind(&self) -> &'static str {
        "SimpleTask"
    }
    fn evaluate(&self, ctx: &mut OperatorContext) -> Result<NodeOutcome, OperatorError> {
        match ctx
            .executor
            .execute(&self.capability.0, &Default::default())
        {
            Ok(output) => Ok(NodeOutcome::Succeeded {
                node_id: ctx.node_run.lock().unwrap().node_id.clone(),
                outputs: output.outputs,
            }),
            Err(e) => Ok(NodeOutcome::Failed {
                node_id: ctx.node_run.lock().unwrap().node_id.clone(),
                reason: e.message,
            }),
        }
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

// ── RED Test 1: outcome_only ─────────────────────────────────────────────────
// eval! that returns (outcome, node_run_arc) for tests needing state inspection.

macro_rules! eval {
    // Takes NodeRun by value — use when test only needs outcome
    ($parallel:expr, $node_run:expr) => {{
        let ir = make_ir();
        let run = make_run();
        let node_run_arc: Arc<Mutex<NodeRun>> = Arc::new(Mutex::new($node_run));
        let mut ctx =
            OperatorContext::for_test(Arc::clone(&node_run_arc), Arc::new(ir), Arc::new(run));
        ($parallel.evaluate(&mut ctx), node_run_arc)
    }};
}

// ── F. Adapted tests ──────────────────────────────────────────────────────────

/// F.1: Parallel with one failing child returns Err(EvalFailed) in one tick.
#[test]
fn parallel_seq_one_child_fails() {
    let children: Vec<Arc<dyn Operator>> = vec![Arc::new(FailOp("child error".into()))];
    let parallel = Parallel {
        children,
        max_concurrency: 1,
    };
    let node_run = NodeRun {
        node_id: NodeId("seq-one-fail".into()),
        state: NodeRunState::Ready,
        dependencies: Default::default(),
        attempts: vec![],
        expansion_permissions: Default::default(),
        schema_version: 1,
    };
    let (outcome, _) = eval!(parallel, node_run);
    // When a child returns Failed, Parallel wraps it as Err(EvalFailed)
    assert!(
        matches!(outcome, Err(sddk_engine::operator::OperatorError::EvalFailed(msg)) if msg == "child error")
    );
}

/// F.2: Empty parallel succeeds immediately (0 children).
#[test]
fn parallel_seq_empty_succeeds() {
    let children: Vec<Arc<dyn Operator>> = vec![];
    let parallel = Parallel {
        children,
        max_concurrency: 0,
    };
    let node_run = NodeRun {
        node_id: NodeId("seq-empty".into()),
        state: NodeRunState::Ready,
        dependencies: Default::default(),
        attempts: vec![],
        expansion_permissions: Default::default(),
        schema_version: 1,
    };
    let (outcome, node_run_arc) = eval!(parallel, node_run);
    let outcome = outcome.unwrap();
    assert!(matches!(outcome, NodeOutcome::Succeeded { node_id, .. } if node_id.0 == "seq-empty"));
    let nr = node_run_arc.lock().unwrap();
    assert_eq!(nr.state, NodeRunState::Completed);
    assert_eq!(nr.attempts.len(), 0);
}

/// F.3: Parallel records N attempts after evaluating N children.
#[test]
fn parallel_seq_records_n_attempts() {
    let children: Vec<Arc<dyn Operator>> = vec![
        Arc::new(SucceedOp),
        Arc::new(SucceedOp),
        Arc::new(SucceedOp),
        Arc::new(SucceedOp),
    ];
    let parallel = Parallel {
        children,
        max_concurrency: 4,
    };
    let node_run = NodeRun {
        node_id: NodeId("seq-4-attempts".into()),
        state: NodeRunState::Ready,
        dependencies: Default::default(),
        attempts: vec![],
        expansion_permissions: Default::default(),
        schema_version: 1,
    };
    let (outcome, node_run_arc) = eval!(parallel, node_run);
    let outcome = outcome.unwrap();
    assert!(matches!(outcome, NodeOutcome::Succeeded { .. }));
    let nr = node_run_arc.lock().unwrap();
    assert_eq!(nr.attempts.len(), 4);
    assert_eq!(nr.state, NodeRunState::Completed);
}

/// F.4: max_concurrency field is correctly recorded in the operator struct.
#[test]
fn parallel_seq_max_concurrency_field_recorded() {
    let children: Vec<Arc<dyn Operator>> = vec![Arc::new(SucceedOp)];
    let parallel = Parallel {
        children,
        max_concurrency: 7,
    };
    assert_eq!(parallel.max_concurrency, 7);
    assert_eq!(parallel.children.len(), 1);
    let node_run = NodeRun {
        node_id: NodeId("seq-max-conc".into()),
        state: NodeRunState::Ready,
        dependencies: Default::default(),
        attempts: vec![],
        expansion_permissions: Default::default(),
        schema_version: 1,
    };
    let (outcome, node_run_arc) = eval!(parallel, node_run);
    let outcome = outcome.unwrap();
    assert!(matches!(outcome, NodeOutcome::Succeeded { .. }));
    let nr = node_run_arc.lock().unwrap();
    assert_eq!(nr.state, NodeRunState::Completed);
}

/// F.5: All children complete in 1 tick (single evaluate call).
#[test]
fn parallel_seq_all_complete_in_one_tick() {
    let children: Vec<Arc<dyn Operator>> = vec![
        Arc::new(SimpleTask {
            capability: CapabilityId("task1".into()),
        }),
        Arc::new(SimpleTask {
            capability: CapabilityId("task2".into()),
        }),
        Arc::new(SimpleTask {
            capability: CapabilityId("task3".into()),
        }),
    ];
    let parallel = Parallel {
        children,
        max_concurrency: 3,
    };
    let node_run = NodeRun {
        node_id: NodeId("seq-one-tick".into()),
        state: NodeRunState::Ready,
        dependencies: Default::default(),
        attempts: vec![],
        expansion_permissions: Default::default(),
        schema_version: 1,
    };
    let (outcome, node_run_arc) = eval!(parallel, node_run);
    let outcome = outcome.unwrap();
    assert!(matches!(outcome, NodeOutcome::Succeeded { .. }));
    let nr = node_run_arc.lock().unwrap();
    assert_eq!(nr.attempts.len(), 3);
    assert_eq!(nr.state, NodeRunState::Completed);
}
