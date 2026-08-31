//! Hexagonal persistence port (Phase 1 M1 exit).
//!
//! `sddk_engine` depends only on this trait; `sddk_storage::Storage` is the
//! concrete SQLite implementation. The trait is object-safe (no associated
//! `Self` fns, no `Self` in generics) so `&dyn Ledger` is usable from the
//! engine's accessor.

use std::sync::{Arc, Mutex};

use crate::StorageError;
use crate::metrics::MetricsRecord;
use crate::models::*;

/// Hexagonal port over the SDDK ledger.
pub trait Ledger {
    // ── Cycle ops ─────────────────────────────────────────────────────────
    /// Loads a cycle snapshot by identifier.
    fn get_cycle(&self, cycle_id: &str) -> Result<CycleRecord, StorageError>;
    /// Lists all ledger events for one cycle in ascending global sequence order.
    fn list_cycle_events(&self, cycle_id: &str) -> Result<Vec<LedgerEvent>, StorageError>;
    /// Inserts a cycle snapshot and its initial event atomically.
    fn insert_cycle_with_event(
        &mut self,
        cycle: &CycleRecord,
        event: &LedgerEventInput,
    ) -> Result<LedgerEvent, StorageError>;
    /// Replaces a cycle snapshot and appends its causal event atomically.
    fn update_cycle_with_event(
        &mut self,
        manifest: &crate::CycleManifest,
        updated_at: &str,
        event: &LedgerEventInput,
        release_lease_on_phase_change: bool,
    ) -> Result<LedgerEvent, StorageError>;

    // ── Lease ops ──────────────────────────────────────────────────────────
    /// Acquires an absent or expired cycle lease.
    fn acquire_cycle_lease(
        &mut self,
        cycle_id: &str,
        owner: &str,
        now_ms: i64,
        expires_at_ms: i64,
    ) -> Result<CycleLease, StorageError>;
    /// Releases a cycle lease only when owner and fencing token still match.
    #[allow(clippy::too_many_arguments)]
    fn release_cycle_lease(
        &mut self,
        project_id: &str,
        cycle_id: &str,
        owner: &str,
        fencing_token: i64,
        actor: &str,
        command_id: &str,
        occurred_at: &str,
    ) -> Result<bool, StorageError>;
    /// Extends the expiry of the lease you already hold without changing the
    /// fencing token (reuse / renew semantics).
    fn renew_cycle_lease(
        &mut self,
        cycle_id: &str,
        owner: &str,
        fencing_token: i64,
        now_ms: i64,
        new_expires_at_ms: i64,
    ) -> Result<CycleLease, StorageError>;
    /// Loads the current cycle lease.
    fn get_cycle_lease(&self, cycle_id: &str) -> Result<CycleLease, StorageError>;
    /// Verifies that the current lease still matches the caller's fencing
    /// token and has not expired at `now_ms`.
    fn verify_cycle_lease(
        &self,
        cycle_id: &str,
        owner: &str,
        fencing_token: i64,
        now_ms: i64,
    ) -> Result<CycleLease, StorageError>;

    // ── Gate receipts ──────────────────────────────────────────────────────
    /// Loads one gate receipt by identifier.
    fn get_gate_receipt(&self, receipt_id: &str) -> Result<GateReceipt, StorageError>;
    /// Persists one authorized gate evaluation receipt with atomic seq allocation.
    fn insert_gate_receipt_next_seq(
        &mut self,
        input: &GateReceiptNextSeqInput,
    ) -> Result<GateReceipt, StorageError>;

    // ── Project / workspace ────────────────────────────────────────────────
    /// Loads a logical project when present.
    fn get_project_optional(&self, project_id: &str)
    -> Result<Option<ProjectRecord>, StorageError>;
    /// Loads a workspace when present.
    fn get_workspace_optional(
        &self,
        workspace_id: &str,
    ) -> Result<Option<WorkspaceRecord>, StorageError>;
    /// Reports whether the database contains any project registration.
    fn has_projects(&self) -> Result<bool, StorageError>;
    /// Registers a project and workspace in one SQLite transaction.
    fn register_project_workspace(
        &mut self,
        project: &ProjectRecord,
        workspace: &WorkspaceRecord,
    ) -> Result<(), StorageError>;

    /// Loads all ledger events from the database in ascending sequence order.
    ///
    /// Used by telemetry ingest to derive metrics for cycles that have no
    /// metrics.jsonl entry.
    fn load_all_ledger_events(&self) -> Result<Vec<LedgerEvent>, StorageError>;
}

// ── Ledger factory ───────────────────────────────────────────────────────────

/// Hexagonal factory port for opening a [`Ledger`] from a path.
///
/// Callers (typically the CLI composition root) use this trait to create
/// ledger instances without depending on the concrete `Storage` type directly.
/// The trait is object-safe; callers can store `Box<dyn LedgerFactory>`.
///
/// # Example
///
/// ```ignore
/// let factory: Box<dyn LedgerFactory> = Box::new(SqliteLedgerFactory);
/// let ledger = factory.open_ledger("/path/to/ledger.sqlite")?;
/// ```
pub trait LedgerFactory: Send + Sync {
    /// The ledger type produced by this factory.
    type Ledger: Ledger;

    /// Opens (or creates) a ledger at the given path.
    ///
    /// Implementations may also open in-memory variants for testing.
    fn open_ledger(&self, path: &std::path::Path) -> Result<Self::Ledger, StorageError>;

    /// Opens an in-memory ledger for testing.
    ///
    /// Default implementation returns an error; override for test-friendly factories.
    fn open_in_memory(&self) -> Result<Self::Ledger, StorageError> {
        Err(StorageError::Database(
            "this factory does not support in-memory ledgers".into(),
        ))
    }
}

// ── Control-plane port ────────────────────────────────────────────────────────

/// Hexagonal port over the SDDK control-plane SQLite store (SDDK2-103).
/// The concrete implementation is `sddk_storage::SqliteControlPlane`.
pub trait ControlPlane {
    /// Returns true if the control-plane store file exists and is readable.
    fn store_exists(&self) -> bool;

    /// Inserts a discovered project (idempotent via INSERT OR IGNORE).
    fn upsert_project(
        &mut self,
        project_id: &str,
        display_name: &str,
        scope: &str,
        remote_url: Option<&str>,
        now: &str,
    ) -> Result<(), StorageError>;

    /// Inserts or replaces a `MetricsRecord` by `cycle_id`.
    fn upsert_cycle(
        &mut self,
        project_id: &str,
        record: &MetricsRecord,
    ) -> Result<(), StorageError>;

    /// Inserts or replaces the aggregate for a rolling window.
    fn upsert_aggregate(
        &mut self,
        window_days: u16,
        computed_at: &str,
        payload_json: &str,
    ) -> Result<(), StorageError>;

    /// Inserts or replaces a `UatResultRow`.
    fn upsert_uat_result(&mut self, result: &UatResultRow) -> Result<(), StorageError>;

    /// Loads all persisted `MetricsRecord` rows.
    fn load_cycles(&self) -> Result<Vec<MetricsRecord>, StorageError>;

    /// Loads all persisted `UatResultRow` rows.
    fn load_uat_results(&self) -> Result<Vec<UatResultRow>, StorageError>;
}

// ── Event-store port ──────────────────────────────────────────────────────────

/// Proof-of-success receipt returned by [`EventStore::append`].
///
/// The `content_hash` mirrors the value already stored in the database; the
/// adapter does NOT recompute it. Callers are expected to have built it via
/// [`EventEnvelopeV1::compute_content_hash`](crate::event_envelope::EventEnvelopeV1::compute_content_hash)
/// before calling `append`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct EventAppended {
    /// Globally unique event identifier.
    pub event_id: String,
    /// Stream this event belongs to.
    pub stream_id: String,
    /// Monotonic sequence number assigned within the stream at append time.
    pub sequence: u64,
    /// SHA-256 content hash — identical to `EventEnvelopeV1::content_hash`.
    pub content_hash: String,
    /// Wall-clock time when the event was recorded (RFC 3339).
    pub recorded_at: String,
    /// SHA-256 chain hash linking this event to the previous one.
    /// `chain_hash[0] = SHA256(content_hash || "genesis")`
    /// `chain_hash[N] = SHA256(content_hash[N] || chain_hash[N-1])`
    pub chain_hash: String,
}

/// Append-only event store for [`EventEnvelopeV1`] envelopes.
///
/// This trait is intentionally separate from [`Ledger`]. The `Ledger` trait
/// covers cycle/lease/gate_receipt/project bookkeeping that lives in the
/// `ledger_events` table (legacy). This trait covers the Common Event Protocol
/// v1 substrate that lives in `events_v1`.
///
/// Implementations MUST:
/// - Validate `content_hash` format (`sha256:<64-hex>`).
/// - Allocate sequence numbers per-stream under a transaction.
/// - Reject updates/deletes (enforced via SQL triggers on the storage side).
///
/// Error responses use [`StorageError::Other`] with a stable `event_store:<code>`
/// prefix contract:
/// - `event_store:content_hash_mismatch` — content hash does not match recomputed value
/// - `event_store:invalid_content_hash` — hash missing `sha256:` prefix or wrong length
/// - `event_store:invalid_event_type` — event_type failed validation
/// - `event_store:hash_drift:<seq>` — stored hash differs from recomputed at given sequence
pub trait EventStore {
    /// Appends an event envelope to the store, assigning a per-stream sequence number.
    ///
    /// The caller's `envelope.content_hash` MUST match `envelope.compute_content_hash()`
    /// and MUST start with `sha256:` before this method is called.
    ///
    /// Idempotency: re-appending the same `event_id` returns the original
    /// `EventAppended` (same `sequence`, same `recorded_at`) without allocating
    /// a new sequence.
    fn append(
        &mut self,
        envelope: &crate::event_envelope::EventEnvelopeV1,
    ) -> Result<EventAppended, StorageError>;

    /// Loads a single event by its global `event_id`.
    fn load_by_event_id(
        &self,
        event_id: &str,
    ) -> Result<Option<crate::event_envelope::EventEnvelopeV1>, StorageError>;

    /// Loads a contiguous range of events from one stream.
    ///
    /// Events are returned in ascending `sequence` order. `after_sequence`
    /// filters out events `≤` the supplied value (`None` = start from sequence 1).
    /// `limit` caps the result set.
    fn load_stream(
        &self,
        stream_id: &str,
        after_sequence: Option<u64>,
        limit: u32,
    ) -> Result<Vec<crate::event_envelope::EventEnvelopeV1>, StorageError>;

    /// Returns the highest allocated sequence number for a stream, or `None`
    /// when the stream has never received an event.
    fn last_sequence(&self, stream_id: &str) -> Result<Option<u64>, StorageError>;

    /// Returns the total number of events across all streams.
    fn count(&self) -> Result<u64, StorageError>;

    /// Returns the `content_hash` of the most-recently recorded event in a stream,
    /// or `None` when the stream is empty.
    fn head_hash(&self, stream_id: &str) -> Result<Option<String>, StorageError>;

    /// Returns the `chain_hash` of the most-recently recorded event in a stream,
    /// or `None` when the stream is empty.
    fn head_chain_hash(&self, stream_id: &str) -> Result<Option<String>, StorageError>;

    /// Verifies the cryptographic chain integrity of a stream.
    ///
    /// Loads every event in the stream and recomputes each
    /// [`EventEnvelopeV1::compute_content_hash`], comparing it against the stored
    /// `content_hash` column. Returns `Ok(())` when all hashes match; returns
    /// `Err(StorageError::Other("event_store:hash_drift:<seq>"))` on first mismatch.
    fn verify_stream_chain(&self, stream_id: &str) -> Result<(), StorageError>;

    /// Verifies the stream hash chain integrity.
    ///
    /// Loads every event in the stream in sequence order and recomputes each
    /// `chain_hash`:
    /// - `chain_hash[0] = SHA256(content_hash[0] || "genesis")`
    /// - `chain_hash[N] = SHA256(content_hash[N] || chain_hash[N-1])`
    ///
    /// Returns `Ok(())` when all chain hashes match; returns
    /// `Err(StorageError::Other("event_store:chain_drift:<seq>"))` on first mismatch.
    fn verify_chain_integrity(&self, stream_id: &str) -> Result<(), StorageError>;

    /// Backfills `chain_hash` for events that lack it (pre-MIGRATION_10 events).
    /// Idempotent: skips events with a non-empty chain_hash.
    /// Returns the number of events updated.
    fn backfill_chain_hash(&mut self, stream_id: &str) -> Result<usize, StorageError>;

    /// Loads a single event by stream identifier and sequence number.
    fn load_by_sequence(
        &self,
        stream_id: &str,
        sequence: u64,
    ) -> Result<Option<crate::event_envelope::EventEnvelopeV1>, StorageError>;
}

// ── Trait object wrappers ───────────────────────────────────────────────────────

/// Wraps a `Box<dyn EventStore>` to implement `EventStore` by forwarding.
impl EventStore for Box<dyn EventStore> {
    fn append(
        &mut self,
        envelope: &crate::event_envelope::EventEnvelopeV1,
    ) -> Result<EventAppended, StorageError> {
        (**self).append(envelope)
    }

    fn load_by_event_id(
        &self,
        event_id: &str,
    ) -> Result<Option<crate::event_envelope::EventEnvelopeV1>, StorageError> {
        (**self).load_by_event_id(event_id)
    }

    fn load_stream(
        &self,
        stream_id: &str,
        after_sequence: Option<u64>,
        limit: u32,
    ) -> Result<Vec<crate::event_envelope::EventEnvelopeV1>, StorageError> {
        (**self).load_stream(stream_id, after_sequence, limit)
    }

    fn last_sequence(&self, stream_id: &str) -> Result<Option<u64>, StorageError> {
        (**self).last_sequence(stream_id)
    }

    fn count(&self) -> Result<u64, StorageError> {
        (**self).count()
    }

    fn head_hash(&self, stream_id: &str) -> Result<Option<String>, StorageError> {
        (**self).head_hash(stream_id)
    }

    fn head_chain_hash(&self, stream_id: &str) -> Result<Option<String>, StorageError> {
        (**self).head_chain_hash(stream_id)
    }

    fn verify_stream_chain(&self, stream_id: &str) -> Result<(), StorageError> {
        (**self).verify_stream_chain(stream_id)
    }

    fn verify_chain_integrity(&self, stream_id: &str) -> Result<(), StorageError> {
        (**self).verify_chain_integrity(stream_id)
    }

    fn backfill_chain_hash(&mut self, stream_id: &str) -> Result<usize, StorageError> {
        (**self).backfill_chain_hash(stream_id)
    }

    fn load_by_sequence(
        &self,
        stream_id: &str,
        sequence: u64,
    ) -> Result<Option<crate::event_envelope::EventEnvelopeV1>, StorageError> {
        (**self).load_by_sequence(stream_id, sequence)
    }
}

/// Wraps an `Arc<Mutex<dyn EventStore>>` to implement `EventStore` by locking
/// the mutex and forwarding to the inner trait object.
impl EventStore for Arc<Mutex<dyn EventStore>> {
    fn append(
        &mut self,
        envelope: &crate::event_envelope::EventEnvelopeV1,
    ) -> Result<EventAppended, StorageError> {
        let mut guard = self.lock().unwrap();
        guard.append(envelope)
    }

    fn load_by_event_id(
        &self,
        event_id: &str,
    ) -> Result<Option<crate::event_envelope::EventEnvelopeV1>, StorageError> {
        let guard = self.lock().unwrap();
        guard.load_by_event_id(event_id)
    }

    fn load_stream(
        &self,
        stream_id: &str,
        after_sequence: Option<u64>,
        limit: u32,
    ) -> Result<Vec<crate::event_envelope::EventEnvelopeV1>, StorageError> {
        let guard = self.lock().unwrap();
        guard.load_stream(stream_id, after_sequence, limit)
    }

    fn last_sequence(&self, stream_id: &str) -> Result<Option<u64>, StorageError> {
        let guard = self.lock().unwrap();
        guard.last_sequence(stream_id)
    }

    fn count(&self) -> Result<u64, StorageError> {
        let guard = self.lock().unwrap();
        guard.count()
    }

    fn head_hash(&self, stream_id: &str) -> Result<Option<String>, StorageError> {
        let guard = self.lock().unwrap();
        guard.head_hash(stream_id)
    }

    fn head_chain_hash(&self, stream_id: &str) -> Result<Option<String>, StorageError> {
        let guard = self.lock().unwrap();
        guard.head_chain_hash(stream_id)
    }

    fn verify_stream_chain(&self, stream_id: &str) -> Result<(), StorageError> {
        let guard = self.lock().unwrap();
        guard.verify_stream_chain(stream_id)
    }

    fn verify_chain_integrity(&self, stream_id: &str) -> Result<(), StorageError> {
        let guard = self.lock().unwrap();
        guard.verify_chain_integrity(stream_id)
    }

    fn backfill_chain_hash(&mut self, stream_id: &str) -> Result<usize, StorageError> {
        let mut guard = self.lock().unwrap();
        guard.backfill_chain_hash(stream_id)
    }

    fn load_by_sequence(
        &self,
        stream_id: &str,
        sequence: u64,
    ) -> Result<Option<crate::event_envelope::EventEnvelopeV1>, StorageError> {
        let guard = self.lock().unwrap();
        guard.load_by_sequence(stream_id, sequence)
    }
}

/// Wraps an `Arc<Mutex<Box<dyn EventStore>>>` to implement `EventStore` by locking
/// the mutex and forwarding to the inner trait object.
impl EventStore for Arc<Mutex<Box<dyn EventStore>>> {
    fn append(
        &mut self,
        envelope: &crate::event_envelope::EventEnvelopeV1,
    ) -> Result<EventAppended, StorageError> {
        let mut guard = self.lock().unwrap();
        guard.append(envelope)
    }

    fn load_by_event_id(
        &self,
        event_id: &str,
    ) -> Result<Option<crate::event_envelope::EventEnvelopeV1>, StorageError> {
        let guard = self.lock().unwrap();
        guard.load_by_event_id(event_id)
    }

    fn load_stream(
        &self,
        stream_id: &str,
        after_sequence: Option<u64>,
        limit: u32,
    ) -> Result<Vec<crate::event_envelope::EventEnvelopeV1>, StorageError> {
        let guard = self.lock().unwrap();
        guard.load_stream(stream_id, after_sequence, limit)
    }

    fn last_sequence(&self, stream_id: &str) -> Result<Option<u64>, StorageError> {
        let guard = self.lock().unwrap();
        guard.last_sequence(stream_id)
    }

    fn count(&self) -> Result<u64, StorageError> {
        let guard = self.lock().unwrap();
        guard.count()
    }

    fn head_hash(&self, stream_id: &str) -> Result<Option<String>, StorageError> {
        let guard = self.lock().unwrap();
        guard.head_hash(stream_id)
    }

    fn head_chain_hash(&self, stream_id: &str) -> Result<Option<String>, StorageError> {
        let guard = self.lock().unwrap();
        guard.head_chain_hash(stream_id)
    }

    fn verify_stream_chain(&self, stream_id: &str) -> Result<(), StorageError> {
        let guard = self.lock().unwrap();
        guard.verify_stream_chain(stream_id)
    }

    fn verify_chain_integrity(&self, stream_id: &str) -> Result<(), StorageError> {
        let guard = self.lock().unwrap();
        guard.verify_chain_integrity(stream_id)
    }

    fn backfill_chain_hash(&mut self, stream_id: &str) -> Result<usize, StorageError> {
        let mut guard = self.lock().unwrap();
        guard.backfill_chain_hash(stream_id)
    }

    fn load_by_sequence(
        &self,
        stream_id: &str,
        sequence: u64,
    ) -> Result<Option<crate::event_envelope::EventEnvelopeV1>, StorageError> {
        let guard = self.lock().unwrap();
        guard.load_by_sequence(stream_id, sequence)
    }
}

/// Graph store port (SPEC-004 §2). The graph is a projection — the ledger is
/// the authority; this store only persists the derived snapshot.
pub trait GraphStore {
    /// Persists the full graph state snapshot (upsert).
    fn save_state(&mut self, state: &crate::graph::GraphState) -> Result<(), StorageError>;

    /// Loads the persisted graph state, or `None` when never saved.
    fn load_state(&self) -> Result<Option<crate::graph::GraphState>, StorageError>;

    /// Returns the persisted checkpoint for the graph projection.
    fn checkpoint(&self) -> Result<Option<crate::projections::Checkpoint>, StorageError>;

    // ── v1.29.0: Workflow IR revision surface (additive) ─────────────────────
    //
    // These methods are REQUIRED — no default body. The `SqliteGraphStore`
    // implementation lives in `crates/sddk-storage/src/graph_store.rs`.

    /// Records a compiled IR digest for deduplication.
    fn record_ir_digest(&mut self, ir_hash: &str, ir_json: &str) -> Result<(), StorageError>;

    /// Records a graph revision with its parent chain.
    fn record_graph_revision(
        &mut self,
        rev: &crate::graph::ExecutionGraphRevision,
    ) -> Result<(), StorageError>;

    /// Loads all attempts for a given node run.
    fn load_node_attempts(
        &self,
        run_id: &crate::workflow_ir::RunId,
        node_id: &crate::workflow_ir::NodeId,
    ) -> Result<Vec<crate::workflow_run::Attempt>, StorageError>;

    /// Returns the number of attempts for a given node.
    fn attempt_count(
        &self,
        run_id: &crate::workflow_ir::RunId,
        node_id: &crate::workflow_ir::NodeId,
    ) -> Result<u32, StorageError>;

    /// Saves a graph revision (alias for `record_graph_revision`).
    fn save_revision(
        &mut self,
        rev: &crate::graph::ExecutionGraphRevision,
    ) -> Result<(), StorageError> {
        self.record_graph_revision(rev)
    }

    /// Loads a specific revision by run_id and revision_id.
    fn load_revision(
        &self,
        run_id: &crate::workflow_ir::RunId,
        rev_id: &crate::workflow_ir::RevisionId,
    ) -> Result<Option<crate::graph::ExecutionGraphRevision>, StorageError>;

    /// Loads the latest (highest-revision) graph revision for a run.
    fn latest_revision(
        &self,
        run_id: &crate::workflow_ir::RunId,
    ) -> Result<Option<crate::graph::ExecutionGraphRevision>, StorageError>;

    // ── v1.37.1: Workflow runtime surface (additive, cycle-16) ────────────────
    //
    // These methods have default implementations returning NotImplemented.
    // Implementations (e.g. SqliteGraphStore) override them with real behavior.

    /// Records a node run in the graph store.
    ///
    /// Default implementation returns `Err(StorageError::Other("not implemented"))`.
    fn record_node_run(&mut self, _run: &crate::workflow_run::NodeRun) -> Result<(), StorageError> {
        Err(StorageError::Other(
            "record_node_run not implemented".into(),
        ))
    }

    /// Records an attempt in the graph store.
    ///
    /// Default implementation returns `Err(StorageError::Other("not implemented"))`.
    fn record_attempt(
        &mut self,
        _attempt: &crate::workflow_run::Attempt,
    ) -> Result<(), StorageError> {
        Err(StorageError::Other("record_attempt not implemented".into()))
    }

    /// Loads a workflow run by run_id.
    ///
    /// Default implementation returns `Err(StorageError::Other("not implemented"))`.
    fn load_run(
        &self,
        _run_id: &crate::workflow_ir::RunId,
    ) -> Result<Option<crate::workflow_run::WorkflowRun>, StorageError> {
        Err(StorageError::Other("load_run not implemented".into()))
    }

    /// Loads a specific node run by run_id and node_id.
    ///
    /// Default implementation returns `Err(StorageError::Other("not implemented"))`.
    fn load_node_run(
        &self,
        _run_id: &crate::workflow_ir::RunId,
        _node_id: &crate::workflow_ir::NodeId,
    ) -> Result<Option<crate::workflow_run::NodeRun>, StorageError> {
        Err(StorageError::Other("load_node_run not implemented".into()))
    }

    /// Lists all attempts for a given node.
    ///
    /// Default implementation returns `Err(StorageError::Other("not implemented"))`.
    fn list_attempts(
        &self,
        _run_id: &crate::workflow_ir::RunId,
        _node_id: &crate::workflow_ir::NodeId,
    ) -> Result<Vec<crate::workflow_run::Attempt>, StorageError> {
        Err(StorageError::Other("list_attempts not implemented".into()))
    }

    /// Returns the latest attempt for a node, or None if no attempts exist.
    ///
    /// Default implementation returns `Err(StorageError::Other("not implemented"))`.
    fn latest_attempt(
        &self,
        _run_id: &crate::workflow_ir::RunId,
        _node_id: &crate::workflow_ir::NodeId,
    ) -> Result<Option<crate::workflow_run::Attempt>, StorageError> {
        Err(StorageError::Other("latest_attempt not implemented".into()))
    }

    /// Streams all node runs for a given workflow run.
    ///
    /// Default implementation returns `Err(StorageError::Other("not implemented"))`.
    fn stream_node_runs(
        &self,
        _run_id: &crate::workflow_ir::RunId,
    ) -> Result<Vec<crate::workflow_run::NodeRun>, StorageError> {
        Err(StorageError::Other(
            "stream_node_runs not implemented".into(),
        ))
    }
}

// ── TaskExecutor port (cycle-16 remediation) ──────────────────────────────────

use serde_json::Value;
use std::collections::BTreeMap;

/// Output from a task execution.
#[derive(Debug, Clone, PartialEq)]
pub struct TaskOutput {
    /// Output key-value pairs produced by the task.
    pub outputs: BTreeMap<String, Value>,
}

/// Errors from task execution.
#[derive(Debug, Clone)]
pub struct TaskError {
    /// Human-readable error message.
    pub message: String,
}

/// Port for executing capability tasks.
///
/// The runtime holds an `Arc<dyn TaskExecutor>` and calls it during
/// `Operator::Task::evaluate`. Cycle-16 ships `NoopTaskExecutor` which
/// returns success immediately. Real routing is deferred to cycle-17.
pub trait TaskExecutor: Send + Sync {
    /// Executes a task with the given inputs.
    ///
    /// Returns `Ok(TaskOutput)` on success or `Err(TaskError)` on failure.
    fn execute(
        &self,
        capability: &str,
        inputs: &BTreeMap<String, Value>,
    ) -> Result<TaskOutput, TaskError>;
}

/// No-op task executor used in cycle-16.
///
/// Always returns success with empty outputs. Real capability routing
/// is deferred to cycle-17.
#[derive(Debug, Clone, Default)]
pub struct NoopTaskExecutor;

impl TaskExecutor for NoopTaskExecutor {
    fn execute(
        &self,
        _capability: &str,
        _inputs: &BTreeMap<String, Value>,
    ) -> Result<TaskOutput, TaskError> {
        Ok(TaskOutput {
            outputs: BTreeMap::new(),
        })
    }
}

/// Artifact store port (Phase 1 MUST).
///
/// Artifacts are immutable blobs stored outside SQLite; this port governs metadata only.
/// The concrete implementation is `sddk_storage::Storage` which satisfies it via
/// `impl ArtifactStore for Storage` in the storage crate.
pub trait ArtifactStore {
    /// Inserts artifact metadata.
    ///
    /// Returns error if `artifact_id` already exists (idempotent — use get+replace
    /// if overwriting is needed).
    fn insert_artifact(
        &mut self,
        artifact: &crate::models::ArtifactRecord,
    ) -> Result<(), StorageError>;

    /// Loads artifact metadata by identifier, or `None` if not found.
    fn get_artifact(
        &self,
        artifact_id: &str,
    ) -> Result<Option<crate::models::ArtifactRecord>, StorageError>;

    /// Lists all artifact metadata for a project.
    fn list_project_artifacts(
        &self,
        project_id: &str,
    ) -> Result<Vec<crate::models::ArtifactRecord>, StorageError>;
}
