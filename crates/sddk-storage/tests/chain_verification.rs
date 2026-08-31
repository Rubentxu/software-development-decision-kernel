//! Chain verification tests for stream hash chaining (Phase 2 SHOULD).
//!
//! Verifies:
//! - Genesis chain_hash: SHA256(content_hash || "genesis")
//! - Chained chain_hash: SHA256(content_hash[N] || chain_hash[N-1])
//! - verify_chain_integrity() passes for valid chains
//! - verify_chain_integrity() detects tampered chains

use rusqlite::params;
use sddk_domain::{ActorKind, ActorRef, EntityRef, EventEnvelopeV1, EventStore};
use sha2::{Digest, Sha256};

/// Compute genesis chain_hash: SHA256(content_hash || "genesis")
fn genesis_chain_hash(content_hash: &str) -> String {
    let mut hasher = Sha256::new();
    hasher.update(content_hash.as_bytes());
    hasher.update(b"genesis");
    format!("sha256:{:x}", hasher.finalize())
}

/// Compute chained chain_hash: SHA256(content_hash || prev_chain_hash)
fn chain_hash(content_hash: &str, prev_chain_hash: &str) -> String {
    let mut hasher = Sha256::new();
    hasher.update(content_hash.as_bytes());
    hasher.update(prev_chain_hash.as_bytes());
    format!("sha256:{:x}", hasher.finalize())
}

/// Insert an event directly into the DB with pre-computed chain_hash.
/// Bypasses append() content_hash validation by using the raw SQLite API.
#[allow(clippy::too_many_arguments)]
fn insert_event_with_chain(
    conn: &rusqlite::Connection,
    event_id: &str,
    event_type: &str,
    stream_id: &str,
    sequence: u64,
    project_id: &str,
    content_hash: &str,
    chain_hash: &str,
    cycle_id: Option<&str>,
) -> Result<(), rusqlite::Error> {
    // Ensure project exists (FK constraint)
    conn.execute(
        "INSERT OR IGNORE INTO projects (project_id) VALUES (?1)",
        params![project_id],
    )?;

    let occurred_at = format!("2026-08-19T10:00:{:02}Z", sequence);
    let recorded_at = occurred_at.clone();
    let actor_json = serde_json::to_string(&sddk_domain::ActorRef {
        kind: ActorKind::System,
        id: "chain-test".into(),
        definition_hash: None,
        policy_hash: None,
        model: None,
    })
    .unwrap();
    let subjects_json = serde_json::to_string::<Vec<EntityRef>>(&vec![]).unwrap();
    let payload_json = serde_json::to_string(&serde_json::Value::Null).unwrap();
    let evidence_refs_json = serde_json::to_string::<Vec<String>>(&vec![]).unwrap();

    conn.execute(
        "INSERT INTO events_v1 (
            event_id, stream_id, sequence, event_type, schema_version, project_id,
            occurred_at, recorded_at, actor_json, causation_id, correlation_id,
            cycle_id, frame_id, fork_id, subjects_json, payload_json,
            evidence_refs_json, content_hash, metadata_json, chain_hash
        ) VALUES (
            ?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12, ?13, ?14, ?15,
            ?16, ?17, ?18, ?19, ?20
        )",
        params![
            event_id,
            stream_id,
            sequence as i64,
            event_type,
            1,
            project_id,
            occurred_at,
            recorded_at,
            actor_json,
            None::<String>,
            None::<String>,
            cycle_id,
            None::<String>,
            None::<String>,
            subjects_json,
            payload_json,
            evidence_refs_json,
            content_hash,
            None::<String>,
            chain_hash,
        ],
    )?;
    Ok(())
}

#[test]
fn genesis_chain_hash_computation() {
    // SHA256("sha256:0000...0000" || "genesis")
    let ch = "sha256:0000000000000000000000000000000000000000000000000000000000000000";
    let expected = "sha256:82c84c93ffca4a3bc766390bb30fc3b58b392f7d2670ad57d39be9d02c6ee629";
    assert_eq!(genesis_chain_hash(ch), expected);
}

#[test]
fn chained_chain_hash_computation() {
    // SHA256("sha256:1111...1111" || "sha256:82c8...")
    let ch2 = "sha256:1111111111111111111111111111111111111111111111111111111111111111";
    let ch1 = "sha256:82c84c93ffca4a3bc766390bb30fc3b58b392f7d2670ad57d39be9d02c6ee629";
    let expected = "sha256:82650f72bfe04f8e1f1068fcb0dc8c3a123096ade5d51f33c37de929fd840112";
    assert_eq!(chain_hash(ch2, ch1), expected);
}

#[test]
fn verify_chain_integrity_empty_stream_succeeds() {
    let store = sddk_storage::event_store::SqliteEventStore::open_in_memory()
        .expect("open in-memory store");
    store
        .verify_chain_integrity("project:empty")
        .expect("verify on empty stream should succeed");
}

#[test]
fn verify_chain_integrity_single_genesis_event_succeeds() {
    let store = sddk_storage::event_store::SqliteEventStore::open_in_memory()
        .expect("open in-memory store");

    let ch = "sha256:0000000000000000000000000000000000000000000000000000000000000000";
    let chain = genesis_chain_hash(ch);

    // Insert directly with pre-computed chain_hash
    insert_event_with_chain(
        store.connection(),
        "evt-1",
        "approval.capability.requested",
        "project:single",
        1,
        "project:single",
        ch,
        &chain,
        Some("c-1"),
    )
    .expect("insert evt-1");

    // Verify passes
    store
        .verify_chain_integrity("project:single")
        .expect("verify on single genesis event should succeed");

    // head_chain_hash returns the genesis chain_hash
    let head = store
        .head_chain_hash("project:single")
        .expect("head_chain_hash should succeed");
    assert_eq!(head, Some(chain));
}

#[test]
fn verify_chain_integrity_two_event_chain_succeeds() {
    let store = sddk_storage::event_store::SqliteEventStore::open_in_memory()
        .expect("open in-memory store");

    // Event 1: genesis
    let ch1 = "sha256:0000000000000000000000000000000000000000000000000000000000000000";
    let chain1 = genesis_chain_hash(ch1);
    insert_event_with_chain(
        store.connection(),
        "evt-1",
        "approval.capability.requested",
        "project:chain",
        1,
        "project:chain",
        ch1,
        &chain1,
        Some("c-1"),
    )
    .expect("insert evt-1");

    // Event 2: chained
    let ch2 = "sha256:1111111111111111111111111111111111111111111111111111111111111111";
    let chain2 = chain_hash(ch2, &chain1);
    insert_event_with_chain(
        store.connection(),
        "evt-2",
        "approval.capability.granted",
        "project:chain",
        2,
        "project:chain",
        ch2,
        &chain2,
        Some("c-1"),
    )
    .expect("insert evt-2");

    // Verify passes
    store
        .verify_chain_integrity("project:chain")
        .expect("verify should pass for valid chain");

    // head_chain_hash returns last event's chain_hash
    let head = store
        .head_chain_hash("project:chain")
        .expect("head_chain_hash should succeed");
    assert_eq!(head, Some(chain2));
}

#[test]
fn verify_chain_integrity_detects_tampered_content() {
    let store = sddk_storage::event_store::SqliteEventStore::open_in_memory()
        .expect("open in-memory store");

    // Event 1: genesis
    let ch1 = "sha256:0000000000000000000000000000000000000000000000000000000000000000";
    let chain1 = genesis_chain_hash(ch1);
    insert_event_with_chain(
        store.connection(),
        "evt-1",
        "approval.capability.requested",
        "project:tamper",
        1,
        "project:tamper",
        ch1,
        &chain1,
        Some("c-1"),
    )
    .expect("insert evt-1");

    // Event 2: chained (correct chain)
    let ch2 = "sha256:1111111111111111111111111111111111111111111111111111111111111111";
    let chain2 = chain_hash(ch2, &chain1);
    insert_event_with_chain(
        store.connection(),
        "evt-2",
        "approval.capability.granted",
        "project:tamper",
        2,
        "project:tamper",
        ch2,
        &chain2,
        Some("c-1"),
    )
    .expect("insert evt-2");

    // Verify clean chain passes
    store
        .verify_chain_integrity("project:tamper")
        .expect("verify should pass for untampered chain");

    // Tamper: drop triggers, update, recreate triggers
    // (events_v1 has no-update and no-delete triggers)
    store
        .connection()
        .execute("DROP TRIGGER IF EXISTS events_v1_no_update", [])
        .expect("drop update trigger");
    store
        .connection()
        .execute("DROP TRIGGER IF EXISTS events_v1_no_delete", [])
        .expect("drop delete trigger");
    store
        .connection()
        .execute(
            "UPDATE events_v1 SET content_hash = 'sha256:deadbeefdeadbeefdeadbeefdeadbeefdeadbeefdeadbeefdeadbeefdeadbeef' WHERE event_id = 'evt-2'",
            [],
        )
        .expect("update tampered row");
    // Recreate triggers (keep them for other tests)
    store
        .connection()
        .execute(
            "CREATE TRIGGER events_v1_no_update BEFORE UPDATE ON events_v1 BEGIN SELECT RAISE(ABORT, 'events_v1 are append-only'); END",
            [],
        )
        .expect("recreate update trigger");
    store
        .connection()
        .execute(
            "CREATE TRIGGER events_v1_no_delete BEFORE DELETE ON events_v1 BEGIN SELECT RAISE(ABORT, 'events_v1 are append-only'); END",
            [],
        )
        .expect("recreate delete trigger");

    // Verify detects the tamper
    let result = store.verify_chain_integrity("project:tamper");
    assert!(
        result.is_err(),
        "verify_chain_integrity should detect tampered content_hash"
    );
}

#[test]
fn verify_chain_integrity_detects_broken_chain() {
    let store = sddk_storage::event_store::SqliteEventStore::open_in_memory()
        .expect("open in-memory store");

    // Event 1: genesis
    let ch1 = "sha256:0000000000000000000000000000000000000000000000000000000000000000";
    let chain1 = genesis_chain_hash(ch1);
    insert_event_with_chain(
        store.connection(),
        "evt-1",
        "approval.capability.requested",
        "project:broken",
        1,
        "project:broken",
        ch1,
        &chain1,
        Some("c-1"),
    )
    .expect("insert evt-1");

    // Event 2: WRONG chain_hash (doesn't match SHA256(ch2 || chain1))
    let ch2 = "sha256:1111111111111111111111111111111111111111111111111111111111111111";
    let wrong_chain2 = "sha256:0000000000000000000000000000000000000000000000000000000000000000"; // wrong!
    insert_event_with_chain(
        store.connection(),
        "evt-2",
        "approval.capability.granted",
        "project:broken",
        2,
        "project:broken",
        ch2,
        wrong_chain2,
        Some("c-1"),
    )
    .expect("insert evt-2");

    // Verify detects the broken chain (wrong chain_hash stored)
    let result = store.verify_chain_integrity("project:broken");
    assert!(
        result.is_err(),
        "verify_chain_integrity should detect wrong chain_hash"
    );
}

#[test]
fn append_returns_correct_chain_hash() {
    // This test uses append() with REAL content_hash values (computed by the envelope)
    let mut store = sddk_storage::event_store::SqliteEventStore::open_in_memory()
        .expect("open in-memory store");

    let mut envelope = EventEnvelopeV1 {
        event_id: "evt-1".into(),
        event_type: "approval.capability.requested".into(),
        schema_version: 1,
        stream_id: "project:append-test".into(),
        sequence: 1,
        project_id: "project:append-test".into(),
        occurred_at: "2026-08-19T10:00:00Z".into(),
        recorded_at: "2026-08-19T10:00:00Z".into(),
        actor: ActorRef {
            kind: ActorKind::System,
            id: "chain-test".into(),
            definition_hash: None,
            policy_hash: None,
            model: None,
        },
        subjects: vec![EntityRef {
            kind: "capability".into(),
            id: "git.commit".into(),
            version: None,
            content_hash: None,
        }],
        payload: serde_json::Value::Null,
        evidence_refs: vec![],
        content_hash: "".into(), // will be overwritten with computed
        metadata: None,
        causation_id: None,
        correlation_id: None,
        cycle_id: Some("c-1".into()),
        frame_id: None,
        fork_id: None,
    };

    // Compute expected content_hash and chain_hash
    let expected_ch = envelope.compute_content_hash();
    envelope.content_hash = expected_ch.clone();
    let expected_chain = genesis_chain_hash(&expected_ch);

    let result = store.append(&envelope).expect("append should succeed");

    assert_eq!(result.chain_hash, expected_chain);

    // head_chain_hash matches
    let head = store
        .head_chain_hash("project:append-test")
        .expect("head_chain_hash should succeed");
    assert_eq!(head, Some(expected_chain.clone()));

    // Second event
    let mut envelope2 = EventEnvelopeV1 {
        event_id: "evt-2".into(),
        event_type: "approval.capability.granted".into(),
        schema_version: 1,
        stream_id: "project:append-test".into(),
        sequence: 2,
        project_id: "project:append-test".into(),
        occurred_at: "2026-08-19T10:00:01Z".into(),
        recorded_at: "2026-08-19T10:00:01Z".into(),
        actor: ActorRef {
            kind: ActorKind::System,
            id: "chain-test".into(),
            definition_hash: None,
            policy_hash: None,
            model: None,
        },
        subjects: vec![],
        payload: serde_json::Value::Null,
        evidence_refs: vec![],
        content_hash: "".into(), // will be computed
        metadata: None,
        causation_id: None,
        correlation_id: None,
        cycle_id: Some("c-1".into()),
        frame_id: None,
        fork_id: None,
    };

    let expected_ch2 = envelope2.compute_content_hash();
    envelope2.content_hash = expected_ch2.clone();
    let expected_chain2 = chain_hash(&expected_ch2, &expected_chain);

    let result2 = store.append(&envelope2).expect("append should succeed");
    assert_eq!(result2.chain_hash, expected_chain2);

    // Verify full chain
    store
        .verify_chain_integrity("project:append-test")
        .expect("verify should pass");
}
