//! ARCH-SPEC-020 storage-schema-ownership fixtures (PR-GAP-005).
//!
//! Proves the canonical migration owner is the single schema authority for the
//! shared `ledger.sqlite`: opening the same database through `SqliteEventStore`
//! *first*, then `Storage`, yields one full schema instead of the historical
//! `MIGRATION_5` minimal `projects` stub. Before the fix, `SqliteEventStore`
//! ran its own migration subset behind a private `sddk_eventstore_version`
//! pragma, so the full `MIGRATION_1` `projects` definition was skipped by
//! `CREATE TABLE IF NOT EXISTS` and the two stores disagreed on the shared
//! bootstrap contract (SSO-001 / SSO-002).

use sddk_domain::{ActorKind, ActorRef, EventEnvelopeV1, EventStore};
use sddk_storage::Storage;
use sddk_storage::event_store::SqliteEventStore;

fn make_event(project_id: &str, stream_id: &str, sequence: u64) -> EventEnvelopeV1 {
    let mut envelope = EventEnvelopeV1 {
        event_id: format!("evt-{stream_id}-{sequence}"),
        event_type: "schema.ownership.test".into(),
        schema_version: 1,
        stream_id: stream_id.into(),
        sequence,
        project_id: project_id.into(),
        occurred_at: "2026-09-14T00:00:00Z".into(),
        recorded_at: "2026-09-14T00:00:01Z".into(),
        actor: ActorRef {
            kind: ActorKind::System,
            id: "schema-ownership-test".into(),
            definition_hash: None,
            policy_hash: None,
            model: None,
            role: None,
        },
        subjects: vec![],
        payload: serde_json::json!({}),
        evidence_refs: vec![],
        content_hash: String::new(),
        metadata: None,
        causation_id: None,
        correlation_id: None,
        cycle_id: None,
        frame_id: None,
        fork_id: None,
    };
    envelope.content_hash = envelope.compute_content_hash();
    envelope
}

fn projects_columns(db_path: &std::path::Path) -> Vec<String> {
    let conn = rusqlite::Connection::open(db_path).unwrap();
    let mut stmt = conn
        .prepare("SELECT name FROM pragma_table_info('projects')")
        .unwrap();
    stmt.query_map([], |row| row.get::<_, String>(0))
        .unwrap()
        .collect::<Result<Vec<_>, _>>()
        .unwrap()
}

fn user_version(db_path: &std::path::Path) -> i32 {
    let conn = rusqlite::Connection::open(db_path).unwrap();
    conn.pragma_query_value(None, "user_version", |row| row.get(0))
        .unwrap()
}

/// Fixture 2b: both stores apply the same ordered migration set and schema
/// version semantics (SSO-001), regardless of which opens the database first.
#[test]
fn single_migration_sequence_version_parity() {
    let dir_storage = tempfile::tempdir().unwrap();
    let dir_event = tempfile::tempdir().unwrap();

    let storage = Storage::open(dir_storage.path().join("ledger.sqlite")).unwrap();
    let _ = storage.schema_version().unwrap();
    drop(storage);

    let _event_store = SqliteEventStore::open(dir_event.path()).unwrap();

    assert_eq!(
        user_version(&dir_storage.path().join("ledger.sqlite")),
        user_version(&dir_event.path().join("ledger.sqlite")),
        "Storage-first and event-store-first must agree on schema version semantics"
    );
}

/// Fixture 1 + 4: fresh DB opened by the event store first yields the full
/// canonical schema, and the cross-store FK/bootstrap row is consumable by
/// `Storage`.
#[test]
fn event_store_first_then_storage_shares_full_schema_and_bootstrap() {
    let dir = tempfile::tempdir().unwrap();
    let db_path = dir.path().join("ledger.sqlite");

    // Event store opens first and owns nothing but its own auxiliary table; the
    // canonical owner creates the shared schema.
    let mut event_store = SqliteEventStore::open(dir.path()).unwrap();
    event_store
        .append(&make_event("p-schema", "stream-1", 1))
        .unwrap();

    // The shared `projects` table must be the canonical full shape, not the
    // `MIGRATION_5` minimal stub.
    let cols = projects_columns(&db_path);
    for required in ["project_id", "display_name", "scope", "created_at"] {
        assert!(
            cols.iter().any(|c| c == required),
            "projects must have canonical column {required:?}; got {cols:?}"
        );
    }

    // Storage consumes the same database without re-bootstrapping and reads the
    // project row the event store created (SSO-002 consumable bootstrap row).
    let storage = Storage::open(&db_path).unwrap();
    let project = storage
        .get_project_optional("p-schema")
        .unwrap()
        .expect("event store bootstrap row must be consumable by Storage");
    assert_eq!(project.project_id, "p-schema");
    assert_eq!(project.display_name, "p-schema");
    assert_eq!(project.scope, ".");
}

/// Fixture 3: reopening the same database through either store is idempotent
/// and does not fork schema authority.
#[test]
fn reopen_via_storage_then_event_store_is_idempotent() {
    let dir = tempfile::tempdir().unwrap();
    let db_path = dir.path().join("ledger.sqlite");

    let storage = Storage::open(&db_path).unwrap();
    let schema_version = storage.schema_version().unwrap();
    drop(storage);

    // Reopen through the event store, then through Storage again.
    let mut event_store = SqliteEventStore::open(dir.path()).unwrap();
    event_store
        .append(&make_event("p-reopen", "stream-2", 1))
        .unwrap();
    drop(event_store);

    let reopened = Storage::open(&db_path).unwrap();
    assert_eq!(
        reopened.schema_version().unwrap(),
        schema_version,
        "reopen must not change the canonical schema version"
    );
    assert!(reopened.get_project_optional("p-reopen").unwrap().is_some());
}

/// Fixture 1 (negative): an event append bootstraps a FK parent that is
/// complete, so a raw insert that only knows `project_id` still resolves.
#[test]
fn append_autocreates_complete_project_row() {
    let dir = tempfile::tempdir().unwrap();
    let db_path = dir.path().join("ledger.sqlite");

    let mut event_store = SqliteEventStore::open(dir.path()).unwrap();
    event_store
        .append(&make_event("p-complete", "stream-3", 1))
        .unwrap();

    let conn = rusqlite::Connection::open(&db_path).unwrap();
    let (display_name, scope, created_at): (String, String, String) = conn
        .query_row(
            "SELECT display_name, scope, created_at FROM projects WHERE project_id = 'p-complete'",
            [],
            |row| Ok((row.get(0)?, row.get(1)?, row.get(2)?)),
        )
        .unwrap();
    assert!(!display_name.is_empty());
    assert!(!scope.is_empty());
    assert!(!created_at.is_empty());
}
