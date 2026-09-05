//! YAML-level spine reconciliation tests (AC-PLN4-03, AC-PLN4-04).
//!
//! Tests reconciliation behavior at the YAML/spine level:
//! - Items present in YAML but not in DB → imported
//! - Items present in DB but not in YAML → NOT deleted (additive reconciliation)
//! - Items in both but spine metadata differs → backfill (if NULL) or conflict (if populated)
//! - Dependency edges reconciled correctly

use sddk_storage::spine_import::import_spine;
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

fn make_item(id: &str, status: &str, order: u32, horizon: &str, depends_on: &[&str]) -> String {
    let deps = depends_on
        .iter()
        .map(|d| format!("\"{}\"", d))
        .collect::<Vec<_>>()
        .join(", ");
    format!(
        r#"  - order: {}
    id: {}
    horizon: {}
    status: {}
    depends_on: [{}]
    objective: Obj for {}
    exit_gate: Gate-{}"#,
        order, id, horizon, status, deps, id, id
    )
}

// ── AC-PLN4-03: additive reconciliation ─────────────────────────────────────────

/// Scenario: YAML has more items than DB → new ones imported.
#[test]
fn reconcile_yaml_has_more_items() {
    let tmp = TempDir::new().unwrap();
    let db_path = tmp.path().join("ledger.sqlite");
    let mut storage = Storage::open(&db_path).unwrap();

    // Import WI-001 only
    let bytes1 = make_spine_yaml(&make_item("WI-001", "PROPOSED", 1, "H1", &[]));
    let r1 = import_spine(&bytes1, &mut storage).expect("first import must succeed");
    assert_eq!(r1.imported, 1);

    // Re-import with WI-001 + WI-002
    let bytes2 = make_spine_yaml(&format!(
        "{}\n{}",
        make_item("WI-001", "PROPOSED", 1, "H1", &[]),
        make_item("WI-002", "ACTIVE", 2, "H1", &[])
    ));
    let r2 = import_spine(&bytes2, &mut storage).expect("second import must succeed");
    assert_eq!(r2.imported, 1, "WI-002 is new");
    assert_eq!(r2.already_present, 1, "WI-001 already present");
}

/// Scenario: YAML has fewer items than DB → DB items persist (additive, not subtractive).
#[test]
fn reconcile_yaml_has_fewer_items() {
    let tmp = TempDir::new().unwrap();
    let db_path = tmp.path().join("ledger.sqlite");
    let mut storage = Storage::open(&db_path).unwrap();

    // Import WI-001 and WI-002
    let bytes1 = make_spine_yaml(&format!(
        "{}\n{}",
        make_item("WI-001", "PROPOSED", 1, "H1", &[]),
        make_item("WI-002", "ACTIVE", 2, "H1", &[])
    ));
    import_spine(&bytes1, &mut storage).expect("first import must succeed");

    // Re-import with WI-001 only
    let bytes2 = make_spine_yaml(&make_item("WI-001", "PROPOSED", 1, "H1", &[]));
    let r2 = import_spine(&bytes2, &mut storage).expect("second import must succeed");
    assert_eq!(r2.imported, 0);
    assert_eq!(r2.already_present, 1, "WI-001 already present");

    // WI-002 must still exist in DB
    let conn = rusqlite::Connection::open(&db_path).unwrap();
    let count: i64 = conn
        .query_row(
            "SELECT COUNT(*) FROM work_items_v1 WHERE id = 'WI-002'",
            [],
            |row| row.get(0),
        )
        .expect("WI-002 must still exist");
    assert_eq!(count, 1, "additive reconciliation: removed from YAML ≠ deleted from DB");
}

/// Scenario: YAML unchanged → already_present (no new imports).
#[test]
fn reconcile_yaml_unchanged_already_present() {
    let tmp = TempDir::new().unwrap();
    let db_path = tmp.path().join("ledger.sqlite");
    let mut storage = Storage::open(&db_path).unwrap();

    let bytes = make_spine_yaml(&make_item("WI-001", "PROPOSED", 1, "H1", &[]));

    let r1 = import_spine(&bytes, &mut storage).expect("first import");
    assert_eq!(r1.imported, 1);

    let r2 = import_spine(&bytes, &mut storage).expect("second import");
    assert_eq!(r2.imported, 0, "no new imports");
    assert_eq!(r2.already_present, 1, "WI-001 already present");
}

// ── AC-PLN4-04: spine metadata reconciliation ─────────────────────────────────

/// Scenario: spine horizon change on re-import: backfill (NULL → populated).
#[test]
fn reconcile_horizon_backfill_from_null() {
    let tmp = TempDir::new().unwrap();
    let db_path = tmp.path().join("ledger.sqlite");
    let mut storage = Storage::open(&db_path).unwrap();

    // Manually insert with NULL horizon (pre-MIGRATION_16 state)
    {
        let conn = rusqlite::Connection::open(&db_path).unwrap();
        conn.execute(
            r#"INSERT INTO projects (project_id, display_name, remote_url, scope, created_at)
               VALUES ('__spine_import__', 'Spine Import', NULL, 'spine-import', 1234567890)"#,
            [],
        ).unwrap();
        conn.execute(
            r#"INSERT INTO workspaces (workspace_id, project_id, canonical_path, created_at)
               VALUES ('__spine_import_ws__', '__spine_import__', 'spine-import', 1234567890)"#,
            [],
        ).unwrap();
        conn.execute(
            r#"INSERT INTO cycles (cycle_id, project_id, workspace_id, status, phase, manifest_json, created_at, updated_at)
               VALUES ('WI-001', '__spine_import__', '__spine_import_ws__', 'OPEN', 'build', '{}', 1234567890, 1234567890)"#,
            [],
        ).unwrap();
        conn.execute(
            r#"INSERT INTO work_items_v1
               (id, cycle_id, title, description, status, created_at, schema_version,
                spine_order, spine_horizon, spine_status, exit_gate)
               VALUES ('WI-001', 'WI-001', 'WI-001', 'Obj for WI-001', '"draft"', 1234567890, 1,
                       100, NULL, 'PROPOSED', 'Gate-WI-001')"#,
            [],
        ).unwrap();
    }

    // Import with H1 horizon → backfill NULL horizon
    let bytes = make_spine_yaml(&make_item("WI-001", "PROPOSED", 100, "H1", &[]));
    let r = import_spine(&bytes, &mut storage).expect("import must succeed");
    assert_eq!(r.backfilled, 1, "NULL horizon must be backfilled");

    // Verify
    let conn = rusqlite::Connection::open(&db_path).unwrap();
    let horizon: String = conn
        .query_row(
            "SELECT spine_horizon FROM work_items_v1 WHERE id = 'WI-001'",
            [],
            |row| row.get(0),
        )
        .expect("must exist");
    assert_eq!(horizon.to_lowercase(), "h1");
}

/// Scenario: spine status change on re-import with populated columns → conflict.
#[test]
fn reconcile_status_change_conflict() {
    let tmp = TempDir::new().unwrap();
    let db_path = tmp.path().join("ledger.sqlite");
    let mut storage = Storage::open(&db_path).unwrap();

    // First import with PROPOSED (→ draft)
    let bytes1 = make_spine_yaml(&make_item("WI-001", "PROPOSED", 1, "H1", &[]));
    import_spine(&bytes1, &mut storage).expect("first import must succeed");

    // Re-import with ACTIVE (→ active) — spine_status changes from PROPOSED to ACTIVE
    let bytes2 = make_spine_yaml(&make_item("WI-001", "ACTIVE", 1, "H1", &[]));
    let err = import_spine(&bytes2, &mut storage).expect_err("status change must conflict");
    let err_str = err.to_string();
    assert!(
        err_str.contains("conflict") || err_str.contains("spine_metadata"),
        "must be a spine metadata conflict: {}",
        err_str
    );
}
