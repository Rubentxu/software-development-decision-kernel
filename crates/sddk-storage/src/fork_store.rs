//! SQLite adapter for the `ForkStore` port (SPEC-009 §3, Phase 7).

use std::path::Path;

use rusqlite::{Connection, OptionalExtension, params};
use sddk_domain::{
    CachedResponse, ForkInput, ForkRecord, ForkStore, ReplayPolicy, ResponseCachePort, StorageError,
};

/// SQLite-backed fork store and response cache.
pub struct SqliteForkStore {
    /// Database connection.
    conn: Connection,
}

impl SqliteForkStore {
    /// Opens (or creates) the ledger database at `dir/ledger.sqlite`.
    pub fn open(dir: &Path) -> Result<Self, StorageError> {
        let conn = Connection::open(dir.join("ledger.sqlite"))
            .map_err(|e| StorageError::Database(format!("open: {e}")))?;
        conn.pragma_update(None, "journal_mode", "WAL")
            .map_err(|e| StorageError::Database(format!("journal_mode: {e}")))?;
        let mut conn = conn;
        crate::migrations::run_migrations(&mut conn)?;
        Ok(Self { conn })
    }

    /// Opens an isolated in-memory database (tests).
    pub fn open_in_memory() -> Result<Self, StorageError> {
        let conn = Connection::open_in_memory()
            .map_err(|e| StorageError::Database(format!("open_in_memory: {e}")))?;
        let mut conn = conn;
        crate::migrations::run_migrations(&mut conn)?;
        Ok(Self { conn })
    }

    fn row_to_fork(row: &rusqlite::Row<'_>) -> rusqlite::Result<ForkRecord> {
        let overrides_json: String = row.get(5)?;
        let overrides = serde_json::from_str(&overrides_json).unwrap_or_default();
        let policy: String = row.get(8)?;
        Ok(ForkRecord {
            fork_id: row.get(0)?,
            parent_stream_id: row.get(1)?,
            at_sequence: row.get::<_, i64>(2)? as u64,
            shared_prefix_hash: row.get(3)?,
            label: row.get(4)?,
            overrides,
            creator: row.get(6)?,
            created_at: row.get(7)?,
            replay_policy: if policy == "strict" {
                ReplayPolicy::Strict
            } else {
                ReplayPolicy::Reconstruct
            },
        })
    }
}

impl ForkStore for SqliteForkStore {
    fn create_fork(
        &mut self,
        input: ForkInput,
        creator: &str,
        created_at: &str,
        prefix_hash: &str,
    ) -> Result<ForkRecord, StorageError> {
        let exists: bool = self
            .conn
            .query_row(
                "SELECT 1 FROM forks_v1 WHERE fork_id = ?1",
                params![input.fork_id],
                |_| Ok(true),
            )
            .unwrap_or(false);
        if exists {
            return Err(StorageError::Other(format!(
                "fork already exists: {}",
                input.fork_id
            )));
        }
        let overrides_json = serde_json::to_string(&input.overrides)
            .map_err(|e| StorageError::Database(format!("overrides serialize: {e}")))?;
        let policy = match input.replay_policy {
            ReplayPolicy::Reconstruct => "reconstruct",
            ReplayPolicy::Strict => "strict",
        };
        self.conn
            .execute(
                "INSERT INTO forks_v1
                 (fork_id, parent_stream_id, at_sequence, shared_prefix_hash, label,
                  overrides_json, creator, created_at, replay_policy)
                 VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9)",
                params![
                    input.fork_id,
                    input.parent_stream_id,
                    i64::try_from(input.at_sequence).unwrap_or(i64::MAX),
                    prefix_hash,
                    input.label,
                    overrides_json,
                    creator,
                    created_at,
                    policy,
                ],
            )
            .map_err(|e| StorageError::Database(format!("insert fork: {e}")))?;
        Ok(ForkRecord {
            fork_id: input.fork_id,
            parent_stream_id: input.parent_stream_id,
            at_sequence: input.at_sequence,
            shared_prefix_hash: prefix_hash.to_string(),
            label: input.label,
            overrides: input.overrides,
            creator: creator.to_string(),
            created_at: created_at.to_string(),
            replay_policy: input.replay_policy,
        })
    }

    fn load_fork(&self, fork_id: &str) -> Result<Option<ForkRecord>, StorageError> {
        let mut stmt = self
            .conn
            .prepare(
                "SELECT fork_id, parent_stream_id, at_sequence, shared_prefix_hash, label,
                        overrides_json, creator, created_at, replay_policy
                 FROM forks_v1 WHERE fork_id = ?1",
            )
            .map_err(|e| StorageError::Database(format!("load_fork prep: {e}")))?;
        let row = stmt
            .query_row(params![fork_id], Self::row_to_fork)
            .optional()
            .map_err(|e| StorageError::Database(format!("load_fork query: {e}")))?;
        Ok(row)
    }

    fn list_forks(&self) -> Result<Vec<ForkRecord>, StorageError> {
        let mut stmt = self
            .conn
            .prepare(
                "SELECT fork_id, parent_stream_id, at_sequence, shared_prefix_hash, label,
                        overrides_json, creator, created_at, replay_policy
                 FROM forks_v1 ORDER BY created_at ASC",
            )
            .map_err(|e| StorageError::Database(format!("list_forks prep: {e}")))?;
        let rows = stmt
            .query_map([], Self::row_to_fork)
            .map_err(|e| StorageError::Database(format!("list_forks query: {e}")))?;
        rows.collect::<Result<Vec<_>, _>>()
            .map_err(|e| StorageError::Database(format!("list_forks collect: {e}")))
    }
}

impl ResponseCachePort for SqliteForkStore {
    fn get_response(&self, request_hash: &str) -> Result<Option<CachedResponse>, StorageError> {
        let mut stmt = self
            .conn
            .prepare(
                "SELECT request_hash, response_json, model, created_at
                 FROM response_cache_v1 WHERE request_hash = ?1",
            )
            .map_err(|e| StorageError::Database(format!("get_response prep: {e}")))?;
        let row = stmt
            .query_row(params![request_hash], |row| {
                Ok(CachedResponse {
                    request_hash: row.get(0)?,
                    response_json: row.get(1)?,
                    model: row.get(2)?,
                    created_at: row.get(3)?,
                })
            })
            .optional()
            .map_err(|e| StorageError::Database(format!("get_response query: {e}")))?;
        Ok(row)
    }

    fn put_response(&mut self, entry: CachedResponse) -> Result<(), StorageError> {
        self.conn
            .execute(
                "INSERT OR REPLACE INTO response_cache_v1
                 (request_hash, response_json, model, created_at)
                 VALUES (?1, ?2, ?3, ?4)",
                params![
                    entry.request_hash,
                    entry.response_json,
                    entry.model,
                    entry.created_at,
                ],
            )
            .map_err(|e| StorageError::Database(format!("put_response: {e}")))?;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::BTreeMap;

    fn fork_input(id: &str) -> ForkInput {
        ForkInput {
            fork_id: id.into(),
            parent_stream_id: "project:p-1".into(),
            at_sequence: 3,
            label: Some("exp".into()),
            overrides: BTreeMap::from([("model".into(), "gpt-x".into())]),
            replay_policy: ReplayPolicy::Strict,
        }
    }

    #[test]
    fn create_load_list_roundtrip() {
        let mut store = SqliteForkStore::open_in_memory().unwrap();
        let record = store
            .create_fork(
                fork_input("f-1"),
                "alice",
                "2026-08-18T10:00:00Z",
                "sha256:abc",
            )
            .unwrap();
        assert_eq!(record.shared_prefix_hash, "sha256:abc");
        assert_eq!(record.replay_policy, ReplayPolicy::Strict);

        let loaded = store.load_fork("f-1").unwrap().unwrap();
        assert_eq!(loaded, record);
        assert_eq!(store.load_fork("f-404").unwrap(), None);

        let list = store.list_forks().unwrap();
        assert_eq!(list.len(), 1);
        assert_eq!(list[0].fork_id, "f-1");
    }

    #[test]
    fn duplicate_fork_rejected() {
        let mut store = SqliteForkStore::open_in_memory().unwrap();
        store
            .create_fork(
                fork_input("f-1"),
                "alice",
                "2026-08-18T10:00:00Z",
                "sha256:abc",
            )
            .unwrap();
        let error = store
            .create_fork(
                fork_input("f-1"),
                "bob",
                "2026-08-18T10:00:00Z",
                "sha256:abc",
            )
            .unwrap_err();
        assert!(error.to_string().contains("fork already exists"));
    }

    #[test]
    fn list_orders_by_created_at() {
        let mut store = SqliteForkStore::open_in_memory().unwrap();
        store
            .create_fork(fork_input("f-1"), "a", "2026-08-18T10:00:00Z", "sha256:1")
            .unwrap();
        store
            .create_fork(fork_input("f-2"), "a", "2026-08-18T09:00:00Z", "sha256:2")
            .unwrap();
        let list = store.list_forks().unwrap();
        assert_eq!(list[0].fork_id, "f-2"); // earlier created_at first
        assert_eq!(list[1].fork_id, "f-1");
    }

    #[test]
    fn cache_put_get_roundtrip() {
        let mut store = SqliteForkStore::open_in_memory().unwrap();
        store
            .put_response(CachedResponse {
                request_hash: "rh-1".into(),
                response_json: r#"{"ok":true}"#.into(),
                model: Some("gpt-x".into()),
                created_at: "2026-08-18T10:00:00Z".into(),
            })
            .unwrap();
        let got = store.get_response("rh-1").unwrap().unwrap();
        assert_eq!(got.response_json, r#"{"ok":true}"#);
        assert_eq!(got.model.as_deref(), Some("gpt-x"));
        assert_eq!(store.get_response("rh-404").unwrap(), None);
    }

    #[test]
    fn cache_put_replaces() {
        let mut store = SqliteForkStore::open_in_memory().unwrap();
        store
            .put_response(CachedResponse {
                request_hash: "rh-1".into(),
                response_json: "v1".into(),
                model: None,
                created_at: "2026-08-18T10:00:00Z".into(),
            })
            .unwrap();
        store
            .put_response(CachedResponse {
                request_hash: "rh-1".into(),
                response_json: "v2".into(),
                model: None,
                created_at: "2026-08-18T11:00:00Z".into(),
            })
            .unwrap();
        assert_eq!(
            store.get_response("rh-1").unwrap().unwrap().response_json,
            "v2"
        );
    }
}
