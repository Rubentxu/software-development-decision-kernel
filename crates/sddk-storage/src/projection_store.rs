//! Persists projection checkpoints to `projection_checkpoints_v1`.

use std::path::Path;

use rusqlite::{OptionalExtension, params};

use sddk_domain::{Checkpoint, ProjectionVersion, StorageError};

/// SQLite-backed projection checkpoint store.
pub struct SqliteProjectionStore {
    conn: rusqlite::Connection,
}

impl SqliteProjectionStore {
    /// Borrows the underlying SQLite connection (read-only).
    pub fn conn(&self) -> &rusqlite::Connection {
        &self.conn
    }

    /// Borrows the underlying SQLite connection (mutable).
    pub fn conn_mut(&mut self) -> &mut rusqlite::Connection {
        &mut self.conn
    }

    /// Opens (or creates) a `ledger.sqlite` file at `$dir/ledger.sqlite` and
    /// applies all pending migrations.
    pub fn open(dir: &Path) -> Result<Self, StorageError> {
        let conn = rusqlite::Connection::open(dir.join("ledger.sqlite"))
            .map_err(|e| StorageError::Database(format!("open: {e}")))?;
        let mut conn = conn;
        crate::migrations::run_migrations(&mut conn)?;
        Ok(Self { conn })
    }

    /// Opens an isolated in-memory database with all migrations applied.
    /// Useful for tests.
    pub fn open_in_memory() -> Result<Self, StorageError> {
        let conn = rusqlite::Connection::open_in_memory()
            .map_err(|e| StorageError::Database(format!("open_in_memory: {e}")))?;
        let mut conn = conn;
        crate::migrations::run_migrations(&mut conn)?;
        Ok(Self { conn })
    }

    /// Persists a checkpoint and its serialized state.
    ///
    /// Uses `INSERT OR REPLACE` so repeated rebuilds overwrite the prior
    /// checkpoint for the same `(projection_name, version)` key.
    pub fn save_checkpoint(
        &mut self,
        cp: &Checkpoint,
        state_json: &str,
    ) -> Result<(), StorageError> {
        self.conn
            .execute(
                "INSERT OR REPLACE INTO projection_checkpoints_v1
                 (projection_name, version, last_event_sequence, last_event_hash, state_json, updated_at)
                 VALUES (?1, ?2, ?3, ?4, ?5, ?6)",
                params![
                    cp.projection_name,
                    i64::from(cp.version),
                    i64::try_from(cp.last_event_sequence).unwrap_or(i64::MAX),
                    cp.last_event_hash,
                    state_json,
                    cp.updated_at,
                ],
            )
            .map_err(|e| StorageError::Database(format!("save_checkpoint: {e}")))?;
        Ok(())
    }

    /// Loads the checkpoint and serialized state for a projection, if one exists.
    pub fn load_checkpoint(
        &self,
        projection_name: &str,
        version: ProjectionVersion,
    ) -> Result<Option<(Checkpoint, String)>, StorageError> {
        let mut stmt = self
            .conn
            .prepare(
                "SELECT projection_name, version, last_event_sequence, last_event_hash,
                        state_json, updated_at
                 FROM projection_checkpoints_v1
                 WHERE projection_name = ?1 AND version = ?2",
            )
            .map_err(|e| StorageError::Database(format!("load_checkpoint prep: {e}")))?;

        let row = stmt
            .query_row(params![projection_name, i64::from(version)], |row| {
                let cp = Checkpoint {
                    projection_name: row.get(0)?,
                    version: row.get::<_, i64>(1)? as u32,
                    last_event_sequence: row.get::<_, i64>(2)? as u64,
                    last_event_hash: row.get(3)?,
                    updated_at: row.get(5)?,
                };
                let state_json: String = row.get(4)?;
                Ok((cp, state_json))
            })
            .optional()
            .map_err(|e| StorageError::Database(format!("load_checkpoint query: {e}")))?;

        Ok(row)
    }

    /// Deletes the checkpoint for a projection, if one exists.
    /// Idempotent: calling this when no checkpoint exists is not an error.
    pub fn delete_checkpoint(
        &self,
        projection_name: &str,
        version: ProjectionVersion,
    ) -> Result<(), StorageError> {
        self.conn
            .execute(
                "DELETE FROM projection_checkpoints_v1
                 WHERE projection_name = ?1 AND version = ?2",
                params![projection_name, i64::from(version)],
            )
            .map_err(|e| StorageError::Database(format!("delete_checkpoint: {e}")))?;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn save_then_load_returns_same_checkpoint() {
        let mut store = SqliteProjectionStore::open_in_memory().unwrap();
        let cp = Checkpoint {
            projection_name: "cycle_state".into(),
            version: 1,
            last_event_sequence: 42,
            last_event_hash:
                "sha256:0000000000000000000000000000000000000000000000000000000000000000".into(),
            updated_at: "2026-08-17T10:00:00Z".into(),
        };
        let state = r#"{"phase":"build"}"#;
        store.save_checkpoint(&cp, state).unwrap();
        let (loaded, loaded_state) = store.load_checkpoint("cycle_state", 1).unwrap().unwrap();
        assert_eq!(loaded, cp);
        assert_eq!(loaded_state, state);
    }

    #[test]
    fn load_missing_returns_none() {
        let store = SqliteProjectionStore::open_in_memory().unwrap();
        let result = store.load_checkpoint("nonexistent", 1).unwrap();
        assert!(result.is_none());
    }

    #[test]
    fn open_in_memory_runs_migrations() {
        // Smoke test: constructing proves migrations ran without error.
        let store = SqliteProjectionStore::open_in_memory().unwrap();
        let _ = store;
    }

    #[test]
    fn delete_checkpoint_is_idempotent() {
        let mut store = SqliteProjectionStore::open_in_memory().unwrap();
        let cp = Checkpoint {
            projection_name: "cycle_state".into(),
            version: 1,
            last_event_sequence: 1,
            last_event_hash:
                "sha256:0000000000000000000000000000000000000000000000000000000000000000".into(),
            updated_at: "2026-08-17T10:00:00Z".into(),
        };
        store.save_checkpoint(&cp, r#"{"phase":"build"}"#).unwrap();
        store.delete_checkpoint("cycle_state", 1).unwrap();
        store.delete_checkpoint("cycle_state", 1).unwrap(); // second call is OK
        assert!(store.load_checkpoint("cycle_state", 1).unwrap().is_none());
    }
}
