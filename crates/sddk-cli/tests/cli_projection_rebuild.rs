//! Integration tests for `sddk dev projection rebuild` CLI command.

use std::fs;
use std::path::PathBuf;

use sddk_cli::run_from;
use sddk_domain::{ActorKind, ActorRef, EventEnvelopeV1, EventStore};
use sddk_storage::SqliteEventStore;
use serde_json::json;
use tempfile::TempDir;

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
        project_id: "p-test".into(),
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

/// Builds a ledger directory with a 3-event cycle stream.
fn build_test_ledger() -> (TempDir, String) {
    let dir = tempfile::tempdir().unwrap();

    let mut event_store = SqliteEventStore::open(dir.path()).unwrap();

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

    (dir, "c-1".to_string())
}

#[test]
fn rebuild_happy_path() {
    let (dir, stream_id) = build_test_ledger();
    let ledger = dir.path().to_path_buf();

    let result = run_from([
        "sddk",
        "dev",
        "projection",
        "rebuild",
        "cycle_state",
        "--stream-id",
        &stream_id,
        "--ledger-dir",
        ledger.to_str().unwrap(),
    ]);

    assert_eq!(
        result.status, 0,
        "rebuild should succeed: {}",
        result.stderr
    );
    assert!(
        result.stdout.contains("cycle_state"),
        "output should mention projection name"
    );
    assert!(
        result.stdout.contains("release"),
        "output should contain final phase"
    );
    assert!(
        result.stdout.contains("seq=3"),
        "output should contain sequence number"
    );
}

#[test]
fn rebuild_unknown_projection_name_fails() {
    let (dir, stream_id) = build_test_ledger();
    let ledger = dir.path().to_path_buf();

    let result = run_from([
        "sddk",
        "dev",
        "projection",
        "rebuild",
        "unknown_proj",
        "--stream-id",
        &stream_id,
        "--ledger-dir",
        ledger.to_str().unwrap(),
    ]);

    assert_ne!(result.status, 0, "unknown projection should fail");
    assert!(
        result.stderr.contains("unknown projection"),
        "error should mention unknown projection: {}",
        result.stderr
    );
}

#[test]
fn rebuild_nonexistent_stream_returns_unknown() {
    let dir = tempfile::tempdir().unwrap();
    // Initialize an empty ledger (no events for "nonexistent-stream").
    let _ = SqliteEventStore::open(dir.path()).unwrap();

    let ledger = dir.path().to_path_buf();

    let result = run_from([
        "sddk",
        "dev",
        "projection",
        "rebuild",
        "cycle_state",
        "--stream-id",
        "nonexistent-stream",
        "--ledger-dir",
        ledger.to_str().unwrap(),
    ]);

    assert_eq!(
        result.status, 0,
        "rebuild of empty stream should succeed: {}",
        result.stderr
    );
    assert!(
        result.stdout.contains("unknown"),
        "empty stream should produce 'unknown' phase: {}",
        result.stdout
    );
}

#[test]
fn rebuild_from_sequence_resumes() {
    let (dir, stream_id) = build_test_ledger();
    let ledger = dir.path().to_path_buf();

    // Rebuild from sequence 2 (skip first event).
    let result = run_from([
        "sddk",
        "dev",
        "projection",
        "rebuild",
        "cycle_state",
        "--stream-id",
        &stream_id,
        "--from-sequence",
        "2",
        "--ledger-dir",
        ledger.to_str().unwrap(),
    ]);

    assert_eq!(
        result.status, 0,
        "resume rebuild should succeed: {}",
        result.stderr
    );
    // Phase should still be "release" since we only skipped the first event.
    assert!(
        result.stdout.contains("release"),
        "phase should be 'release' after resume: {}",
        result.stdout
    );
}

#[test]
fn rebuild_requires_existing_ledger_dir() {
    let nonexistent: PathBuf = "/tmp/sddk-test-nonexistent-ledger-12345".into();
    let _ = fs::remove_dir_all(&nonexistent);

    let result = run_from([
        "sddk",
        "dev",
        "projection",
        "rebuild",
        "cycle_state",
        "--stream-id",
        "any-stream",
        "--ledger-dir",
        nonexistent.to_str().unwrap(),
    ]);

    assert_ne!(result.status, 0, "nonexistent ledger dir should fail");
}
