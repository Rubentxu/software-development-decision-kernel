//! SQLite adapter for the `GraphStore` port (SPEC-004 §2).
//!
//! The graph is a projection: the event ledger is the authority and this
//! adapter persists the derived snapshot + checkpoint in
//! `projection_checkpoints_v1` under the `graph` projection name.

use std::path::{Path, PathBuf};

use rusqlite::{OptionalExtension, params};
use sddk_domain::{
    Attempt, Checkpoint, EventStore, ExecutionGraphRevision, GraphProjection, GraphState,
    GraphStore, NodeId, Projection, ProjectionError, RevisionId, RunId, StorageError,
};

use crate::event_store::SqliteEventStore;
use crate::projection_store::SqliteProjectionStore;

/// SQLite-backed graph store using the projection checkpoint table.
pub struct SqliteGraphStore {
    /// Projection store that owns the checkpoint persistence.
    proj_store: SqliteProjectionStore,
    /// Directory containing `ledger.sqlite` (retained for ledger access).
    ledger_dir_path: Option<PathBuf>,
}

impl SqliteGraphStore {
    /// Opens (or creates) the ledger database at `dir/ledger.sqlite`.
    pub fn open(dir: &Path) -> Result<Self, StorageError> {
        Ok(Self {
            proj_store: SqliteProjectionStore::open(dir)?,
            ledger_dir_path: Some(dir.to_path_buf()),
        })
    }

    /// Opens an isolated in-memory database (tests).
    pub fn open_in_memory() -> Result<Self, StorageError> {
        Ok(Self {
            proj_store: SqliteProjectionStore::open_in_memory()?,
            ledger_dir_path: None,
        })
    }

    /// Mutable access to the underlying projection store connection.
    ///
    /// **Test-only surface.** The only known caller is the integration test
    /// `tests/graph_store_roundtrip.rs`, which seeds a parent row in
    /// `workflow_runs_v1` before invoking `record_graph_revision` (whose FK
    /// requirement the test setup must satisfy).
    ///
    /// Kept `pub` because Rust integration tests compile the library target
    /// *without* `cfg(test)`, so `#[cfg(test)]` on this method would hide it
    /// from the very consumer it serves. Documented as test-only via this
    /// rustdoc and `#[doc(hidden)]` so production callers do not reach for it.
    #[doc(hidden)]
    pub fn proj_store_conn_mut(&mut self) -> &mut rusqlite::Connection {
        self.proj_store.conn_mut()
    }

    /// Rebuilds the graph projection from the event ledger and persists it.
    ///
    /// Mirrors the generic `rebuild()` contract: verifies chain integrity
    /// (fail-closed), applies every event, then persists checkpoint + state.
    pub fn rebuild(
        &mut self,
        event_store: &SqliteEventStore,
        stream_id: &str,
    ) -> Result<GraphState, ProjectionError> {
        let events = event_store
            .load_stream(stream_id, None, u32::MAX)
            .map_err(|e| ProjectionError::Storage(format!("load_stream: {e}")))?;

        event_store.verify_stream_chain(stream_id).map_err(|_e| {
            ProjectionError::ChainIntegrityBroken {
                stream_id: stream_id.to_string(),
                sequence: events.last().map(|ev| ev.sequence).unwrap_or(0),
            }
        })?;

        let mut projection = GraphProjection::new(stream_id);
        for event in &events {
            projection.apply(event)?;
        }

        if events.is_empty() {
            // Empty ledger → empty state, no checkpoint.
            return Ok(projection.state_ref().clone());
        }

        let state_json = serde_json::to_string(projection.state_ref())
            .map_err(|e| ProjectionError::Storage(format!("state serialize: {e}")))?;
        let cp = projection.checkpoint();
        self.proj_store
            .save_checkpoint(&cp, &state_json)
            .map_err(|e| ProjectionError::Storage(format!("save_checkpoint: {e}")))?;
        Ok(projection.state_ref().clone())
    }

    /// Rebuilds the graph from the ledger at the same `ledger.sqlite` path.
    ///
    /// The graph is project-global: when `stream_id` starts with `project:`,
    /// ALL streams of the ledger are replayed (each chain-verified); otherwise
    /// only the given stream is replayed. Convenience for CLI consumers that
    /// do not hold an `SqliteEventStore`.
    pub fn rebuild_from_ledger(&mut self, stream_id: &str) -> Result<GraphState, ProjectionError> {
        // Both stores share `dir/ledger.sqlite`; the graph store keeps its
        // projection connection, and this opens a second read connection.
        let dir = self
            .ledger_dir()
            .map_err(|e| ProjectionError::Storage(e.to_string()))?;
        let event_store = SqliteEventStore::open(&dir)
            .map_err(|e| ProjectionError::Storage(format!("open event store: {e}")))?;

        let streams: Vec<String> = if stream_id.starts_with("project:") {
            event_store
                .list_streams()
                .map_err(|e| ProjectionError::Storage(format!("list_streams: {e}")))?
        } else {
            vec![stream_id.to_string()]
        };

        // Apply all streams in deterministic order into one global projection.
        let mut projection = GraphProjection::new(stream_id);
        if streams.is_empty() {
            // CEP events_v1 is empty — fall back to the kernel ledger
            // (`ledger_events`) which the CLI writes for workflow/approval
            // cycles. Map each kernel event into an EventEnvelopeV1 and apply.
            let ledger = crate::Storage::open(dir.join("ledger.sqlite"))
                .map_err(|e| ProjectionError::Storage(format!("open kernel storage: {e}")))?;
            let kernel_events = ledger
                .load_all_ledger_events()
                .map_err(|e| ProjectionError::Storage(format!("load_all_ledger_events: {e}")))?;
            for kernel in &kernel_events {
                let envelope = kernel_envelope_to_v1(kernel);
                projection.apply(&envelope)?;
            }
        } else {
            for stream in &streams {
                let events = event_store
                    .load_stream(stream, None, u32::MAX)
                    .map_err(|e| ProjectionError::Storage(format!("load_stream: {e}")))?;
                if events.is_empty() {
                    continue;
                }
                event_store.verify_stream_chain(stream).map_err(|_e| {
                    ProjectionError::ChainIntegrityBroken {
                        stream_id: stream.clone(),
                        sequence: events.last().map(|ev| ev.sequence).unwrap_or(0),
                    }
                })?;
                for event in &events {
                    projection.apply(event)?;
                }
            }
        }

        let state = projection.state_ref().clone();
        if state.last_event_sequence > 0 || !state.edges.is_empty() {
            let state_json = serde_json::to_string(&state)
                .map_err(|e| ProjectionError::Storage(format!("state serialize: {e}")))?;
            let cp = projection.checkpoint();
            self.proj_store
                .save_checkpoint(&cp, &state_json)
                .map_err(|e| ProjectionError::Storage(format!("save_checkpoint: {e}")))?;
        }
        Ok(state)
    }
}

/// Maps a kernel `LedgerEvent` into an `EventEnvelopeV1` for graph projection.
fn kernel_envelope_to_v1(event: &sddk_domain::LedgerEvent) -> sddk_domain::EventEnvelopeV1 {
    use sddk_domain::{ActorKind, ActorRef};
    sddk_domain::EventEnvelopeV1 {
        event_id: event.event_id.clone(),
        event_type: event.event_type.clone(),
        schema_version: 1,
        stream_id: event
            .cycle_id
            .clone()
            .unwrap_or_else(|| format!("project:{}", event.project_id)),
        sequence: event.sequence as u64,
        project_id: event.project_id.clone(),
        occurred_at: event.occurred_at.clone(),
        recorded_at: event.occurred_at.clone(),
        actor: ActorRef {
            kind: ActorKind::System,
            id: event.actor.clone(),
            definition_hash: None,
            policy_hash: None,
            model: None,
        },
        subjects: vec![sddk_domain::EntityRef {
            kind: "cycle".into(),
            id: event
                .cycle_id
                .clone()
                .unwrap_or_else(|| event.project_id.clone()),
            version: None,
            content_hash: None,
        }],
        payload: event.payload.clone(),
        evidence_refs: vec![],
        content_hash: event.event_hash.clone(),
        metadata: None,
        causation_id: None,
        correlation_id: None,
        cycle_id: event.cycle_id.clone(),
        frame_id: Some(event.frame_id.clone()),
        fork_id: None,
    }
}

impl SqliteGraphStore {
    /// Returns the directory containing `ledger.sqlite` (derived from the
    /// projection store connection path via the open directory).
    fn ledger_dir(&self) -> Result<std::path::PathBuf, StorageError> {
        // The projection store does not retain its path; callers of
        // `open(dir)` know it. We reconstruct it by convention: this adapter
        // is constructed with the directory, so we store it at open time.
        // Fallback: current directory (should not happen in practice).
        self.ledger_dir_path
            .clone()
            .ok_or_else(|| StorageError::Database("ledger dir not retained".into()))
    }
}

impl GraphStore for SqliteGraphStore {
    fn save_state(&mut self, state: &GraphState) -> Result<(), StorageError> {
        let state_json = serde_json::to_string(state)
            .map_err(|e| StorageError::Database(format!("graph state serialize: {e}")))?;
        let cp = Checkpoint {
            projection_name: GraphProjection::NAME.to_string(),
            version: GraphProjection::VERSION,
            last_event_sequence: state.last_event_sequence,
            last_event_hash: state.last_event_hash.clone(),
            updated_at: state_updated_at(state),
        };
        self.proj_store.save_checkpoint(&cp, &state_json)
    }

    fn load_state(&self) -> Result<Option<GraphState>, StorageError> {
        match self
            .proj_store
            .load_checkpoint(GraphProjection::NAME, GraphProjection::VERSION)?
        {
            Some((_, state_json)) => serde_json::from_str(&state_json)
                .map(Some)
                .map_err(|e| StorageError::Database(format!("graph state deserialize: {e}"))),
            None => Ok(None),
        }
    }

    fn checkpoint(&self) -> Result<Option<Checkpoint>, StorageError> {
        Ok(self
            .proj_store
            .load_checkpoint(GraphProjection::NAME, GraphProjection::VERSION)?
            .map(|(cp, _)| cp))
    }

    fn record_ir_digest(&mut self, ir_hash: &str, ir_json: &str) -> Result<(), StorageError> {
        let conn = self.proj_store.conn_mut();
        conn.execute(
            "INSERT OR REPLACE INTO ir_digests_v1 (ir_hash, ir_json, compiled_at)
             VALUES (?1, ?2, ?3)",
            params![ir_hash, ir_json, current_iso8601()],
        )
        .map_err(|e| StorageError::Database(format!("record_ir_digest: {e}")))?;
        Ok(())
    }

    fn record_graph_revision(&mut self, rev: &ExecutionGraphRevision) -> Result<(), StorageError> {
        let rev_id = rev.revision_id.0.clone();
        let run_id = rev
            .nodes
            .keys()
            .next()
            .map(|n| n.0.clone())
            .unwrap_or_default();
        let parent_id = rev.parent.as_ref().map(|p| p.revision_id.0.clone());
        let events_json = serde_json::to_string(&rev.events)
            .map_err(|e| StorageError::Database(format!("events serialize: {e}")))?;
        let nodes_json = serde_json::to_string(&rev.nodes)
            .map_err(|e| StorageError::Database(format!("nodes serialize: {e}")))?;
        let edges_json = serde_json::to_string(&rev.edges)
            .map_err(|e| StorageError::Database(format!("edges serialize: {e}")))?;
        let digest = format!(
            "sha256:{}",
            rev.digest
                .iter()
                .map(|b| format!("{:02x}", b))
                .collect::<String>()
        );

        let conn = self.proj_store.conn_mut();
        conn.execute(
            "INSERT OR REPLACE INTO execution_graph_revisions_v1
                (revision_id, run_id, revision, parent_revision_id,
                 events_json, nodes_json, edges_json, digest, schema_version)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9)",
            params![
                rev_id,
                run_id,
                i64::try_from(rev.revision).unwrap_or(i64::MAX),
                parent_id,
                events_json,
                nodes_json,
                edges_json,
                digest,
                ExecutionGraphRevision::SCHEMA_VERSION as i64,
            ],
        )
        .map_err(|e| StorageError::Database(format!("record_graph_revision: {e}")))?;
        Ok(())
    }

    fn load_node_attempts(
        &self,
        run_id: &RunId,
        node_id: &NodeId,
    ) -> Result<Vec<Attempt>, StorageError> {
        let conn = self.proj_store.conn();
        let mut stmt = conn
            .prepare(
                "SELECT attempt_id, run_id, node_id, route_json, started_at, ended_at,
                        outcome_json, usage_json, context_capsule_json, idempotency_key, schema_version
                 FROM attempts_v1
                 WHERE run_id = ?1 AND node_id = ?2
                 ORDER BY started_at ASC",
            )
            .map_err(|e| StorageError::Database(format!("load_node_attempts prep: {e}")))?;
        let rows = stmt
            .query_map(params![run_id.0, node_id.0], |row| {
                let attempt_id: String = row.get(0)?;
                let run_id: String = row.get(1)?;
                let node_id: String = row.get(2)?;
                let route_json: String = row.get(3)?;
                let started_at: String = row.get(4)?;
                let ended_at: Option<String> = row.get(5)?;
                let outcome_json: Option<String> = row.get(6)?;
                let usage_json: String = row.get(7)?;
                let capsule_json: String = row.get(8)?;
                let idempotency_key: String = row.get(9)?;
                let schema_version: i64 = row.get(10)?;
                Ok(RawAttemptRow {
                    attempt_id,
                    run_id,
                    node_id,
                    route_json,
                    started_at,
                    ended_at,
                    outcome_json,
                    usage_json,
                    capsule_json,
                    idempotency_key,
                    schema_version: schema_version as u32,
                })
            })
            .map_err(|e| StorageError::Database(format!("load_node_attempts query: {e}")))?;
        let mut out = Vec::new();
        for row in rows {
            let row = row.map_err(|e| StorageError::Database(format!("row: {e}")))?;
            out.push(row.into_attempt()?);
        }
        Ok(out)
    }

    fn attempt_count(&self, run_id: &RunId, node_id: &NodeId) -> Result<u32, StorageError> {
        let conn = self.proj_store.conn();
        let count: i64 = conn
            .query_row(
                "SELECT COUNT(*) FROM attempts_v1 WHERE run_id = ?1 AND node_id = ?2",
                params![run_id.0, node_id.0],
                |row| row.get(0),
            )
            .map_err(|e| StorageError::Database(format!("attempt_count: {e}")))?;
        Ok(count.max(0) as u32)
    }

    fn load_revision(
        &self,
        run_id: &RunId,
        rev_id: &RevisionId,
    ) -> Result<Option<ExecutionGraphRevision>, StorageError> {
        let conn = self.proj_store.conn();
        let row = conn
            .query_row(
                "SELECT revision_id, run_id, revision, parent_revision_id,
                        events_json, nodes_json, edges_json, digest, schema_version
                 FROM execution_graph_revisions_v1
                 WHERE revision_id = ?1 AND run_id = ?2",
                params![rev_id.0, run_id.0],
                RawRevisionRow::from_row,
            )
            .optional()
            .map_err(|e| StorageError::Database(format!("load_revision: {e}")))?;
        match row {
            Some(r) => Ok(Some(r.into_revision()?)),
            None => Ok(None),
        }
    }

    fn latest_revision(
        &self,
        run_id: &RunId,
    ) -> Result<Option<ExecutionGraphRevision>, StorageError> {
        let conn = self.proj_store.conn();
        let row = conn
            .query_row(
                "SELECT revision_id, run_id, revision, parent_revision_id,
                        events_json, nodes_json, edges_json, digest, schema_version
                 FROM execution_graph_revisions_v1
                 WHERE run_id = ?1
                 ORDER BY revision DESC
                 LIMIT 1",
                params![run_id.0],
                RawRevisionRow::from_row,
            )
            .optional()
            .map_err(|e| StorageError::Database(format!("latest_revision: {e}")))?;
        match row {
            Some(r) => Ok(Some(r.into_revision()?)),
            None => Ok(None),
        }
    }
}

/// RFC 3339 / ISO 8601 UTC timestamp.
///
/// Delegates to `sddk_domain::format::format_rfc3339_utc` so the algorithm
/// lives in one place across the workspace (cycle 3 W-DV-7 cleanup).
fn current_iso8601() -> String {
    use std::time::{SystemTime, UNIX_EPOCH};
    let secs = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs();
    sddk_domain::format::format_rfc3339_utc(secs)
}

/// Raw row from attempts_v1.
struct RawAttemptRow {
    attempt_id: String,
    run_id: String,
    node_id: String,
    route_json: String,
    started_at: String,
    ended_at: Option<String>,
    outcome_json: Option<String>,
    usage_json: String,
    capsule_json: String,
    idempotency_key: String,
    schema_version: u32,
}

impl RawAttemptRow {
    fn into_attempt(self) -> Result<Attempt, StorageError> {
        use sddk_domain::{
            AttemptId, AttemptOutcome, ContextCapsuleRef, IdempotencyKey, Route, Usage,
        };
        let route: Route = serde_json::from_str(&self.route_json)
            .map_err(|e| StorageError::Database(format!("route deserialize: {e}")))?;
        let outcome: Option<AttemptOutcome> = match self.outcome_json {
            Some(j) => Some(
                serde_json::from_str(&j)
                    .map_err(|e| StorageError::Database(format!("outcome deserialize: {e}")))?,
            ),
            None => None,
        };
        let usage: Usage = serde_json::from_str(&self.usage_json)
            .map_err(|e| StorageError::Database(format!("usage deserialize: {e}")))?;
        let context_capsule: ContextCapsuleRef = serde_json::from_str(&self.capsule_json)
            .map_err(|e| StorageError::Database(format!("capsule deserialize: {e}")))?;
        let idempotency_key = IdempotencyKey {
            project_id: String::new(),
            run_id: RunId(self.run_id.clone()),
            node_id: NodeId(self.node_id.clone()),
            attempt_seq: 0,
        };
        let _ = self.idempotency_key; // original string not currently needed
        Ok(Attempt {
            attempt_id: AttemptId(self.attempt_id),
            node_id: NodeId(self.node_id),
            route,
            started_at: self.started_at,
            ended_at: self.ended_at,
            outcome,
            usage,
            context_capsule,
            idempotency_key,
            schema_version: self.schema_version,
        })
    }
}

/// Raw row from execution_graph_revisions_v1.
#[allow(dead_code)]
struct RawRevisionRow {
    revision_id: String,
    run_id: String,
    revision: u64,
    parent_revision_id: Option<String>,
    events_json: String,
    nodes_json: String,
    edges_json: String,
    digest: String,
    schema_version: u32,
}

impl RawRevisionRow {
    fn from_row(row: &rusqlite::Row<'_>) -> rusqlite::Result<Self> {
        Ok(Self {
            revision_id: row.get(0)?,
            run_id: row.get(1)?,
            revision: row.get::<_, i64>(2)?.max(0) as u64,
            parent_revision_id: row.get(3)?,
            events_json: row.get(4)?,
            nodes_json: row.get(5)?,
            edges_json: row.get(6)?,
            digest: row.get(7)?,
            schema_version: row.get::<_, i64>(8)? as u32,
        })
    }
    fn into_revision(self) -> Result<ExecutionGraphRevision, StorageError> {
        use sddk_domain::{EdgeId, EdgeSnapshot, GraphEvent, NodeSnapshot};
        use std::collections::BTreeMap;
        let events: BTreeMap<sddk_domain::EventId, GraphEvent> =
            serde_json::from_str(&self.events_json)
                .map_err(|e| StorageError::Database(format!("events deserialize: {e}")))?;
        let nodes: BTreeMap<NodeId, NodeSnapshot> = serde_json::from_str(&self.nodes_json)
            .map_err(|e| StorageError::Database(format!("nodes deserialize: {e}")))?;
        let edges: BTreeMap<EdgeId, EdgeSnapshot> = serde_json::from_str(&self.edges_json)
            .map_err(|e| StorageError::Database(format!("edges deserialize: {e}")))?;
        let mut digest_bytes = [0u8; 32];
        let hex = self.digest.strip_prefix("sha256:").unwrap_or(&self.digest);
        if hex.len() == 64 {
            for i in 0..32 {
                digest_bytes[i] = u8::from_str_radix(&hex[i * 2..i * 2 + 2], 16).unwrap_or(0);
            }
        }
        Ok(ExecutionGraphRevision {
            revision: self.revision,
            revision_id: RevisionId(self.revision_id),
            parent: None,
            events,
            nodes,
            edges,
            digest: digest_bytes,
            schema_version: self.schema_version,
        })
    }
}

/// Derives a stable `updated_at` for the checkpoint from state (RFC 3339),
/// or a fallback timestamp when no events have been applied.
///
/// Used only for the checkpoint audit field; the graph state itself is
/// deterministic. Cycle 3 (W-DV-1) replaced the broken `"2026-08-18T00:00:00Z (day N)"`
/// stub with a real RFC 3339 timestamp via `sddk_domain::format`.
fn state_updated_at(_state: &GraphState) -> String {
    use std::time::{SystemTime, UNIX_EPOCH};
    let secs = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs();
    sddk_domain::format::format_rfc3339_utc(secs)
}

#[cfg(test)]
mod tests {
    use super::*;
    use sddk_domain::{ActorKind, ActorRef, EntityRef, EventEnvelopeV1, EventStore};
    use serde_json::json;

    fn make_event(
        stream: &str,
        event_type: &str,
        seq: u64,
        subjects: Vec<EntityRef>,
    ) -> EventEnvelopeV1 {
        EventEnvelopeV1 {
            event_id: format!("evt-{seq}"),
            event_type: event_type.into(),
            schema_version: 1,
            stream_id: stream.into(),
            sequence: seq,
            project_id: "p-1".into(),
            occurred_at: format!("2026-08-18T10:00:{seq:02}Z"),
            recorded_at: format!("2026-08-18T10:00:{seq:02}Z"),
            actor: ActorRef {
                kind: ActorKind::System,
                id: "sddk-test".into(),
                definition_hash: None,
                policy_hash: None,
                model: None,
            },
            subjects,
            payload: json!({}),
            evidence_refs: vec![],
            content_hash: format!("sha256:{seq:064x}"),
            metadata: None,
            causation_id: None,
            correlation_id: None,
            cycle_id: Some("c-1".into()),
            frame_id: None,
            fork_id: None,
        }
    }

    fn subject(kind: &str, id: &str) -> EntityRef {
        EntityRef {
            kind: kind.into(),
            id: id.into(),
            version: None,
            content_hash: None,
        }
    }

    #[test]
    fn save_then_load_roundtrips() {
        let mut store = SqliteGraphStore::open_in_memory().unwrap();
        let mut state = GraphState::default();
        state.nodes.insert(
            "capability:git.commit".into(),
            sddk_domain::GraphNode {
                kind: "capability".into(),
                id: "git.commit".into(),
                created_by: "evt-1".into(),
                content_hash: "sha256:1".into(),
                occurred_at: "2026-08-18T10:00:00Z".into(),
            },
        );
        state.edges.push(sddk_domain::GraphEdge {
            from: "actor:alice".into(),
            relation: "approval.capability.granted".into(),
            to: "capability:git.commit".into(),
            event_id: "evt-1".into(),
            occurred_at: "2026-08-18T10:00:00Z".into(),
            actor: "alice".into(),
        });
        state.last_event_sequence = 1;
        state.last_event_hash = "sha256:1".into();

        store.save_state(&state).unwrap();
        let loaded = store.load_state().unwrap().unwrap();
        assert_eq!(loaded, state);
    }

    #[test]
    fn load_empty_returns_none() {
        let store = SqliteGraphStore::open_in_memory().unwrap();
        assert!(store.load_state().unwrap().is_none());
        assert!(store.checkpoint().unwrap().is_none());
    }

    #[test]
    fn rebuild_from_ledger_builds_graph() {
        let dir = std::env::temp_dir().join(format!("sddk-graph-store-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).unwrap();

        let mut event_store = SqliteEventStore::open(&dir).unwrap();
        for seq in 1..=3u64 {
            let subjects = match seq {
                1 => vec![subject("cycle", "c-1"), subject("capability", "git.commit")],
                2 => vec![
                    subject("actor", "alice"),
                    subject("capability", "git.commit"),
                ],
                _ => vec![subject("cycle", "c-1"), subject("capability", "git.push")],
            };
            let event_type = match seq {
                1 => "approval.capability.requested",
                2 => "approval.capability.granted",
                _ => "approval.capability.requested",
            };
            let event = make_event("project:p-1", event_type, seq, subjects);
            let hash = event.compute_content_hash();
            let mut envelope = event;
            envelope.content_hash = hash;
            event_store.append(&envelope).unwrap();
        }

        let mut graph_store = SqliteGraphStore::open(&dir).unwrap();
        let state = graph_store.rebuild(&event_store, "project:p-1").unwrap();
        assert_eq!(state.nodes.len(), 4); // cycle, capability:git.commit, actor, capability:git.push
        assert_eq!(state.edges.len(), 3);
        assert_eq!(state.last_event_sequence, 3);

        // Rebuild is idempotent → same state.
        let state2 = graph_store.rebuild(&event_store, "project:p-1").unwrap();
        assert_eq!(state, state2);

        let loaded = graph_store.load_state().unwrap().unwrap();
        assert_eq!(loaded, state);

        std::fs::remove_dir_all(&dir).ok();
    }

    #[test]
    fn rebuild_empty_ledger_is_safe() {
        let dir =
            std::env::temp_dir().join(format!("sddk-graph-store-empty-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).unwrap();

        let event_store = SqliteEventStore::open(&dir).unwrap();
        let mut graph_store = SqliteGraphStore::open(&dir).unwrap();
        let state = graph_store.rebuild(&event_store, "project:p-1").unwrap();
        assert!(state.nodes.is_empty());
        assert!(state.edges.is_empty());

        std::fs::remove_dir_all(&dir).ok();
    }

    /// Integration test: verifies that `verify_stream_chain` (used by graph rebuild)
    /// detects content_hash drift, and that `verify_chain_integrity` detects chain
    /// tampering. Both gates are fail-closed. Uses raw SQL to inject tampered events
    /// since `append()` validates content_hash (preventing injection there).
    #[test]
    fn graph_rebuild_detects_content_hash_drift_and_chain_tamper() {
        use sddk_domain::{ActorKind, ActorRef, EventEnvelopeV1};

        let dir = std::env::temp_dir().join(format!("sddk-graph-integrity-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).unwrap();

        // Append two valid events with correctly-computed content_hash via append().
        let mut event_store = SqliteEventStore::open(&dir).unwrap();
        for (seq, event_type) in [
            (1, "approval.capability.requested"),
            (2, "approval.capability.granted"),
        ] {
            let mut envelope = EventEnvelopeV1 {
                event_id: format!("evt-{seq}"),
                event_type: event_type.into(),
                schema_version: 1,
                stream_id: "project:p-1".into(),
                sequence: seq,
                project_id: "p-1".into(),
                occurred_at: format!("2026-08-18T10:00:{seq:02}Z"),
                recorded_at: format!("2026-08-18T10:00:{seq:02}Z"),
                actor: ActorRef {
                    kind: ActorKind::System,
                    id: "sddk-test".into(),
                    definition_hash: None,
                    policy_hash: None,
                    model: None,
                },
                subjects: vec![subject("cycle", "c-1")],
                payload: serde_json::json!({}),
                evidence_refs: vec![],
                content_hash: String::new(),
                metadata: None,
                causation_id: None,
                correlation_id: None,
                cycle_id: Some("c-1".into()),
                frame_id: None,
                fork_id: None,
            };
            envelope.content_hash = envelope.compute_content_hash();
            event_store.append(&envelope).unwrap();
        }

        // Rebuild succeeds with valid content hashes.
        let mut graph_store = SqliteGraphStore::open(&dir).unwrap();
        let state = graph_store.rebuild(&event_store, "project:p-1").unwrap();
        assert_eq!(state.last_event_sequence, 2);

        // verify_stream_chain passes for correct content_hash.
        assert!(
            event_store.verify_stream_chain("project:p-1").is_ok(),
            "verify_stream_chain should pass for valid content_hash"
        );
        // verify_chain_integrity passes for valid chain.
        assert!(
            event_store.verify_chain_integrity("project:p-1").is_ok(),
            "verify_chain_integrity should pass for valid chain"
        );

        // Inject a tampered event with wrong content_hash using raw SQL.
        // We insert evt-2 again with a different content_hash (simulating tamper after append).
        // Use INSERT OR REPLACE to overwrite the original evt-2.
        let conn = rusqlite::Connection::open(dir.join("ledger.sqlite")).unwrap();
        let tampered_content_hash =
            "sha256:0000000000000000000000000000000000000000000000000000000000000000";
        conn.execute(
            "INSERT OR REPLACE INTO events_v1 \
             (event_id, event_type, schema_version, stream_id, sequence, project_id, \
              occurred_at, recorded_at, actor_json, subjects_json, payload_json, \
              evidence_refs_json, content_hash, metadata_json, causation_id, \
              correlation_id, cycle_id, frame_id, fork_id, chain_hash) \
             SELECT \
              'evt-2', 'approval.capability.granted', 1, 'project:p-1', 2, 'p-1', \
              '2026-08-18T10:00:02Z', '2026-08-18T10:00:02Z', \
              actor_json, subjects_json, payload_json, evidence_refs_json, \
              ?1, metadata_json, causation_id, correlation_id, cycle_id, frame_id, fork_id, \
              chain_hash \
             FROM events_v1 WHERE event_id = 'evt-2'",
            rusqlite::params![tampered_content_hash],
        )
        .unwrap();

        // verify_stream_chain MUST fail when content_hash doesn't match recomputed value.
        let drift_err = event_store.verify_stream_chain("project:p-1").unwrap_err();
        assert!(
            matches!(drift_err, sddk_domain::StorageError::Other(ref msg)
                if msg.contains("hash_drift")),
            "expected hash_drift error, got: {drift_err}"
        );

        // verify_chain_integrity also fails because chain_hash depends on content_hash.
        let chain_err = event_store
            .verify_chain_integrity("project:p-1")
            .unwrap_err();
        assert!(
            matches!(chain_err, sddk_domain::StorageError::Other(ref msg)
                if msg.contains("chain_drift")),
            "expected chain_drift error, got: {chain_err}"
        );

        std::fs::remove_dir_all(&dir).ok();
    }

    /// Regression test for CRIT-DV-1: `current_iso8601()` must return a real
    /// RFC 3339 timestamp, not the broken `1970-01-01T00:00:00+00:00 (epoch ...)`
    /// stub that landed in commit `41957f9`.
    #[test]
    fn current_iso8601_is_real_timestamp() {
        let s = current_iso8601();
        assert!(
            !s.contains("(epoch"),
            "current_iso8601() leaked the (epoch ...) debug suffix: {s}"
        );
        assert!(s.ends_with('Z'), "expected UTC Z suffix: {s}");
        // Format: YYYY-MM-DDTHH:MM:SSZ (20 chars)
        assert_eq!(s.len(), 20, "expected 20-char RFC 3339, got: {s}");
        // Year prefix should be the current civil year (2026 in the dev era).
        let year_prefix: i64 = s[..4].parse().expect("year prefix");
        assert!(
            (2020..=2100).contains(&year_prefix),
            "current year out of plausible range: {year_prefix}"
        );
    }

    /// Regression test for W-DV-1: `state_updated_at()` must return a real
    /// RFC 3339 timestamp, not the `"2026-08-18T00:00:00Z (day N)"` stub
    /// that was present in the codebase before cycle 3.
    #[test]
    fn state_updated_at_is_real_timestamp() {
        let s = state_updated_at(&GraphState::default());
        assert!(
            !s.contains("(day"),
            "state_updated_at() leaked the (day ...) debug suffix: {s}"
        );
        assert!(s.ends_with('Z'), "expected UTC Z suffix: {s}");
        assert_eq!(s.len(), 20, "expected 20-char RFC 3339, got: {s}");
        let year_prefix: i64 = s[..4].parse().expect("year prefix");
        assert!(
            (2020..=2100).contains(&year_prefix),
            "current year out of plausible range: {year_prefix}"
        );
    }
}
