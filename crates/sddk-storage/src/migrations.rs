pub(crate) const LATEST_SCHEMA_VERSION: i32 = 13;

/// Runs all pending migrations on an open SQLite connection.
pub(crate) fn run_migrations(conn: &mut rusqlite::Connection) -> Result<(), super::StorageError> {
    use rusqlite::TransactionBehavior;
    let version: i32 = conn
        .pragma_query_value(None, "user_version", |row| row.get(0))
        .map_err(super::StorageError::Database)?;
    if version < 1 {
        let tx = conn
            .transaction_with_behavior(TransactionBehavior::Immediate)
            .map_err(super::StorageError::Database)?;
        tx.execute_batch(MIGRATION_1)
            .map_err(super::StorageError::Database)?;
        tx.pragma_update(None, "user_version", 1)
            .map_err(super::StorageError::Database)?;
        tx.commit().map_err(super::StorageError::Database)?;
    }
    if version < 2 {
        let tx = conn
            .transaction_with_behavior(TransactionBehavior::Immediate)
            .map_err(super::StorageError::Database)?;
        tx.execute_batch(MIGRATION_2)
            .map_err(super::StorageError::Database)?;
        tx.pragma_update(None, "user_version", 2)
            .map_err(super::StorageError::Database)?;
        tx.commit().map_err(super::StorageError::Database)?;
    }
    if version < 3 {
        let tx = conn
            .transaction_with_behavior(TransactionBehavior::Immediate)
            .map_err(super::StorageError::Database)?;
        tx.execute_batch(MIGRATION_3)
            .map_err(super::StorageError::Database)?;
        tx.pragma_update(None, "user_version", 3)
            .map_err(super::StorageError::Database)?;
        tx.commit().map_err(super::StorageError::Database)?;
    }
    if version < 4 {
        let tx = conn
            .transaction_with_behavior(TransactionBehavior::Immediate)
            .map_err(super::StorageError::Database)?;
        tx.execute_batch(MIGRATION_4)
            .map_err(super::StorageError::Database)?;
        tx.pragma_update(None, "user_version", 4)
            .map_err(super::StorageError::Database)?;
        tx.commit().map_err(super::StorageError::Database)?;
    }
    if version < 5 {
        let tx = conn
            .transaction_with_behavior(TransactionBehavior::Immediate)
            .map_err(super::StorageError::Database)?;
        tx.execute_batch(MIGRATION_5)
            .map_err(super::StorageError::Database)?;
        tx.pragma_update(None, "user_version", 5)
            .map_err(super::StorageError::Database)?;
        tx.commit().map_err(super::StorageError::Database)?;
    }
    if version < 6 {
        let tx = conn
            .transaction_with_behavior(TransactionBehavior::Immediate)
            .map_err(super::StorageError::Database)?;
        tx.execute_batch(MIGRATION_6)
            .map_err(super::StorageError::Database)?;
        tx.pragma_update(None, "user_version", 6)
            .map_err(super::StorageError::Database)?;
        tx.commit().map_err(super::StorageError::Database)?;
    }
    if version < 7 {
        let tx = conn
            .transaction_with_behavior(TransactionBehavior::Immediate)
            .map_err(super::StorageError::Database)?;
        // MIGRATION_7 adds columns to capability_receipts for governed capabilities.
        // Defensively check if the table exists before ALTERing, since databases
        // created by SqliteEventStore (migrations 5-6 only) never ran migrations
        // 1-4 and lack the capability_receipts table entirely.
        let table_exists: bool = tx
            .query_row(
                "SELECT 1 FROM sqlite_master WHERE type='table' AND name='capability_receipts'",
                [],
                |_row| Ok(true),
            )
            .unwrap_or(false);
        if table_exists {
            // Only add columns if they don't already exist (idempotent on re-run)
            let col_exists: bool = tx
                .query_row(
                    "SELECT 1 FROM pragma_table_info('capability_receipts') WHERE name='agent_version_hash'",
                    [],
                    |_row| Ok(true),
                )
                .unwrap_or(false);
            if !col_exists {
                tx.execute(
                    "ALTER TABLE capability_receipts ADD COLUMN agent_version_hash TEXT",
                    [],
                )
                .map_err(super::StorageError::Database)?;
            }
            let behavior_col_exists: bool = tx
                .query_row(
                    "SELECT 1 FROM pragma_table_info('capability_receipts') WHERE name='behavior_version_hash'",
                    [],
                    |_row| Ok(true),
                )
                .unwrap_or(false);
            if !behavior_col_exists {
                tx.execute(
                    "ALTER TABLE capability_receipts ADD COLUMN behavior_version_hash TEXT",
                    [],
                )
                .map_err(super::StorageError::Database)?;
            }
        }
        tx.pragma_update(None, "user_version", 7)
            .map_err(super::StorageError::Database)?;
        tx.commit().map_err(super::StorageError::Database)?;
    }
    if version < 8 {
        let tx = conn
            .transaction_with_behavior(TransactionBehavior::Immediate)
            .map_err(super::StorageError::Database)?;
        tx.execute_batch(MIGRATION_8)
            .map_err(super::StorageError::Database)?;
        tx.pragma_update(None, "user_version", 8)
            .map_err(super::StorageError::Database)?;
        tx.commit().map_err(super::StorageError::Database)?;
    }
    if version < 9 {
        let tx = conn
            .transaction_with_behavior(TransactionBehavior::Immediate)
            .map_err(super::StorageError::Database)?;
        tx.execute_batch(MIGRATION_9)
            .map_err(super::StorageError::Database)?;
        tx.pragma_update(None, "user_version", 9)
            .map_err(super::StorageError::Database)?;
        tx.commit().map_err(super::StorageError::Database)?;
    }
    if version < 10 {
        let tx = conn
            .transaction_with_behavior(TransactionBehavior::Immediate)
            .map_err(super::StorageError::Database)?;
        // Defensively check if events_v1 exists before ALTERing, since databases
        // created before MIGRATION_5 (schema v6) never had the events_v1 table.
        let table_exists: bool = tx
            .query_row(
                "SELECT 1 FROM sqlite_master WHERE type='table' AND name='events_v1'",
                [],
                |_row| Ok(true),
            )
            .unwrap_or(false);
        if table_exists {
            // Only add column if it doesn't already exist (idempotent on re-run)
            let col_exists: bool = tx
                .query_row(
                    "SELECT 1 FROM pragma_table_info('events_v1') WHERE name='chain_hash'",
                    [],
                    |_row| Ok(true),
                )
                .unwrap_or(false);
            if !col_exists {
                tx.execute(
                    "ALTER TABLE events_v1 ADD COLUMN chain_hash TEXT NOT NULL DEFAULT ''",
                    [],
                )
                .map_err(super::StorageError::Database)?;
            }
        }
        tx.pragma_update(None, "user_version", 10)
            .map_err(super::StorageError::Database)?;
        tx.commit().map_err(super::StorageError::Database)?;
    }
    if version < 11 {
        let tx = conn
            .transaction_with_behavior(TransactionBehavior::Immediate)
            .map_err(super::StorageError::Database)?;
        tx.execute_batch(MIGRATION_11)
            .map_err(super::StorageError::Database)?;
        tx.pragma_update(None, "user_version", 11)
            .map_err(super::StorageError::Database)?;
        tx.commit().map_err(super::StorageError::Database)?;
    }
    if version < 12 {
        let tx = conn
            .transaction_with_behavior(TransactionBehavior::Immediate)
            .map_err(super::StorageError::Database)?;
        tx.execute_batch(MIGRATION_12)
            .map_err(super::StorageError::Database)?;
        tx.pragma_update(None, "user_version", 12)
            .map_err(super::StorageError::Database)?;
        tx.commit().map_err(super::StorageError::Database)?;
    }
    if version < 13 {
        let tx = conn
            .transaction_with_behavior(TransactionBehavior::Immediate)
            .map_err(super::StorageError::Database)?;
        tx.execute_batch(MIGRATION_13)
            .map_err(super::StorageError::Database)?;
        tx.pragma_update(None, "user_version", 13)
            .map_err(super::StorageError::Database)?;
        tx.commit().map_err(super::StorageError::Database)?;
    }
    Ok(())
}

pub(crate) const MIGRATION_1: &str = r#"
CREATE TABLE IF NOT EXISTS projects (
    project_id TEXT PRIMARY KEY,
    display_name TEXT NOT NULL CHECK (display_name <> ''),
    remote_url TEXT,
    scope TEXT NOT NULL CHECK (scope <> ''),
    created_at TEXT NOT NULL,
    UNIQUE (remote_url, scope)
);

CREATE TABLE IF NOT EXISTS workspaces (
    workspace_id TEXT PRIMARY KEY,
    project_id TEXT NOT NULL REFERENCES projects(project_id) ON DELETE RESTRICT,
    canonical_path TEXT NOT NULL CHECK (canonical_path <> ''),
    created_at TEXT NOT NULL,
    UNIQUE (project_id, canonical_path),
    UNIQUE (project_id, workspace_id)
);

CREATE TABLE IF NOT EXISTS cycles (
    cycle_id TEXT PRIMARY KEY,
    project_id TEXT NOT NULL,
    workspace_id TEXT NOT NULL,
    status TEXT NOT NULL CHECK (status IN (
        'OPEN', 'BLOCKED', 'REMEDIATING', 'PAUSED', 'RELEASE_PENDING',
        'RELEASED', 'CLOSED', 'ABANDONED', 'RECOVERING'
    )),
    phase TEXT NOT NULL CHECK (phase IN (
        'explore', 'specify', 'design', 'plan', 'build',
        'verify', 'review', 'release', 'archive'
    )),
    manifest_json TEXT NOT NULL,
    created_at TEXT NOT NULL,
    updated_at TEXT NOT NULL,
    FOREIGN KEY (project_id, workspace_id)
        REFERENCES workspaces(project_id, workspace_id) ON DELETE RESTRICT,
    UNIQUE (project_id, cycle_id)
);

CREATE INDEX IF NOT EXISTS cycles_project_status_idx ON cycles(project_id, status);

CREATE TABLE IF NOT EXISTS ledger_events (
    sequence INTEGER PRIMARY KEY CHECK (sequence > 0),
    event_id TEXT NOT NULL UNIQUE,
    project_id TEXT NOT NULL REFERENCES projects(project_id) ON DELETE RESTRICT,
    cycle_id TEXT,
    frame_id TEXT NOT NULL,
    command_id TEXT NOT NULL,
    actor TEXT NOT NULL,
    event_type TEXT NOT NULL,
    occurred_at TEXT NOT NULL,
    state_before_json TEXT,
    state_after_json TEXT,
    payload_json TEXT NOT NULL,
    previous_hash TEXT,
    event_hash TEXT NOT NULL UNIQUE,
    CHECK (
        (sequence = 1 AND previous_hash IS NULL)
        OR (sequence > 1 AND previous_hash IS NOT NULL)
    ),
    FOREIGN KEY (project_id, cycle_id)
        REFERENCES cycles(project_id, cycle_id) ON DELETE RESTRICT
);

CREATE INDEX IF NOT EXISTS ledger_events_cycle_sequence_idx
    ON ledger_events(cycle_id, sequence);

CREATE TRIGGER IF NOT EXISTS ledger_events_no_update
BEFORE UPDATE ON ledger_events
BEGIN
    SELECT RAISE(ABORT, 'ledger events are append-only');
END;

CREATE TRIGGER IF NOT EXISTS ledger_events_no_delete
BEFORE DELETE ON ledger_events
BEGIN
    SELECT RAISE(ABORT, 'ledger events are append-only');
END;

CREATE TABLE IF NOT EXISTS artifacts (
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

CREATE INDEX IF NOT EXISTS artifacts_project_hash_idx ON artifacts(project_id, sha256);
CREATE INDEX IF NOT EXISTS artifacts_cycle_idx ON artifacts(cycle_id);

CREATE TABLE IF NOT EXISTS capability_receipts (
    receipt_id TEXT PRIMARY KEY,
    project_id TEXT NOT NULL REFERENCES projects(project_id) ON DELETE RESTRICT,
    cycle_id TEXT,
    capability TEXT NOT NULL CHECK (capability <> ''),
    request_hash TEXT NOT NULL,
    request_json TEXT NOT NULL,
    status TEXT NOT NULL CHECK (status IN ('started', 'succeeded', 'failed', 'unknown')),
    result_json TEXT,
    started_at TEXT NOT NULL,
    completed_at TEXT,
    FOREIGN KEY (project_id, cycle_id)
        REFERENCES cycles(project_id, cycle_id) ON DELETE RESTRICT
);

CREATE TABLE IF NOT EXISTS idempotency_records (
    project_id TEXT NOT NULL REFERENCES projects(project_id) ON DELETE RESTRICT,
    idempotency_key TEXT NOT NULL,
    request_hash TEXT NOT NULL,
    receipt_id TEXT NOT NULL UNIQUE
        REFERENCES capability_receipts(receipt_id) ON DELETE RESTRICT,
    created_at TEXT NOT NULL,
    PRIMARY KEY (project_id, idempotency_key)
);

CREATE TABLE IF NOT EXISTS cycle_leases (
    cycle_id TEXT PRIMARY KEY REFERENCES cycles(cycle_id) ON DELETE RESTRICT,
    owner TEXT NOT NULL CHECK (owner <> ''),
    acquired_at_ms INTEGER NOT NULL CHECK (acquired_at_ms >= 0),
    expires_at_ms INTEGER NOT NULL CHECK (expires_at_ms > acquired_at_ms),
    fencing_token INTEGER NOT NULL CHECK (fencing_token > 0)
);
"#;

pub(crate) const MIGRATION_2: &str = r#"
CREATE TABLE gate_receipts (
    receipt_id TEXT PRIMARY KEY,
    project_id TEXT NOT NULL REFERENCES projects(project_id) ON DELETE RESTRICT,
    cycle_id TEXT,
    gate TEXT NOT NULL CHECK (gate <> ''),
    evaluator TEXT NOT NULL CHECK (evaluator <> ''),
    transition_id TEXT NOT NULL CHECK (transition_id <> ''),
    plan_hash TEXT NOT NULL CHECK (plan_hash <> ''),
    outcome TEXT NOT NULL CHECK (outcome IN ('passed', 'failed')),
    evidence TEXT NOT NULL,
    actor TEXT NOT NULL CHECK (actor <> ''),
    command_id TEXT NOT NULL CHECK (command_id <> ''),
    frame_id TEXT NOT NULL CHECK (frame_id <> ''),
    evaluated_at TEXT NOT NULL,
    FOREIGN KEY (project_id, cycle_id)
        REFERENCES cycles(project_id, cycle_id) ON DELETE RESTRICT
);

CREATE INDEX gate_receipts_cycle_idx ON gate_receipts(cycle_id);
CREATE INDEX gate_receipts_plan_hash_idx ON gate_receipts(plan_hash);
"#;

pub(crate) const MIGRATION_3: &str = r#"
ALTER TABLE gate_receipts ADD COLUMN seq INTEGER NOT NULL DEFAULT 1;

CREATE UNIQUE INDEX gate_receipts_gate_plan_seq_uniq
    ON gate_receipts(gate, plan_hash, seq);
"#;

pub(crate) const MIGRATION_4: &str = r#"
-- GateOutcomeStatus gains `waived`; SQLite cannot ALTER a CHECK constraint,
-- so the table is recreated. Nothing references gate_receipts, so the rename
-- is safe; the old composite FK (project_id, cycle_id) -> cycles(project_id,
-- cycle_id) pointed at a non-existent composite key (cycles' PK is cycle_id)
-- and is corrected to cycle_id -> cycles(cycle_id) in the recreated table.
-- Runs with foreign_keys=ON: every copied row must reference an existing
-- cycle (NULL cycle_id rows are exempt from FK enforcement).
ALTER TABLE gate_receipts RENAME TO gate_receipts_old;

CREATE TABLE gate_receipts (
    receipt_id TEXT NOT NULL PRIMARY KEY,
    project_id TEXT NOT NULL,
    cycle_id TEXT,
    gate TEXT NOT NULL CHECK (gate <> ''),
    evaluator TEXT NOT NULL CHECK (evaluator <> ''),
    transition_id TEXT NOT NULL CHECK (transition_id <> ''),
    plan_hash TEXT NOT NULL CHECK (plan_hash <> ''),
    outcome TEXT NOT NULL CHECK (outcome IN ('passed', 'failed', 'waived')),
    evidence TEXT NOT NULL,
    actor TEXT NOT NULL CHECK (actor <> ''),
    command_id TEXT NOT NULL CHECK (command_id <> ''),
    frame_id TEXT NOT NULL,
    evaluated_at TEXT NOT NULL,
    seq INTEGER NOT NULL DEFAULT 1,
    FOREIGN KEY (cycle_id)
        REFERENCES cycles(cycle_id) ON DELETE RESTRICT
);

INSERT INTO gate_receipts SELECT * FROM gate_receipts_old;
DROP TABLE gate_receipts_old;

CREATE UNIQUE INDEX gate_receipts_gate_plan_seq_uniq
    ON gate_receipts(gate, plan_hash, seq);
CREATE INDEX gate_receipts_cycle_idx ON gate_receipts(cycle_id);
CREATE INDEX gate_receipts_plan_hash_idx ON gate_receipts(plan_hash);
"#;

pub(crate) const MIGRATION_5: &str = r#"
-- events_v1: append-only event-sourced store for EventEnvelopeV1 (SDDK2-202).
-- Mirrors the ledger_events immutability policy via SQL triggers.
--
-- Minimal projects stub so the events_v1 FK reference is satisfiable when
-- SqliteEventStore runs without the full Storage migrations (e.g. in tests).
-- IF NOT EXISTS avoids conflict when both Storage and SqliteEventStore share
-- the same ledger.sqlite file.
CREATE TABLE IF NOT EXISTS projects (
    project_id  TEXT NOT NULL PRIMARY KEY
);

CREATE TABLE IF NOT EXISTS events_v1 (
    event_id           TEXT NOT NULL PRIMARY KEY
                       CHECK (event_id <> ''),
    stream_id          TEXT NOT NULL
                       CHECK (stream_id <> ''),
    sequence           INTEGER NOT NULL
                       CHECK (sequence > 0),
    event_type         TEXT NOT NULL
                       CHECK (event_type <> ''),
    schema_version     INTEGER NOT NULL
                       CHECK (schema_version = 1),
    project_id         TEXT NOT NULL
                       REFERENCES projects(project_id) ON DELETE RESTRICT,
    occurred_at        TEXT NOT NULL
                       CHECK (occurred_at <> ''),
    recorded_at        TEXT NOT NULL
                       CHECK (recorded_at <> ''),
    actor_json         TEXT NOT NULL,
    causation_id       TEXT,
    correlation_id     TEXT,
    cycle_id           TEXT,
    frame_id           TEXT,
    fork_id            TEXT,
    subjects_json      TEXT NOT NULL,
    payload_json       TEXT NOT NULL,
    evidence_refs_json TEXT NOT NULL,
    content_hash       TEXT NOT NULL
                       CHECK (content_hash LIKE 'sha256:%')
                       UNIQUE,
    metadata_json      TEXT,
    UNIQUE (stream_id, sequence)
);

CREATE INDEX IF NOT EXISTS events_v1_project_idx      ON events_v1(project_id);
CREATE INDEX IF NOT EXISTS events_v1_stream_seq_idx   ON events_v1(stream_id, sequence);
CREATE INDEX IF NOT EXISTS events_v1_content_hash_idx ON events_v1(content_hash);

CREATE TRIGGER IF NOT EXISTS events_v1_no_update
BEFORE UPDATE ON events_v1
BEGIN
    SELECT RAISE(ABORT, 'events_v1 are append-only');
END;

CREATE TRIGGER IF NOT EXISTS events_v1_no_delete
BEFORE DELETE ON events_v1
BEGIN
    SELECT RAISE(ABORT, 'events_v1 are append-only');
END;
"#;

pub(crate) const MIGRATION_6: &str = r#"
-- projection_checkpoints_v1: durable progress markers for read-model projections.
-- The table is mutable (no append-only triggers) because checkpoints are
-- regenerable from the event ledger via the rebuild() algorithm.
CREATE TABLE IF NOT EXISTS projection_checkpoints_v1 (
    projection_name      TEXT    NOT NULL,
    version              INTEGER NOT NULL,
    last_event_sequence  INTEGER NOT NULL
                         CHECK (last_event_sequence >= 0),
    last_event_hash     TEXT    NOT NULL
                         CHECK (last_event_hash LIKE 'sha256:%'),
    state_json           TEXT    NOT NULL
                         CHECK (length(state_json) > 0),
    updated_at           TEXT    NOT NULL
                         CHECK (updated_at <> ''),
    PRIMARY KEY (projection_name, version)
);

CREATE INDEX IF NOT EXISTS projection_checkpoints_v1_name_idx
    ON projection_checkpoints_v1(projection_name);
"#;

pub(crate) const MIGRATION_8: &str = r#"
-- graph_state_v1: reserved for graph snapshots.
-- The current GraphStore adapter persists in projection_checkpoints_v1 under
-- the `graph` projection name; this migration reserves the schema version so
-- future graph-native storage can add tables without a version bump.
SELECT 1;
"#;

pub(crate) const MIGRATION_9: &str = r#"
-- forks_v1: durable fork records (SPEC-009 §3, Phase 7).
CREATE TABLE IF NOT EXISTS forks_v1 (
    fork_id            TEXT PRIMARY KEY CHECK (fork_id <> ''),
    parent_stream_id   TEXT NOT NULL CHECK (parent_stream_id <> ''),
    at_sequence        INTEGER NOT NULL CHECK (at_sequence > 0),
    shared_prefix_hash TEXT NOT NULL CHECK (shared_prefix_hash LIKE 'sha256:%'),
    label              TEXT,
    overrides_json     TEXT NOT NULL DEFAULT '{}',
    creator            TEXT NOT NULL CHECK (creator <> ''),
    created_at         TEXT NOT NULL CHECK (created_at <> ''),
    replay_policy      TEXT NOT NULL CHECK (replay_policy IN ('reconstruct', 'strict'))
);

-- response_cache_v1: recorded LLM/tool responses (SPEC-009 §4, Phase 7).
CREATE TABLE IF NOT EXISTS response_cache_v1 (
    request_hash   TEXT PRIMARY KEY CHECK (request_hash <> ''),
    response_json  TEXT NOT NULL,
    model          TEXT,
    created_at     TEXT NOT NULL CHECK (created_at <> '')
);
"#;

#[allow(dead_code)]
pub(crate) const MIGRATION_10: &str = r#"
-- events_v1 chain_hash: stream hash chaining for cryptographic integrity (Phase 2 SHOULD).
-- chain_hash[0] = SHA256(content_hash || "genesis")
-- chain_hash[N] = SHA256(content_hash[N] || chain_hash[N-1])
-- Only applies if events_v1 exists (legacy databases created before MIGRATION_5
-- never had events_v1, so this is a no-op for them).
ALTER TABLE events_v1 ADD COLUMN chain_hash TEXT NOT NULL DEFAULT '';
"#;

pub(crate) const MIGRATION_11: &str = r#"
-- workflow_runs_v1: runtime instance metadata (projection table).
-- Source of truth is events_v1; this table is a materialized lookup index.
CREATE TABLE IF NOT EXISTS workflow_runs_v1 (
    run_id            TEXT NOT NULL PRIMARY KEY
                      CHECK (run_id <> '' AND length(run_id) <= 64),
    template_id       TEXT NOT NULL,
    template_version  TEXT NOT NULL,
    ir_hash           TEXT NOT NULL CHECK (ir_hash LIKE 'sha256:%'),
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
CREATE INDEX IF NOT EXISTS workflow_runs_v1_template_idx ON workflow_runs_v1(template_id);
CREATE INDEX IF NOT EXISTS workflow_runs_v1_state_idx    ON workflow_runs_v1(state);

-- node_runs_v1: per-node state + last attempt pointer.
CREATE TABLE IF NOT EXISTS node_runs_v1 (
    run_id          TEXT NOT NULL REFERENCES workflow_runs_v1(run_id) ON DELETE CASCADE,
    node_id         TEXT NOT NULL,
    state           TEXT NOT NULL
                    CHECK (state IN ('pending','ready','running','completed','failed','skipped')),
    dependencies_json TEXT NOT NULL,
    last_attempt_id TEXT,
    PRIMARY KEY (run_id, node_id)
);

-- attempts_v1: append-only per execution.
CREATE TABLE IF NOT EXISTS attempts_v1 (
    attempt_id        TEXT NOT NULL PRIMARY KEY
                      CHECK (attempt_id <> ''),
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
CREATE TRIGGER IF NOT EXISTS attempts_v1_no_update BEFORE UPDATE ON attempts_v1
    BEGIN SELECT RAISE(ABORT, 'attempts_v1 are append-only'); END;
CREATE TRIGGER IF NOT EXISTS attempts_v1_no_delete BEFORE DELETE ON attempts_v1
    BEGIN SELECT RAISE(ABORT, 'attempts_v1 are append-only'); END;

-- execution_graph_revisions_v1: parent-chain + digest per run.
CREATE TABLE IF NOT EXISTS execution_graph_revisions_v1 (
    revision_id         TEXT NOT NULL PRIMARY KEY,
    run_id              TEXT NOT NULL REFERENCES workflow_runs_v1(run_id) ON DELETE CASCADE,
    revision            INTEGER NOT NULL CHECK (revision >= 0),
    parent_revision_id   TEXT REFERENCES execution_graph_revisions_v1(revision_id),
    events_json         TEXT NOT NULL,
    nodes_json          TEXT NOT NULL,
    edges_json          TEXT NOT NULL,
    digest              TEXT NOT NULL CHECK (digest LIKE 'sha256:%'),
    schema_version      INTEGER NOT NULL CHECK (schema_version = 1),
    UNIQUE (run_id, revision)
);

-- ir_digests_v1: dedup table for compiled IRs.
CREATE TABLE IF NOT EXISTS ir_digests_v1 (
    ir_hash     TEXT NOT NULL PRIMARY KEY CHECK (ir_hash LIKE 'sha256:%'),
    ir_json     TEXT NOT NULL,
    compiled_at TEXT NOT NULL
);
"#;

pub(crate) const MIGRATION_12: &str = r#"
CREATE TABLE IF NOT EXISTS incs_v1 (
    inc_id              TEXT NOT NULL PRIMARY KEY,
    finding_id          TEXT NOT NULL,
    cycle_id            TEXT NOT NULL,
    status              TEXT NOT NULL,
    severity            TEXT NOT NULL,
    priority            TEXT NOT NULL,
    fingerprint        TEXT NOT NULL,
    fingerprint_aliases TEXT NOT NULL DEFAULT '[]',
    cluster_id          TEXT NOT NULL,
    created_at          TEXT NOT NULL,
    created_by          TEXT NOT NULL,
    owner               TEXT NOT NULL,
    inc_path            TEXT NOT NULL,
    lifecycle_events    TEXT NOT NULL DEFAULT '[]',
    evidence_refs       TEXT NOT NULL DEFAULT '[]'
);
CREATE INDEX IF NOT EXISTS idx_incs_v1_fingerprint ON incs_v1(fingerprint);
CREATE INDEX IF NOT EXISTS idx_incs_v1_cycle ON incs_v1(cycle_id);
"#;

pub(crate) const MIGRATION_13: &str = r#"
-- Append-only triggers for workflow_runs_v1.
-- These tables were created in MIGRATION_11; attempts_v1 already has these triggers.
-- This migration adds them to workflow_runs_v1 and node_runs_v1.
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
