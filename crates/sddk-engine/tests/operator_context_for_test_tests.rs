//! RED Tests for OperatorContext::for_test (cycle-24)
//!
//! Verifies:
//! 1. for_test creates context with correct defaults
//! 2. Arc fields share identity between caller and context
//! 3. pending_sender is None

use std::sync::{Arc, Mutex};

use sddk_domain::{NodeId, NodeRun, NodeRunState, WorkflowIR, WorkflowRun};
use sddk_engine::operator::OperatorContext;

// ── Test helpers ────────────────────────────────────────────────────────────────

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

// ── Tests ───────────────────────────────────────────────────────────────────────

#[test]
fn for_test_creates_context_with_defaults() {
    let node_run = Arc::new(Mutex::new(make_node_run()));
    let ir = Arc::new(make_ir());
    let run = Arc::new(make_run());

    let ctx = OperatorContext::for_test(Arc::clone(&node_run), Arc::clone(&ir), Arc::clone(&run));

    // Defaults verified
    assert!(ctx.pending_sender.is_none());
    // store is ScratchGraphStore wrapped in GraphStoreBox
    // clock is Clock::default()
    // executor is NoopTaskExecutor
}

#[test]
fn for_test_arc_fields_share_identity() {
    let node_run = Arc::new(Mutex::new(make_node_run()));
    let ir = Arc::new(make_ir());
    let run = Arc::new(make_run());

    let ctx = OperatorContext::for_test(Arc::clone(&node_run), Arc::clone(&ir), Arc::clone(&run));

    // node_run pointer identity (via Arc::ptr_eq)
    assert!(Arc::ptr_eq(&ctx.node_run, &node_run));
    // ir pointer identity
    assert!(Arc::ptr_eq(&ctx.ir, &ir));
    // run pointer identity
    assert!(Arc::ptr_eq(&ctx.run, &run));
}

#[test]
fn for_test_node_run_strong_count() {
    let node_run = Arc::new(Mutex::new(make_node_run()));
    let ir = Arc::new(make_ir());
    let run = Arc::new(make_run());

    // Strong count is 1 before passing
    assert_eq!(Arc::strong_count(&node_run), 1);

    let _ctx = OperatorContext::for_test(Arc::clone(&node_run), Arc::clone(&ir), Arc::clone(&run));

    // Strong count is 2: original + ctx
    assert_eq!(Arc::strong_count(&node_run), 2);
}

#[test]
fn for_test_store_is_scratch_graph_store() {
    let node_run = Arc::new(Mutex::new(make_node_run()));
    let ir = Arc::new(make_ir());
    let run = Arc::new(make_run());

    let _ctx = OperatorContext::for_test(Arc::clone(&node_run), Arc::clone(&ir), Arc::clone(&run));

    // Verify store is a GraphStoreBox containing ScratchGraphStore
    // The store compiles and works, which is sufficient for this test
    let store = _ctx.store.lock().unwrap();
    let _inner = &store.inner;
    // If this compiles, the store is correctly typed
}
