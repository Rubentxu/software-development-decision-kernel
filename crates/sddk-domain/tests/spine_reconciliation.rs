//! Spine reconciliation tests (AC-PLN4-03, AC-PLN4-04).
//!
//! Tests the reconciliation behavior between spine YAML and the storage DB:
//! - New spine items → imported
//! - Removed spine items → NOT deleted (reconciliation is additive)
//! - Spine metadata NULL → backfilled on re-import
//! - Spine metadata populated but same → already_present
//! - Spine metadata populated but different → conflict

use sddk_domain::spine::SpineStatus;
use sddk_storage::Storage;
use sddk_storage::spine_import::{ImportSummary, import_spine, map_spine_status};
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

fn make_spine_item(id: &str, status: &str, order: u32, horizon: &str) -> String {
    format!(
        r#"  - order: {}
    id: {}
    horizon: {}
    status: {}
    depends_on: []
    objective: Obj for {}
    exit_gate: Gate-{}"#,
        order, id, horizon, status, id, id
    )
}

// ── AC-PLN4-03: reconciliation is additive ─────────────────────────────────────

/// Scenario: new items in spine are imported.
#[test]
fn reconcile_new_items_imported() {
    let tmp = TempDir::new().unwrap();
    let db_path = tmp.path().join("ledger.sqlite");
    let mut storage = Storage::open(&db_path).unwrap();

    let bytes = make_spine_yaml(&make_spine_item("WI-001", "PROPOSED", 1, "H1"));
    let result = import_spine(&bytes, &mut storage).expect("import must succeed");

    assert_eq!(result.imported, 1, "one item imported");
    assert_eq!(result.backfilled, 0);
    assert_eq!(result.conflicts, 0);

    // Second import with new item adds it
    let bytes2 = make_spine_yaml(&format!(
        "{}\n{}",
        make_spine_item("WI-001", "PROPOSED", 1, "H1"),
        make_spine_item("WI-002", "ACTIVE", 2, "H1")
    ));
    let result2 = import_spine(&bytes2, &mut storage).expect("second import must succeed");

    assert_eq!(result2.imported, 1, "one new item imported (WI-002)");
    assert_eq!(result2.already_present, 1, "WI-001 already present");
}

/// Scenario: spine items removed from YAML are NOT deleted from DB (additive).
#[test]
fn reconcile_removed_items_persist() {
    let tmp = TempDir::new().unwrap();
    let db_path = tmp.path().join("ledger.sqlite");
    let mut storage = Storage::open(&db_path).unwrap();

    // Import two items
    let bytes = make_spine_yaml(&format!(
        "{}\n{}",
        make_spine_item("WI-001", "PROPOSED", 1, "H1"),
        make_spine_item("WI-002", "ACTIVE", 2, "H1")
    ));
    import_spine(&bytes, &mut storage).expect("import must succeed");

    // Re-import with only WI-001
    let bytes2 = make_spine_yaml(&make_spine_item("WI-001", "PROPOSED", 1, "H1"));
    import_spine(&bytes2, &mut storage).expect("re-import must succeed");

    // WI-002 should still exist in DB
    let conn = rusqlite::Connection::open(&db_path).unwrap();
    let count: i64 = conn
        .query_row(
            "SELECT COUNT(*) FROM work_items_v1 WHERE id = 'WI-002'",
            [],
            |row| row.get(0),
        )
        .expect("query must succeed");

    assert_eq!(
        count, 1,
        "WI-002 must persist after being removed from YAML"
    );
}

// ── AC-PLN4-04: spine metadata backfill and conflict ───────────────────────────

/// Scenario: spine metadata backfilled when DB columns are NULL (e.g. pre-MIGRATION_16 data).
#[test]
fn reconcile_backfill_null_columns() {
    let tmp = TempDir::new().unwrap();
    let db_path = tmp.path().join("ledger.sqlite");
    let mut storage = Storage::open(&db_path).unwrap();

    // Pre-populate work item with NULL spine columns (simulating pre-MIGRATION_16 state)
    {
        let conn = rusqlite::Connection::open(&db_path).unwrap();
        // Must create the cycle/project/workspace first (FK constraints)
        conn.execute(
            r#"INSERT INTO projects (project_id, display_name, scope, created_at)
               VALUES ('__spine_import__', 'Spine Import', 'spine-import', 1234567890)"#,
            [],
        )
        .unwrap();
        conn.execute(
            r#"INSERT INTO workspaces (workspace_id, project_id, canonical_path, created_at)
               VALUES ('__spine_import_ws__', '__spine_import__', 'spine-import', 1234567890)"#,
            [],
        )
        .unwrap();
        conn.execute(
            r#"INSERT INTO cycles (cycle_id, project_id, workspace_id, status, phase, manifest_json, created_at, updated_at)
               VALUES ('WI-001', '__spine_import__', '__spine_import_ws__', 'OPEN', 'build', '{}', 1234567890, 1234567890)"#,
            [],
        )
        .unwrap();
        conn.execute(
            r#"INSERT INTO work_items_v1
               (id, cycle_id, title, description, status, created_at, schema_version,
                spine_order, spine_horizon, spine_status, exit_gate)
               VALUES ('WI-001', 'WI-001', 'WI-001', 'Obj for WI-001', '"draft"', 1234567890, 1,
                       NULL, NULL, NULL, NULL)"#,
            [],
        )
        .unwrap();
    }

    // Import spine with the same item — should backfill the NULL columns
    let bytes = make_spine_yaml(&make_spine_item("WI-001", "PROPOSED", 100, "H1"));
    let result = import_spine(&bytes, &mut storage).expect("import must succeed");
    assert_eq!(
        result.backfilled, 1,
        "NULL spine columns must be backfilled"
    );

    // Verify columns are now populated
    let conn = rusqlite::Connection::open(&db_path).unwrap();
    let row: (i64, String, String) = conn
        .query_row(
            "SELECT spine_order, spine_horizon, spine_status FROM work_items_v1 WHERE id = 'WI-001'",
            [],
            |row| Ok((row.get(0)?, row.get(1)?, row.get(2)?)),
        )
        .expect("row must exist");
    assert_eq!(row.0, 100);
    assert_eq!(row.1.to_lowercase(), "h1");
    assert_eq!(row.2, "PROPOSED");
}

/// Scenario: spine metadata conflict when populated column differs.
#[test]
fn reconcile_spine_metadata_conflict() {
    let tmp = TempDir::new().unwrap();
    let db_path = tmp.path().join("ledger.sqlite");
    let mut storage = Storage::open(&db_path).unwrap();

    // First import
    let bytes1 = make_spine_yaml(&make_spine_item("WI-001", "PROPOSED", 100, "H1"));
    import_spine(&bytes1, &mut storage).expect("first import must succeed");

    // Re-import with different spine_order → conflict
    let bytes2 = make_spine_yaml(&make_spine_item("WI-001", "PROPOSED", 200, "H1"));
    let err = import_spine(&bytes2, &mut storage).expect_err("must conflict");

    let err_str = err.to_string();
    assert!(
        err_str.contains("conflict") || err_str.contains("Conflict"),
        "error must mention conflict: {}",
        err_str
    );
}

/// Scenario: spine metadata unchanged → already_present (no backfill, no conflict).
#[test]
fn reconcile_unchanged_spine_metadata_already_present() {
    let tmp = TempDir::new().unwrap();
    let db_path = tmp.path().join("ledger.sqlite");
    let mut storage = Storage::open(&db_path).unwrap();

    let bytes = make_spine_yaml(&make_spine_item("WI-001", "PROPOSED", 100, "H1"));

    // First import — inserted
    let result1 = import_spine(&bytes, &mut storage).expect("first import must succeed");
    assert_eq!(result1.imported, 1);

    // Second import — already present (spine columns populated on first insert)
    let result2 = import_spine(&bytes, &mut storage).expect("second import must succeed");
    assert_eq!(result2.already_present, 1, "must be already_present");
    assert_eq!(result2.backfilled, 0, "no backfill needed");
    assert_eq!(result2.conflicts, 0, "no conflict");
}
