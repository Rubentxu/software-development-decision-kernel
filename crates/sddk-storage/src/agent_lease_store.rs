//! `SqliteLeaseStore` — the durable, multi-process-safe [`LeaseStore`].
//!
//! C3l.5 / AIW-S8 X04. Before this existed, `LeaseStore` had exactly one
//! implementation (`InMemoryLeaseStore`) and it lived in the engine, so the
//! package's exit gate — *"at least two PIDs and a shared durable SQLite
//! ledger"* — was unreachable **by construction**, not by accident.
//!
//! ## Concurrency contract
//!
//! The acquire path is a single `BEGIN IMMEDIATE` transaction, so two OS
//! processes racing on the same file serialise at the write lock rather than
//! both reading "free" and both winning. `busy_timeout` plus the retry ladder
//! borrowed from [`crate::Storage`] (INC-DEBT-029) turn contention into
//! latency instead of a propagated `database is locked`.
//!
//! Fencing is compare-and-swap: `release` only applies when `(owner,
//! fencing_token)` still match, and `acquire` always increments the token —
//! so a token is never reused, which is what makes a stale holder detectable
//! after a crash.

use std::path::Path;
use std::time::Duration;

use sddk_domain::ports::{LeaseError, LeaseRecord, LeaseStore};

const BUSY_TIMEOUT_MS: u32 = 5_000;
const MAX_RETRIES: u32 = 10;
const BASE_DELAY_MS: u64 = 20;
const MAX_DELAY_MS: u64 = 400;

/// Durable lease store backed by a SQLite file.
#[derive(Debug)]
pub struct SqliteLeaseStore {
    path: std::path::PathBuf,
}

impl SqliteLeaseStore {
    /// Opens (or creates) the lease database at `path`.
    pub fn open(path: impl AsRef<Path>) -> Result<Self, LeaseError> {
        let path = path.as_ref().to_path_buf();
        if let Some(parent) = path.parent()
            && !parent.as_os_str().is_empty()
        {
            std::fs::create_dir_all(parent).map_err(|e| LeaseError::Storage(e.to_string()))?;
        }
        let store = Self { path };
        store.with_conn(|conn| {
            conn.pragma_update(None, "journal_mode", "WAL")
                .map_err(|e| LeaseError::Storage(e.to_string()))?;
            conn.pragma_update(None, "busy_timeout", BUSY_TIMEOUT_MS)
                .map_err(|e| LeaseError::Storage(e.to_string()))?;
            conn.execute_batch(
                "CREATE TABLE IF NOT EXISTS agent_leases (
                     cycle_id      TEXT PRIMARY KEY,
                     owner         TEXT NOT NULL,
                     fencing_token INTEGER NOT NULL,
                     acquired_at_ms INTEGER NOT NULL,
                     expires_at_ms  INTEGER NOT NULL
                 );",
            )
            .map_err(|e| LeaseError::Storage(e.to_string()))
        })?;
        Ok(store)
    }

    /// The file this store is bound to.
    pub fn path(&self) -> &Path {
        &self.path
    }

    fn connect(&self) -> Result<rusqlite::Connection, LeaseError> {
        rusqlite::Connection::open(&self.path).map_err(|e| LeaseError::Storage(e.to_string()))
    }

    /// Opens a connection, runs `f`, and retries only on `DatabaseBusy`.
    ///
    /// Retrying a *read* would be wrong (it hides nothing but wastes time), so
    /// the ladder is applied at the write sites, matching the `Storage` policy.
    fn with_retry<T>(
        &self,
        mut op: impl FnMut(&rusqlite::Connection) -> Result<T, LeaseError>,
    ) -> Result<T, LeaseError> {
        let mut attempt = 0u32;
        loop {
            let conn = self.connect()?;
            match op(&conn) {
                Ok(v) => return Ok(v),
                Err(LeaseError::Storage(msg)) if msg.contains("database is locked") => {
                    if attempt >= MAX_RETRIES {
                        return Err(LeaseError::Storage(msg));
                    }
                    let exp = BASE_DELAY_MS.saturating_mul(1u64 << attempt.min(5));
                    std::thread::sleep(Duration::from_millis(exp.min(MAX_DELAY_MS)));
                    attempt += 1;
                }
                Err(e) => return Err(e),
            }
        }
    }

    fn with_conn<T>(
        &self,
        f: impl FnOnce(&rusqlite::Connection) -> Result<T, LeaseError>,
    ) -> Result<T, LeaseError> {
        let conn = self.connect()?;
        f(&conn)
    }

    fn read_record(
        conn: &rusqlite::Connection,
        cycle_id: &str,
    ) -> Result<Option<LeaseRecord>, LeaseError> {
        let mut stmt = conn
            .prepare(
                "SELECT cycle_id, owner, fencing_token, acquired_at_ms, expires_at_ms
                 FROM agent_leases WHERE cycle_id = ?1",
            )
            .map_err(|e| LeaseError::Storage(e.to_string()))?;
        let mut rows = stmt
            .query([cycle_id])
            .map_err(|e| LeaseError::Storage(e.to_string()))?;
        let row = rows
            .next()
            .map_err(|e| LeaseError::Storage(e.to_string()))?;
        match row {
            None => Ok(None),
            Some(r) => Ok(Some(LeaseRecord {
                cycle_id: r.get(0).map_err(|e| LeaseError::Storage(e.to_string()))?,
                owner: r.get(1).map_err(|e| LeaseError::Storage(e.to_string()))?,
                fencing_token: r.get(2).map_err(|e| LeaseError::Storage(e.to_string()))?,
                acquired_at_ms: r.get(3).map_err(|e| LeaseError::Storage(e.to_string()))?,
                expires_at_ms: r.get(4).map_err(|e| LeaseError::Storage(e.to_string()))?,
            })),
        }
    }
}

impl LeaseStore for SqliteLeaseStore {
    fn acquire(
        &self,
        cycle_id: &str,
        owner: &str,
        now_ms: i64,
        expires_at_ms: i64,
    ) -> Result<LeaseRecord, LeaseError> {
        // BEGIN IMMEDIATE takes the write lock up front, so the read of the
        // current holder and the write of the new one are one atomic step.
        // Without it two processes can both observe "free" and both win.
        self.with_retry(|conn| {
            conn.execute_batch("BEGIN IMMEDIATE")
                .map_err(|e| LeaseError::Storage(e.to_string()))?;
            let result = (|| -> Result<LeaseRecord, LeaseError> {
                let current = Self::read_record(conn, cycle_id)?;
                if let Some(cur) = &current
                    && cur.expires_at_ms > now_ms
                    && cur.owner != owner
                {
                    return Err(LeaseError::Conflict {
                        owner: cur.owner.clone(),
                        fencing_token: cur.fencing_token,
                    });
                }
                let new_token = current.as_ref().map_or(1, |c| c.fencing_token + 1);
                conn.execute(
                    "INSERT INTO agent_leases
                        (cycle_id, owner, fencing_token, acquired_at_ms, expires_at_ms)
                     VALUES (?1, ?2, ?3, ?4, ?5)
                     ON CONFLICT(cycle_id) DO UPDATE SET
                        owner = excluded.owner,
                        fencing_token = excluded.fencing_token,
                        acquired_at_ms = excluded.acquired_at_ms,
                        expires_at_ms = excluded.expires_at_ms",
                    rusqlite::params![cycle_id, owner, new_token, now_ms, expires_at_ms],
                )
                .map_err(|e| LeaseError::Storage(e.to_string()))?;
                Ok(LeaseRecord {
                    cycle_id: cycle_id.to_owned(),
                    owner: owner.to_owned(),
                    fencing_token: new_token,
                    acquired_at_ms: now_ms,
                    expires_at_ms,
                })
            })();
            // Commit on success, roll back on conflict, so a loser leaves the
            // table byte-identical to how it found it.
            let _ = if result.is_ok() {
                conn.execute_batch("COMMIT")
            } else {
                conn.execute_batch("ROLLBACK")
            };
            result
        })
    }

    fn release(&self, cycle_id: &str, owner: &str, fencing_token: i64) -> Result<bool, LeaseError> {
        self.with_retry(|conn| {
            conn.execute_batch("BEGIN IMMEDIATE")
                .map_err(|e| LeaseError::Storage(e.to_string()))?;
            let result = (|| -> Result<bool, LeaseError> {
                let current = Self::read_record(conn, cycle_id)?;
                let Some(cur) = current else {
                    return Ok(false);
                };
                if cur.owner != owner || cur.fencing_token != fencing_token {
                    return Ok(false); // stale holder: not an error, just not applied
                }
                // Clear the holder but KEEP the row and its fencing_token.
                //
                // Deleting the row here would reset the counter to 1 on the
                // next acquire, re-issuing a token this cycle already used —
                // a stale holder from before the release would then look
                // current. Monotonicity is per-cycle, not per-lease.
                let n = conn
                    .execute(
                        "UPDATE agent_leases
                            SET owner = '', expires_at_ms = 0
                          WHERE cycle_id = ?1",
                        [cycle_id],
                    )
                    .map_err(|e| LeaseError::Storage(e.to_string()))?;
                Ok(n > 0)
            })();
            let _ = if result.is_ok() {
                conn.execute_batch("COMMIT")
            } else {
                conn.execute_batch("ROLLBACK")
            };
            result
        })
    }
}

/// Reads the current record without acquiring. Used by tests to assert what
/// the durable state actually is, rather than what a caller believes.
pub fn peek(path: &Path, cycle_id: &str) -> Result<Option<LeaseRecord>, LeaseError> {
    let store = SqliteLeaseStore::open(path)?;
    store.with_conn(|conn| SqliteLeaseStore::read_record(conn, cycle_id))
}
