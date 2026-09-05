//! Tests for MIGRATION_15 — evidence_attachments_v1 + decision_records_v1 (PLN-LEDGER-002).
//!
//! Covers AC-PLN2-09:
//! - Fresh DB opens at schema 15 with both tables present
//! - Legacy v14-shaped DB migrates to 15 preserving prior rows
//! - Idempotent reopen

use sddk_storage::Storage;
use tempfile::TempDir;

const CREATED_AT: &str = "2026-09-05T00:00:00Z";

/// Verifies a fresh DB lands at schema version 15 with both new tables.
#[test]
fn fresh_db_opens_at_schema_15() {
    let tmp = TempDir::new().unwrap();
    let db_path = tmp.path().join("ledger.sqlite");
    let storage = Storage::open(&db_path).expect("must open fresh DB");

    assert_eq!(
        storage
            .schema_version()
            .expect("schema_version must be queryable"),
        15,
        "LATEST_SCHEMA_VERSION must be 15"
    );
}

/// Verifies evidence_attachments_v1 table exists in a fresh DB.
#[test]
fn evidence_attachments_table_exists() {
    let tmp = TempDir::new().unwrap();
    let db_path = tmp.path().join("ledger.sqlite");
    let _storage = Storage::open(&db_path).expect("must open fresh DB");

    // Use rusqlite directly to verify table existence
    let conn = rusqlite::Connection::open(&db_path).unwrap();
    let result: Result<i64, _> =
        conn.query_row("SELECT COUNT(*) FROM evidence_attachments_v1", [], |row| {
            row.get(0)
        });
    assert!(
        result.is_ok(),
        "evidence_attachments_v1 must exist: {:?}",
        result.err()
    );
}

/// Verifies decision_records_v1 table exists in a fresh DB.
#[test]
fn decision_records_table_exists() {
    let tmp = TempDir::new().unwrap();
    let db_path = tmp.path().join("ledger.sqlite");
    let _storage = Storage::open(&db_path).expect("must open fresh DB");

    let conn = rusqlite::Connection::open(&db_path).unwrap();
    let result: Result<i64, _> =
        conn.query_row("SELECT COUNT(*) FROM decision_records_v1", [], |row| {
            row.get(0)
        });
    assert!(
        result.is_ok(),
        "decision_records_v1 must exist: {:?}",
        result.err()
    );
}

/// Verifies that a DB migrated from v14 preserves existing rows.
#[test]
fn migration_from_v14_preserves_existing_rows() {
    let tmp = TempDir::new().unwrap();
    let db_path = tmp.path().join("ledger.sqlite");

    // Simulate a v14 DB by manually constructing the schema
    {
        let conn = rusqlite::Connection::open(&db_path).unwrap();
        // Set user_version to 14 to simulate pre-migration state
        conn.pragma_update(None, "user_version", 14).unwrap();
        // Create work_items_v1 (from MIGRATION_14) with a row
        conn.execute_batch(
            "CREATE TABLE IF NOT EXISTS work_items_v1 (\
                id TEXT PRIMARY KEY,\
                cycle_id TEXT NOT NULL,\
                title TEXT NOT NULL,\
                description TEXT NOT NULL,\
                status TEXT NOT NULL,\
                actor_ref_kind TEXT,\
                actor_ref_id TEXT,\
                actor_ref_label TEXT,\
                created_at INTEGER NOT NULL,\
                schema_version INTEGER NOT NULL\
            );\
            INSERT INTO work_items_v1\
                (id, cycle_id, title, description, status, created_at, schema_version)\
            VALUES\
                ('wi-preexisting', 'c-legacy', 'pre-existing item', 'desc', '\"draft\"', 0, 1);",
        )
        .unwrap();
    }

    // Open with Storage — this should trigger migrations from 14 → 15
    let storage = Storage::open(&db_path).expect("must open and migrate");
    assert_eq!(
        storage.schema_version().expect("schema_version"),
        15,
        "must be at schema 15 after migration"
    );

    // Verify pre-existing row survived migration
    let pre_existing = storage
        .get_work_item("wi-preexisting")
        .expect("get must not error")
        .expect("pre-existing work_item row must survive migration");
    assert_eq!(pre_existing.title, "pre-existing item");
}

/// Verifies that re-opening a v15 DB is idempotent (no re-migration errors).
#[test]
fn reopen_v15_db_is_idempotent() {
    let tmp = TempDir::new().unwrap();
    let db_path = tmp.path().join("ledger.sqlite");

    // First open
    {
        let storage = Storage::open(&db_path).expect("first open must succeed");
        assert_eq!(storage.schema_version().unwrap(), 15);
    }
    // Second open
    {
        let storage = Storage::open(&db_path).expect("re-open must succeed");
        assert_eq!(storage.schema_version().unwrap(), 15);
    }
    // Third open
    {
        let storage = Storage::open(&db_path).expect("third open must succeed");
        assert_eq!(storage.schema_version().unwrap(), 15);
    }
}

/// Verifies both new tables have the expected column structure.
#[test]
fn evidence_attachments_has_expected_columns() {
    let tmp = TempDir::new().unwrap();
    let db_path = tmp.path().join("ledger.sqlite");
    let _storage = Storage::open(&db_path).expect("must open");

    let conn = rusqlite::Connection::open(&db_path).unwrap();
    // Check all expected columns exist (not exhaustive FK/index)
    let cols: Vec<String> = conn
        .prepare("PRAGMA table_info(evidence_attachments_v1)")
        .unwrap()
        .query_map([], |row| row.get(1))
        .unwrap()
        .map(|r| r.unwrap())
        .collect();
    assert!(
        cols.contains(&"id".to_string()),
        "evidence_attachments_v1 must have 'id' column"
    );
    assert!(
        cols.contains(&"work_item_id".to_string()),
        "evidence_attachments_v1 must have 'work_item_id' column"
    );
    assert!(
        cols.contains(&"kind".to_string()),
        "evidence_attachments_v1 must have 'kind' column"
    );
    assert!(
        cols.contains(&"body_ref".to_string()),
        "evidence_attachments_v1 must have 'body_ref' column"
    );
}

#[test]
fn decision_records_has_expected_columns() {
    let tmp = TempDir::new().unwrap();
    let db_path = tmp.path().join("ledger.sqlite");
    let _storage = Storage::open(&db_path).expect("must open");

    let conn = rusqlite::Connection::open(&db_path).unwrap();
    let cols: Vec<String> = conn
        .prepare("PRAGMA table_info(decision_records_v1)")
        .unwrap()
        .query_map([], |row| row.get(1))
        .unwrap()
        .map(|r| r.unwrap())
        .collect();
    assert!(
        cols.contains(&"id".to_string()),
        "decision_records_v1 must have 'id' column"
    );
    assert!(
        cols.contains(&"work_item_id".to_string()),
        "decision_records_v1 must have 'work_item_id' column"
    );
    assert!(
        cols.contains(&"kind".to_string()),
        "decision_records_v1 must have 'kind' column"
    );
    assert!(
        cols.contains(&"rationale".to_string()),
        "decision_records_v1 must have 'rationale' column (inline, per Q3 lock)"
    );
}
