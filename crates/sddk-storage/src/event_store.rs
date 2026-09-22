//! SQLite-backed [`sddk_domain::EventStore`] adapter for the Common Event Protocol v1.
//!
//! Persists [`sddk_domain::EventEnvelopeV1`] to the `events_v1` table, the
//! single canonical append authority (C1.5, MIGRATION_20 dropped the frozen
//! legacy `ledger_events` table).
//!
//! ## Stable error prefix contract (R2)
//!
//! All error responses use [`sddk_domain::StorageError::Other`] with a stable
//! `event_store:<code>` prefix:
//!
//! | Code | Meaning |
//! |------|---------|
//! | `event_store:content_hash_mismatch` | `content_hash` != recomputed hash |
//! | `event_store:invalid_content_hash` | missing `sha256:` prefix or wrong length |
//! | `event_store:invalid_event_type` | event_type failed validation |
//! | `event_store:hash_drift:<seq>` | stored hash differs from recomputed at sequence |
//!
//! ## Connection model (R6)
//!
//! Each `SqliteEventStore` instance owns its own `rusqlite::Connection` to
//! `ledger.sqlite`. This is a second connection separate from `Storage`'s
//! connection — both connect to the same file. They serialize writers via
//! `busy_timeout=5s` + WAL mode. The shared physical schema is owned by the
//! single migration authority (`crate::migrations::run_migrations`); this
//! store MUST NOT run a competing shared-schema migration sequence
//! (ARCH-SPEC-020 SSO-001).

use std::path::Path;
use std::time::Duration;

use rusqlite::Connection;
use sddk_domain::{EventAppended, EventEnvelopeV1, EventStore, StorageError as DomainStorageError};
use sha2::{Digest, Sha256};

/// SQLite-backed [`EventStore`] implementation.
pub struct SqliteEventStore {
    conn: Connection,
}

impl SqliteEventStore {
    /// Opens (or creates) a `ledger.sqlite` file at `$dir/ledger.sqlite` and
    /// applies all pending migrations.
    ///
    /// Same WAL + busy-timeout + FK pragma policy as [`Storage::open`].
    pub fn open(dir: &Path) -> Result<Self, DomainStorageError> {
        let path = dir.join("ledger.sqlite");
        let conn = Connection::open(&path)
            .map_err(|e| DomainStorageError::Database(format!("open: {e}")))?;
        conn.busy_timeout(Duration::from_secs(5))
            .map_err(|e| DomainStorageError::Database(format!("busy_timeout: {e}")))?;
        conn.pragma_update(None, "foreign_keys", true)
            .map_err(|e| DomainStorageError::Database(format!("foreign_keys: {e}")))?;
        conn.pragma_update(None, "journal_mode", "WAL")
            .map_err(|e| DomainStorageError::Database(format!("journal_mode: {e}")))?;
        let mut conn = conn;
        Self::run_migrations(&mut conn)?;
        Ok(Self { conn })
    }

    /// Opens an isolated in-memory database with all migrations applied.
    /// Useful for tests.
    pub fn open_in_memory() -> Result<Self, DomainStorageError> {
        let conn = Connection::open_in_memory()
            .map_err(|e| DomainStorageError::Database(format!("open_in_memory: {e}")))?;
        let mut conn = conn;
        Self::run_migrations(&mut conn)?;
        Ok(Self { conn })
    }

    /// Opens (or creates) the canonical events table at an explicit database
    /// file path.
    ///
    /// Unlike [`Self::open`], which derives `ledger.sqlite` from a directory,
    /// this accepts the exact file path so an existing [`Storage`] handle and
    /// the canonical event store can point at the same database file. Writer
    /// serialization is delegated to WAL mode + busy timeout (same policy as
    /// every other connection to `ledger.sqlite`).
    pub fn open_path(path: impl AsRef<Path>) -> Result<Self, DomainStorageError> {
        let conn = Connection::open(path.as_ref())
            .map_err(|e| DomainStorageError::Database(format!("open: {e}")))?;
        conn.busy_timeout(Duration::from_secs(5))
            .map_err(|e| DomainStorageError::Database(format!("busy_timeout: {e}")))?;
        conn.pragma_update(None, "foreign_keys", true)
            .map_err(|e| DomainStorageError::Database(format!("foreign_keys: {e}")))?;
        conn.pragma_update(None, "journal_mode", "WAL")
            .map_err(|e| DomainStorageError::Database(format!("journal_mode: {e}")))?;
        let mut conn = conn;
        Self::run_migrations(&mut conn)?;
        Ok(Self { conn })
    }

    /// Lists all stream ids present in the event ledger (distinct).
    pub fn list_streams(&self) -> Result<Vec<String>, DomainStorageError> {
        let mut stmt = self
            .conn
            .prepare("SELECT DISTINCT stream_id FROM events_v1 ORDER BY stream_id")
            .map_err(|e| DomainStorageError::Database(format!("list_streams prep: {e}")))?;
        let streams = stmt
            .query_map([], |row| row.get::<_, String>(0))
            .map_err(|e| DomainStorageError::Database(format!("list_streams query: {e}")))?
            .collect::<Result<Vec<_>, _>>()
            .map_err(|e| DomainStorageError::Database(format!("list_streams collect: {e}")))?;
        Ok(streams)
    }

    /// Applies the single canonical schema/migration sequence owned by
    /// [`crate::migrations::run_migrations`], then ensures the
    /// event-store-owned `event_snapshots_v1` table exists.
    ///
    /// ARCH-SPEC-020 SSO-001: `SqliteEventStore` opens the same physical
    /// `ledger.sqlite` as `Storage`; it MUST NOT maintain a competing
    /// migration history or a private schema-version pragma for shared
    /// schema. It previously re-ran a subset (`MIGRATION_5/6`) behind a
    /// private `sddk_eventstore_version` pragma. That could leave the shared
    /// `projects` table as the minimal `MIGRATION_5` stub whenever the event
    /// store opened the database before `Storage`, because the full
    /// `MIGRATION_1` definition is then skipped by `CREATE TABLE IF NOT
    /// EXISTS`. Delegating to the canonical owner removes that divergence.
    fn run_migrations(conn: &mut Connection) -> Result<(), DomainStorageError> {
        crate::migrations::run_migrations(conn)
            .map_err(|e| DomainStorageError::Database(e.to_string()))?;
        // `event_snapshots_v1` is owned solely by this store (no other store
        // consumes it), so its single DDL definition lives here rather than in
        // the canonical owner. `IF NOT EXISTS` keeps reopen idempotent.
        conn.execute_batch(EVENT_SNAPSHOTS_DDL)
            .map_err(|e| DomainStorageError::Database(e.to_string()))?;
        Ok(())
    }

    /// Returns the underlying SQLite connection.
    pub fn connection(&self) -> &Connection {
        &self.conn
    }
}

// ── EventStore impl ────────────────────────────────────────────────────────────

impl EventStore for SqliteEventStore {
    fn append(&mut self, envelope: &EventEnvelopeV1) -> Result<EventAppended, DomainStorageError> {
        // 1. Validate content_hash format before entering the transaction.
        if !envelope.content_hash.starts_with("sha256:") {
            return Err(DomainStorageError::Other(
                "event_store:invalid_content_hash".into(),
            ));
        }
        if envelope.content_hash.len() != "sha256:".len() + 64 {
            return Err(DomainStorageError::Other(
                "event_store:invalid_content_hash".into(),
            ));
        }
        // Also validate that the hash matches the recomputed value.
        let computed = envelope.compute_content_hash();
        if computed != envelope.content_hash {
            return Err(DomainStorageError::Other(
                "event_store:content_hash_mismatch".into(),
            ));
        }

        let tx = self
            .conn
            .transaction_with_behavior(rusqlite::TransactionBehavior::Immediate)
            .map_err(|e| DomainStorageError::Database(format!("begin tx: {e}")))?;

        // 2. Ensure the project row exists (satisfies the events_v1 FK).
        //    ARCH-SPEC-020 SSO-002: the shared `projects` row is a
        //    cross-store bootstrap contract. The event store MUST write a
        //    row that every store can consume (all NOT NULL columns), not a
        //    partial shape. The previous `INSERT ... (project_id)` relied on
        //    the minimal `MIGRATION_5` stub: against the canonical full
        //    `MIGRATION_1` schema, `OR IGNORE` silently swallowed the NOT NULL
        //    violation and left the FK parent missing. `display_name` defaults
        //    to the project id and `scope` to "."; `OR IGNORE` keeps this
        //    idempotent when the canonical owner already created the project.
        tx.execute(
            "INSERT OR IGNORE INTO projects (project_id, display_name, scope, created_at)
             VALUES (?1, ?1, '.', ?2)",
            rusqlite::params![envelope.project_id, envelope.recorded_at],
        )
        .map_err(|e| DomainStorageError::Database(format!("project upsert: {e}")))?;

        // 3. Compute next sequence per stream.
        let next_seq: i64 = tx
            .query_row(
                "SELECT COALESCE(MAX(sequence), 0) + 1 FROM events_v1 WHERE stream_id = ?1",
                rusqlite::params![envelope.stream_id],
                |row| row.get(0),
            )
            .map_err(|e| DomainStorageError::Database(format!("seq: {e}")))?;

        // 4. Compute chain_hash: SHA256(content_hash || previous_chain_hash)
        //    Genesis event (stream empty): SHA256(content_hash || "genesis")
        //    Subsequent events: SHA256(content_hash || head_chain_hash)
        let prev_chain_hash: Option<String> = tx
            .query_row(
                "SELECT chain_hash FROM events_v1
                 WHERE stream_id = ?1
                 ORDER BY sequence DESC
                 LIMIT 1",
                rusqlite::params![envelope.stream_id],
                |row| row.get::<_, Option<String>>(0),
            )
            .ok()
            .flatten();

        let chain_input: Vec<u8> = if prev_chain_hash.as_deref().unwrap_or("").is_empty() {
            // Genesis event
            envelope
                .content_hash
                .as_bytes()
                .iter()
                .chain("genesis".as_bytes())
                .cloned()
                .collect()
        } else {
            // Chained event
            envelope
                .content_hash
                .as_bytes()
                .iter()
                .chain(prev_chain_hash.unwrap().as_bytes())
                .cloned()
                .collect()
        };
        let chain_digest = Sha256::digest(&chain_input);
        let chain_hash = format!("sha256:{:x}", chain_digest);

        // 5. Serialize JSON fields.
        let actor_json = serde_json::to_string(&envelope.actor)
            .map_err(|e| DomainStorageError::Other(format!("actor: {e}")))?;
        let subjects_json = serde_json::to_string(&envelope.subjects)
            .map_err(|e| DomainStorageError::Other(format!("subjects: {e}")))?;
        let payload_json = serde_json::to_string(&envelope.payload)
            .map_err(|e| DomainStorageError::Other(format!("payload: {e}")))?;
        let evidence_refs_json = serde_json::to_string(&envelope.evidence_refs)
            .map_err(|e| DomainStorageError::Other(format!("evidence_refs: {e}")))?;
        let metadata_json: Option<String> = envelope
            .metadata
            .as_ref()
            .map(serde_json::to_string)
            .transpose()
            .map_err(|e| DomainStorageError::Other(format!("metadata: {e}")))?;

        // 6. Insert the event (chain_hash is now a real column).
        //
        // Use INSERT OR IGNORE on event_id for idempotency: re-append of the
        // same event_id returns the original row without allocating a new sequence.
        // WU-C1.3 (CLOSE-01b): a DIFFERENT event colliding with a stored
        // event_id is a real integrity failure, not an idempotent retry —
        // the caller must see the typed guard instead of a silent no-op.
        let rows_affected = tx
            .execute(
                "INSERT OR IGNORE INTO events_v1 (
                    event_id, stream_id, sequence, event_type, schema_version, project_id,
                    occurred_at, recorded_at, actor_json, causation_id, correlation_id,
                    cycle_id, frame_id, fork_id, subjects_json, payload_json,
                    evidence_refs_json, content_hash, metadata_json, chain_hash
                ) VALUES (
                    ?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12, ?13, ?14, ?15,
                    ?16, ?17, ?18, ?19, ?20
                )",
                rusqlite::params![
                    envelope.event_id,
                    envelope.stream_id,
                    next_seq,
                    envelope.event_type,
                    envelope.schema_version,
                    envelope.project_id,
                    envelope.occurred_at,
                    envelope.recorded_at,
                    actor_json,
                    envelope.causation_id,
                    envelope.correlation_id,
                    envelope.cycle_id,
                    envelope.frame_id,
                    envelope.fork_id,
                    subjects_json,
                    payload_json,
                    evidence_refs_json,
                    envelope.content_hash,
                    metadata_json,
                    chain_hash,
                ],
            )
            .map_err(|e| DomainStorageError::Database(format!("insert: {e}")))?;

        if rows_affected == 0 {
            // event_id already stored. Idempotent retry only when the stored
            // event is byte-identical (same content hash); otherwise reject.
            let stored_hash: String = tx
                .query_row(
                    "SELECT content_hash FROM events_v1 WHERE event_id = ?1",
                    rusqlite::params![envelope.event_id],
                    |row| row.get(0),
                )
                .map_err(|e| DomainStorageError::Database(format!("dup probe: {e}")))?;
            if stored_hash != envelope.content_hash {
                return Err(DomainStorageError::Other(format!(
                    "event_store:duplicate_event_id:{}",
                    envelope.event_id
                )));
            }
        }

        // 7. Read back the row (handles both first-insert and idempotent re-append).
        let appended = tx
            .query_row(
                "SELECT event_id, stream_id, sequence, content_hash, recorded_at, chain_hash
                 FROM events_v1 WHERE event_id = ?1",
                rusqlite::params![envelope.event_id],
                |row| {
                    Ok(EventAppended {
                        event_id: row.get(0)?,
                        stream_id: row.get(1)?,
                        sequence: row.get::<_, i64>(2)? as u64,
                        content_hash: row.get(3)?,
                        recorded_at: row.get(4)?,
                        chain_hash: row.get(5)?,
                    })
                },
            )
            .map_err(|e| DomainStorageError::Database(format!("read back: {e}")))?;

        tx.commit()
            .map_err(|e| DomainStorageError::Database(format!("commit: {e}")))?;

        Ok(appended)
    }

    fn load_by_event_id(
        &self,
        event_id: &str,
    ) -> Result<Option<EventEnvelopeV1>, DomainStorageError> {
        let mut stmt = self
            .conn
            .prepare(
                "SELECT
                    event_id, stream_id, sequence, event_type, schema_version, project_id,
                    occurred_at, recorded_at, actor_json, causation_id, correlation_id,
                    cycle_id, frame_id, fork_id, subjects_json, payload_json,
                    evidence_refs_json, content_hash, metadata_json
                 FROM events_v1 WHERE event_id = ?1",
            )
            .map_err(|e| DomainStorageError::Database(format!("prepare: {e}")))?;
        let mut rows = stmt
            .query(rusqlite::params![event_id])
            .map_err(|e| DomainStorageError::Database(format!("query: {e}")))?;
        match rows.next() {
            Ok(Some(row)) => row_to_envelope(row).map(Some),
            Ok(None) => Ok(None),
            Err(e) => Err(DomainStorageError::Database(format!("next: {e}"))),
        }
    }

    fn load_stream(
        &self,
        stream_id: &str,
        after_sequence: Option<u64>,
        limit: u32,
    ) -> Result<Vec<EventEnvelopeV1>, DomainStorageError> {
        let after = after_sequence.map(|s| s as i64).unwrap_or(0i64);
        let mut stmt = self
            .conn
            .prepare(
                "SELECT
                    event_id, stream_id, sequence, event_type, schema_version, project_id,
                    occurred_at, recorded_at, actor_json, causation_id, correlation_id,
                    cycle_id, frame_id, fork_id, subjects_json, payload_json,
                    evidence_refs_json, content_hash, metadata_json
                 FROM events_v1
                 WHERE stream_id = ?1 AND sequence > ?2
                 ORDER BY sequence ASC
                 LIMIT ?3",
            )
            .map_err(|e| DomainStorageError::Database(format!("prepare: {e}")))?;
        let mut rows = stmt
            .query(rusqlite::params![stream_id, after, limit as i64])
            .map_err(|e| DomainStorageError::Database(format!("query: {e}")))?;
        let mut out = Vec::new();
        while let Some(row) = rows
            .next()
            .map_err(|e| DomainStorageError::Database(format!("next: {e}")))?
        {
            let env = row_to_envelope(row)?;
            out.push(env);
        }
        Ok(out)
    }

    fn last_sequence(&self, stream_id: &str) -> Result<Option<u64>, DomainStorageError> {
        let seq: Option<i64> = self
            .conn
            .query_row(
                "SELECT MAX(sequence) FROM events_v1 WHERE stream_id = ?1",
                rusqlite::params![stream_id],
                |row| row.get(0),
            )
            .map_err(|e| DomainStorageError::Database(format!("query: {e}")))?;
        Ok(seq.map(|s| s as u64))
    }

    fn count(&self) -> Result<u64, DomainStorageError> {
        let n: i64 = self
            .conn
            .query_row("SELECT COUNT(*) FROM events_v1", [], |row| row.get(0))
            .map_err(|e| DomainStorageError::Database(format!("count: {e}")))?;
        Ok(n as u64)
    }

    fn head_hash(&self, stream_id: &str) -> Result<Option<String>, DomainStorageError> {
        let mut stmt = self
            .conn
            .prepare(
                "SELECT content_hash FROM events_v1
                 WHERE stream_id = ?1
                 ORDER BY sequence DESC
                 LIMIT 1",
            )
            .map_err(|e| DomainStorageError::Database(format!("prepare: {e}")))?;
        let mut rows = stmt
            .query(rusqlite::params![stream_id])
            .map_err(|e| DomainStorageError::Database(format!("query: {e}")))?;
        match rows.next() {
            Ok(Some(row)) => row
                .get(0)
                .map(Some)
                .map_err(|e| DomainStorageError::Database(format!("head_hash: {e}"))),
            Ok(None) => Ok(None),
            Err(e) => Err(DomainStorageError::Database(format!("head_hash: {e}"))),
        }
    }

    fn head_chain_hash(&self, stream_id: &str) -> Result<Option<String>, DomainStorageError> {
        let mut stmt = self
            .conn
            .prepare(
                "SELECT chain_hash FROM events_v1
                 WHERE stream_id = ?1
                 ORDER BY sequence DESC
                 LIMIT 1",
            )
            .map_err(|e| DomainStorageError::Database(format!("prepare: {e}")))?;
        let mut rows = stmt
            .query(rusqlite::params![stream_id])
            .map_err(|e| DomainStorageError::Database(format!("query: {e}")))?;
        match rows.next() {
            Ok(Some(row)) => row
                .get(0)
                .map(Some)
                .map_err(|e| DomainStorageError::Database(format!("head_chain_hash: {e}"))),
            Ok(None) => Ok(None),
            Err(e) => Err(DomainStorageError::Database(format!(
                "head_chain_hash: {e}"
            ))),
        }
    }

    fn verify_stream_chain(&self, stream_id: &str) -> Result<(), DomainStorageError> {
        // Load full stream via load_stream (no limit = u32::MAX).
        let events = self.load_stream(stream_id, None, u32::MAX)?;
        // Recompute each event's content_hash and compare to stored value.
        // This is a STORAGE-side invariant; per-row hash is self-contained.
        for ev in &events {
            let computed = ev.compute_content_hash();
            if computed != ev.content_hash {
                return Err(DomainStorageError::Other(format!(
                    "event_store:hash_drift:{}",
                    ev.sequence
                )));
            }
        }
        Ok(())
    }

    fn verify_chain_integrity(&self, stream_id: &str) -> Result<(), DomainStorageError> {
        use sha2::{Digest, Sha256};

        let events = self.load_stream(stream_id, None, u32::MAX)?;
        if events.is_empty() {
            return Ok(());
        }

        let genesis_prev = "genesis".as_bytes();
        let mut prev_chain_hash: Option<String> = None;

        for ev in &events {
            // Determine the previous chain hash input.
            let prev_input = match prev_chain_hash {
                Some(ref prev) => prev.as_bytes().to_vec(),
                None => genesis_prev.to_vec(),
            };

            // Compute: SHA256(content_hash || prev_input)
            let mut hasher = Sha256::new();
            hasher.update(ev.content_hash.as_bytes());
            hasher.update(&prev_input);
            let result = hasher.finalize();
            let expected_chain = format!("sha256:{:x}", result);

            // Read stored chain_hash (column may be empty for pre-migration events).
            let stored_chain = self
                .conn
                .query_row(
                    "SELECT chain_hash FROM events_v1 WHERE event_id = ?1",
                    rusqlite::params![ev.event_id],
                    |row| row.get::<_, String>(0),
                )
                .map_err(|e| DomainStorageError::Database(format!("chain_hash read: {e}")))?;

            // Empty chain_hash means pre-migration event — skip verification.
            if stored_chain.is_empty() {
                prev_chain_hash = None; // Reset for next event (treat as new genesis)
                continue;
            }

            if stored_chain != expected_chain {
                return Err(DomainStorageError::Other(format!(
                    "event_store:chain_drift:{}",
                    ev.sequence
                )));
            }

            prev_chain_hash = Some(stored_chain);
        }
        Ok(())
    }

    /// Backfills `chain_hash` for existing events that were inserted before the
    /// chain_hash column was added (MIGRATION_10). Idempotent: skips events with
    /// a non-empty chain_hash. Operates within a transaction per stream.
    fn backfill_chain_hash(&mut self, stream_id: &str) -> Result<usize, DomainStorageError> {
        use sha2::{Digest, Sha256};

        let events = self.load_stream(stream_id, None, u32::MAX)?;
        if events.is_empty() {
            return Ok(0);
        }

        let mut updated = 0;
        let genesis_prev = b"genesis";
        let mut prev_chain_hash: Option<String> = None;

        // Process events in sequence order, collecting backfills
        let mut backfills: Vec<(String, String)> = Vec::new();
        for ev in &events {
            // Read current chain_hash from DB
            let stored_chain: Option<String> = self
                .conn
                .query_row(
                    "SELECT chain_hash FROM events_v1 WHERE event_id = ?1",
                    rusqlite::params![ev.event_id],
                    |row| row.get::<_, Option<String>>(0),
                )
                .map_err(|e| DomainStorageError::Database(format!("read chain_hash: {e}")))?;

            // Skip if already has chain_hash
            if stored_chain.as_deref().unwrap_or("").is_empty() {
                // Compute chain_hash
                let prev_input: Vec<u8> = match prev_chain_hash {
                    Some(ref prev) => prev.as_bytes().to_vec(),
                    None => genesis_prev.to_vec(),
                };
                let mut hasher = Sha256::new();
                hasher.update(ev.content_hash.as_bytes());
                hasher.update(&prev_input);
                let digest = hasher.finalize();
                let chain_hash = format!("sha256:{:x}", digest);
                backfills.push((ev.event_id.clone(), chain_hash.clone()));
                prev_chain_hash = Some(chain_hash);
            } else {
                prev_chain_hash = stored_chain;
            }
        }

        // Apply all backfills in a single transaction
        if !backfills.is_empty() {
            let tx = self
                .conn
                .transaction_with_behavior(rusqlite::TransactionBehavior::Immediate)
                .map_err(|e| DomainStorageError::Database(format!("backfill tx: {e}")))?;
            for (event_id, chain_hash) in &backfills {
                let rows = tx
                    .execute(
                        "UPDATE events_v1 SET chain_hash = ?1 WHERE event_id = ?2 AND chain_hash = ''",
                        rusqlite::params![chain_hash, event_id],
                    )
                    .map_err(|e| DomainStorageError::Database(format!("backfill update: {e}")))?;
                updated += rows;
            }
            tx.commit()
                .map_err(|e| DomainStorageError::Database(format!("backfill commit: {e}")))?;
        }

        Ok(updated)
    }

    fn load_by_sequence(
        &self,
        stream_id: &str,
        sequence: u64,
    ) -> Result<Option<EventEnvelopeV1>, DomainStorageError> {
        let mut stmt = self
            .conn
            .prepare(
                "SELECT
                    event_id, stream_id, sequence, event_type, schema_version, project_id,
                    occurred_at, recorded_at, actor_json, causation_id, correlation_id,
                    cycle_id, frame_id, fork_id, subjects_json, payload_json,
                    evidence_refs_json, content_hash, metadata_json
                 FROM events_v1
                 WHERE stream_id = ?1 AND sequence = ?2",
            )
            .map_err(|e| DomainStorageError::Database(format!("prepare: {e}")))?;
        let mut rows = stmt
            .query(rusqlite::params![stream_id, sequence as i64])
            .map_err(|e| DomainStorageError::Database(format!("query: {e}")))?;
        match rows.next() {
            Ok(Some(row)) => row_to_envelope(row).map(Some),
            Ok(None) => Ok(None),
            Err(e) => Err(DomainStorageError::Database(format!("next: {e}"))),
        }
    }
}

// ── SnapshotPort impl (AC-EVT-LEDGER-04) ─────────────────────────────────────

use sddk_domain::ports::SnapshotError as DomainSnapshotError;
use sddk_domain::replay::Snapshot;

impl sddk_domain::ports::SnapshotPort for SqliteEventStore {
    fn save_snapshot(&mut self, snapshot: &Snapshot) -> Result<(), DomainSnapshotError> {
        self.conn
            .execute(
                "INSERT OR REPLACE INTO event_snapshots_v1
                 (name, stream_id, sequence, content_hash, chain_hash, taken_at_ms)
                 VALUES (?1, ?2, ?3, ?4, ?5, ?6)",
                rusqlite::params![
                    snapshot.name,
                    snapshot.stream_id,
                    snapshot.sequence as i64,
                    snapshot.content_hash,
                    snapshot.chain_hash,
                    snapshot.taken_at_ms,
                ],
            )
            .map_err(|e| DomainSnapshotError::Storage(e.to_string()))?;
        Ok(())
    }

    fn load_snapshot(&self, name: &str) -> Result<Option<Snapshot>, DomainSnapshotError> {
        let result = self.conn.query_row(
            "SELECT name, stream_id, sequence, content_hash, chain_hash, taken_at_ms
             FROM event_snapshots_v1 WHERE name = ?1",
            [name],
            |row| {
                Ok(Snapshot {
                    name: row.get(0)?,
                    stream_id: row.get(1)?,
                    sequence: row.get::<_, i64>(2)? as u64,
                    content_hash: row.get(3)?,
                    chain_hash: row.get(4)?,
                    taken_at_ms: row.get(5)?,
                })
            },
        );
        match result {
            Ok(s) => Ok(Some(s)),
            Err(rusqlite::Error::QueryReturnedNoRows) => Ok(None),
            Err(e) => Err(DomainSnapshotError::Storage(e.to_string())),
        }
    }

    fn list_snapshots(&self, stream_id: &str) -> Result<Vec<String>, DomainSnapshotError> {
        let mut stmt = self
            .conn
            .prepare("SELECT name FROM event_snapshots_v1 WHERE stream_id = ?1 ORDER BY name")
            .map_err(|e| DomainSnapshotError::Storage(e.to_string()))?;
        let rows = stmt
            .query_map([stream_id], |row| row.get::<_, String>(0))
            .map_err(|e| DomainSnapshotError::Storage(e.to_string()))?;
        rows.map(|r| r.map_err(|e| DomainSnapshotError::Storage(e.to_string())))
            .collect::<Result<Vec<String>, DomainSnapshotError>>()
    }

    fn delete_snapshot(&mut self, name: &str) -> Result<(), DomainSnapshotError> {
        let deleted = self
            .conn
            .execute("DELETE FROM event_snapshots_v1 WHERE name = ?1", [name])
            .map_err(|e| DomainSnapshotError::Storage(e.to_string()))?;
        if deleted == 0 {
            return Err(DomainSnapshotError::NotFound(name.to_string()));
        }
        Ok(())
    }
}

// ── Helper ───────────────────────────────────────────────────────────────────

fn row_to_envelope(row: &rusqlite::Row) -> Result<EventEnvelopeV1, DomainStorageError> {
    let actor_json: String = row
        .get(8)
        .map_err(|e| DomainStorageError::Database(format!("actor_json: {e}")))?;
    let actor: sddk_domain::ActorRef = serde_json::from_str(&actor_json)
        .map_err(|e| DomainStorageError::Other(format!("actor parse: {e}")))?;

    let subjects_json: String = row
        .get(14)
        .map_err(|e| DomainStorageError::Database(format!("subjects_json: {e}")))?;
    let subjects: Vec<sddk_domain::EntityRef> = serde_json::from_str(&subjects_json)
        .map_err(|e| DomainStorageError::Other(format!("subjects parse: {e}")))?;

    let payload_json: String = row
        .get(15)
        .map_err(|e| DomainStorageError::Database(format!("payload_json: {e}")))?;
    let payload: serde_json::Value = serde_json::from_str(&payload_json)
        .map_err(|e| DomainStorageError::Other(format!("payload parse: {e}")))?;

    let evidence_refs_json: String = row
        .get(16)
        .map_err(|e| DomainStorageError::Database(format!("evidence_refs_json: {e}")))?;
    let evidence_refs: Vec<String> = serde_json::from_str(&evidence_refs_json)
        .map_err(|e| DomainStorageError::Other(format!("evidence_refs parse: {e}")))?;

    let metadata_json: Option<String> = row
        .get(18)
        .map_err(|e| DomainStorageError::Database(format!("metadata_json: {e}")))?;
    let metadata: Option<serde_json::Value> = metadata_json
        .map(|m| serde_json::from_str(&m))
        .transpose()
        .map_err(|e| DomainStorageError::Other(format!("metadata parse: {e}")))?;

    Ok(EventEnvelopeV1 {
        event_id: row
            .get(0)
            .map_err(|e| DomainStorageError::Database(format!("event_id: {e}")))?,
        stream_id: row
            .get(1)
            .map_err(|e| DomainStorageError::Database(format!("stream_id: {e}")))?,
        sequence: row
            .get::<_, i64>(2)
            .map_err(|e| DomainStorageError::Database(format!("sequence: {e}")))?
            as u64,
        event_type: row
            .get(3)
            .map_err(|e| DomainStorageError::Database(format!("event_type: {e}")))?,
        schema_version: row
            .get::<_, u32>(4)
            .map_err(|e| DomainStorageError::Database(format!("schema_version: {e}")))?,
        project_id: row
            .get(5)
            .map_err(|e| DomainStorageError::Database(format!("project_id: {e}")))?,
        occurred_at: row
            .get(6)
            .map_err(|e| DomainStorageError::Database(format!("occurred_at: {e}")))?,
        recorded_at: row
            .get(7)
            .map_err(|e| DomainStorageError::Database(format!("recorded_at: {e}")))?,
        actor,
        causation_id: row
            .get(9)
            .map_err(|e| DomainStorageError::Database(format!("causation_id: {e}")))?,
        correlation_id: row
            .get(10)
            .map_err(|e| DomainStorageError::Database(format!("correlation_id: {e}")))?,
        cycle_id: row
            .get(11)
            .map_err(|e| DomainStorageError::Database(format!("cycle_id: {e}")))?,
        frame_id: row
            .get(12)
            .map_err(|e| DomainStorageError::Database(format!("frame_id: {e}")))?,
        fork_id: row
            .get(13)
            .map_err(|e| DomainStorageError::Database(format!("fork_id: {e}")))?,
        subjects,
        payload,
        evidence_refs,
        content_hash: row
            .get(17)
            .map_err(|e| DomainStorageError::Database(format!("content_hash: {e}")))?,
        metadata,
    })
}

// ── EVENT_SNAPSHOTS_DDL: event_snapshots_v1 (AC-EVT-LEDGER-04) ───────────────

/// DDL for `event_snapshots_v1`, owned solely by this store.
///
/// AC-EVT-LEDGER-04: persists named replay snapshots so replay can resume
/// from a known position rather than reprocessing the entire stream.
///
/// Ownership note (ARCH-SPEC-020 SSO-006): this is a store-owned auxiliary
/// table, not shared schema. It is created idempotently after the canonical
/// schema owner runs; it is not a competing shared-schema migration. The name
/// deliberately avoids `MIGRATION_11` to prevent confusion with the canonical
/// `crate::migrations::MIGRATION_11` (workflow_runs_v1).
const EVENT_SNAPSHOTS_DDL: &str = r#"
CREATE TABLE IF NOT EXISTS event_snapshots_v1 (
    name            TEXT NOT NULL,
    stream_id       TEXT NOT NULL,
    sequence        INTEGER NOT NULL,
    content_hash    TEXT NOT NULL,
    chain_hash      TEXT NOT NULL,
    taken_at_ms     INTEGER NOT NULL,
    PRIMARY KEY (name, stream_id)
);

CREATE INDEX IF NOT EXISTS event_snapshots_v1_stream_idx
    ON event_snapshots_v1(stream_id);
"#;

#[cfg(test)]
mod tests {
    //! C3b (session-11) — T21 idempotency + reopen + chain, T22 SQLite contention.
    use super::*;
    use sddk_domain::{ActorKind, ActorRef, EventEnvelopeV1, EventStore};
    use serde_json::json;
    use std::collections::HashSet;
    use std::path::PathBuf;
    use std::sync::{Arc, Barrier};
    use tempfile::TempDir;

    /// Build a valid envelope with a correctly computed content_hash.
    /// `sequence=0` is set so the hash is stable regardless of caller context
    /// (per `compute_content_hash` semantics — sequence is excluded from the
    /// hash). `recorded_at` is a fixed non-empty RFC-3339 timestamp because
    /// the schema enforces `recorded_at <> ''`; the hash function resets
    /// it to "" before hashing, so the value choice does not affect the hash.
    fn envelope_with_event_id(event_id: &str, stream_id: &str) -> EventEnvelopeV1 {
        let mut env = EventEnvelopeV1 {
            event_id: event_id.to_string(),
            event_type: "uat.acceptance.granted".to_string(),
            schema_version: 1,
            stream_id: stream_id.to_string(),
            sequence: 0,
            project_id: "p-c3b-test".to_string(),
            occurred_at: "2026-09-22T00:00:00Z".to_string(),
            recorded_at: "2026-09-22T00:00:00Z".to_string(),
            actor: ActorRef {
                kind: ActorKind::Human,
                id: "operator-test".to_string(),
                definition_hash: None,
                policy_hash: None,
                model: None,
                role: None,
            },
            subjects: vec![],
            payload: json!({ "fixed": true }),
            evidence_refs: vec![],
            content_hash: String::new(),
            metadata: None,
            causation_id: None,
            correlation_id: None,
            cycle_id: None,
            frame_id: None,
            fork_id: None,
        };
        env.content_hash = env.compute_content_hash();
        env
    }

    /// T21 — re-append of the SAME event_id returns the original sequence
    /// (idempotent) without allocating a new sequence or new chain link.
    #[test]
    fn append_is_idempotent_for_same_event_id() {
        let mut store = SqliteEventStore::open_in_memory().expect("open");
        let env = envelope_with_event_id("evt-dup-001", "stream-dup");
        let first = store.append(&env).expect("first append");
        let second = store.append(&env).expect("second append");

        // Same sequence, same chain_hash, same recorded_at — the contract
        // documented at ports.rs:330-332.
        assert_eq!(first.sequence, 1);
        assert_eq!(first.sequence, second.sequence);
        assert_eq!(first.chain_hash, second.chain_hash);
        assert_eq!(first.recorded_at, second.recorded_at);

        // And only ONE row in the database.
        assert_eq!(store.count().expect("count"), 1);
        assert_eq!(store.last_sequence("stream-dup").expect("last"), Some(1));
    }

    /// T21 — a DIFFERENT envelope (different payload) colliding on event_id
    /// MUST be rejected with the typed guard instead of silently overwriting
    /// the stored event. The production code detects this at the dup-probe
    /// after `INSERT OR IGNORE` (event_store.rs:287-303) and returns
    /// `event_store:duplicate_event_id:<id>` — different from the
    /// pre-transaction `content_hash_mismatch` guard, but equally typed.
    /// Either guard is acceptable; the contract is "the stored event is
    /// not silently overwritten".
    #[test]
    fn append_rejects_event_id_collision_with_different_content() {
        let mut store = SqliteEventStore::open_in_memory().expect("open");
        // First envelope with payload {fixed: true}
        let env_a = envelope_with_event_id("evt-collide-001", "stream-c");
        let first = store.append(&env_a).expect("first append");
        assert_eq!(first.sequence, 1);

        // Second envelope with the SAME event_id but DIFFERENT payload.
        let mut env_b = envelope_with_event_id("evt-collide-001", "stream-c");
        env_b.payload = json!({ "fixed": false, "tampered": true });
        env_b.content_hash = env_b.compute_content_hash();
        let result = store.append(&env_b);
        // The stored event must not be silently overwritten. The exact
        // error code depends on which guard fires first — both are typed
        // and both are part of the documented contract.
        match &result {
            Err(sddk_domain::StorageError::Other(s))
                if s.contains("event_store:content_hash_mismatch")
                    || s.contains("event_store:duplicate_event_id") =>
            {
                // Acceptable: typed guard, content not overwritten.
            }
            other => panic!("expected typed content-collision guard, got {:?}", other),
        }

        // Original event must still be the only row.
        assert_eq!(store.count().expect("count"), 1);
        let reloaded = store
            .load_by_event_id("evt-collide-001")
            .expect("load")
            .expect("present");
        // Reloaded payload must be the FIRST one (not the tampered one).
        assert_eq!(reloaded.payload, json!({ "fixed": true }));
    }

    /// T21 — drop the connection, reopen on the same file, verify the
    /// chain_hash of the most-recent event matches what was computed at
    /// append time. This proves append durability under reopen.
    #[test]
    fn reopen_preserves_chain_and_sequence() {
        let dir = TempDir::new().expect("tempdir");
        let db_path: PathBuf = dir.path().join("ledger.sqlite");

        // Phase 1: append 5 events and capture the head-of-chain markers.
        let (head_hash, head_chain, last_seq) = {
            let mut store = SqliteEventStore::open_path(&db_path).expect("first open");
            let mut head_hash = None;
            let mut head_chain = None;
            let mut last_seq = 0u64;
            for i in 1..=5u64 {
                let env = envelope_with_event_id(&format!("evt-reopen-{i:03}"), "stream-reopen");
                let r = store.append(&env).expect("append");
                assert_eq!(r.sequence, i, "sequence must be contiguous per stream");
                head_hash = Some(r.content_hash);
                head_chain = Some(r.chain_hash);
                last_seq = r.sequence;
            }
            (head_hash, head_chain, last_seq)
            // `store` drops here → WAL checkpoints.
        };

        // Phase 2: reopen and verify chain continuity.
        let store = SqliteEventStore::open_path(&db_path).expect("reopen");
        assert_eq!(store.count().expect("count after reopen"), 5);
        assert_eq!(
            store.last_sequence("stream-reopen").expect("last"),
            Some(last_seq)
        );
        assert_eq!(
            store.head_hash("stream-reopen").expect("head").as_deref(),
            head_hash.as_deref()
        );
        assert_eq!(
            store.head_chain_hash("stream-reopen").expect("head chain"),
            head_chain
        );

        // Chain integrity must hold end-to-end.
        store
            .verify_chain_integrity("stream-reopen")
            .expect("chain integrity");
        store
            .verify_stream_chain("stream-reopen")
            .expect("stream chain");
    }

    /// T21 — multi-stream: distinct streams do not share sequence space
    /// and list_streams reports them all.
    #[test]
    fn multi_stream_isolates_sequences_and_lists_them() {
        let mut store = SqliteEventStore::open_in_memory().expect("open");
        for i in 1..=3u64 {
            let _ = store
                .append(&envelope_with_event_id(
                    &format!("evt-alpha-{i:03}"),
                    "stream-alpha",
                ))
                .expect("alpha");
            let _ = store
                .append(&envelope_with_event_id(
                    &format!("evt-beta-{i:03}"),
                    "stream-beta",
                ))
                .expect("beta");
        }
        let streams = store.list_streams().expect("list_streams");
        assert_eq!(streams, vec!["stream-alpha", "stream-beta"]);
        // Each stream got sequences 1..3 in isolation.
        assert_eq!(store.last_sequence("stream-alpha").expect("last"), Some(3));
        assert_eq!(store.last_sequence("stream-beta").expect("last"), Some(3));
    }

    /// T22 — concurrent appends from N threads to N distinct streams,
    /// each thread appending M events. Each thread opens its own
    /// `SqliteEventStore` pointing at the same `ledger.sqlite` — the
    /// production multi-connection model (rusqlite `Connection: Send`).
    /// The contention surface is the SQLite IMMEDIATE transaction +
    /// WAL + busy_timeout exercised on `append`.
    ///
    /// Open happens BEFORE the barrier so migrations run sequentially
    /// (SQLite cannot change journal_mode concurrently); the barrier
    /// aligns the threads so they enter `append` roughly simultaneously,
    /// which is where the contention actually matters.
    #[test]
    fn concurrent_append_across_distinct_streams_loses_no_events() {
        let dir = TempDir::new().expect("tempdir");
        let db_path = Arc::new(dir.path().join("ledger.sqlite"));

        const THREADS: usize = 4;
        const PER_THREAD: usize = 10;

        // Pre-open all stores BEFORE the barrier to serialize migrations
        // and avoid the `journal_mode: database is locked` race on
        // concurrent opens.
        let mut stores = Vec::with_capacity(THREADS);
        for _ in 0..THREADS {
            let p = (*db_path).clone();
            stores.push(SqliteEventStore::open_path(&p).expect("pre-open"));
        }

        let barrier = Arc::new(Barrier::new(THREADS));
        let mut handles = Vec::new();
        for t in 0..THREADS {
            let barrier = Arc::clone(&barrier);
            // Move the pre-opened store into the thread by wrapping it.
            // We use a `Vec<...>` index trick to avoid `&mut self` over
            // the whole Vec — each thread owns its own `SqliteEventStore`.
            let mut store = stores.pop().expect("store");
            handles.push(std::thread::spawn(move || {
                // Synchronize the start so all threads enter `append`
                // roughly simultaneously — this is the contention
                // surface we want to exercise.
                barrier.wait();
                let stream_id = format!("stream-t{t}");
                for i in 0..PER_THREAD {
                    let env = envelope_with_event_id(&format!("evt-t{t}-i{i:03}"), &stream_id);
                    let r = store.append(&env).expect("append");
                    assert_eq!(
                        r.sequence,
                        (i as u64) + 1,
                        "per-stream sequence must be contiguous"
                    );
                }
                store
            }));
        }

        // Re-collect the stores (kept alive across the test for clean Drop).
        let mut final_stores = Vec::new();
        for h in handles {
            final_stores.push(h.join().expect("thread join"));
        }

        // Single re-open to verify the post-condition.
        let store = SqliteEventStore::open_path(&*db_path).expect("verify open");
        let total = THREADS * PER_THREAD;
        assert_eq!(store.count().expect("count"), total as u64);

        // No duplicate event_ids.
        let mut all_event_ids = HashSet::new();
        for t in 0..THREADS {
            let stream = format!("stream-t{t}");
            let events = store
                .load_stream(&stream, None, PER_THREAD as u32)
                .expect("load");
            assert_eq!(
                events.len(),
                PER_THREAD,
                "stream {stream} must have all events"
            );
            for e in &events {
                assert!(
                    all_event_ids.insert(e.event_id.clone()),
                    "duplicate event_id {}",
                    e.event_id
                );
            }
            // Sequences per stream are 1..PER_THREAD.
            for (i, e) in events.iter().enumerate() {
                assert_eq!(e.sequence, (i as u64) + 1);
            }
        }
        assert_eq!(all_event_ids.len(), total);

        // Suppress unused warning: the stores must outlive the joins.
        drop(final_stores);
    }
}

// ── C3d (session-11) — T26-append microbench (opt-in via #[ignore]) ──────────

#[cfg(test)]
mod bench {
    //! Lightweight performance baseline for `SqliteEventStore::append`.
    //! Opt-in via `#[ignore]` because these tests do real I/O and take
    //! meaningful wall time. Run with:
    //!
    //! ```text
    //! cargo test -p sddk-storage --lib event_store::bench -- --ignored --nocapture
    //! ```
    use super::*;
    use sddk_domain::{ActorKind, ActorRef, EventEnvelopeV1, EventStore};
    use serde_json::json;
    use std::time::Instant;

    /// Local mirror of the helper in `super::tests`. Avoids exposing the
    /// helper from a sibling test module purely for the benchmark.
    fn bench_envelope(event_id: &str, stream_id: &str) -> EventEnvelopeV1 {
        let mut env = EventEnvelopeV1 {
            event_id: event_id.to_string(),
            event_type: "uat.acceptance.granted".to_string(),
            schema_version: 1,
            stream_id: stream_id.to_string(),
            sequence: 0,
            project_id: "p-c3d".to_string(),
            occurred_at: "2026-09-22T00:00:00Z".to_string(),
            recorded_at: "2026-09-22T00:00:00Z".to_string(),
            actor: ActorRef {
                kind: ActorKind::Human,
                id: "bench".to_string(),
                definition_hash: None,
                policy_hash: None,
                model: None,
                role: None,
            },
            subjects: vec![],
            payload: json!({ "bench": true }),
            evidence_refs: vec![],
            content_hash: String::new(),
            metadata: None,
            causation_id: None,
            correlation_id: None,
            cycle_id: None,
            frame_id: None,
            fork_id: None,
        };
        env.content_hash = env.compute_content_hash();
        env
    }

    /// T26-append: mean / p50 / p99 of `append` latency over 1000 events
    /// on a single stream, single thread. Print to stdout.
    #[test]
    #[ignore]
    fn bench_append_throughput() {
        let mut store = SqliteEventStore::open_in_memory().expect("open");
        const N: usize = 1000;

        // Warm-up: 10 ops to amortize first-call costs (SQLite prepare cache,
        // tempdir setup).
        for i in 0..10 {
            let env = bench_envelope(&format!("evt-warm-{i:04}"), "stream-bench");
            let _ = store.append(&env).expect("warm");
        }

        // Measure.
        let mut samples_us: Vec<u64> = Vec::with_capacity(N);
        for i in 0..N {
            let env = bench_envelope(&format!("evt-bench-{i:04}"), "stream-bench");
            let t0 = Instant::now();
            store.append(&env).expect("append");
            samples_us.push(t0.elapsed().as_micros() as u64);
        }

        let total: u64 = samples_us.iter().sum();
        let mean_us = total / N as u64;
        let mut sorted = samples_us.clone();
        sorted.sort_unstable();
        let p50_us = sorted[N / 2];
        let p99_us = sorted[(N as f64 * 0.99) as usize];

        println!("T26-append over N={N}: mean={mean_us} µs, p50={p50_us} µs, p99={p99_us} µs");

        // Sanity upper bound: 10 ms per append is conservative for SQLite
        // INSERT under WAL+busy_timeout. A regression here would mean the
        // path got catastrophically slower (e.g. unindexed scan).
        assert!(
            mean_us < 10_000,
            "mean append latency {mean_us} µs exceeds 10ms threshold"
        );
    }
}
