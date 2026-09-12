//! `BacklogStore` trait and `SqliteBacklogStore` implementation.
//!
//! Cycle: `p-63676b11dc0ef88f/backlog-ledger-substrate`.
//!
//! Implements the engine-side persistence for the Backlog Ledger:
//! - [[REQ-Backlog-Item-Capture]] (the `BacklogItemRegistered` event)
//! - [[REQ-Backlog-Item-Triage-Priority]] (the `BacklogItemTriaged` event)
//! - [[REQ-Backlog-Item-Promote-Discard]] (the `BacklogItemPromoted` +
//!   `BacklogItemDiscarded` events)
//! - [[REQ-Backlog-Roadmap-Projection]] (the `live_items` predicate)
//!
//! CLI surface is deferred to subsequent cycles. This module ships the
//! storage layer that CLI cycle 2/4 (`backlog capture`), 3/4 (`backlog
//! triage / promote / discard`), and 4/4 (`backlog render` /
//! `roadmap render`) will all read from and write to.

use sddk_domain::backlog::{
    BacklogError, BacklogEventLogEntry, BacklogItemId, BacklogItemRow, BacklogPriority,
    BacklogStatus,
};

// Local helper: convert a rusqlite error into a BacklogError::Storage.
// We can't `impl From<rusqlite::Error> for BacklogError` here because
// of the orphan rule (both types are foreign to this crate), so we
// surface rusqlite errors via this helper at every `?` site.
fn sqlite_err(e: rusqlite::Error) -> BacklogError {
    BacklogError::Storage(e.to_string())
}

/// A single backlog ledger event (Fact per ADR-0094).
///
/// The `BacklogStore::append_event` method accepts this and writes
/// one row to `backlog_item_events_v1` plus one UPSERT into
/// `backlog_items_v1` (materialising the new state).
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum BacklogEvent {
    /// `backlog.item.registered` event.
    Registered {
        /// Stable item id.
        item_id: BacklogItemId,
        /// Origin cycle id.
        origin_cycle_id: String,
        /// Origin phase (e.g. "explore", "specify").
        origin_phase: String,
        /// Origin artifacts (e.g. ["spec.md"]).
        origin_artifacts: Vec<String>,
        /// One-line summary.
        summary: String,
        /// RFC3339 timestamp.
        captured_at: String,
        /// Actor reference (e.g. `human:alice`).
        actor_ref: Option<String>,
    },
    /// `backlog.item.triaged` event.
    Triaged {
        /// Stable item id.
        item_id: BacklogItemId,
        /// New priority (closed set P0..P3).
        priority: BacklogPriority,
        /// Monotonic priority version (1 on first triage, increments on re-triage).
        priority_version: u32,
        /// RFC3339 valid_from (prior valid_to becomes this minus 1).
        valid_from: String,
        /// RFC3339 valid_to (∞ == "9999-12-31T23:59:59Z" sentinel).
        valid_to: String,
        /// Actor reference.
        actor_ref: Option<String>,
    },
    /// `backlog.item.promoted` event.
    Promoted {
        /// Stable item id.
        item_id: BacklogItemId,
        /// Target kind (e.g. "cycle", "adr").
        target_kind: String,
        /// Target id.
        target_id: String,
        /// RFC3339 timestamp.
        promoted_at: String,
        /// Actor reference.
        actor_ref: Option<String>,
    },
    /// `backlog.item.discarded` event.
    Discarded {
        /// Stable item id.
        item_id: BacklogItemId,
        /// Discard reason (non-empty per schema validation).
        reason: String,
        /// RFC3339 timestamp.
        discarded_at: String,
        /// Actor reference.
        actor_ref: Option<String>,
    },
}

impl BacklogEvent {
    /// Returns the event type string ("backlog.item.registered", etc.).
    pub fn event_type(&self) -> &'static str {
        match self {
            Self::Registered { .. } => "backlog.item.registered",
            Self::Triaged { .. } => "backlog.item.triaged",
            Self::Promoted { .. } => "backlog.item.promoted",
            Self::Discarded { .. } => "backlog.item.discarded",
        }
    }

    /// Returns the schema version (always 1 in this cycle).
    pub fn schema_version(&self) -> u32 {
        1
    }

    /// Serialises the event payload to JSON.
    pub fn payload_json(&self) -> Result<serde_json::Value, BacklogError> {
        let v = match self {
            Self::Registered {
                item_id,
                origin_cycle_id,
                origin_phase,
                origin_artifacts,
                summary,
                captured_at,
                actor_ref,
            } => serde_json::json!({
                "item_id": item_id,
                "origin_cycle_id": origin_cycle_id,
                "origin_phase": origin_phase,
                "origin_artifacts": origin_artifacts,
                "summary": summary,
                "captured_at": captured_at,
                "actor_ref": actor_ref,
            }),
            Self::Triaged {
                item_id,
                priority,
                priority_version,
                valid_from,
                valid_to,
                actor_ref,
            } => serde_json::json!({
                "item_id": item_id,
                "priority": priority.to_string(),
                "priority_version": priority_version,
                "valid_from": valid_from,
                "valid_to": valid_to,
                "actor_ref": actor_ref,
            }),
            Self::Promoted {
                item_id,
                target_kind,
                target_id,
                promoted_at,
                actor_ref,
            } => serde_json::json!({
                "item_id": item_id,
                "target_kind": target_kind,
                "target_id": target_id,
                "promoted_at": promoted_at,
                "actor_ref": actor_ref,
            }),
            Self::Discarded {
                item_id,
                reason,
                discarded_at,
                actor_ref,
            } => serde_json::json!({
                "item_id": item_id,
                "reason": reason,
                "discarded_at": discarded_at,
                "actor_ref": actor_ref,
            }),
        };
        Ok(v)
    }

    /// Returns the `item_id` of the event (extracted from the variant).
    pub fn item_id(&self) -> &str {
        match self {
            Self::Registered { item_id, .. } => item_id,
            Self::Triaged { item_id, .. } => item_id,
            Self::Promoted { item_id, .. } => item_id,
            Self::Discarded { item_id, .. } => item_id,
        }
    }
}

/// Storage trait for the Backlog Ledger.
///
/// Mirrors the `GraphStore` precedent (DW-RUNTIME-002 cycle): the engine
/// consumes this trait; the CLI commands in cycles 2/3/4 consume it
/// too. `SqliteBacklogStore` is the only production impl.
pub trait BacklogStore {
    /// Returns all live items (status in {Registered, Triaged}), ordered
    /// by `item_id` ASC for deterministic renderer output.
    ///
    /// Implements the predicate from
    /// [[REQ-Backlog-Roadmap-Projection]] §Scenario 1.
    fn live_items(&mut self) -> Result<Vec<BacklogItemRow>, BacklogError>;

    /// Returns a single item by id, or None if not found.
    fn item(&mut self, id: &BacklogItemId) -> Result<Option<BacklogItemRow>, BacklogError>;

    /// Appends one event to the ledger and materialises the resulting
    /// item state in a single transaction. Returns the new event_id.
    fn append_event(&mut self, event: &BacklogEvent) -> Result<i64, BacklogError>;

    /// Returns the chronological event log for one item
    /// (empty vec if the item has no events — including the
    /// case where the item id itself doesn't exist).
    ///
    /// Used by `sddk backlog show <id>` to render the full audit
    /// trail and by the cycle 3/4 rendering engine to replay events
    /// into the BACKLOG / ROADMAP projections.
    fn events(
        &mut self,
        id: &BacklogItemId,
    ) -> Result<Vec<BacklogEventLogEntry>, BacklogError>;
}

/// SQLite-backed implementation of [`BacklogStore`].
///
/// The connection is borrowed mutably (not owned) to match the existing
/// precedent of `SqliteGraphStore` and `Storage::open` callers: the
/// caller manages the connection lifecycle and transaction scope.
///
/// Cycle 2/4 adds [`SqliteBacklogStoreOwned`] for CLI use cases that
/// don't have a pre-existing connection to borrow.
pub struct SqliteBacklogStore<'a> {
    conn: &'a mut rusqlite::Connection,
}

/// SQLite-backed implementation of [`BacklogStore`] that owns its
/// `rusqlite::Connection`. Used by CLI subcommands (`sddk backlog
/// capture|triage|list|show`) which open a fresh connection from the
/// project ledger path and don't need to share that connection with
/// other storage adapters.
///
/// Internally delegates to a borrowed [`SqliteBacklogStore`] so all
/// query / mutation logic stays in one place.
pub struct SqliteBacklogStoreOwned {
    conn: rusqlite::Connection,
}

/// Raw row extracted from the SQL query, used to keep
/// `row_from_sqlite`'s argument count below clippy's threshold.
struct BacklogItemRowRaw {
    item_id: String,
    origin_cycle_id: String,
    origin_phase: String,
    summary: String,
    current_priority: Option<String>,
    current_status: String,
    captured_at: String,
    emitted_event_count: i64,
}

impl<'a> SqliteBacklogStore<'a> {
    /// Creates a new `SqliteBacklogStore` over the given connection.
    pub fn new(conn: &'a mut rusqlite::Connection) -> Self {
        Self { conn }
    }

    fn row_from_sqlite(input: BacklogItemRowRaw) -> Result<BacklogItemRow, BacklogError> {
        let priority = match input.current_priority.as_deref() {
            None => None,
            Some(s) => Some(s.parse::<BacklogPriority>()?),
        };
        let status = input.current_status.parse::<BacklogStatus>()?;
        Ok(BacklogItemRow {
            item_id: input.item_id,
            origin_cycle_id: input.origin_cycle_id,
            origin_phase: input.origin_phase,
            summary: input.summary,
            current_priority: priority,
            current_status: status,
            captured_at: input.captured_at,
            emitted_event_count: input.emitted_event_count,
        })
    }
}

impl<'a> BacklogStore for SqliteBacklogStore<'a> {
    fn live_items(&mut self) -> Result<Vec<BacklogItemRow>, BacklogError> {
        let mut stmt = self.conn.prepare(
            "SELECT item_id, origin_cycle_id, origin_phase, summary, current_priority, current_status, captured_at, emitted_event_count \
             FROM backlog_items_v1 \
             WHERE current_status IN ('registered', 'triaged') \
             ORDER BY item_id ASC",
        ).map_err(sqlite_err)?;
        let rows = stmt.query_map([], |row| {
            Ok((
                row.get::<_, String>(0)?,
                row.get::<_, String>(1)?,
                row.get::<_, String>(2)?,
                row.get::<_, String>(3)?,
                row.get::<_, Option<String>>(4)?,
                row.get::<_, String>(5)?,
                row.get::<_, String>(6)?,
                row.get::<_, i64>(7)?,
            ))
        }).map_err(sqlite_err)?;
        let mut out = Vec::new();
        for r in rows {
            let (id, oc, op, sm, cp, cs, ca, ec) = r.map_err(sqlite_err)?;
            out.push(Self::row_from_sqlite(BacklogItemRowRaw {
                item_id: id,
                origin_cycle_id: oc,
                origin_phase: op,
                summary: sm,
                current_priority: cp,
                current_status: cs,
                captured_at: ca,
                emitted_event_count: ec,
            })?);
        }
        Ok(out)
    }

    fn item(&mut self, id: &BacklogItemId) -> Result<Option<BacklogItemRow>, BacklogError> {
        let mut stmt = self.conn.prepare(
            "SELECT item_id, origin_cycle_id, origin_phase, summary, current_priority, current_status, captured_at, emitted_event_count \
             FROM backlog_items_v1 WHERE item_id = ?1",
        ).map_err(sqlite_err)?;
        let mut rows = stmt.query(rusqlite::params![id]).map_err(sqlite_err)?;
        if let Some(row) = rows.next().map_err(sqlite_err)? {
            let id = row.get(0).map_err(sqlite_err)?;
            let oc = row.get(1).map_err(sqlite_err)?;
            let op = row.get(2).map_err(sqlite_err)?;
            let sm = row.get(3).map_err(sqlite_err)?;
            let cp = row.get(4).map_err(sqlite_err)?;
            let cs = row.get(5).map_err(sqlite_err)?;
            let ca = row.get(6).map_err(sqlite_err)?;
            let ec = row.get(7).map_err(sqlite_err)?;
            Ok(Some(Self::row_from_sqlite(BacklogItemRowRaw {
                item_id: id,
                origin_cycle_id: oc,
                origin_phase: op,
                summary: sm,
                current_priority: cp,
                current_status: cs,
                captured_at: ca,
                emitted_event_count: ec,
            })?))
        } else {
            Ok(None)
        }
    }

    fn append_event(&mut self, event: &BacklogEvent) -> Result<i64, BacklogError> {
        use rusqlite::TransactionBehavior;
        let tx = self.conn.transaction_with_behavior(TransactionBehavior::Immediate).map_err(sqlite_err)?;
        let payload_json = serde_json::to_string(&event.payload_json()?)?;
        // 1) Materialise item state FIRST so the FK from backlog_item_events_v1
        //    to backlog_items_v1 is satisfied when we insert the event row.
        //    (inline to avoid double-mutable-borrow on self.conn)
        match event {
            BacklogEvent::Registered { .. } => {
                let BacklogEvent::Registered {
                    item_id,
                    origin_cycle_id,
                    origin_phase,
                    summary,
                    captured_at,
                    ..
                } = event
                else { unreachable!() };
                tx.execute(
                    "INSERT INTO backlog_items_v1 \
                        (item_id, origin_cycle_id, origin_phase, summary, current_priority, current_status, captured_at, emitted_event_count) \
                     VALUES (?1, ?2, ?3, ?4, NULL, 'registered', ?5, 1) \
                     ON CONFLICT(item_id) DO NOTHING",
                    rusqlite::params![item_id, origin_cycle_id, origin_phase, summary, captured_at],
                ).map_err(sqlite_err)?;
            }
            BacklogEvent::Triaged { .. } => {
                let BacklogEvent::Triaged { item_id, priority, .. } = event
                else { unreachable!() };
                let priority_str = priority.to_string();
                tx.execute(
                    "UPDATE backlog_items_v1 \
                     SET current_priority = ?1, current_status = 'triaged', emitted_event_count = emitted_event_count + 1 \
                     WHERE item_id = ?2",
                    rusqlite::params![priority_str, item_id],
                ).map_err(sqlite_err)?;
            }
            BacklogEvent::Promoted { .. } => {
                let BacklogEvent::Promoted { item_id, .. } = event
                else { unreachable!() };
                tx.execute(
                    "UPDATE backlog_items_v1 \
                     SET current_status = 'promoted', emitted_event_count = emitted_event_count + 1 \
                     WHERE item_id = ?1",
                    rusqlite::params![item_id],
                ).map_err(sqlite_err)?;
            }
            BacklogEvent::Discarded { .. } => {
                let BacklogEvent::Discarded { item_id, .. } = event
                else { unreachable!() };
                tx.execute(
                    "UPDATE backlog_items_v1 \
                     SET current_status = 'discarded', emitted_event_count = emitted_event_count + 1 \
                     WHERE item_id = ?1",
                    rusqlite::params![item_id],
                ).map_err(sqlite_err)?;
            }
        }
        // 2) Insert into events log
        tx.execute(
            "INSERT INTO backlog_item_events_v1 \
                (item_id, event_type, schema_version, payload_json, valid_from, valid_to, actor_ref, causation_id, correlation_id, recorded_at) \
             VALUES (?1, ?2, ?3, ?4, ?5, '9999-12-31T23:59:59Z', ?6, NULL, NULL, ?7)",
            rusqlite::params![
                event.item_id(),
                event.event_type(),
                event.schema_version(),
                payload_json,
                now_iso_for(event),
                event.actor_ref(),
                now_iso_for(event),
            ],
        ).map_err(sqlite_err)?;
        let event_id = tx.last_insert_rowid();
        tx.commit().map_err(sqlite_err)?;
        Ok(event_id)
    }

    fn events(
        &mut self,
        id: &BacklogItemId,
    ) -> Result<Vec<BacklogEventLogEntry>, BacklogError> {
        let mut stmt = self.conn.prepare(
            "SELECT event_id, event_type, schema_version, recorded_at, actor_ref, payload_json \
             FROM backlog_item_events_v1 \
             WHERE item_id = ?1 \
             ORDER BY recorded_at ASC, event_id ASC",
        ).map_err(sqlite_err)?;
        let rows = stmt
            .query_map(rusqlite::params![id], |row| {
                let payload_str: String = row.get(5)?;
                let payload: serde_json::Value =
                    serde_json::from_str(&payload_str).map_err(|e| {
                        rusqlite::Error::FromSqlConversionFailure(
                            5,
                            rusqlite::types::Type::Text,
                            Box::new(e),
                        )
                    })?;
                Ok(BacklogEventLogEntry {
                    event_id: row.get(0)?,
                    event_type: row.get(1)?,
                    schema_version: row.get(2)?,
                    emitted_at: row.get(3)?,
                    actor_ref: row
                        .get::<_, Option<String>>(4)?
                        .unwrap_or_default(),
                    payload,
                })
            })
            .map_err(sqlite_err)?;
        let mut out = Vec::new();
        for r in rows {
            out.push(r.map_err(sqlite_err)?);
        }
        Ok(out)
    }
}

impl SqliteBacklogStoreOwned {
    /// Opens (or creates) `<dir>/ledger.sqlite` and applies pending
    /// migrations. Used by the CLI commands introduced in cycle 2/4.
    pub fn open(dir: &std::path::Path) -> Result<Self, BacklogError> {
        use crate::migrations::run_migrations;
        let db_path = dir.join("ledger.sqlite");
        let mut conn = rusqlite::Connection::open(&db_path).map_err(sqlite_err)?;
        run_migrations(&mut conn).map_err(|e| BacklogError::Storage(e.to_string()))?;
        Ok(Self { conn })
    }

    /// Opens an isolated in-memory database (CLI tests / smoke tests).
    pub fn open_in_memory() -> Result<Self, BacklogError> {
        use crate::migrations::run_migrations;
        let mut conn = rusqlite::Connection::open_in_memory().map_err(sqlite_err)?;
        run_migrations(&mut conn).map_err(|e| BacklogError::Storage(e.to_string()))?;
        Ok(Self { conn })
    }
}

impl BacklogStore for SqliteBacklogStoreOwned {
    fn live_items(&mut self) -> Result<Vec<BacklogItemRow>, BacklogError> {
        SqliteBacklogStore::new(&mut self.conn).live_items()
    }

    fn item(&mut self, id: &BacklogItemId) -> Result<Option<BacklogItemRow>, BacklogError> {
        SqliteBacklogStore::new(&mut self.conn).item(id)
    }

    fn append_event(&mut self, event: &BacklogEvent) -> Result<i64, BacklogError> {
        SqliteBacklogStore::new(&mut self.conn).append_event(event)
    }

    fn events(
        &mut self,
        id: &BacklogItemId,
    ) -> Result<Vec<BacklogEventLogEntry>, BacklogError> {
        SqliteBacklogStore::new(&mut self.conn).events(id)
    }
}

impl BacklogEvent {
    fn actor_ref(&self) -> Option<&str> {
        match self {
            Self::Registered { actor_ref, .. } => actor_ref.as_deref(),
            Self::Triaged { actor_ref, .. } => actor_ref.as_deref(),
            Self::Promoted { actor_ref, .. } => actor_ref.as_deref(),
            Self::Discarded { actor_ref, .. } => actor_ref.as_deref(),
        }
    }
}

fn now_iso_for(e: &BacklogEvent) -> &str {
    match e {
        BacklogEvent::Registered { captured_at, .. } => captured_at,
        BacklogEvent::Triaged { valid_from, .. } => valid_from,
        BacklogEvent::Promoted { promoted_at, .. } => promoted_at,
        BacklogEvent::Discarded { discarded_at, .. } => discarded_at,
    }
}

// ── Tests ─────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;
    use crate::migrations::run_migrations;

    fn fresh_db() -> rusqlite::Connection {
        let mut conn = rusqlite::Connection::open_in_memory().unwrap();
        run_migrations(&mut conn).unwrap();
        conn
    }

    fn reg_event(id: &str, summary: &str) -> BacklogEvent {
        BacklogEvent::Registered {
            item_id: id.to_string(),
            origin_cycle_id: "p-test/cycle-x".to_string(),
            origin_phase: "explore".to_string(),
            origin_artifacts: vec!["spec.md".to_string()],
            summary: summary.to_string(),
            captured_at: "2026-09-12T12:00:00Z".to_string(),
            actor_ref: Some("human:alice".to_string()),
        }
    }

    #[test]
    fn append_registered_creates_item() {
        let mut conn = fresh_db();
        let mut store = SqliteBacklogStore::new(&mut conn);
        store.append_event(&reg_event("B-001", "first")).unwrap();
        let row = store.item(&"B-001".to_string()).unwrap().unwrap();
        assert_eq!(row.item_id, "B-001");
        assert_eq!(row.summary, "first");
        assert_eq!(row.current_status, BacklogStatus::Registered);
        assert_eq!(row.current_priority, None);
    }

    #[test]
    fn append_triaged_updates_priority() {
        let mut conn = fresh_db();
        let mut store = SqliteBacklogStore::new(&mut conn);
        store.append_event(&reg_event("B-001", "x")).unwrap();
        store
            .append_event(&BacklogEvent::Triaged {
                item_id: "B-001".to_string(),
                priority: BacklogPriority::P1,
                priority_version: 1,
                valid_from: "2026-09-12T13:00:00Z".to_string(),
                valid_to: "9999-12-31T23:59:59Z".to_string(),
                actor_ref: None,
            })
            .unwrap();
        let row = store.item(&"B-001".to_string()).unwrap().unwrap();
        assert_eq!(row.current_priority, Some(BacklogPriority::P1));
        assert_eq!(row.current_status, BacklogStatus::Triaged);
    }

    #[test]
    fn append_discarded_ends_lifecycle() {
        let mut conn = fresh_db();
        let mut store = SqliteBacklogStore::new(&mut conn);
        store.append_event(&reg_event("B-001", "x")).unwrap();
        store
            .append_event(&BacklogEvent::Discarded {
                item_id: "B-001".to_string(),
                reason: "won't fix".to_string(),
                discarded_at: "2026-09-12T13:00:00Z".to_string(),
                actor_ref: None,
            })
            .unwrap();
        let row = store.item(&"B-001".to_string()).unwrap().unwrap();
        assert_eq!(row.current_status, BacklogStatus::Discarded);
        // live_items excludes discarded
        assert_eq!(store.live_items().unwrap().len(), 0);
    }

    #[test]
    fn latest_priority_wins_after_re_triage() {
        let mut conn = fresh_db();
        let mut store = SqliteBacklogStore::new(&mut conn);
        store.append_event(&reg_event("B-001", "x")).unwrap();
        store
            .append_event(&BacklogEvent::Triaged {
                item_id: "B-001".to_string(),
                priority: BacklogPriority::P2,
                priority_version: 1,
                valid_from: "2026-09-12T13:00:00Z".to_string(),
                valid_to: "9999-12-31T23:59:59Z".to_string(),
                actor_ref: None,
            })
            .unwrap();
        store
            .append_event(&BacklogEvent::Triaged {
                item_id: "B-001".to_string(),
                priority: BacklogPriority::P0,
                priority_version: 2,
                valid_from: "2026-09-12T14:00:00Z".to_string(),
                valid_to: "9999-12-31T23:59:59Z".to_string(),
                actor_ref: None,
            })
            .unwrap();
        let row = store.item(&"B-001".to_string()).unwrap().unwrap();
        assert_eq!(row.current_priority, Some(BacklogPriority::P0));
        assert_eq!(row.emitted_event_count, 3);
    }

    #[test]
    fn latest_event_decides_status() {
        let mut conn = fresh_db();
        let mut store = SqliteBacklogStore::new(&mut conn);
        store.append_event(&reg_event("B-001", "x")).unwrap();
        store
            .append_event(&BacklogEvent::Triaged {
                item_id: "B-001".to_string(),
                priority: BacklogPriority::P1,
                priority_version: 1,
                valid_from: "2026-09-12T13:00:00Z".to_string(),
                valid_to: "9999-12-31T23:59:59Z".to_string(),
                actor_ref: None,
            })
            .unwrap();
        store
            .append_event(&BacklogEvent::Discarded {
                item_id: "B-001".to_string(),
                reason: "done".to_string(),
                discarded_at: "2026-09-12T14:00:00Z".to_string(),
                actor_ref: None,
            })
            .unwrap();
        let row = store.item(&"B-001".to_string()).unwrap().unwrap();
        assert_eq!(row.current_status, BacklogStatus::Discarded);
    }

    #[test]
    fn live_items_filter_registered_and_triaged_only() {
        let mut conn = fresh_db();
        let mut store = SqliteBacklogStore::new(&mut conn);
        store.append_event(&reg_event("B-001", "a")).unwrap();
        store.append_event(&reg_event("B-002", "b")).unwrap();
        store.append_event(&reg_event("B-003", "c")).unwrap();
        store.append_event(&reg_event("B-004", "d")).unwrap();
        // B-002 triaged
        store
            .append_event(&BacklogEvent::Triaged {
                item_id: "B-002".to_string(),
                priority: BacklogPriority::P1,
                priority_version: 1,
                valid_from: "2026-09-12T13:00:00Z".to_string(),
                valid_to: "9999-12-31T23:59:59Z".to_string(),
                actor_ref: None,
            })
            .unwrap();
        // B-003 promoted (excluded from live)
        store
            .append_event(&BacklogEvent::Promoted {
                item_id: "B-003".to_string(),
                target_kind: "cycle".to_string(),
                target_id: "p-test/cycle-y".to_string(),
                promoted_at: "2026-09-12T13:00:00Z".to_string(),
                actor_ref: None,
            })
            .unwrap();
        // B-004 discarded (excluded from live)
        store
            .append_event(&BacklogEvent::Discarded {
                item_id: "B-004".to_string(),
                reason: "x".to_string(),
                discarded_at: "2026-09-12T13:00:00Z".to_string(),
                actor_ref: None,
            })
            .unwrap();
        let live = store.live_items().unwrap();
        assert_eq!(live.len(), 2);
        assert_eq!(live[0].item_id, "B-001");
        assert_eq!(live[1].item_id, "B-002");
    }

    #[test]
    fn deterministic_live_items_order() {
        let mut conn = fresh_db();
        let mut store = SqliteBacklogStore::new(&mut conn);
        store.append_event(&reg_event("B-003", "c")).unwrap();
        store.append_event(&reg_event("B-001", "a")).unwrap();
        store.append_event(&reg_event("B-002", "b")).unwrap();
        let a = store.live_items().unwrap();
        let b = store.live_items().unwrap();
        assert_eq!(a.iter().map(|r| &r.item_id).collect::<Vec<_>>(),
                   b.iter().map(|r| &r.item_id).collect::<Vec<_>>());
        assert_eq!(a[0].item_id, "B-001");
        assert_eq!(a[1].item_id, "B-002");
        assert_eq!(a[2].item_id, "B-003");
    }

    #[test]
    fn m18_runs_idempotently() {
        let mut conn = rusqlite::Connection::open_in_memory().unwrap();
        run_migrations(&mut conn).unwrap();
        // Re-run on a separate connection
        let conn2 = rusqlite::Connection::open_in_memory().unwrap();
        // can't share mem db across connections, so this is enough
        let v: i32 = conn.pragma_query_value(None, "user_version", |row| row.get(0)).unwrap();
        assert_eq!(v, 18);
        let _ = conn2;
    }

    #[test]
    fn m18_constraint_rejects_invalid_priority() {
        let mut conn = fresh_db();
        let result = conn.execute(
            "INSERT INTO backlog_items_v1 (item_id, origin_cycle_id, origin_phase, summary, current_priority, current_status, captured_at) \
             VALUES ('B-X', 'c', 'p', 's', 'P9', 'registered', '2026-09-12T12:00:00Z')",
            [],
        );
        assert!(result.is_err(), "CHECK constraint must reject P9");
    }

    #[test]
    fn item_not_found_returns_none() {
        let mut conn = fresh_db();
        let mut store = SqliteBacklogStore::new(&mut conn);
        assert!(store.item(&"B-nonexistent".to_string()).unwrap().is_none());
    }

    // ── Cycle 2/4 additions ──────────────────────────────────────────────

    #[test]
    fn events_returns_chronological_log() {
        let mut conn = fresh_db();
        let mut store = SqliteBacklogStore::new(&mut conn);
        store.append_event(&reg_event("B-100", "first")).unwrap();
        store
            .append_event(&BacklogEvent::Triaged {
                item_id: "B-100".to_string(),
                priority: BacklogPriority::P1,
                priority_version: 1,
                valid_from: "2026-09-12T12:00:00Z".to_string(),
                valid_to: "9999-12-31T23:59:59Z".to_string(),
                actor_ref: Some("test".to_string()),
            })
            .unwrap();
        store
            .append_event(&BacklogEvent::Triaged {
                item_id: "B-100".to_string(),
                priority: BacklogPriority::P0,
                priority_version: 2,
                valid_from: "2026-09-12T13:00:00Z".to_string(),
                valid_to: "9999-12-31T23:59:59Z".to_string(),
                actor_ref: Some("test".to_string()),
            })
            .unwrap();
        let log = store.events(&"B-100".to_string()).unwrap();
        assert_eq!(log.len(), 3, "expected 3 events, got {}", log.len());
        assert_eq!(log[0].event_type, "backlog.item.registered");
        assert_eq!(log[1].event_type, "backlog.item.triaged");
        assert_eq!(log[2].event_type, "backlog.item.triaged");
        assert_eq!(log[2].payload["priority_version"], 2);
    }

    #[test]
    fn events_for_unknown_item_returns_empty() {
        let mut conn = fresh_db();
        let mut store = SqliteBacklogStore::new(&mut conn);
        let log = store.events(&"B-ghost".to_string()).unwrap();
        assert!(log.is_empty());
    }

    #[test]
    fn open_in_memory_owned_round_trip() {
        let mut owned = SqliteBacklogStoreOwned::open_in_memory().unwrap();
        let id = "B-200".to_string();
        owned.append_event(&reg_event(&id, "owned")).unwrap();
        let row = owned.item(&id).unwrap().unwrap();
        assert_eq!(row.summary, "owned");
    }

    #[test]
    fn open_owned_creates_ledger_if_missing() {
        let dir = std::env::temp_dir().join("sddk-backlog-test-open-owned");
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).unwrap();
        let mut owned = SqliteBacklogStoreOwned::open(&dir).unwrap();
        let id = "B-300".to_string();
        owned.append_event(&reg_event(&id, "fresh")).unwrap();
        let row = owned.item(&id).unwrap().unwrap();
        assert_eq!(row.summary, "fresh");
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn open_owned_is_idempotent() {
        let dir = std::env::temp_dir().join("sddk-backlog-test-open-idempotent");
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).unwrap();
        // First open creates and applies migrations.
        let mut a = SqliteBacklogStoreOwned::open(&dir).unwrap();
        a.append_event(&reg_event("B-A", "first")).unwrap();
        // Second open on same dir must not fail (idempotent migrations).
        let mut b = SqliteBacklogStoreOwned::open(&dir).unwrap();
        let row = b.item(&"B-A".to_string()).unwrap().unwrap();
        assert_eq!(row.summary, "first");
        let _ = std::fs::remove_dir_all(&dir);
    }
}
