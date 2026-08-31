//! Integration tests for the [`rebuild`](sddk_storage::rebuild) algorithm.

use sddk_domain::{
    ActorKind, ActorRef, CycleState, CycleStateProjection, EventEnvelopeV1, EventStore,
    ProjectionError,
};
use sddk_storage::{
    event_store::SqliteEventStore, projection_store::SqliteProjectionStore, rebuild::rebuild,
};
use serde_json::json;

/// Helper to create a minimal `EventEnvelopeV1` with a computed content_hash.
fn make_event(
    stream_id: &str,
    event_type: &str,
    sequence: u64,
    payload: serde_json::Value,
) -> EventEnvelopeV1 {
    let mut env = EventEnvelopeV1 {
        event_id: format!("e-{stream_id}-{sequence}"),
        event_type: event_type.into(),
        schema_version: 1,
        stream_id: stream_id.into(),
        sequence,
        project_id: "p-1".into(),
        occurred_at: "2026-08-17T10:00:00Z".into(),
        recorded_at: "2026-08-17T10:00:01Z".into(),
        actor: ActorRef {
            kind: ActorKind::System,
            id: "sddk-cli".into(),
            definition_hash: None,
            policy_hash: None,
            model: None,
        },
        subjects: vec![],
        payload,
        evidence_refs: vec![],
        content_hash: String::new(),
        metadata: None,
        causation_id: None,
        correlation_id: None,
        cycle_id: None,
        frame_id: None,
        fork_id: None,
    };
    env.content_hash = env.compute_content_hash();
    env
}

/// Opens two store instances sharing the same `ledger.sqlite` temp file.
/// The file is initialized with a `projects` stub so the FK constraint is satisfied.
fn setup_shared_stores(dir: &tempfile::TempDir) -> (SqliteEventStore, SqliteProjectionStore) {
    // Pre-create the projects stub so SqliteEventStore can open the DB.
    let conn_setup = rusqlite::Connection::open(dir.path().join("ledger.sqlite")).unwrap();
    conn_setup
        .execute_batch(
            "CREATE TABLE IF NOT EXISTS projects (project_id TEXT NOT NULL PRIMARY KEY);
             INSERT OR REPLACE INTO projects VALUES('p-1');",
        )
        .unwrap();
    drop(conn_setup);

    // Now open both stores on the same file.
    let event_store = SqliteEventStore::open(dir.path()).unwrap();
    let proj_store = SqliteProjectionStore::open(dir.path()).unwrap();
    (event_store, proj_store)
}

#[test]
fn rebuild_happy_path() {
    let dir = tempfile::tempdir().unwrap();
    let (mut event_store, mut proj_store) = setup_shared_stores(&dir);

    // Append 3 phase events on stream "c-1".
    let events = [
        make_event(
            "c-1",
            "workflow.phase.entered",
            1,
            json!({ "phase": "design" }),
        ),
        make_event(
            "c-1",
            "workflow.phase.entered",
            2,
            json!({ "phase": "build" }),
        ),
        make_event(
            "c-1",
            "workflow.phase.entered",
            3,
            json!({ "phase": "release" }),
        ),
    ];
    for ev in &events {
        event_store.append(ev).unwrap();
    }

    // Rebuild the cycle_state projection.
    let state: CycleState = rebuild::<CycleStateProjection, _, _>(
        &event_store,
        &mut proj_store,
        || CycleStateProjection::new("c-1"),
        "c-1",
        None,
    )
    .unwrap();

    assert_eq!(state.phase, "release");
    assert_eq!(state.last_event_sequence, 3);
    assert!(state.last_event_hash.starts_with("sha256:"));
    assert!(state.entered_at.is_some());

    // Verify the checkpoint was persisted.
    let (cp, state_json) = proj_store
        .load_checkpoint("cycle_state", 1)
        .unwrap()
        .unwrap();
    assert_eq!(cp.last_event_sequence, 3);
    assert_eq!(state_json, serde_json::to_string(&state).unwrap());
}

#[test]
fn rebuild_from_sequence_resumes() {
    let dir = tempfile::tempdir().unwrap();
    let (mut event_store, mut proj_store) = setup_shared_stores(&dir);

    let events = [
        make_event(
            "c-1",
            "workflow.phase.entered",
            1,
            json!({ "phase": "design" }),
        ),
        make_event(
            "c-1",
            "workflow.phase.entered",
            2,
            json!({ "phase": "build" }),
        ),
        make_event(
            "c-1",
            "workflow.phase.entered",
            3,
            json!({ "phase": "test" }),
        ),
        make_event(
            "c-1",
            "workflow.phase.entered",
            4,
            json!({ "phase": "release" }),
        ),
        make_event(
            "c-1",
            "workflow.phase.entered",
            5,
            json!({ "phase": "archive" }),
        ),
    ];
    for ev in &events {
        event_store.append(ev).unwrap();
    }

    // First rebuild: full replay.
    let state: CycleState = rebuild::<CycleStateProjection, _, _>(
        &event_store,
        &mut proj_store,
        || CycleStateProjection::new("c-1"),
        "c-1",
        None,
    )
    .unwrap();
    assert_eq!(state.phase, "archive");
    assert_eq!(state.last_event_sequence, 5);

    // Second rebuild: resume from sequence 3.
    let state: CycleState = rebuild::<CycleStateProjection, _, _>(
        &event_store,
        &mut proj_store,
        || CycleStateProjection::new("c-1"),
        "c-1",
        Some(3),
    )
    .unwrap();
    assert_eq!(state.phase, "archive");
    assert_eq!(state.last_event_sequence, 5);
}

#[test]
fn rebuild_tampering_blocks_checkpoint() {
    let dir = tempfile::tempdir().unwrap();
    let (mut event_store, mut proj_store) = setup_shared_stores(&dir);

    // Append 3 events.
    let events = [
        make_event(
            "c-1",
            "workflow.phase.entered",
            1,
            json!({ "phase": "build" }),
        ),
        make_event(
            "c-1",
            "workflow.phase.entered",
            2,
            json!({ "phase": "test" }),
        ),
        make_event(
            "c-1",
            "workflow.phase.entered",
            3,
            json!({ "phase": "release" }),
        ),
    ];
    for ev in &events {
        event_store.append(ev).unwrap();
    }

    // Tamper with the second event's content_hash.
    // Drop the append-only trigger first (it would RAISE(ABORT) on UPDATE),
    // tamper, then re-create the trigger.
    let raw_conn = rusqlite::Connection::open(dir.path().join("ledger.sqlite")).unwrap();
    raw_conn
        .execute_batch(
            "DROP TRIGGER IF EXISTS events_v1_no_update;
             UPDATE events_v1 SET content_hash = 'sha256:0000000000000000000000000000000000000000000000000000000000000000' WHERE sequence = 2;
             CREATE TRIGGER events_v1_no_update BEFORE UPDATE ON events_v1 BEGIN SELECT RAISE(ABORT, 'events_v1 are append-only'); END;",
        )
        .unwrap();
    drop(raw_conn);

    // Rebuild must fail with ChainIntegrityBroken and write NO checkpoint.
    let result: Result<CycleState, ProjectionError> = rebuild::<CycleStateProjection, _, _>(
        &event_store,
        &mut proj_store,
        || CycleStateProjection::new("c-1"),
        "c-1",
        None,
    );

    assert!(
        matches!(result, Err(ProjectionError::ChainIntegrityBroken { .. })),
        "expected ChainIntegrityBroken, got {result:?}"
    );

    // Verify no checkpoint was written.
    let checkpoint = proj_store.load_checkpoint("cycle_state", 1).unwrap();
    assert!(
        checkpoint.is_none(),
        "no checkpoint should be persisted after chain integrity failure"
    );
}

#[test]
fn rebuild_empty_stream_returns_default_state() {
    let dir = tempfile::tempdir().unwrap();
    let (event_store, mut proj_store) = setup_shared_stores(&dir);

    let state: CycleState = rebuild::<CycleStateProjection, _, _>(
        &event_store,
        &mut proj_store,
        || CycleStateProjection::new("c-1"),
        "c-1",
        None,
    )
    .unwrap();

    assert_eq!(state.phase, "unknown");
    assert_eq!(state.last_event_sequence, 0);
}

#[test]
fn rebuild_other_stream_ignored() {
    let dir = tempfile::tempdir().unwrap();
    let (mut event_store, mut proj_store) = setup_shared_stores(&dir);

    // Append events on "c-1" and "c-2".
    event_store
        .append(&make_event(
            "c-1",
            "workflow.phase.entered",
            1,
            json!({ "phase": "build" }),
        ))
        .unwrap();
    event_store
        .append(&make_event(
            "c-2",
            "workflow.phase.entered",
            1,
            json!({ "phase": "design" }),
        ))
        .unwrap();

    // Rebuild "c-1" — c-2 events must not affect it.
    let state: CycleState = rebuild::<CycleStateProjection, _, _>(
        &event_store,
        &mut proj_store,
        || CycleStateProjection::new("c-1"),
        "c-1",
        None,
    )
    .unwrap();

    assert_eq!(state.phase, "build");
}
