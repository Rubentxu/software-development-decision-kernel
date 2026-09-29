//! Regression test: MIGRATION_21 widens the `cycles.status` CHECK to accept
//! `PAUSED` on databases created BEFORE `PAUSED` was added to the DDL.
//!
//! Background. `MIGRATION_1` creates the `cycles` table with
//! `CREATE TABLE IF NOT EXISTS`. Adding `'PAUSED'` to that DDL therefore only
//! ever takes effect for databases created *after* the edit: an already-created
//! table keeps its original CHECK constraint forever, because SQLite cannot
//! ALTER a CHECK constraint and no migration rebuilt the table.
//!
//! Consequence. `sddk cycle pause` on a pre-existing ledger failed with
//! `ENGINE_STORAGE: CHECK constraint failed: status IN ('OPEN', 'BLOCKED',
//! 'REMEDIATING', 'RELEASE_PENDING', 'RELEASED', 'CLOSED', 'ABANDONED',
//! 'RECOVERING')`, i.e. the governance primitive was unusable on exactly the
//! installations that had the most history. The source DDL and the contract in
//! `workflow/workflow.yaml` (`cycle.pause` -> `status: PAUSED`) both declared
//! `PAUSED`; only the live schema disagreed.
//!
//! These tests reproduce that by constructing a legacy-shaped `cycles` table
//! and asserting the migration upgrades it in place, preserving rows.

use rusqlite::Connection;
use sddk_storage::Storage;
use tempfile::TempDir;

/// The legacy CHECK constraint: exactly what MIGRATION_1 shipped before
/// `'PAUSED'` was inserted. Used to build a database that looks like one
/// created by an older binary.
const LEGACY_CYCLES_DDL: &str = r#"
CREATE TABLE IF NOT EXISTS cycles (
    cycle_id TEXT PRIMARY KEY,
    project_id TEXT NOT NULL,
    workspace_id TEXT NOT NULL,
    status TEXT NOT NULL CHECK (status IN (
        'OPEN', 'BLOCKED', 'REMEDIATING', 'RELEASE_PENDING',
        'RELEASED', 'CLOSED', 'ABANDONED', 'RECOVERING'
    )),
    phase TEXT NOT NULL CHECK (phase IN (
        'explore', 'specify', 'design', 'plan', 'build',
        'verify', 'review', 'release', 'archive'
    )),
    manifest_json TEXT NOT NULL,
    created_at TEXT NOT NULL,
    updated_at TEXT NOT NULL,
    UNIQUE (project_id, cycle_id)
);
"#;

/// Schema dependencies of `cycles` that a real ledger at `user_version = 20`
/// already had. `cycles` carries a composite FK into `workspaces`, and several
/// tables carry FKs back into `cycles`, so both sides must exist for the table
/// rebuild in `MIGRATION_21` to be reproducible.
const LEGACY_SUPPORT_DDL: &str = r#"
CREATE TABLE projects (
    project_id TEXT PRIMARY KEY,
    display_name TEXT NOT NULL CHECK (display_name <> ''),
    remote_url TEXT,
    scope TEXT NOT NULL CHECK (scope <> ''),
    created_at TEXT NOT NULL,
    UNIQUE (remote_url, scope)
);

CREATE TABLE workspaces (
    workspace_id TEXT PRIMARY KEY,
    project_id TEXT NOT NULL REFERENCES projects(project_id) ON DELETE RESTRICT,
    canonical_path TEXT NOT NULL CHECK (canonical_path <> ''),
    created_at TEXT NOT NULL,
    UNIQUE (project_id, canonical_path),
    UNIQUE (project_id, workspace_id)
);
"#;

/// The other tables in the real schema that carry a foreign key into
/// `cycles`. `MIGRATION_21` rebuilds the `cycles` table, so every one of these
/// is a potential casualty: a rebuild that drops a referenced name leaves them
/// dangling, and `PRAGMA foreign_key_check` (run by the migration driver before
/// it commits) is what catches it. The list is derived from the production
/// schema, which declares references into `cycles` from `artifacts`,
/// `capability_receipts`, `cycle_events` and `work_items_v1`.
const LEGACY_DEPENDENT_DDL: &str = r#"
CREATE TABLE artifacts (
    artifact_id TEXT PRIMARY KEY,
    project_id TEXT NOT NULL REFERENCES projects(project_id) ON DELETE RESTRICT,
    cycle_id TEXT,
    kind TEXT NOT NULL CHECK (kind <> ''),
    path TEXT NOT NULL CHECK (path <> ''),
    sha256 TEXT,
    producer TEXT,
    created_at TEXT NOT NULL,
    metadata_json TEXT NOT NULL,
    FOREIGN KEY (project_id, cycle_id)
        REFERENCES cycles(project_id, cycle_id) ON DELETE RESTRICT
);

CREATE TABLE capability_receipts (
    receipt_id TEXT PRIMARY KEY,
    project_id TEXT NOT NULL,
    cycle_id TEXT NOT NULL,
    capability TEXT NOT NULL,
    verdict TEXT NOT NULL,
    created_at TEXT NOT NULL,
    metadata_json TEXT NOT NULL,
    FOREIGN KEY (project_id, cycle_id)
        REFERENCES cycles(project_id, cycle_id) ON DELETE RESTRICT
);

CREATE TABLE cycle_events (
    event_id TEXT PRIMARY KEY,
    cycle_id TEXT NOT NULL REFERENCES cycles(cycle_id) ON DELETE RESTRICT
);

CREATE TABLE work_items_v1 (
    item_id TEXT PRIMARY KEY,
    cycle_id TEXT,
    FOREIGN KEY (cycle_id) REFERENCES cycles(cycle_id)
);
"#;

fn legacy_db(dir: &TempDir) -> std::path::PathBuf {
    let db_path = dir.path().join("legacy.sqlite");
    let conn = Connection::open(&db_path).expect("must open legacy DB");
    conn.execute_batch(LEGACY_CYCLES_DDL)
        .expect("legacy DDL must apply");
    conn.execute_batch(LEGACY_SUPPORT_DDL)
        .expect("support DDL must apply");
    conn.execute_batch(LEGACY_DEPENDENT_DDL)
        .expect("dependent DDL must apply");
    // Pin the schema version the older binary would have left behind, so the
    // new migration is the only thing that can explain the widened constraint.
    conn.pragma_update(None, "user_version", 20)
        .expect("must pin user_version");
    conn.execute(
        "INSERT INTO projects (project_id, display_name, scope, created_at)
         VALUES ('p-test', 'test', '.', '2026-01-01')",
        [],
    )
    .expect("project row must insert");
    conn.execute(
        "INSERT INTO workspaces (workspace_id, project_id, canonical_path, created_at)
         VALUES ('w-test', 'p-test', '/tmp/w-test', '2026-01-01')",
        [],
    )
    .expect("workspace row must insert");
    conn.execute(
        "INSERT INTO cycles (cycle_id, project_id, workspace_id, status, phase, manifest_json, created_at, updated_at)
         VALUES ('c-legacy', 'p-test', 'w-test', 'OPEN', 'explore', '{}', '2026-01-01', '2026-01-01')",
        [],
    )
    .expect("legacy row must insert");
    drop(conn);
    db_path
}

/// A database whose `cycles` CHECK predates `PAUSED` must be upgraded by
/// `Storage::open`, which runs the pending migrations.
#[test]
fn migration_21_widens_legacy_cycles_status_to_accept_paused() {
    let tmp = TempDir::new().unwrap();
    let db_path = legacy_db(&tmp);

    // Pre-condition: the legacy table really does reject PAUSED. Without this
    // the test would pass vacuously if the fixture were wrong.
    {
        let conn = Connection::open(&db_path).unwrap();
        let err = conn.execute(
            "UPDATE cycles SET status = 'PAUSED' WHERE cycle_id = 'c-legacy'",
            [],
        );
        assert!(
            err.is_err(),
            "fixture must reproduce the legacy constraint rejecting PAUSED"
        );
    }

    // Opening through Storage runs migrations.
    let _storage = Storage::open(&db_path).expect("must open legacy DB through Storage");

    let conn = Connection::open(&db_path).unwrap();
    conn.execute(
        "UPDATE cycles SET status = 'PAUSED' WHERE cycle_id = 'c-legacy'",
        [],
    )
    .expect("after migration, PAUSED must be accepted");

    let status: String = conn
        .query_row(
            "SELECT status FROM cycles WHERE cycle_id = 'c-legacy'",
            [],
            |r| r.get(0),
        )
        .unwrap();
    assert_eq!(status, "PAUSED");
}

/// The migration must preserve every pre-existing row, and must not disturb
/// the other statuses that were already legal.
#[test]
fn migration_21_preserves_existing_rows_and_statuses() {
    let tmp = TempDir::new().unwrap();
    let db_path = tmp.path().join("legacy.sqlite");
    let conn = Connection::open(&db_path).unwrap();
    conn.execute_batch(LEGACY_CYCLES_DDL).unwrap();
    conn.execute_batch(LEGACY_SUPPORT_DDL).unwrap();
    conn.execute_batch(LEGACY_DEPENDENT_DDL).unwrap();
    conn.pragma_update(None, "user_version", 20).unwrap();
    conn.execute(
        "INSERT INTO projects (project_id, display_name, scope, created_at)
         VALUES ('p-test', 'test', '.', '2026-01-01')",
        [],
    )
    .unwrap();
    conn.execute(
        "INSERT INTO workspaces (workspace_id, project_id, canonical_path, created_at)
         VALUES ('w-test', 'p-test', '/tmp/w-test', '2026-01-01')",
        [],
    )
    .unwrap();
    for (i, status) in ["OPEN", "BLOCKED", "RELEASED", "CLOSED"].iter().enumerate() {
        conn.execute(
            "INSERT INTO cycles (cycle_id, project_id, workspace_id, status, phase, manifest_json, created_at, updated_at)
             VALUES (?1, 'p-test', 'w-test', ?2, 'explore', '{}', '2026-01-01', '2026-01-01')",
            rusqlite::params![format!("c-{i}"), status],
        )
        .unwrap();
    }
    drop(conn);

    let _storage = Storage::open(&db_path).expect("must open legacy DB through Storage");

    let conn = Connection::open(&db_path).unwrap();
    let mut stmt = conn
        .prepare("SELECT cycle_id, status FROM cycles ORDER BY cycle_id")
        .unwrap();
    let rows: Vec<(String, String)> = stmt
        .query_map([], |r| Ok((r.get(0)?, r.get(1)?)))
        .unwrap()
        .map(Result::unwrap)
        .collect();

    assert_eq!(
        rows,
        vec![
            ("c-0".to_string(), "OPEN".to_string()),
            ("c-1".to_string(), "BLOCKED".to_string()),
            ("c-2".to_string(), "RELEASED".to_string()),
            ("c-3".to_string(), "CLOSED".to_string()),
        ],
        "migration must preserve every legacy row and its status"
    );
}

/// The rebuild must not silently re-point `artifacts`' foreign key at the
/// temporary table name.
///
/// This is the subtle failure mode of the SQLite table-rebuild pattern. Renaming
/// the *original* table (`ALTER TABLE cycles RENAME TO cycles_old`) makes
/// SQLite rewrite the referencing FK clause inside `artifacts` so it names the
/// temporary table (this is the default behaviour on SQLite >= 3.25).
/// Dropping that table then leaves `artifacts` pointing at a table that no
/// longer exists, and the next artifact insert fails at runtime with a
/// confusing error far from the migration.
#[test]
fn migration_21_does_not_repoint_artifacts_foreign_key() {
    let tmp = TempDir::new().unwrap();
    let db_path = legacy_db(&tmp);

    // Pre-condition: the legacy DB has a working artifacts -> cycles FK.
    {
        let conn = Connection::open(&db_path).unwrap();
        conn.execute(
            "INSERT INTO artifacts (artifact_id, project_id, cycle_id, kind, path, created_at, metadata_json)
             VALUES ('art-1', 'p-test', 'c-legacy', 'exploration', 'reports/a.md', '2026-01-01', '{}')",
            [],
        )
        .expect("legacy artifact must insert against the legacy FK");
    }

    let _storage = Storage::open(&db_path).expect("must open legacy DB through Storage");

    let conn = Connection::open(&db_path).unwrap();
    conn.pragma_update(None, "foreign_keys", "ON")
        .expect("must enable FK enforcement");

    // The FK must still resolve to the recreated `cycles` table.
    conn.execute(
        "INSERT INTO artifacts (artifact_id, project_id, cycle_id, kind, path, created_at, metadata_json)
         VALUES ('art-2', 'p-test', 'c-legacy', 'exploration', 'reports/b.md', '2026-01-01', '{}')",
        [],
    )
    .expect("artifacts FK must still resolve to cycles after the rebuild");

    // An artifact pointing at a nonexistent cycle must still be REJECTED.
    let err = conn.execute(
        "INSERT INTO artifacts (artifact_id, project_id, cycle_id, kind, path, created_at, metadata_json)
         VALUES ('art-3', 'p-test', 'c-absent', 'exploration', 'reports/c.md', '2026-01-01', '{}')",
        [],
    );
    assert!(
        err.is_err(),
        "artifacts FK must remain enforced, not dangling after the rebuild"
    );

    // And no temporary table must survive the migration.
    let leftover: i64 = conn
        .query_row(
            "SELECT COUNT(*) FROM sqlite_master WHERE type = 'table' AND name = 'cycles_m21_widened'",
            [],
            |r| r.get(0),
        )
        .unwrap();
    assert_eq!(
        leftover, 0,
        "temporary table must not survive the migration"
    );
}

/// A database created fresh by the current binary must also accept `PAUSED`.
///
/// This pins that the DDL in `MIGRATION_1` and `MIGRATION_21` agree, so a
/// future edit that keeps only one of them is caught. It goes through the real
/// `Storage::open` path and seeds the `workspaces` row the composite foreign
/// key `(project_id, workspace_id)` requires.
#[test]
fn fresh_database_accepts_paused() {
    let tmp = TempDir::new().unwrap();
    let db_path = tmp.path().join("fresh.sqlite");
    let storage = Storage::open(&db_path).expect("must open fresh DB");

    let conn = storage.connection_for_tests();
    conn.execute(
        "INSERT INTO projects (project_id, display_name, scope, created_at)
         VALUES ('p-fresh', 'fresh', '.', '2026-01-01')",
        [],
    )
    .expect("project row must insert");
    conn.execute(
        "INSERT INTO workspaces (workspace_id, project_id, canonical_path, created_at)
         VALUES ('w-fresh', 'p-fresh', '/tmp/w-fresh', '2026-01-01')",
        [],
    )
    .expect("workspace row must insert");
    conn.execute(
        "INSERT INTO cycles (cycle_id, project_id, workspace_id, status, phase, manifest_json, created_at, updated_at)
         VALUES ('c-fresh', 'p-fresh', 'w-fresh', 'PAUSED', 'explore', '{}', '2026-01-01', '2026-01-01')",
        [],
    )
    .expect("a fresh database must accept PAUSED directly");
}
