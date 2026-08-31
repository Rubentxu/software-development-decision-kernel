//! Generic projection rebuild over the event ledger.
//!
//! The [`rebuild`] algorithm is canonical:
//! 1. Load events from the event store
//! 2. Verify chain integrity (fail-closed)
//! 3. Build a fresh projection instance via the closure factory
//! 4. Apply each event in sequence order
//! 5. Delete any prior checkpoint (idempotent reset)
//! 6. Persist the new checkpoint + serialized state
//!
//! Concurrency: NOT safe for concurrent `rebuild` calls on the same projection
//! due to SQLite's lack of row-level locking. The CLI is single-process; this
//! is acceptable for v1.

use sddk_domain::{EventStore, Projection, ProjectionError, StorageError as DomainStorageError};

use crate::event_store::SqliteEventStore;
use crate::projection_store::SqliteProjectionStore;

/// Rebuild a projection from the event ledger.
///
/// Reads events via `event_store`, verifies chain integrity (fail-closed),
/// applies each event through the projection, and persists the resulting
/// checkpoint + state to `projection_checkpoints_v1`.
///
/// The closure factory `make` is called exactly once to construct a fresh
/// projection instance. It accepts no arguments and returns `P`. This pattern
/// avoids requiring `Default` on `P` (which would conflict with
/// `CycleStateProjection::new(stream_id)`).
///
/// On chain integrity failure, this function returns `Err` BEFORE constructing
/// the projection or persisting any checkpoint — fail-closed guarantee.
pub fn rebuild<P, F, S>(
    event_store: &SqliteEventStore,
    proj_store: &mut SqliteProjectionStore,
    make: F,
    stream_id: &str,
    from_sequence: Option<u64>,
) -> Result<S, ProjectionError>
where
    P: Projection<State = S>,
    S: serde::Serialize + serde::de::DeserializeOwned + Default + Clone,
    F: FnOnce() -> P,
{
    // 1. Load events from the ledger.
    let events = event_store
        .load_stream(stream_id, from_sequence, u32::MAX)
        .map_err(|e| ProjectionError::Storage(format!("load_stream: {e}")))?;

    // 2. Chain verify — fail-closed before any mutation.
    if let Err(e) = event_store.verify_stream_chain(stream_id) {
        return Err(match e {
            DomainStorageError::Other(_msg) => ProjectionError::ChainIntegrityBroken {
                stream_id: stream_id.to_string(),
                sequence: events.last().map(|ev| ev.sequence).unwrap_or(0),
            },
            other => ProjectionError::Storage(format!("verify_stream_chain: {other:?}")),
        });
    }

    // 3. Build a fresh projection instance (closure factory — called exactly once).
    let mut projection = make();

    // 4. Apply each event in sequence order.
    for event in &events {
        projection.apply(event)?;
    }

    // 5. Only persist if we actually applied events (empty stream → no checkpoint).
    if events.is_empty() {
        return Ok(projection.state_ref().clone());
    }

    // 5a. Serialize state for persistence.
    let state_json = serde_json::to_string(projection.state_ref())
        .map_err(|e| ProjectionError::Storage(format!("state serialize: {e}")))?;
    let cp = projection.checkpoint();

    // 5b. Delete any prior checkpoint (idempotent — first rebuild has no prior).
    proj_store
        .delete_checkpoint(&cp.projection_name, cp.version)
        .map_err(|e| ProjectionError::Storage(format!("delete_checkpoint: {e}")))?;

    // 5c. Persist the new checkpoint + state.
    proj_store
        .save_checkpoint(&cp, &state_json)
        .map_err(|e| ProjectionError::Storage(format!("save_checkpoint: {e}")))?;

    Ok(projection.state_ref().clone())
}
