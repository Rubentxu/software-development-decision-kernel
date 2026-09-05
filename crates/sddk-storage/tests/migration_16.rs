//! Tests for MIGRATION_16 — spine metadata columns (AC-PLN4-01).
//!
//! MIGRATION_16 adds four additive NULL-able columns to work_items_v1:
//! - spine_order INTEGER NULL
//! - spine_horizon TEXT NULL
//! - spine_status TEXT NULL
//! - exit_gate TEXT NULL
//!
//! These columns are NOT part of WorkItemIdentityProjection (PLN-LEDGER-002 §8 invariant).

use sddk_domain::planning::WorkItemStatus;
use sddk_domain::spine::SpineStatus;
use sddk_storage::Storage;
use tempfile::TempDir;

// ── Test helpers ───────────────────────────────────────────────────────────────

fn make_spine_yaml(items_yaml: &str) -> Vec<u8> {
    format!(
        r#"schema_version: 2
plan_id: test-plan
cycle_binding:
  identity: semantic_work_item_id
  execution_instance: cycle_or_run_id
items:
{}
"#,
        items_yaml
    )
    .into_bytes()
}

fn make_spine_item(id: &str, status: &str, order: u32, horizon: &str, exit_gate: &str) -> String {
    format!(
        r#"  - order: {}
    id: {}
    horizon: {}
    status: {}
    depends_on: []
    objective: Test objective for {}
    exit_gate: {}"#,
        order, id, horizon, status, id, exit_gate
    )
}

// ── AC-PLN4-01: MIGRATION_16 scenarios ─────────────────────────────────────

/// Scenario: fresh database has LATEST_SCHEMA_VERSION == 16.
#[test]
fn migration_16_schema_version_16() {
    let tmp = TempDir::new().unwrap();
    let db_path = tmp.path().join("ledger.sqlite");
    let storage = Storage::open(&db_path).expect("must open fresh DB");

    assert_eq!(
        storage
            .schema_version()
            .expect("schema_version must be queryable"),
        16,
        "LATEST_SCHEMA_VERSION must be 16"
    );
}

/// Scenario: fresh database has four new NULL-able columns on work_items_v1.
#[test]
fn migration_16_adds_four_null_columns() {
    let tmp = TempDir::new().unwrap();
    let db_path = tmp.path().join("ledger.sqlite");
    let _storage = Storage::open(&db_path).expect("must open fresh DB");

    let conn = rusqlite::Connection::open(&db_path).unwrap();

    // Check that the four new columns exist
    let info: Vec<(String, String)> = conn
        .prepare("PRAGMA table_info(work_items_v1)")
        .unwrap()
        .query_map([], |row| {
            Ok((row.get(1)?, row.get(2)?))
        })
        .unwrap()
        .filter_map(|r| r.ok())
        .collect();

    let col_names: Vec<_> = info.iter().map(|(n, _)| n.as_str()).collect();

    assert!(
        col_names.contains(&"spine_order"),
        "spine_order column must exist"
    );
    assert!(
        col_names.contains(&"spine_horizon"),
        "spine_horizon column must exist"
    );
    assert!(
        col_names.contains(&"spine_status"),
        "spine_status column must exist"
    );
    assert!(
        col_names.contains(&"exit_gate"),
        "exit_gate column must exist"
    );
}

/// Scenario: imported spine row populates the four new columns.
#[test]
fn migration_16_import_populates_columns() {
    let tmp = TempDir::new().unwrap();
    let db_path = tmp.path().join("ledger.sqlite");
    let mut storage = Storage::open(&db_path).expect("must open fresh DB");

    let yaml = make_spine_yaml(&make_spine_item(
        "WI-001",
        "PROPOSED",
        100,
        "H1",
        "Test gate",
    ));
    sddk_storage::spine_import::import_spine(&yaml, &mut storage).expect("import must succeed");

    // Query the row directly to verify columns are populated
    let conn = rusqlite::Connection::open(&db_path).unwrap();
    let row: (i64, String, String, String) = conn
        .query_row(
            "SELECT spine_order, spine_horizon, spine_status, exit_gate FROM work_items_v1 WHERE id = 'WI-001'",
            [],
            |row| Ok((row.get(0)?, row.get(1)?, row.get(2)?, row.get(3)?)),
        )
        .expect("row must exist");

    assert_eq!(row.0, 100, "spine_order must be 100");
    assert_eq!(row.1, "h1", "spine_horizon must be h1");
    assert_eq!(row.2, "PROPOSED", "spine_status must be SCREAMING_SNAKE_CASE");
    assert_eq!(row.3, "Test gate", "exit_gate must be populated");
}

/// Scenario: MIGRATION_15-era database survives migration to 16.
#[test]
fn migration_16_preserves_existing_rows() {
    let tmp = TempDir::new().unwrap();
    let db_path = tmp.path().join("ledger.sqlite");

    // Simulate a MIGRATION_15 database by setting user_version and creating schema
    {
        let conn = rusqlite::Connection::open(&db_path).unwrap();
        conn.pragma_update(None, "user_version", 15).unwrap();

        // Create work_items_v1 (MIGRATION_15 shape)
        conn.execute_batch(
            r#"
            CREATE TABLE IF NOT EXISTS work_items_v1 (
                id TEXT NOT NULL PRIMARY KEY,
                cycle_id TEXT NOT NULL,
                title TEXT NOT NULL,
                description TEXT NOT NULL,
                status TEXT NOT NULL,
                actor_ref_kind TEXT,
                actor_ref_id TEXT,
                actor_ref_label TEXT,
                created_at INTEGER NOT NULL,
                schema_version INTEGER NOT NULL DEFAULT 1
            );
            CREATE TABLE IF NOT EXISTS dependency_edges_v1 (
                from_id TEXT NOT NULL,
                to_id TEXT NOT NULL,
                kind TEXT NOT NULL,
                schema_version INTEGER NOT NULL DEFAULT 1,
                PRIMARY KEY (from_id, to_id, kind)
            );
            INSERT INTO work_items_v1 (id, cycle_id, title, description, status, created_at, schema_version)
            VALUES ('LEGACY-001', 'LEGACY-001', 'Legacy Item', 'Desc', 'Draft', 1234567890, 1);
            "#,
        )
        .unwrap();
    }

    // Open with MIGRATION_16 storage
    let storage = Storage::open(&db_path).expect("must reopen with new schema");

    assert_eq!(
        storage.schema_version().expect("schema_version must be queryable"),
        16,
        "schema version must be 16 after migration"
    );

    // Verify legacy row survived
    let conn = rusqlite::Connection::open(&db_path).unwrap();
    let count: i64 = conn
        .query_row(
            "SELECT COUNT(*) FROM work_items_v1 WHERE id = 'LEGACY-001'",
            [],
            |row| row.get(0),
        )
        .expect("legacy row must exist");

    assert_eq!(count, 1, "legacy row must survive migration");
}
