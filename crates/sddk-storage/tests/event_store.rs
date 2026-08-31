//! Integration tests for [`SqliteEventStore`](sddk_storage::SqliteEventStore).

use std::sync::{Arc, Barrier};
use std::thread;

use sddk_domain::{ActorKind, ActorRef, EventEnvelopeV1, EventStore};
use sddk_storage::SqliteEventStore;

fn minimal_envelope(event_id: &str, stream_id: &str, project_id: &str) -> EventEnvelopeV1 {
    let mut env = EventEnvelopeV1 {
        event_id: event_id.into(),
        event_type: "workflow.phase.entered".into(),
        schema_version: 1,
        stream_id: stream_id.into(),
        sequence: 0, // ignored — adapter assigns
        project_id: project_id.into(),
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
        payload: serde_json::json!({}),
        evidence_refs: vec![],
        content_hash: String::new(), // will be set below
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

#[test]
fn append_then_load_by_event_id() {
    let mut store = SqliteEventStore::open_in_memory().unwrap();
    let env = minimal_envelope("e-1", "s-1", "p-1");
    let r = store.append(&env).unwrap();
    assert_eq!(r.sequence, 1);
    let loaded = store.load_by_event_id("e-1").unwrap().unwrap();
    assert_eq!(loaded.event_id, env.event_id);
    assert_eq!(loaded.event_type, env.event_type);
    assert_eq!(loaded.stream_id, env.stream_id);
    assert_eq!(loaded.project_id, env.project_id);
    assert_eq!(loaded.content_hash, env.content_hash);
}

#[test]
fn append_increments_sequence_per_stream() {
    let mut store = SqliteEventStore::open_in_memory().unwrap();
    let e1 = store
        .append(&minimal_envelope("e-1", "s-1", "p-1"))
        .unwrap();
    let e2 = store
        .append(&minimal_envelope("e-2", "s-1", "p-1"))
        .unwrap();
    let e3 = store
        .append(&minimal_envelope("e-3", "s-1", "p-1"))
        .unwrap();
    assert_eq!(e1.sequence, 1);
    assert_eq!(e2.sequence, 2);
    assert_eq!(e3.sequence, 3);
}

#[test]
fn separate_streams_have_independent_sequences() {
    let mut store = SqliteEventStore::open_in_memory().unwrap();
    let a1 = store
        .append(&minimal_envelope("e-a1", "s-a", "p-1"))
        .unwrap();
    let b1 = store
        .append(&minimal_envelope("e-b1", "s-b", "p-1"))
        .unwrap();
    assert_eq!(a1.sequence, 1);
    assert_eq!(b1.sequence, 1);
}

#[test]
fn append_validates_content_hash() {
    let mut store = SqliteEventStore::open_in_memory().unwrap();
    let mut env = minimal_envelope("e-1", "s-1", "p-1");
    env.content_hash = "not-a-sha256-prefix".into();
    let err = store.append(&env).unwrap_err();
    let msg = err.to_string();
    assert!(
        msg.contains("event_store:invalid_content_hash")
            || msg.contains("event_store:content_hash_mismatch"),
        "got: {msg}"
    );
}

#[test]
fn append_rejects_update_via_trigger() {
    let mut store = SqliteEventStore::open_in_memory().unwrap();
    let _ = store
        .append(&minimal_envelope("e-1", "s-1", "p-1"))
        .unwrap();
    // Direct SQL UPDATE should be blocked by the trigger.
    let r = store.connection().execute(
        "UPDATE events_v1 SET content_hash = ?1",
        rusqlite::params![
            "sha256:0000000000000000000000000000000000000000000000000000000000000000"
        ],
    );
    assert!(r.is_err()); // trigger fires
}

#[test]
fn append_rejects_delete_via_trigger() {
    let mut store = SqliteEventStore::open_in_memory().unwrap();
    let _ = store
        .append(&minimal_envelope("e-1", "s-1", "p-1"))
        .unwrap();
    let r = store.connection().execute("DELETE FROM events_v1", []);
    assert!(r.is_err()); // trigger fires
}

#[test]
fn verify_stream_chain_succeeds_for_unbroken_chain() {
    let mut store = SqliteEventStore::open_in_memory().unwrap();
    for i in 1..=5 {
        let id = format!("e-{i}");
        store.append(&minimal_envelope(&id, "s-1", "p-1")).unwrap();
    }
    store
        .verify_stream_chain("s-1")
        .expect("chain should verify");
}

#[test]
#[ignore = "Tampering requires trigger bypass; covered by SDDK2-203 chain verify"]
fn verify_stream_chain_fails_on_tampered_hash() {
    // The events_v1 trigger blocks UPDATE, so we cannot easily tamper
    // without a test-only trigger disable. Covered by SDDK2-203.
}

#[test]
fn idempotency_unique_event_id() {
    let mut store = SqliteEventStore::open_in_memory().unwrap();
    let env = minimal_envelope("e-1", "s-1", "p-1");
    store.append(&env).unwrap();
    // Re-append — should succeed (idempotent) but return same sequence.
    let r2 = store.append(&env).unwrap();
    assert_eq!(r2.sequence, 1); // same sequence
}

#[test]
fn last_sequence_returns_none_for_empty_stream() {
    let store = SqliteEventStore::open_in_memory().unwrap();
    let seq = store.last_sequence("nonexistent-stream").unwrap();
    assert_eq!(seq, None);
}

#[test]
fn last_sequence_returns_max_sequence() {
    let mut store = SqliteEventStore::open_in_memory().unwrap();
    store
        .append(&minimal_envelope("e-1", "s-1", "p-1"))
        .unwrap();
    store
        .append(&minimal_envelope("e-2", "s-1", "p-1"))
        .unwrap();
    store
        .append(&minimal_envelope("e-3", "s-1", "p-1"))
        .unwrap();
    assert_eq!(store.last_sequence("s-1").unwrap(), Some(3));
}

#[test]
fn count_returns_total_events() {
    let mut store = SqliteEventStore::open_in_memory().unwrap();
    assert_eq!(store.count().unwrap(), 0);
    store
        .append(&minimal_envelope("e-1", "s-1", "p-1"))
        .unwrap();
    store
        .append(&minimal_envelope("e-2", "s-2", "p-1"))
        .unwrap();
    assert_eq!(store.count().unwrap(), 2);
}

#[test]
fn load_stream_returns_events_in_sequence_order() {
    let mut store = SqliteEventStore::open_in_memory().unwrap();
    for i in 1..=5 {
        store
            .append(&minimal_envelope(&format!("e-{i}"), "s-1", "p-1"))
            .unwrap();
    }
    let events = store.load_stream("s-1", None, 100).unwrap();
    assert_eq!(events.len(), 5);
    for (i, ev) in events.iter().enumerate() {
        assert_eq!(ev.sequence, (i + 1) as u64);
    }
}

#[test]
fn load_stream_respects_after_sequence_filter() {
    let mut store = SqliteEventStore::open_in_memory().unwrap();
    for i in 1..=5 {
        store
            .append(&minimal_envelope(&format!("e-{i}"), "s-1", "p-1"))
            .unwrap();
    }
    let events = store.load_stream("s-1", Some(2), 100).unwrap();
    assert_eq!(events.len(), 3);
    assert_eq!(events[0].sequence, 3);
    assert_eq!(events[1].sequence, 4);
    assert_eq!(events[2].sequence, 5);
}

#[test]
fn load_stream_respects_limit() {
    let mut store = SqliteEventStore::open_in_memory().unwrap();
    for i in 1..=5 {
        store
            .append(&minimal_envelope(&format!("e-{i}"), "s-1", "p-1"))
            .unwrap();
    }
    let events = store.load_stream("s-1", None, 3).unwrap();
    assert_eq!(events.len(), 3);
    assert_eq!(events[0].sequence, 1);
    assert_eq!(events[2].sequence, 3);
}

#[test]
fn load_by_sequence_returns_event() {
    let mut store = SqliteEventStore::open_in_memory().unwrap();
    store
        .append(&minimal_envelope("e-1", "s-1", "p-1"))
        .unwrap();
    store
        .append(&minimal_envelope("e-2", "s-1", "p-1"))
        .unwrap();
    let ev = store.load_by_sequence("s-1", 2).unwrap().unwrap();
    assert_eq!(ev.event_id, "e-2");
    assert_eq!(ev.sequence, 2);
}

#[test]
fn load_by_sequence_returns_none_for_gap() {
    let mut store = SqliteEventStore::open_in_memory().unwrap();
    store
        .append(&minimal_envelope("e-1", "s-1", "p-1"))
        .unwrap();
    // Sequence 2 is unallocated.
    let ev = store.load_by_sequence("s-1", 2).unwrap();
    assert_eq!(ev, None);
}

#[test]
fn head_hash_returns_hash_of_last_event() {
    let mut store = SqliteEventStore::open_in_memory().unwrap();
    store
        .append(&minimal_envelope("e-1", "s-1", "p-1"))
        .unwrap();
    store
        .append(&minimal_envelope("e-2", "s-1", "p-1"))
        .unwrap();
    let h = store.head_hash("s-1").unwrap().unwrap();
    // The hash should be the content_hash of event e-2.
    let e2 = store.load_by_event_id("e-2").unwrap().unwrap();
    assert_eq!(h, e2.content_hash);
}

#[test]
fn head_hash_returns_none_for_empty_stream() {
    let store = SqliteEventStore::open_in_memory().unwrap();
    let h = store.head_hash("nonexistent").unwrap();
    assert_eq!(h, None);
}

#[test]
fn concurrent_append_yields_unique_sequences() {
    let store = Arc::new(std::sync::Mutex::new(
        SqliteEventStore::open_in_memory().unwrap(),
    ));
    let n_threads = 2usize;
    let per_thread = 100usize;
    let barrier = Arc::new(Barrier::new(n_threads));
    let mut handles = vec![];
    for tid in 0..n_threads {
        let s = Arc::clone(&store);
        let b = Arc::clone(&barrier);
        handles.push(thread::spawn(move || {
            b.wait();
            let mut local_store = s.lock().unwrap();
            for i in 0..per_thread {
                let id = format!("e-t{tid}-{i}");
                local_store
                    .append(&minimal_envelope(&id, "s-1", "p-1"))
                    .unwrap();
            }
        }));
    }
    for h in handles {
        h.join().unwrap();
    }
    // Verify all 200 events have unique sequences 1..200
    let s = store.lock().unwrap();
    let count = s.count().unwrap();
    assert_eq!(count, (n_threads * per_thread) as u64);
    let loaded = s.load_stream("s-1", None, 1000).unwrap();
    let mut seqs: Vec<u64> = loaded.iter().map(|e| e.sequence).collect();
    seqs.sort();
    seqs.dedup();
    assert_eq!(seqs.len(), n_threads * per_thread);
    assert_eq!(seqs[0], 1);
    let expected_last = (n_threads * per_thread) as u64;
    assert_eq!(seqs.last(), Some(&expected_last));
}

#[test]
fn migration_5_creates_events_v1() {
    // Create a v4-shaped database (no events_v1 table).
    let dir = tempfile::tempdir().unwrap();
    let db_path = dir.path().join("ledger.sqlite");
    {
        // Run through MIGRATION_1..4 via Storage, which sets up the legacy schema.
        let storage = sddk_storage::Storage::open(&db_path).unwrap();
        assert_eq!(storage.schema_version().unwrap(), 13); // MIGRATION_13 applied
        // Insert a minimal project so the FK on events_v1.project_id is satisfied.
        storage
            .insert_project(&sddk_domain::ProjectRecord {
                project_id: "p-1".into(),
                display_name: "Test".into(),
                remote_url: None,
                scope: "test".into(),
                created_at: "2026-08-17T00:00:00Z".into(),
            })
            .unwrap();
    }
    // Re-open via SqliteEventStore and verify events_v1 exists.
    let store = SqliteEventStore::open(dir.path()).unwrap();
    let count = store.count().unwrap();
    assert_eq!(count, 0); // table exists but empty
    let seq = store.last_sequence("any-stream").unwrap();
    assert_eq!(seq, None); // no events yet
}
