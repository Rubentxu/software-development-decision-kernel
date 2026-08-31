//! Tests for MIGRATION_13 append-only triggers on workflow_runs_v1 and node_runs_v1.
//!
//! These triggers ensure that once a workflow_run or node_run row reaches a
//! terminal state, it cannot be UPDATEd or DELETEd — preserving audit integrity.

use rusqlite::Connection;
use tempfile::TempDir;

fn run_migrations(conn: &mut Connection) {
    // Inline minimal migrations to set up the schema up to MIGRATION_11
    // (we only test the MIGRATION_13 triggers, not the full migration suite)
    conn.execute_batch(
        r#"
        CREATE TABLE IF NOT EXISTS workflow_runs_v1 (
            run_id            TEXT NOT NULL PRIMARY KEY,
            template_id       TEXT NOT NULL,
            template_version  TEXT NOT NULL,
            ir_hash           TEXT NOT NULL,
            graph_revision_id TEXT NOT NULL,
            state             TEXT NOT NULL
                              CHECK (state IN ('pending','running','paused','completed','failed','cancelled')),
            inputs_json       TEXT NOT NULL,
            outputs_json      TEXT,
            correlation_id    TEXT,
            budget_json       TEXT NOT NULL,
            created_at        TEXT NOT NULL,
            updated_at        TEXT NOT NULL
        );

        CREATE TABLE IF NOT EXISTS node_runs_v1 (
            run_id          TEXT NOT NULL,
            node_id         TEXT NOT NULL,
            state           TEXT NOT NULL
                            CHECK (state IN ('pending','ready','running','completed','failed','skipped')),
            dependencies_json TEXT NOT NULL,
            last_attempt_id TEXT,
            PRIMARY KEY (run_id, node_id),
            FOREIGN KEY (run_id) REFERENCES workflow_runs_v1(run_id) ON DELETE CASCADE
        );
        "#,
    )
    .unwrap();
}

/// Verifies that the append-only trigger on workflow_runs_v1 rejects UPDATE.
#[test]
fn update_on_workflow_runs_v1_fails() {
    let tmp = TempDir::new().unwrap();
    let db_path = tmp.path().join("test.db");
    let mut conn = Connection::open(&db_path).unwrap();

    run_migrations(&mut conn);

    // Apply MIGRATION_13 inline (simulate what the real migration does)
    let migration_13 = r#"
        CREATE TRIGGER IF NOT EXISTS workflow_runs_v1_no_update
            BEFORE UPDATE ON workflow_runs_v1
        BEGIN
            SELECT RAISE(ABORT, 'workflow_runs_v1 is append-only');
        END;
        CREATE TRIGGER IF NOT EXISTS workflow_runs_v1_no_delete
            BEFORE DELETE ON workflow_runs_v1
        BEGIN
            SELECT RAISE(ABORT, 'workflow_runs_v1 is append-only');
        END;
    "#;
    conn.execute_batch(migration_13).unwrap();

    // Insert a row
    conn.execute(
        "INSERT INTO workflow_runs_v1 (run_id, template_id, template_version, ir_hash,
         graph_revision_id, state, inputs_json, budget_json, created_at, updated_at)
         VALUES ('run-1', 'tmpl-1', 'v1', 'sha256:abc', 'rev-1', 'completed',
                 '{}', '{}', '2026-08-23T00:00:00Z', '2026-08-23T00:00:00Z')",
        [],
    )
    .unwrap();

    // Attempt to UPDATE → must fail with SqliteError
    let result = conn.execute(
        "UPDATE workflow_runs_v1 SET state='failed' WHERE run_id='run-1'",
        [],
    );
    assert!(
        result.is_err(),
        "UPDATE on workflow_runs_v1 should be rejected by trigger"
    );
}

/// Verifies that the append-only trigger on workflow_runs_v1 rejects DELETE.
#[test]
fn delete_on_workflow_runs_v1_fails() {
    let tmp = TempDir::new().unwrap();
    let db_path = tmp.path().join("test.db");
    let mut conn = Connection::open(&db_path).unwrap();

    run_migrations(&mut conn);

    let migration_13 = r#"
        CREATE TRIGGER IF NOT EXISTS workflow_runs_v1_no_update
            BEFORE UPDATE ON workflow_runs_v1
        BEGIN
            SELECT RAISE(ABORT, 'workflow_runs_v1 is append-only');
        END;
        CREATE TRIGGER IF NOT EXISTS workflow_runs_v1_no_delete
            BEFORE DELETE ON workflow_runs_v1
        BEGIN
            SELECT RAISE(ABORT, 'workflow_runs_v1 is append-only');
        END;
    "#;
    conn.execute_batch(migration_13).unwrap();

    conn.execute(
        "INSERT INTO workflow_runs_v1 (run_id, template_id, template_version, ir_hash,
         graph_revision_id, state, inputs_json, budget_json, created_at, updated_at)
         VALUES ('run-1', 'tmpl-1', 'v1', 'sha256:abc', 'rev-1', 'completed',
                 '{}', '{}', '2026-08-23T00:00:00Z', '2026-08-23T00:00:00Z')",
        [],
    )
    .unwrap();

    let result = conn.execute("DELETE FROM workflow_runs_v1 WHERE run_id='run-1'", []);
    assert!(
        result.is_err(),
        "DELETE on workflow_runs_v1 should be rejected by trigger"
    );
}

/// Verifies that the append-only trigger on node_runs_v1 rejects UPDATE.
#[test]
fn update_on_node_runs_v1_fails() {
    let tmp = TempDir::new().unwrap();
    let db_path = tmp.path().join("test.db");
    let mut conn = Connection::open(&db_path).unwrap();

    run_migrations(&mut conn);

    let migration_13 = r#"
        CREATE TRIGGER IF NOT EXISTS node_runs_v1_no_update
            BEFORE UPDATE ON node_runs_v1
        BEGIN
            SELECT RAISE(ABORT, 'node_runs_v1 is append-only');
        END;
        CREATE TRIGGER IF NOT EXISTS node_runs_v1_no_delete
            BEFORE DELETE ON node_runs_v1
        BEGIN
            SELECT RAISE(ABORT, 'node_runs_v1 is append-only');
        END;
    "#;
    conn.execute_batch(migration_13).unwrap();

    // Insert parent workflow run first
    conn.execute(
        "INSERT INTO workflow_runs_v1 (run_id, template_id, template_version, ir_hash,
         graph_revision_id, state, inputs_json, budget_json, created_at, updated_at)
         VALUES ('run-1', 'tmpl-1', 'v1', 'sha256:abc', 'rev-1', 'running',
                 '{}', '{}', '2026-08-23T00:00:00Z', '2026-08-23T00:00:00Z')",
        [],
    )
    .unwrap();

    // Insert node run
    conn.execute(
        "INSERT INTO node_runs_v1 (run_id, node_id, state, dependencies_json)
         VALUES ('run-1', 'node-1', 'completed', '[]')",
        [],
    )
    .unwrap();

    let result = conn.execute(
        "UPDATE node_runs_v1 SET state='failed' WHERE run_id='run-1' AND node_id='node-1'",
        [],
    );
    assert!(
        result.is_err(),
        "UPDATE on node_runs_v1 should be rejected by trigger"
    );
}

/// Verifies that the append-only trigger on node_runs_v1 rejects DELETE.
#[test]
fn delete_on_node_runs_v1_fails() {
    let tmp = TempDir::new().unwrap();
    let db_path = tmp.path().join("test.db");
    let mut conn = Connection::open(&db_path).unwrap();

    run_migrations(&mut conn);

    let migration_13 = r#"
        CREATE TRIGGER IF NOT EXISTS node_runs_v1_no_update
            BEFORE UPDATE ON node_runs_v1
        BEGIN
            SELECT RAISE(ABORT, 'node_runs_v1 is append-only');
        END;
        CREATE TRIGGER IF NOT EXISTS node_runs_v1_no_delete
            BEFORE DELETE ON node_runs_v1
        BEGIN
            SELECT RAISE(ABORT, 'node_runs_v1 is append-only');
        END;
    "#;
    conn.execute_batch(migration_13).unwrap();

    conn.execute(
        "INSERT INTO workflow_runs_v1 (run_id, template_id, template_version, ir_hash,
         graph_revision_id, state, inputs_json, budget_json, created_at, updated_at)
         VALUES ('run-1', 'tmpl-1', 'v1', 'sha256:abc', 'rev-1', 'running',
                 '{}', '{}', '2026-08-23T00:00:00Z', '2026-08-23T00:00:00Z')",
        [],
    )
    .unwrap();

    conn.execute(
        "INSERT INTO node_runs_v1 (run_id, node_id, state, dependencies_json)
         VALUES ('run-1', 'node-1', 'completed', '[]')",
        [],
    )
    .unwrap();

    let result = conn.execute(
        "DELETE FROM node_runs_v1 WHERE run_id='run-1' AND node_id='node-1'",
        [],
    );
    assert!(
        result.is_err(),
        "DELETE on node_runs_v1 should be rejected by trigger"
    );
}
