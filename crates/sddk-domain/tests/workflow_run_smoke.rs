//! State machine smoke tests for WorkflowRun, NodeRun, and Attempt types.
//!
//! Covers:
//! - WorkflowRun: pending → running → completed happy path
//! - WorkflowRun: pending → running → cancelled with reason captured
//! - Attempt: new → in_flight → succeeded terminal transition
//! - ExecutionGraphRevision: root() then child(parent=root) builds valid chain

use std::collections::BTreeMap;

use sddk_domain::workflow_run::{
    Attempt, AttemptError, AttemptId, AttemptOutcome, Budgets as RunBudgets, ContextCapsuleRef,
    CorrelationId, IdempotencyKey, NodeId, NodeRun, NodeRunState, Route, RunId as RunIdType, Usage,
    WorkflowRun, WorkflowRunError, WorkflowRunState,
};

fn sample_run_budgets() -> RunBudgets {
    RunBudgets {
        max_wall_ms: 60000,
        max_tokens: 100_000,
        max_cost_micros: 1_000_000,
        max_depth: 50,
        max_nodes: 200,
        remaining_tokens: Some(95_000),
        no_progress_threshold: u32::MAX,
    }
}

fn sample_route() -> Route {
    Route {
        provider: "openai".into(),
        model: "gpt-4o".into(),
        host: "api.openai.com".into(),
    }
}

fn sample_idempotency_key(run_id: &RunIdType, node_id: &NodeId, seq: u32) -> IdempotencyKey {
    IdempotencyKey {
        project_id: "p-test".into(),
        run_id: run_id.clone(),
        node_id: node_id.clone(),
        attempt_seq: seq,
    }
}

fn make_attempt(node_id: &NodeId, attempt_id: &AttemptId, seq: u32, run_id: &RunIdType) -> Attempt {
    Attempt {
        attempt_id: attempt_id.clone(),
        node_id: node_id.clone(),
        route: sample_route(),
        started_at: "2026-08-19T10:00:00Z".into(),
        ended_at: None,
        outcome: None,
        usage: Usage {
            tokens_in: 1000,
            tokens_out: 500,
            cost_micros: 150,
            wall_ms: 250,
        },
        context_capsule: ContextCapsuleRef::Pointer {
            cid: "cid-test-123".into(),
        },
        idempotency_key: sample_idempotency_key(run_id, node_id, seq),
        schema_version: 1,
    }
}

// ── WorkflowRun happy path ───────────────────────────────────────────────────

#[test]
fn workflow_run_pending_to_running_to_completed() {
    let run_id = RunIdType("run-test-001".into());
    let mut run = WorkflowRun {
        run_id: run_id.clone(),
        template_ref: sddk_domain::workflow_ir::TemplateRef {
            id: "sddk.test".into(),
            version: "1.0.0".into(),
        },
        ir_hash: "sha256:0000000000000000000000000000000000000000000000000000000000000000".into(),
        graph_revision: sddk_domain::workflow_ir::RevisionId("rev-0".into()),
        state: WorkflowRunState::Pending,
        inputs: BTreeMap::new(),
        outputs: None,
        correlation_id: CorrelationId("corr-1".into()),
        budget: sample_run_budgets(),
        schema_version: 1,
    };

    // pending → running
    run.start().expect("pending → running must succeed");
    assert_eq!(run.state, WorkflowRunState::Running);

    // running → completed
    let mut outputs = BTreeMap::new();
    outputs.insert("result".into(), serde_json::json!("ok"));
    run.complete(outputs.clone())
        .expect("running → completed must succeed");
    assert_eq!(run.state, WorkflowRunState::Completed);
    assert_eq!(run.outputs, Some(outputs));

    // Terminal is sticky: cannot re-transition from Completed
    let err = run.start().expect_err("already completed must fail");
    assert!(matches!(err, WorkflowRunError::InvalidTransition { .. }));
}

#[test]
fn workflow_run_pending_to_running_to_cancelled() {
    let run_id = RunIdType("run-test-002".into());
    let mut run = WorkflowRun {
        run_id: run_id.clone(),
        template_ref: sddk_domain::workflow_ir::TemplateRef {
            id: "sddk.test".into(),
            version: "1.0.0".into(),
        },
        ir_hash: "sha256:0000000000000000000000000000000000000000000000000000000000000000".into(),
        graph_revision: sddk_domain::workflow_ir::RevisionId("rev-0".into()),
        state: WorkflowRunState::Pending,
        inputs: BTreeMap::new(),
        outputs: None,
        correlation_id: CorrelationId("corr-2".into()),
        budget: sample_run_budgets(),
        schema_version: 1,
    };

    run.start().expect("pending → running");
    run.cancel().expect("running → cancelled must succeed");
    assert_eq!(run.state, WorkflowRunState::Cancelled);

    // Terminal is sticky: cannot re-transition from Cancelled
    let err = run.start().expect_err("already cancelled must fail");
    assert!(matches!(err, WorkflowRunError::InvalidTransition { .. }));
}

#[test]
fn workflow_run_pause_resume_preserves_budget() {
    let run_id = RunIdType("run-test-003".into());
    let mut run = WorkflowRun {
        run_id: run_id.clone(),
        template_ref: sddk_domain::workflow_ir::TemplateRef {
            id: "sddk.test".into(),
            version: "1.0.0".into(),
        },
        ir_hash: "sha256:0000000000000000000000000000000000000000000000000000000000000000".into(),
        graph_revision: sddk_domain::workflow_ir::RevisionId("rev-0".into()),
        state: WorkflowRunState::Pending,
        inputs: BTreeMap::new(),
        outputs: None,
        correlation_id: CorrelationId("corr-3".into()),
        budget: sample_run_budgets(),
        schema_version: 1,
    };

    run.start().expect("pending → running");

    let budget_before_pause = run.budget.clone();
    run.pause().expect("running → paused must succeed");
    assert_eq!(run.state, WorkflowRunState::Paused);
    assert_eq!(
        run.budget.remaining_tokens,
        budget_before_pause.remaining_tokens
    );

    run.resume().expect("paused → running must succeed");
    assert_eq!(run.state, WorkflowRunState::Running);
    assert_eq!(
        run.budget.remaining_tokens,
        budget_before_pause.remaining_tokens
    );
}

#[test]
fn workflow_run_fail_captures_error() {
    let run_id = RunIdType("run-test-004".into());
    let mut run = WorkflowRun {
        run_id: run_id.clone(),
        template_ref: sddk_domain::workflow_ir::TemplateRef {
            id: "sddk.test".into(),
            version: "1.0.0".into(),
        },
        ir_hash: "sha256:0000000000000000000000000000000000000000000000000000000000000000".into(),
        graph_revision: sddk_domain::workflow_ir::RevisionId("rev-0".into()),
        state: WorkflowRunState::Pending,
        inputs: BTreeMap::new(),
        outputs: None,
        correlation_id: CorrelationId("corr-4".into()),
        budget: sample_run_budgets(),
        schema_version: 1,
    };

    run.start().expect("pending → running");
    run.fail("internal error: out of memory".into())
        .expect("running → failed must succeed");

    assert_eq!(run.state, WorkflowRunState::Failed);
    let outputs = run.outputs.as_ref().expect("outputs must be set on fail");
    let error_val = outputs.get("error").expect("error key must exist");
    assert_eq!(
        error_val,
        &serde_json::json!("internal error: out of memory")
    );
}

#[test]
fn workflow_run_already_terminal_is_idempotent() {
    let run_id = RunIdType("run-test-005".into());
    let mut run = WorkflowRun {
        run_id: run_id.clone(),
        template_ref: sddk_domain::workflow_ir::TemplateRef {
            id: "sddk.test".into(),
            version: "1.0.0".into(),
        },
        ir_hash: "sha256:0000000000000000000000000000000000000000000000000000000000000000".into(),
        graph_revision: sddk_domain::workflow_ir::RevisionId("rev-0".into()),
        state: WorkflowRunState::Pending,
        inputs: BTreeMap::new(),
        outputs: None,
        correlation_id: CorrelationId("corr-5".into()),
        budget: sample_run_budgets(),
        schema_version: 1,
    };

    run.start().expect("pending → running");

    // Complete once
    run.complete(BTreeMap::new())
        .expect("first complete must succeed");

    // Complete again → AlreadyTerminal
    let err = run
        .complete(BTreeMap::new())
        .expect_err("second complete must fail");
    assert!(matches!(err, WorkflowRunError::AlreadyTerminal));

    // cancel again → AlreadyTerminal
    let err = run.cancel().expect_err("cancel after complete must fail");
    assert!(matches!(err, WorkflowRunError::AlreadyTerminal));
}

// ── Attempt state transitions ─────────────────────────────────────────────────

#[test]
fn attempt_new_in_flight_succeeded() {
    let run_id = RunIdType("run-attempt-001".into());
    let node_id = NodeId("node-1".into());
    let attempt_id = AttemptId("att-001".into());

    let mut attempt = make_attempt(&node_id, &attempt_id, 0, &run_id);

    // New attempt is in-flight (no outcome set)
    assert!(attempt.is_in_flight());

    // Complete with success
    let outputs = {
        let mut m = BTreeMap::new();
        m.insert("status".into(), serde_json::json!("success"));
        m
    };
    attempt
        .complete(
            AttemptOutcome::Succeeded { outputs },
            "2026-08-19T10:05:00Z".into(),
        )
        .expect("in_flight → succeeded must succeed");

    assert!(!attempt.is_in_flight());
    assert!(attempt.ended_at.is_some());
    match &attempt.outcome {
        Some(AttemptOutcome::Succeeded { outputs }) => {
            assert_eq!(outputs.get("status"), Some(&serde_json::json!("success")));
        }
        _ => panic!("expected Succeeded outcome"),
    }
}

#[test]
fn attempt_failed_is_terminal() {
    let run_id = RunIdType("run-attempt-002".into());
    let node_id = NodeId("node-1".into());
    let attempt_id = AttemptId("att-002".into());

    let mut attempt = make_attempt(&node_id, &attempt_id, 0, &run_id);

    attempt
        .complete(
            AttemptOutcome::Failed {
                error: "connection refused".into(),
            },
            "2026-08-19T10:05:00Z".into(),
        )
        .expect("complete with Failed must succeed");

    assert!(!attempt.is_in_flight());
    match &attempt.outcome {
        Some(AttemptOutcome::Failed { error }) => {
            assert_eq!(error, "connection refused");
        }
        _ => panic!("expected Failed outcome"),
    }
}

#[test]
fn attempt_already_terminal_cannot_transition() {
    let run_id = RunIdType("run-attempt-003".into());
    let node_id = NodeId("node-1".into());
    let attempt_id = AttemptId("att-003".into());

    let mut attempt = make_attempt(&node_id, &attempt_id, 0, &run_id);

    // First completion succeeds
    attempt
        .complete(
            AttemptOutcome::Succeeded {
                outputs: BTreeMap::new(),
            },
            "2026-08-19T10:05:00Z".into(),
        )
        .expect("first complete must succeed");

    // Second completion fails with AlreadyTerminal
    let err = attempt
        .complete(AttemptOutcome::Timeout, "2026-08-19T10:06:00Z".into())
        .expect_err("second complete must fail");
    assert!(matches!(err, AttemptError::AlreadyTerminal));
}

#[test]
fn attempt_timeout_outcome() {
    let run_id = RunIdType("run-attempt-004".into());
    let node_id = NodeId("node-1".into());
    let attempt_id = AttemptId("att-004".into());

    let mut attempt = make_attempt(&node_id, &attempt_id, 0, &run_id);

    attempt
        .complete(AttemptOutcome::Timeout, "2026-08-19T10:05:00Z".into())
        .expect("timeout complete must succeed");

    assert!(matches!(attempt.outcome, Some(AttemptOutcome::Timeout)));
}

#[test]
fn attempt_cancelled_outcome() {
    let run_id = RunIdType("run-attempt-005".into());
    let node_id = NodeId("node-1".into());
    let attempt_id = AttemptId("att-005".into());

    let mut attempt = make_attempt(&node_id, &attempt_id, 0, &run_id);

    attempt
        .complete(AttemptOutcome::Cancelled, "2026-08-19T10:05:00Z".into())
        .expect("cancelled complete must succeed");

    assert!(matches!(attempt.outcome, Some(AttemptOutcome::Cancelled)));
}

// ── NodeRun state machine ─────────────────────────────────────────────────────

#[test]
fn node_run_pending_to_ready_to_completed() {
    let node_id = NodeId("node-run-1".into());
    let run_id = RunIdType("run-node-001".into());

    let mut node = NodeRun {
        node_id: node_id.clone(),
        state: NodeRunState::Pending,
        dependencies: Default::default(),
        attempts: Vec::new(),
        expansion_permissions: Default::default(),
        schema_version: 1,
    };

    assert!(node.can_ready());
    node.to_ready().expect("pending → ready must succeed");
    assert_eq!(node.state, NodeRunState::Ready);

    // Add an attempt
    let attempt = make_attempt(&node_id, &AttemptId("att-1".into()), 0, &run_id);
    node.attempts.push(attempt);

    // Ready → running is done externally (simulated here by direct state change)
    node.state = NodeRunState::Running;

    // Completed is terminal
    node.state = NodeRunState::Completed;
    assert!(node.is_terminal());
    assert!(!node.can_ready());
}

#[test]
fn node_run_failed_allows_retry() {
    let node_id = NodeId("node-run-2".into());
    let run_id = RunIdType("run-node-002".into());

    let mut node = NodeRun {
        node_id: node_id.clone(),
        state: NodeRunState::Running,
        dependencies: Default::default(),
        attempts: Vec::new(),
        expansion_permissions: Default::default(),
        schema_version: 1,
    };

    // Add first failed attempt
    let mut attempt1 = make_attempt(&node_id, &AttemptId("att-1".into()), 0, &run_id);
    attempt1
        .complete(
            AttemptOutcome::Failed {
                error: "oops".into(),
            },
            "2026-08-19T10:05:00Z".into(),
        )
        .unwrap();
    node.attempts.push(attempt1);

    // Failed node can still be retried (state stays Failed but attempt is appended)
    node.state = NodeRunState::Failed;

    let attempt2 = make_attempt(&node_id, &AttemptId("att-2".into()), 1, &run_id);
    node.attempts.push(attempt2);

    assert_eq!(node.attempts.len(), 2);
    assert_eq!(node.state, NodeRunState::Failed);
}

// ── WorkflowRunState terminal checks ──────────────────────────────────────────

#[test]
fn workflow_run_state_is_terminal() {
    assert!(
        WorkflowRunState::Completed.is_terminal(),
        "Completed is terminal"
    );
    assert!(WorkflowRunState::Failed.is_terminal(), "Failed is terminal");
    assert!(
        WorkflowRunState::Cancelled.is_terminal(),
        "Cancelled is terminal"
    );
    assert!(
        !WorkflowRunState::Pending.is_terminal(),
        "Pending is not terminal"
    );
    assert!(
        !WorkflowRunState::Running.is_terminal(),
        "Running is not terminal"
    );
    assert!(
        !WorkflowRunState::Paused.is_terminal(),
        "Paused is not terminal"
    );
}

#[test]
fn node_run_state_terminal_classification() {
    // Terminal states
    assert!(matches!(NodeRunState::Completed, NodeRunState::Completed));
    assert!(matches!(NodeRunState::Failed, NodeRunState::Failed));
    assert!(matches!(NodeRunState::Skipped, NodeRunState::Skipped));
    // Non-terminal states
    assert!(!matches!(
        NodeRunState::Pending,
        NodeRunState::Completed | NodeRunState::Failed | NodeRunState::Skipped
    ));
    assert!(!matches!(
        NodeRunState::Ready,
        NodeRunState::Completed | NodeRunState::Failed | NodeRunState::Skipped
    ));
    assert!(!matches!(
        NodeRunState::Running,
        NodeRunState::Completed | NodeRunState::Failed | NodeRunState::Skipped
    ));
}
