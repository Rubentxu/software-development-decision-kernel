//! R3 (concurrency correctness) + R6 (idempotent retry) falsification probes.
//!
//! Cycle: `p-63676b11dc0ef88f/a5-3-concurrency-cas-authority-races`
//! Risks: R3 (P0), R6 (P1)
//! Gates: G4 (concurrency correctness)
//!
//! The substrate has PRIMARY KEY (run_id, node_id) on `node_runs_v1` and
//! `BEFORE DELETE`/`BEFORE UPDATE` triggers that abort with
//! 'node_runs_v1 is append-only'. The current `record_node_run_for_run`
//! uses `INSERT OR REPLACE` which compiles to DELETE+INSERT — that path
//! MUST be fail-closed (typed error), never silent overwrite.
//!
//! These tests pin the contract. If any of them changes behaviour, the
//! cycle has regressed.

use sddk_domain::graph::ExecutionGraphRevision;
use sddk_domain::workflow_ir::{Budgets, NodeId, RevisionId, RunId, TemplateRef};
use sddk_domain::workflow_run::{
    Attempt, AttemptOutcome, ContextCapsuleRef, IdempotencyKey, NodeRun, NodeRunState, Route,
    Usage, WorkflowRun,
};
use sddk_domain::{CorrelationId, GraphStore, StorageError, WorkflowRunState};
use sddk_storage::SqliteGraphStore;

fn make_revision() -> ExecutionGraphRevision {
    ExecutionGraphRevision {
        revision: 0,
        revision_id: RevisionId("rev-test".into()),
        parent: None,
        events: Default::default(),
        nodes: Default::default(),
        edges: Default::default(),
        digest: [0u8; 32],
        schema_version: 1,
    }
}

fn make_run(plan: &str) -> (RunId, WorkflowRun) {
    let correlation_id = CorrelationId(format!("corr-{}", plan));
    let run_id = RunId::derive(plan, &correlation_id);
    let run = WorkflowRun {
        run_id: run_id.clone(),
        template_ref: TemplateRef {
            id: "test".into(),
            version: "1.0".into(),
        },
        ir_hash: "sha256:test".into(),
        graph_revision: RevisionId("rev-test".into()),
        state: WorkflowRunState::Pending,
        inputs: Default::default(),
        outputs: None,
        correlation_id,
        budget: Budgets {
            max_wall_ms: u64::MAX,
            max_tokens: u64::MAX,
            max_cost_micros: u64::MAX,
            max_depth: 8,
            max_nodes: 16,
            remaining_tokens: None,
            no_progress_threshold: u32::MAX,
        },
        schema_version: 1,
    };
    (run_id, run)
}

fn make_attempt(run_id: &RunId, node_id: &str, seq: u32) -> Attempt {
    Attempt {
        attempt_id: sddk_domain::workflow_run::AttemptId(format!("att-{}-{}", node_id, seq)),
        node_id: NodeId(node_id.into()),
        route: Route {
            provider: "test".into(),
            model: "test".into(),
            host: "test".into(),
        },
        started_at: "2026-09-17T00:00:00.000Z".into(),
        ended_at: Some("2026-09-17T00:00:01.000Z".into()),
        outcome: Some(AttemptOutcome::Succeeded {
            outputs: Default::default(),
        }),
        usage: Usage {
            tokens_in: 0,
            tokens_out: 0,
            cost_micros: 0,
            wall_ms: 1,
        },
        context_capsule: ContextCapsuleRef::Pointer {
            cid: format!("capsule-{}-{}", node_id, seq),
        },
        idempotency_key: IdempotencyKey {
            project_id: "test-project".into(),
            run_id: run_id.clone(),
            node_id: NodeId(node_id.into()),
            attempt_seq: seq,
        },
        schema_version: 1,
    }
}

fn open_store() -> (tempfile::TempDir, SqliteGraphStore) {
    let dir = tempfile::tempdir().expect("temp dir");
    let store = SqliteGraphStore::open(dir.path()).expect("open store");
    (dir, store)
}

fn seed_run(store: &mut SqliteGraphStore, run: &WorkflowRun) {
    let compiled = make_revision();
    store.record_run(run, &compiled).expect("record_run");
}

// ─────────────────────────────────────────────────────────────────────────────
// R3 — two writers on same (run_id, node_id) → first wins, second typed conflict
// ─────────────────────────────────────────────────────────────────────────────

#[test]
fn a5_3_r3_first_writer_wins_second_writer_typed_conflict_on_node_run() {
    let (_dir, mut store) = open_store();
    let (run_id, run) = make_run("plan-r3-node-run");
    seed_run(&mut store, &run);

    let node_run_a = NodeRun {
        node_id: NodeId("node-A".into()),
        state: NodeRunState::Running,
        dependencies: Default::default(),
        attempts: vec![],
        expansion_permissions: Default::default(),
        schema_version: 1,
    };

    // First writer succeeds.
    store
        .record_node_run_for_run(&run_id, &node_run_a)
        .expect("first record_node_run_for_run must succeed");

    // Second writer on the same (run_id, node_id) with a DIFFERENT state.
    // Expected: typed conflict (IdempotencyConflict, Database error from
    // the append-only trigger, or UNIQUE violation). NOT silent overwrite.
    let node_run_b = NodeRun {
        state: NodeRunState::Completed,
        ..node_run_a.clone()
    };
    let second = store.record_node_run_for_run(&run_id, &node_run_b);

    assert!(
        second.is_err(),
        "second writer on same PK MUST fail closed, got Ok(()) — silent overwrite!"
    );
    let err = second.unwrap_err();
    let err_str = format!("{:?}", err);
    assert!(
        err_str.contains("IdempotencyConflict")
            || err_str.contains("append-only")
            || err_str.contains("UNIQUE")
            || err_str.contains("Database"),
        "second writer must return a typed conflict, got: {}",
        err_str
    );

    // The persisted row must still be the FIRST writer's state (Running),
    // not the second's (Completed).
    let persisted = store
        .load_node_run(&run_id, &NodeId("node-A".into()))
        .expect("load_node_run")
        .expect("row must exist");
    assert_eq!(
        persisted.state,
        NodeRunState::Running,
        "first writer's state must be authoritative; got {:?}",
        persisted.state
    );
}

// ─────────────────────────────────────────────────────────────────────────────
// R6 — identical retry is idempotent; divergent retry is typed conflict
// ─────────────────────────────────────────────────────────────────────────────

#[test]
fn a5_3_r6_identical_retry_returns_ok_no_double_apply() {
    let (_dir, mut store) = open_store();
    let (run_id, run) = make_run("plan-r6-attempt-replay");
    seed_run(&mut store, &run);

    let attempt = make_attempt(&run_id, "node-replay", 0);

    // First write succeeds.
    store
        .record_attempt(&attempt)
        .expect("first record_attempt must succeed");

    // Identical retry must succeed (idempotent replay) OR return a
    // typed StorageError::IdempotencyConflict that the caller treats
    // as a no-op.
    let second = store.record_attempt(&attempt);
    match second {
        Ok(()) => {} // idempotent replay (also acceptable)
        Err(StorageError::IdempotencyConflict { .. }) => {} // typed no-op
        Err(e) => panic!(
            "identical retry must be Ok or typed IdempotencyConflict, got: {:?}",
            e
        ),
    }

    // The persisted row count must be exactly 1 (no double-apply).
    let attempts = store
        .list_attempts(&run_id, &NodeId("node-replay".into()))
        .expect("list_attempts");
    assert_eq!(
        attempts.len(),
        1,
        "identical retry must not create a duplicate row, got {} rows",
        attempts.len()
    );
}

#[test]
fn a5_3_r6_divergent_retry_returns_typed_idempotency_conflict() {
    let (_dir, mut store) = open_store();
    let (run_id, run) = make_run("plan-r6-attempt-conflict");
    seed_run(&mut store, &run);

    let attempt_v1 = make_attempt(&run_id, "node-conflict", 0);
    store
        .record_attempt(&attempt_v1)
        .expect("first record_attempt must succeed");

    // Retry with SAME idempotency_key but DIFFERENT attempt_id and payload.
    // This is the divergence case — a real conflict, not a replay.
    let divergent = Attempt {
        attempt_id: sddk_domain::workflow_run::AttemptId("att-different-id-for-same-key".into()),
        outcome: Some(AttemptOutcome::Failed {
            error: "divergent outcome".into(),
        }),
        usage: Usage {
            tokens_in: 99,
            tokens_out: 0,
            cost_micros: 0,
            wall_ms: 1,
        },
        ..attempt_v1.clone()
    };
    let second = store.record_attempt(&divergent);
    assert!(
        second.is_err(),
        "divergent retry with same idempotency_key MUST fail closed"
    );
    let err = second.unwrap_err();
    assert!(
        matches!(err, StorageError::IdempotencyConflict { .. }),
        "divergent retry must return typed IdempotencyConflict, got: {:?}",
        err
    );
}

// ─────────────────────────────────────────────────────────────────────────────
// R3 — node_runs_v1 row count invariant under concurrent writers (separate stores)
// ─────────────────────────────────────────────────────────────────────────────

#[test]
fn a5_3_r3_concurrent_writers_serialize_through_append_only_substrate() {
    use std::thread;

    let (dir, mut seed_store) = open_store();
    let (run_id, run) = make_run("plan-r3-parallel");
    // Pre-seed the WorkflowRun so threads contend ONLY on node_runs_v1.
    let compiled = make_revision();
    seed_store
        .record_run(&run, &compiled)
        .expect("seed record_run");
    drop(seed_store);

    // Spawn N threads that each open their own SqliteGraphStore on the
    // same file and try to record_node_run_for_run for the SAME
    // (run_id, node_id). The substrate (PK + append-only trigger) must
    // ensure at most ONE writer succeeds; the rest must fail closed.
    const N: usize = 4;
    let dir_path = dir.path().to_path_buf();

    let handles: Vec<_> = (0..N)
        .map(|i| {
            let path = dir_path.clone();
            let run_id_for_thread = run_id.clone();
            thread::spawn(move || {
                let mut store = SqliteGraphStore::open(&path).expect("open store");
                // All threads target the SAME (run_id, node_id) — this
                // is the actual contention the substrate must defend
                // against. The WorkflowRun FK is pre-seeded by the main
                // thread (below) so the threads can race on the node
                // row only.
                let node_run = NodeRun {
                    node_id: NodeId("node-shared".into()),
                    state: if i == 0 {
                        NodeRunState::Running
                    } else {
                        NodeRunState::Completed
                    },
                    dependencies: Default::default(),
                    attempts: vec![],
                    expansion_permissions: Default::default(),
                    schema_version: 1,
                };
                store.record_node_run_for_run(&run_id_for_thread, &node_run)
            })
        })
        .collect();

    let results: Vec<_> = handles.into_iter().map(|h| h.join().unwrap()).collect();

    // R3 invariant: exactly ONE writer must succeed; the rest must
    // fail closed with a typed conflict (NOT silent overwrite).
    let ok_count = results.iter().filter(|r| r.is_ok()).count();
    let err_count = results.iter().filter(|r| r.is_err()).count();
    assert_eq!(
        ok_count, 1,
        "exactly ONE writer must succeed for the same PK; got {} ok / {} err",
        ok_count, err_count
    );
    assert_eq!(
        err_count,
        N - 1,
        "{} writers must fail closed with typed conflict",
        N - 1
    );
    for (i, r) in results.iter().enumerate() {
        if let Err(e) = r {
            let s = format!("{:?}", e);
            assert!(
                s.contains("IdempotencyConflict")
                    || s.contains("UNIQUE")
                    || s.contains("locked")
                    || s.contains("database is locked")
                    || s.contains("Database"),
                "thread {} failed with untyped error: {}",
                i,
                s
            );
        }
    }

    // Verify the canonical run has at most ONE node_run row for the
    // contended node_id (the substrate must serialize the writers).
    {
        let store = SqliteGraphStore::open(&dir_path).expect("open store");
        let all = store.stream_node_runs(&run_id).expect("stream_node_runs");
        let shared: Vec<_> = all
            .iter()
            .filter(|nr| nr.node_id.0 == "node-shared")
            .collect();
        assert!(
            shared.len() <= 1,
            "at most ONE row may exist for the same (run_id, node_id), got {}",
            shared.len()
        );
    }
}
