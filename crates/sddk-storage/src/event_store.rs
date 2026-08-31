//! SQLite-backed [`sddk_domain::EventStore`] adapter for the Common Event Protocol v1.
//!
//! Persists [`sddk_domain::EventEnvelopeV1`] to the `events_v1` table. Co-exists
//! with `ledger_events` (legacy ledger bookkeeping) — they are independent tables
//! within the same `ledger.sqlite` file.
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
//! `busy_timeout=5s` + WAL mode. A future cycle (SDDK2-203/204) that needs
//! cross-table atomic transactions between `ledger_events` and `events_v1`
//! will need to merge the two connections into one.

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

    fn run_migrations(conn: &mut Connection) -> Result<(), DomainStorageError> {
        // SqliteEventStore manages its own schema version via a private pragma,
        // leaving user_version for Storage (full schema) to manage.
        // This prevents version conflicts when both stores share the same file.
        let version: i32 = conn
            .pragma_query_value(None, "sddk_eventstore_version", |row| row.get(0))
            .unwrap_or(0);
        if version < 5 {
            let tx = conn
                .transaction_with_behavior(rusqlite::TransactionBehavior::Immediate)
                .map_err(|e| DomainStorageError::Database(e.to_string()))?;
            tx.execute_batch(crate::migrations::MIGRATION_5)
                .map_err(|e| DomainStorageError::Database(e.to_string()))?;
            tx.pragma_update(None, "sddk_eventstore_version", 5)
                .map_err(|e| DomainStorageError::Database(e.to_string()))?;
            tx.commit()
                .map_err(|e| DomainStorageError::Database(e.to_string()))?;
        }
        if version < 6 {
            let tx = conn
                .transaction_with_behavior(rusqlite::TransactionBehavior::Immediate)
                .map_err(|e| DomainStorageError::Database(e.to_string()))?;
            tx.execute_batch(crate::migrations::MIGRATION_6)
                .map_err(|e| DomainStorageError::Database(e.to_string()))?;
            tx.pragma_update(None, "sddk_eventstore_version", 6)
                .map_err(|e| DomainStorageError::Database(e.to_string()))?;
            tx.commit()
                .map_err(|e| DomainStorageError::Database(e.to_string()))?;
        }
        if version < 10 {
            let tx = conn
                .transaction_with_behavior(rusqlite::TransactionBehavior::Immediate)
                .map_err(|e| DomainStorageError::Database(e.to_string()))?;
            // Defensively check if events_v1 exists before ALTERing, since databases
            // created before MIGRATION_5 never had the events_v1 table.
            let table_exists: bool = tx
                .query_row(
                    "SELECT 1 FROM sqlite_master WHERE type='table' AND name='events_v1'",
                    [],
                    |_row| Ok(true),
                )
                .unwrap_or(false);
            if table_exists {
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
                    .map_err(|e| DomainStorageError::Database(e.to_string()))?;
                }
            }
            tx.pragma_update(None, "sddk_eventstore_version", 10)
                .map_err(|e| DomainStorageError::Database(e.to_string()))?;
            tx.commit()
                .map_err(|e| DomainStorageError::Database(e.to_string()))?;
        }
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

        // 2. Ensure the project row exists (satisfies FK constraint).
        tx.execute(
            "INSERT OR IGNORE INTO projects (project_id) VALUES (?1)",
            rusqlite::params![envelope.project_id],
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
        let _rows_affected = tx
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
