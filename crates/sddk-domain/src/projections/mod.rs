//! Read-model projections over [`EventEnvelopeV1`].
//!
//! Projections are deterministic, stateless read-models that derive from the
//! append-only event ledger. Each projection implements [`Projection::apply`] to
//! consume events and [`Projection::checkpoint`] to produce a durable progress
//! marker. The [`rebuild`] algorithm in `sddk_storage` uses these to reconstruct
//! a projection from the ledger.
//!
//! [`rebuild`]: sddk_storage::rebuild

use serde::{Deserialize, Serialize};

pub mod approval;
pub mod cycle_state;
pub mod journal;

// ── Shared types defined here (used by multiple submodules) ──────────────────

/// Schema version for a projection — bumped when [`apply`](Projection::apply) semantics change.
pub type ProjectionVersion = u32;

/// Persistent checkpoint for a projection, persisted to the
/// `projection_checkpoints_v1` table.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Checkpoint {
    /// Canonical projection name used in the checkpoint table primary key.
    pub projection_name: String,
    /// Schema version of the projection that wrote this checkpoint.
    pub version: ProjectionVersion,
    /// Monotonic sequence number of the last event applied.
    pub last_event_sequence: u64,
    /// SHA-256 content hash of the last event applied, in `sha256:<64-hex>` format.
    pub last_event_hash: String,
    /// Wall-clock time when this checkpoint was written (RFC 3339).
    pub updated_at: String,
}

/// Errors that may arise when applying events or rebuilding a projection.
#[derive(Debug, thiserror::Error, PartialEq, Eq)]
pub enum ProjectionError {
    /// The named projection is not registered.
    #[error("projection '{0}' is not registered")]
    UnknownProjection(String),

    /// An event payload could not be interpreted.
    #[error("invalid event payload for '{event_type}': {detail}")]
    InvalidPayload {
        /// The event type that failed parsing.
        event_type: String,
        /// Why parsing failed.
        detail: String,
    },

    /// The event store's content-hash chain is broken at the given sequence.
    /// The rebuild algorithm fails closed: no checkpoint is persisted when
    /// chain integrity is lost.
    #[error("event chain integrity broken for stream '{stream_id}' at sequence {sequence}")]
    ChainIntegrityBroken {
        /// Stream where the break was detected.
        stream_id: String,
        /// Sequence at which verification failed.
        sequence: u64,
    },

    /// Underlying storage failure.
    #[error("storage: {0}")]
    Storage(String),
}

// ── Re-exports (must come after submodules so items are accessible) ─────────────

// Re-export all public types for external consumers.
// External consumers (e.g. crates/sddk-cli/src/approval.rs, crates/sddk-domain/src/replay.rs,
// crates/sddk-domain/src/graph.rs) use:
//   sddk_domain::projections::{Checkpoint, CycleState, CycleStateProjection, JournalEntry,
//   JournalProjection, Projection, ProjectionError, ProjectionVersion}
pub use approval::ApprovalProjection;
pub use cycle_state::{CycleState, CycleStateProjection};
pub use journal::{JournalEntry, JournalProjection};

// ── Projection trait ───────────────────────────────────────────────────────────

/// A read-model projection over [`EventEnvelopeV1`]. The projection must
/// be deterministic for a fixed input stream: calling [`apply`](Projection::apply)
/// with the same ordered events always produces the same checkpoint.
///
/// Implementations are expected to be idempotent for a given `(event_id, event_hash)`.
///
/// [`EventEnvelopeV1`]: super::EventEnvelopeV1
pub trait Projection {
    /// The serialized state produced by this projection.
    type State: serde::Serialize + for<'de> serde::Deserialize<'de> + Default + Clone;

    /// Canonical name used as the primary key in the checkpoint table.
    fn name(&self) -> &str;

    /// Schema version. Increase when [`apply`](Projection::apply) semantics change.
    fn version(&self) -> ProjectionVersion;

    /// Apply one event to the projection's state.
    ///
    /// Implementations must update monotone fields (`last_event_sequence`,
    /// `last_event_hash`) on every call regardless of event type, so that a
    /// restarted rebuild can pick up where it left off.
    fn apply(&mut self, event: &super::EventEnvelopeV1) -> Result<(), ProjectionError>;

    /// Build the current checkpoint from in-memory state.
    fn checkpoint(&self) -> Checkpoint;

    /// Borrow the current state for serialization.
    fn state_ref(&self) -> &Self::State;
}
