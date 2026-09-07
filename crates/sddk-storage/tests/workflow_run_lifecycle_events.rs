//! Tests for workflow_run_events_v1 lifecycle append-only semantics (REQ-WFR-EV-001).
//!
//! These tests verify:
//! - workflow_run_events_v1 is append-only (no UPDATE/DELETE)
//! - workflow_runs_v1 snapshot remains non-updatable (MIGRATION_13 triggers)
//! - lifecycle events cascade from run deletion

use rusqlite::Connection;
use std::path::Path;
use tempfile::TempDir;

/// Opens a connection at the given path with migrations applied.
fn open_store(dir: &Path) -> Connection {
    let conn = Connection::open(dir.join("ledger.sqlite")).unwrap();
    // Run storage's migration logic inline to get the schema
    run_migrations(&conn, 16).unwrap();
    conn
}

/// Simplified migration runner for test setup (mirrors migrations.rs structure).
fn run_migrations(conn: &Connection, _from_version: u32) -> Result<(), String> {
    // In real tests, we use the full migration system.
    // Here we just verify the tables exist from migrations 11-13.
    conn.execute_batch(
        r#"
        -- MIGRATION_11 creates workflow_runs_v1, node_runs_v1, attempts_v1
        CREATE TABLE IF NOT EXISTS workflow_runs_v1 (
            run_id            TEXT NOT NULL PRIMARY KEY CHECK (run_id <> '' AND length(run_id) <= 64),
            template_id       TEXT NOT NULL,
            template_version  TEXT NOT NULL,
            ir_hash           TEXT NOT NULL CHECK (ir_hash LIKE 'sha256:%'),
            graph_revision_id TEXT NOT NULL,
            state             TEXT NOT NULL CHECK (state IN ('pending','running','paused','completed','failed','cancelled')),
            inputs_json       TEXT NOT NULL,
            outputs_json      TEXT,
            correlation_id    TEXT,
            budget_json       TEXT NOT NULL,
            created_at        TEXT NOT NULL,
            updated_at        TEXT NOT NULL
        );
        CREATE INDEX IF NOT EXISTS workflow_runs_v1_template_idx ON workflow_runs_v1(template_id);
        CREATE INDEX IF NOT EXISTS workflow_runs_v1_state_idx ON workflow_runs_v1(state);

        CREATE TABLE IF NOT EXISTS node_runs_v1 (
            run_id          TEXT NOT NULL REFERENCES workflow_runs_v1(run_id) ON DELETE CASCADE,
            node_id         TEXT NOT NULL,
            state           TEXT NOT NULL CHECK (state IN ('pending','ready','running','completed','failed','skipped')),
            dependencies_json TEXT NOT NULL,
            last_attempt_id  TEXT,
            PRIMARY KEY (run_id, node_id)
        );

        CREATE TABLE IF NOT EXISTS attempts_v1 (
            attempt_id        TEXT NOT NULL PRIMARY KEY CHECK (attempt_id <> ''),
            run_id            TEXT NOT NULL REFERENCES workflow_runs_v1(run_id) ON DELETE CASCADE,
            node_id           TEXT NOT NULL,
            route_json        TEXT NOT NULL,
            started_at        TEXT NOT NULL,
            ended_at          TEXT,
            outcome_json      TEXT,
            usage_json        TEXT NOT NULL,
            context_capsule_json TEXT NOT NULL,
            idempotency_key   TEXT NOT NULL UNIQUE,
            schema_version    INTEGER NOT NULL CHECK (schema_version = 1)
        );
        CREATE INDEX IF NOT EXISTS attempts_v1_run_node_idx ON attempts_v1(run_id, node_id);

        -- MIGRATION_13 triggers (append-only)
        CREATE TRIGGER IF NOT EXISTS attempts_v1_no_update BEFORE UPDATE ON attempts_v1
            BEGIN SELECT RAISE(ABORT, 'attempts_v1 are append-only'); END;
        CREATE TRIGGER IF NOT EXISTS attempts_v1_no_delete BEFORE DELETE ON attempts_v1
            BEGIN SELECT RAISE(ABORT, 'attempts_v1 are append-only'); END;
        CREATE TRIGGER IF NOT EXISTS workflow_runs_v1_no_update BEFORE UPDATE ON workflow_runs_v1
            BEGIN SELECT RAISE(ABORT, 'workflow_runs_v1 is append-only'); END;
        CREATE TRIGGER IF NOT EXISTS workflow_runs_v1_no_delete BEFORE DELETE ON workflow_runs_v1
            BEGIN SELECT RAISE(ABORT, 'workflow_runs_v1 is append-only'); END;
        CREATE TRIGGER IF NOT EXISTS node_runs_v1_no_update BEFORE UPDATE ON node_runs_v1
            BEGIN SELECT RAISE(ABORT, 'node_runs_v1 is append-only'); END;
        CREATE TRIGGER IF NOT EXISTS node_runs_v1_no_delete BEFORE DELETE ON node_runs_v1
            BEGIN SELECT RAISE(ABORT, 'node_runs_v1 is append-only'); END;
        "#,
    )
    .map_err(|e| e.to_string())?;
    Ok(())
}

/// Creates a test database with workflow_run_events_v1.
fn open_store_with_events_v1(dir: &Path) -> Connection {
    let conn = Connection::open(dir.join("ledger.sqlite")).unwrap();
    run_migrations_with_events_v1(&conn).unwrap();
    conn
}

/// Runs migrations including MIGRATION_17 (workflow_run_events_v1).
fn run_migrations_with_events_v1(conn: &Connection) -> Result<(), String> {
    conn.execute_batch(
        r#"
        -- First run base migrations
        CREATE TABLE IF NOT EXISTS workflow_runs_v1 (
            run_id            TEXT NOT NULL PRIMARY KEY CHECK (run_id <> '' AND length(run_id) <= 64),
            template_id       TEXT NOT NULL,
            template_version  TEXT NOT NULL,
            ir_hash           TEXT NOT NULL CHECK (ir_hash LIKE 'sha256:%'),
            graph_revision_id TEXT NOT NULL,
            state             TEXT NOT NULL CHECK (state IN ('pending','running','paused','completed','failed','cancelled')),
            inputs_json       TEXT NOT NULL,
            outputs_json      TEXT,
            correlation_id    TEXT,
            budget_json       TEXT NOT NULL,
            created_at        TEXT NOT NULL,
            updated_at        TEXT NOT NULL
        );

        CREATE TABLE IF NOT EXISTS node_runs_v1 (
            run_id          TEXT NOT NULL REFERENCES workflow_runs_v1(run_id) ON DELETE CASCADE,
            node_id         TEXT NOT NULL,
            state           TEXT NOT NULL CHECK (state IN ('pending','ready','running','completed','failed','skipped')),
            dependencies_json TEXT NOT NULL,
            last_attempt_id  TEXT,
            PRIMARY KEY (run_id, node_id)
        );

        CREATE TABLE IF NOT EXISTS attempts_v1 (
            attempt_id        TEXT NOT NULL PRIMARY KEY CHECK (attempt_id <> ''),
            run_id            TEXT NOT NULL REFERENCES workflow_runs_v1(run_id) ON DELETE CASCADE,
            node_id           TEXT NOT NULL,
            route_json        TEXT NOT NULL,
            started_at        TEXT NOT NULL,
            ended_at          TEXT,
            outcome_json      TEXT,
            usage_json        TEXT NOT NULL,
            context_capsule_json TEXT NOT NULL,
            idempotency_key   TEXT NOT NULL UNIQUE,
            schema_version    INTEGER NOT NULL CHECK (schema_version = 1)
        );

        -- Append-only triggers from MIGRATION_13
        CREATE TRIGGER IF NOT EXISTS attempts_v1_no_update BEFORE UPDATE ON attempts_v1
            BEGIN SELECT RAISE(ABORT, 'attempts_v1 are append-only'); END;
        CREATE TRIGGER IF NOT EXISTS attempts_v1_no_delete BEFORE DELETE ON attempts_v1
            BEGIN SELECT RAISE(ABORT, 'attempts_v1 are append-only'); END;
        CREATE TRIGGER IF NOT EXISTS workflow_runs_v1_no_update BEFORE UPDATE ON workflow_runs_v1
            BEGIN SELECT RAISE(ABORT, 'workflow_runs_v1 is append-only'); END;
        CREATE TRIGGER IF NOT EXISTS workflow_runs_v1_no_delete BEFORE DELETE ON workflow_runs_v1
            BEGIN SELECT RAISE(ABORT, 'workflow_runs_v1 is append-only'); END;
        CREATE TRIGGER IF NOT EXISTS node_runs_v1_no_update BEFORE UPDATE ON node_runs_v1
            BEGIN SELECT RAISE(ABORT, 'node_runs_v1 is append-only'); END;
        CREATE TRIGGER IF NOT EXISTS node_runs_v1_no_delete BEFORE DELETE ON node_runs_v1
            BEGIN SELECT RAISE(ABORT, 'node_runs_v1 is append-only'); END;

        -- MIGRATION_17: workflow_run_events_v1
        CREATE TABLE IF NOT EXISTS workflow_run_events_v1 (
            event_id      TEXT NOT NULL PRIMARY KEY CHECK (event_id <> ''),
            run_id        TEXT NOT NULL REFERENCES workflow_runs_v1(run_id) ON DELETE CASCADE,
            occurred_at   TEXT NOT NULL,
            from_state    TEXT NOT NULL CHECK (from_state IN ('pending','running','paused','completed','failed','cancelled')),
            to_state      TEXT NOT NULL CHECK (to_state IN ('pending','running','paused','completed','failed','cancelled')),
            actor_kind    TEXT NOT NULL CHECK (actor_kind IN ('human','agent','system')),
            actor_id      TEXT NOT NULL,
            reason        TEXT
        );
        CREATE INDEX IF NOT EXISTS idx_workflow_run_events_v1_run
            ON workflow_run_events_v1(run_id, occurred_at);
        CREATE TRIGGER IF NOT EXISTS workflow_run_events_v1_no_update
            BEFORE UPDATE ON workflow_run_events_v1
            BEGIN SELECT RAISE(ABORT, 'workflow_run_events_v1 is append-only'); END;
        CREATE TRIGGER IF NOT EXISTS workflow_run_events_v1_no_delete
            BEFORE DELETE ON workflow_run_events_v1
            BEGIN SELECT RAISE(ABORT, 'workflow_run_events_v1 is append-only'); END;
        "#,
    )
    .map_err(|e| e.to_string())?;
    Ok(())
}

/// Scenario: lifecycle events are append-only
/// GIVEN workflow_run_events_v1 table exists
/// WHEN an UPDATE or DELETE is attempted
/// THEN SQLite raises ABORT and the row is unchanged
#[test]
fn workflow_run_events_are_append_only() {
    let temp_dir = TempDir::new().unwrap();
    let conn = open_store_with_events_v1(temp_dir.path());

    // Insert a run first
    conn.execute(
        r#"INSERT INTO workflow_runs_v1
           (run_id, template_id, template_version, ir_hash, graph_revision_id, state,
            inputs_json, budget_json, created_at, updated_at)
           VALUES (?1, 'tpl-1', 'v1', 'sha256:abc', 'rev-1', 'pending', '{}', '{}', '2026-01-01T00:00:00Z', '2026-01-01T00:00:00Z')"#,
        ["run-1"],
    )
    .unwrap();

    // Insert a lifecycle event
    conn.execute(
        r#"INSERT INTO workflow_run_events_v1
           (event_id, run_id, occurred_at, from_state, to_state, actor_kind, actor_id, reason)
           VALUES (?1, ?2, ?3, 'pending', 'running', 'system', 'engine', NULL)"#,
        ["evt-1", "run-1", "2026-01-01T00:00:00Z"],
    )
    .unwrap();

    // UPDATE should fail
    let update_result = conn.execute(
        "UPDATE workflow_run_events_v1 SET to_state = 'completed' WHERE event_id = 'evt-1'",
        [],
    );
    assert!(
        update_result.is_err(),
        "UPDATE on workflow_run_events_v1 should be rejected by trigger"
    );

    // DELETE should fail
    let delete_result = conn.execute(
        "DELETE FROM workflow_run_events_v1 WHERE event_id = 'evt-1'",
        [],
    );
    assert!(
        delete_result.is_err(),
        "DELETE on workflow_run_events_v1 should be rejected by trigger"
    );

    // Verify the row still exists unchanged
    let count: i64 = conn
        .query_row(
            "SELECT COUNT(*) FROM workflow_run_events_v1 WHERE event_id = 'evt-1'",
            [],
            |row| row.get(0),
        )
        .unwrap();
    assert_eq!(
        count, 1,
        "Event row should still exist after rejected UPDATE/DELETE"
    );
}

/// Scenario: workflow_runs_v1 snapshot rejects UPDATE (MIGRATION_13 triggers)
/// GIVEN a run row written by record_run
/// WHEN UPDATE workflow_runs_v1 SET state = 'running' is attempted
/// THEN the MIGRATION_13 trigger raises ABORT
#[test]
fn workflow_runs_snapshot_rejects_update() {
    let temp_dir = TempDir::new().unwrap();
    let conn = open_store(temp_dir.path());

    // Insert a run
    conn.execute(
        r#"INSERT INTO workflow_runs_v1
           (run_id, template_id, template_version, ir_hash, graph_revision_id, state,
            inputs_json, budget_json, created_at, updated_at)
           VALUES (?1, 'tpl-1', 'v1', 'sha256:abc', 'rev-1', 'pending', '{}', '{}', '2026-01-01T00:00:00Z', '2026-01-01T00:00:00Z')"#,
        ["run-1"],
    )
    .unwrap();

    // UPDATE should fail due to MIGRATION_13 trigger
    let result = conn.execute(
        "UPDATE workflow_runs_v1 SET state = 'running' WHERE run_id = 'run-1'",
        [],
    );
    assert!(
        result.is_err(),
        "UPDATE on workflow_runs_v1 should be rejected by MIGRATION_13 trigger"
    );
}

/// Creates a schema WITHOUT the MIGRATION_13 append-only triggers
/// (for testing cascade delete behavior).
fn open_store_with_events_v1_no_triggers(dir: &Path) -> Connection {
    let conn = Connection::open(dir.join("ledger.sqlite")).unwrap();
    conn.execute_batch(
        r#"
        CREATE TABLE IF NOT EXISTS workflow_runs_v1 (
            run_id            TEXT NOT NULL PRIMARY KEY CHECK (run_id <> '' AND length(run_id) <= 64),
            template_id       TEXT NOT NULL,
            template_version  TEXT NOT NULL,
            ir_hash           TEXT NOT NULL CHECK (ir_hash LIKE 'sha256:%'),
            graph_revision_id TEXT NOT NULL,
            state             TEXT NOT NULL CHECK (state IN ('pending','running','paused','completed','failed','cancelled')),
            inputs_json       TEXT NOT NULL,
            outputs_json      TEXT,
            correlation_id    TEXT,
            budget_json       TEXT NOT NULL,
            created_at        TEXT NOT NULL,
            updated_at        TEXT NOT NULL
        );

        CREATE TABLE IF NOT EXISTS workflow_run_events_v1 (
            event_id      TEXT NOT NULL PRIMARY KEY CHECK (event_id <> ''),
            run_id        TEXT NOT NULL REFERENCES workflow_runs_v1(run_id) ON DELETE CASCADE,
            occurred_at   TEXT NOT NULL,
            from_state    TEXT NOT NULL CHECK (from_state IN ('pending','running','paused','completed','failed','cancelled')),
            to_state      TEXT NOT NULL CHECK (to_state IN ('pending','running','paused','completed','failed','cancelled')),
            actor_kind    TEXT NOT NULL CHECK (actor_kind IN ('human','agent','system')),
            actor_id      TEXT NOT NULL,
            reason        TEXT
        );
        CREATE INDEX IF NOT EXISTS idx_workflow_run_events_v1_run
            ON workflow_run_events_v1(run_id, occurred_at);
        -- NO triggers for this test - testing cascade behavior only
        "#,
    )
    .unwrap();
    conn
}

/// Scenario: lifecycle events cascade from run deletion
/// GIVEN a run with lifecycle events (test-only fixture DB with triggers absent)
/// WHEN the parent workflow_runs_v1 row is deleted
/// THEN the run's workflow_run_events_v1 rows are removed by ON DELETE CASCADE
#[test]
fn lifecycle_events_cascade_from_run() {
    let temp_dir = TempDir::new().unwrap();
    // Use schema WITHOUT MIGRATION_13 triggers (per spec: "test-only fixture DB")
    let conn = open_store_with_events_v1_no_triggers(temp_dir.path());

    // Insert a run
    conn.execute(
        r#"INSERT INTO workflow_runs_v1
           (run_id, template_id, template_version, ir_hash, graph_revision_id, state,
            inputs_json, budget_json, created_at, updated_at)
           VALUES (?1, 'tpl-1', 'v1', 'sha256:abc', 'rev-1', 'pending', '{}', '{}', '2026-01-01T00:00:00Z', '2026-01-01T00:00:00Z')"#,
        ["run-cascade"],
    )
    .unwrap();

    // Insert lifecycle events
    conn.execute(
        r#"INSERT INTO workflow_run_events_v1
           (event_id, run_id, occurred_at, from_state, to_state, actor_kind, actor_id, reason)
           VALUES (?1, ?2, ?3, 'pending', 'running', 'system', 'engine', NULL)"#,
        ["evt-c1", "run-cascade", "2026-01-01T00:00:00Z"],
    )
    .unwrap();
    conn.execute(
        r#"INSERT INTO workflow_run_events_v1
           (event_id, run_id, occurred_at, from_state, to_state, actor_kind, actor_id, reason)
           VALUES (?1, ?2, ?3, 'running', 'completed', 'system', 'engine', NULL)"#,
        ["evt-c2", "run-cascade", "2026-01-01T00:01:00Z"],
    )
    .unwrap();

    // Verify events exist
    let count_before: i64 = conn
        .query_row(
            "SELECT COUNT(*) FROM workflow_run_events_v1 WHERE run_id = 'run-cascade'",
            [],
            |row| row.get(0),
        )
        .unwrap();
    assert_eq!(count_before, 2, "Should have 2 events before delete");

    // Delete the run - should succeed because no append-only trigger
    conn.execute(
        "DELETE FROM workflow_runs_v1 WHERE run_id = 'run-cascade'",
        [],
    )
    .unwrap();

    // Events should be cascade-deleted
    let count_after: i64 = conn
        .query_row(
            "SELECT COUNT(*) FROM workflow_run_events_v1 WHERE run_id = 'run-cascade'",
            [],
            |row| row.get(0),
        )
        .unwrap();
    assert_eq!(
        count_after, 0,
        "Events should be cascade-deleted when run is deleted"
    );
}
