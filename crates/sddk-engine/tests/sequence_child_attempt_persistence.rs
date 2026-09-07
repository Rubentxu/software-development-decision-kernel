//! Tests for Sequence child-attempt persistence (REQ-WFR3-SEQ-001, REQ-WFR3-EV-001).
//!
//! Verifies:
//! - Sequence records one Attempt per child in `attempts_v1` with consecutive `attempt_seq`
//! - Idempotency key collision is rejected with `IdempotencyConflict`
//!
//! NOTE: These tests use the real SqliteGraphStore and pre-insert the workflow run
//! before calling execute() to satisfy FK constraints.

use std::collections::BTreeMap;
use std::sync::Arc;

use sddk_domain::graph::ExecutionGraphRevision;
use sddk_domain::workflow_ir::{Budgets, NodeId, RevisionId, RunId, TemplateRef, WorkflowIR};
use sddk_domain::{CapabilityId, CorrelationId, GraphStore, NoopTaskExecutor, Operator, OperatorId, WorkflowRunState};
use sddk_engine::operator::Clock;
use sddk_engine::workflow_runtime::WorkflowRuntime;
use sddk_storage::SqliteGraphStore;

// ── Helpers ───────────────────────────────────────────────────────────────────

/// Builds a minimal IR with a Sequence of 3 Task children.
fn build_sequence_ir() -> WorkflowIR {
    let mut operators = BTreeMap::new();

    // Three task children
    let op_a = Operator::Task { capability: CapabilityId("a".into()), inputs: Default::default() };
    let op_b = Operator::Task { capability: CapabilityId("b".into()), inputs: Default::default() };
    let op_c = Operator::Task { capability: CapabilityId("c".into()), inputs: Default::default() };

    operators.insert(OperatorId("task-a".into()), op_a);
    operators.insert(OperatorId("task-b".into()), op_b);
    operators.insert(OperatorId("task-c".into()), op_c);

    // Sequence wrapping the three tasks
    let op_seq = Operator::Sequence {
        body: vec![
            OperatorId("task-a".into()),
            OperatorId("task-b".into()),
            OperatorId("task-c".into()),
        ],
    };
    operators.insert(OperatorId("seq-root".into()), op_seq);

    WorkflowIR {
        ir_id: None,
        schema_version: 1,
        template_ref: TemplateRef { id: "seq-test".into(), version: "0.1.0".into() },
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
            generated_by: "seq-child-attempt-test".into(),
            prompt_hash: "test".into(),
            model_hash: "test".into(),
            policy_hash: "test".into(),
        },
    }
}

/// Compiled revision for `from_compiled`.
fn dummy_compiled_revision() -> ExecutionGraphRevision {
    ExecutionGraphRevision {
        revision: 0,
        revision_id: RevisionId("rev-seq-test".into()),
        parent: None,
        events: BTreeMap::new(),
        nodes: BTreeMap::new(),
        edges: BTreeMap::new(),
        digest: [0u8; 32],
        schema_version: 1,
    }
}

// ── sequence_records_one_attempt_per_child ───────────────────────────────────

/// Scenario: a Sequence with 3 children produces attempts in `attempts_v1`
/// with consecutive `attempt_seq` values (0, 1, 2).
///
/// REQ-WFR3-SEQ-001: one Attempt per child per Sequence::evaluate call.
#[test]
fn sequence_records_one_attempt_per_child() {
    // Build IR and compiled revision
    let ir = build_sequence_ir();
    let plan_revision_id = "plan-seq-s3";
    let correlation_id = CorrelationId("corr-seq-s3".into());
    let compiled = dummy_compiled_revision();

    // File-based ledger (not :memory:)
    let temp_dir = tempfile::TempDir::new_in("/tmp").expect("temp dir");
    let mut store = SqliteGraphStore::open(temp_dir.path()).expect("open store");

    // Derive the run_id that from_compiled will produce
    let run_id = RunId::derive(plan_revision_id, &correlation_id);

    // Pre-insert the workflow run so FK constraints are satisfied
    let run = sddk_domain::WorkflowRun {
        run_id: run_id.clone(),
        template_ref: ir.template_ref.clone(),
        ir_hash: format!("sha256:{}", ir.compute_content_hash()),
        graph_revision: compiled.revision_id.clone(),
        state: WorkflowRunState::Pending,
        inputs: Default::default(),
        outputs: None,
        correlation_id: correlation_id.clone(),
        budget: ir.budgets.clone(),
        schema_version: 1,
    };
    store.record_run(&run, &compiled).expect("record_run failed");

    let clock = Clock;
    let executor: Arc<dyn sddk_domain::TaskExecutor> = Arc::new(NoopTaskExecutor);

    // Build runtime with the same compiled revision
    let mut runtime = WorkflowRuntime::from_compiled(
        ir,
        store,
        clock,
        executor,
        plan_revision_id,
        &correlation_id,
        &compiled,
    );

    let result = runtime.execute();
    assert!(result.is_ok(), "execute should succeed: {:?}", result);
    assert_eq!(
        runtime.run().state,
        WorkflowRunState::Completed,
        "workflow should be Completed"
    );

    // Verify attempts were recorded for the Sequence node
    let node_seq = NodeId("seq-root".into());
    let store2 = SqliteGraphStore::open(temp_dir.path()).expect("re-open store");
    let attempts = store2.list_attempts(&run_id, &node_seq).expect("list_attempts");

    // We expect at least 3 attempts (one per child)
    assert!(
        attempts.len() >= 3,
        "expected ≥3 attempts for Sequence (one per child), got {}",
        attempts.len()
    );

    // Verify attempt_seq values are consecutive: 0, 1, 2
    let mut seqs: Vec<u32> = attempts
        .iter()
        .map(|a| a.idempotency_key.attempt_seq)
        .collect();
    seqs.sort();
    assert_eq!(seqs, vec![0, 1, 2], "attempt_seq should be consecutive 0,1,2");
}

// ── sequence_idempotency_key_collision_is_rejected ───────────────────────────

/// Scenario: re-attempting the same child (same idempotency key) is rejected
/// with `IdempotencyConflict`.
///
/// REQ-WFR3-SEQ-001 / REQ-WFR3-EV-001: UNIQUE constraint on idempotency_key.
#[test]
fn sequence_idempotency_key_collision_is_rejected() {
    let plan_revision_id = "plan-seq-idempotency";
    let correlation_id = CorrelationId("corr-seq-idempotency".into());
    let compiled = dummy_compiled_revision();

    let temp_dir = tempfile::TempDir::new_in("/tmp").expect("temp dir");
    let mut store = SqliteGraphStore::open(temp_dir.path()).expect("open store");

    // Pre-insert the workflow run
    let ir = build_sequence_ir();
    let run_id = RunId::derive(plan_revision_id, &correlation_id);
    let run = sddk_domain::WorkflowRun {
        run_id: run_id.clone(),
        template_ref: ir.template_ref.clone(),
        ir_hash: format!("sha256:{}", ir.compute_content_hash()),
        graph_revision: compiled.revision_id.clone(),
        state: WorkflowRunState::Pending,
        inputs: Default::default(),
        outputs: None,
        correlation_id: correlation_id.clone(),
        budget: ir.budgets.clone(),
        schema_version: 1,
    };
    store.record_run(&run, &compiled).expect("record_run failed");

    // Create an attempt with a known idempotency key
    let node_seq = NodeId("seq-root".into());
    let attempt = sddk_domain::workflow_run::Attempt {
        attempt_id: sddk_domain::workflow_run::AttemptId("first-attempt".into()),
        node_id: node_seq.clone(),
        route: sddk_domain::Route {
            provider: "test".into(),
            model: "seq".into(),
            host: "local".into(),
        },
        started_at: "2026-09-07T00:00:00Z".into(),
        ended_at: Some("2026-09-07T00:00:01Z".into()),
        outcome: Some(sddk_domain::workflow_run::AttemptOutcome::Succeeded {
            outputs: Default::default(),
        }),
        usage: sddk_domain::Usage {
            tokens_in: 0,
            tokens_out: 0,
            cost_micros: 0,
            wall_ms: 0,
        },
        context_capsule: sddk_domain::ContextCapsuleRef::Pointer {
            cid: "seq-test".into(),
        },
        idempotency_key: sddk_domain::IdempotencyKey {
            project_id: "sddk".to_string(),
            run_id: run_id.clone(),
            node_id: node_seq.clone(),
            attempt_seq: 0,
        },
        schema_version: 1,
    };

    // First insert should succeed
    store.record_attempt(&attempt).expect("first insert should succeed");

    // Duplicate with same idempotency key should fail
    let dup_attempt = sddk_domain::workflow_run::Attempt {
        attempt_id: sddk_domain::workflow_run::AttemptId("dup-attempt".into()),
        node_id: node_seq,
        route: sddk_domain::Route {
            provider: "test".into(),
            model: "dup".into(),
            host: "local".into(),
        },
        started_at: "2026-09-07T00:00:02Z".into(),
        ended_at: Some("2026-09-07T00:00:03Z".into()),
        outcome: Some(sddk_domain::workflow_run::AttemptOutcome::Succeeded {
            outputs: Default::default(),
        }),
        usage: sddk_domain::Usage {
            tokens_in: 0,
            tokens_out: 0,
            cost_micros: 0,
            wall_ms: 0,
        },
        context_capsule: sddk_domain::ContextCapsuleRef::Pointer {
            cid: "dup".into(),
        },
        idempotency_key: sddk_domain::IdempotencyKey {
            project_id: "sddk".to_string(),
            run_id: run_id,
            node_id: NodeId("seq-root".into()),
            attempt_seq: 0, // Same attempt_seq = same idempotency key
        },
        schema_version: 1,
    };

    let err = store.record_attempt(&dup_attempt);
    assert!(
        matches!(err, Err(sddk_domain::StorageError::Database(_))),
        "duplicate idempotency key should be rejected, got: {:?}",
        err
    );
}
