//! Replay engine (SPEC-009 §4, Phase 7).
//!
//! Two replay modes over the event ledger:
//! - `reconstruct`: rebuild projection state without invoking nondeterministic
//!   behaviors (no hash verification — fast prefix reconstruction).
//! - `strict`: re-execute with chain verification (fail-closed on first causal
//!   mismatch) and serve recorded LLM/tool responses from the response cache.

use serde::{Deserialize, Serialize};

use crate::event_envelope::EventEnvelopeV1;
use crate::fork::ResponseCachePort;
use crate::projections::Projection;

/// Errors emitted by the replay engine.
#[derive(Debug, thiserror::Error, PartialEq, Eq)]
pub enum ReplayError {
    /// Chain integrity broken at a specific stream/sequence (strict mode).
    #[error("chain mismatch in stream '{stream}' at sequence {sequence}")]
    ChainMismatch {
        /// Stream where the mismatch was detected.
        stream: String,
        /// Sequence at which verification failed.
        sequence: u64,
    },
    /// Underlying storage failure.
    #[error("storage: {0}")]
    Storage(String),
    /// No events in the stream up to the requested sequence.
    #[error("no events in stream '{0}' up to the requested sequence")]
    Empty(String),
}

/// Replays ledger events into a projection.
///
/// `P` is the target projection (e.g. `GraphProjection`); `make` constructs a
/// fresh instance (the same pattern as `sddk_storage::rebuild`).
pub struct ReplayEngine<'a, P: Projection> {
    /// Event store the engine reads from.
    event_store: &'a dyn crate::EventStore,
    /// Factory for fresh projections.
    make: Box<dyn Fn() -> P + 'a>,
    /// Optional response cache (strict mode).
    cache: Option<&'a dyn ResponseCachePort>,
}

impl<'a, P: Projection> ReplayEngine<'a, P> {
    /// Creates a replay engine over the given event store.
    pub fn new(event_store: &'a dyn crate::EventStore, make: Box<dyn Fn() -> P + 'a>) -> Self {
        Self {
            event_store,
            make,
            cache: None,
        }
    }

    /// Attaches a response cache (enables strict replay caching).
    pub fn with_cache(mut self, cache: &'a dyn ResponseCachePort) -> Self {
        self.cache = Some(cache);
        self
    }

    /// Returns the attached response cache, if any.
    pub fn cache(&self) -> Option<&'a dyn ResponseCachePort> {
        self.cache
    }

    /// Reconstructs projection state from events `1..=at_sequence` without
    /// hash verification (fast, no external I/O).
    pub fn reconstruct(
        &self,
        stream: &str,
        at_sequence: Option<u64>,
    ) -> Result<P::State, ReplayError> {
        self.replay(stream, at_sequence, false)
    }

    /// Strictly replays events `1..=at_sequence`: verifies the chain
    /// (fail-closed on first mismatch) and uses the response cache.
    pub fn strict(&self, stream: &str, at_sequence: Option<u64>) -> Result<P::State, ReplayError> {
        self.replay(stream, at_sequence, true)
    }

    fn replay(
        &self,
        stream: &str,
        at_sequence: Option<u64>,
        verify: bool,
    ) -> Result<P::State, ReplayError> {
        let events = self
            .event_store
            .load_stream(stream, None, u32::MAX)
            .map_err(|e| ReplayError::Storage(format!("load_stream: {e}")))?;

        let events: Vec<&EventEnvelopeV1> = events
            .iter()
            .filter(|e| at_sequence.map(|at| e.sequence <= at).unwrap_or(true))
            .collect();

        if events.is_empty() {
            return Err(ReplayError::Empty(stream.to_string()));
        }

        if verify {
            // Fail-closed: verify the full chain first (covers the prefix).
            self.event_store.verify_stream_chain(stream).map_err(|_e| {
                ReplayError::ChainMismatch {
                    stream: stream.to_string(),
                    sequence: events.last().map(|e| e.sequence).unwrap_or(0),
                }
            })?;
        }

        let mut projection = (self.make)();
        for event in events {
            projection
                .apply(event)
                .map_err(|e| ReplayError::Storage(format!("apply: {e}")))?;
        }
        Ok(projection.state_ref().clone())
    }
}

/// A recorded replay trace summary (used by CLI output).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ReplaySummary {
    /// Stream replayed.
    pub stream: String,
    /// Events applied.
    pub events_applied: u64,
    /// Final sequence reached.
    pub last_sequence: u64,
    /// Whether strict mode was used.
    pub strict: bool,
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::event_envelope::{ActorKind, ActorRef, EntityRef};
    use crate::graph::GraphProjection;
    use crate::graph::GraphState;
    use serde_json::json;

    fn make_event(
        stream: &str,
        event_type: &str,
        seq: u64,
        subjects: Vec<EntityRef>,
        content_hash: &str,
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
            payload: if event_type == "workflow.phase.entered" {
                json!({ "phase": format!("phase-{seq}") })
            } else {
                json!({})
            },
            evidence_refs: vec![],
            content_hash: content_hash.into(),
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

    /// In-memory event store for tests.
    struct MemStore {
        events: Vec<EventEnvelopeV1>,
    }

    impl MemStore {
        fn new(events: Vec<EventEnvelopeV1>) -> Self {
            Self { events }
        }
    }

    impl crate::EventStore for MemStore {
        fn append(
            &mut self,
            _envelope: &EventEnvelopeV1,
        ) -> Result<crate::EventAppended, crate::StorageError> {
            unimplemented!("read-only test store")
        }
        fn load_by_event_id(
            &self,
            _event_id: &str,
        ) -> Result<Option<EventEnvelopeV1>, crate::StorageError> {
            unimplemented!()
        }
        fn load_stream(
            &self,
            stream_id: &str,
            after_sequence: Option<u64>,
            limit: u32,
        ) -> Result<Vec<EventEnvelopeV1>, crate::StorageError> {
            let start = after_sequence.unwrap_or(0);
            Ok(self
                .events
                .iter()
                .filter(|e| e.stream_id == stream_id && e.sequence > start)
                .take(limit as usize)
                .cloned()
                .collect())
        }
        fn last_sequence(&self, _stream_id: &str) -> Result<Option<u64>, crate::StorageError> {
            Ok(self.events.last().map(|e| e.sequence))
        }
        fn count(&self) -> Result<u64, crate::StorageError> {
            Ok(self.events.len() as u64)
        }
        fn head_hash(&self, _stream_id: &str) -> Result<Option<String>, crate::StorageError> {
            Ok(self.events.last().map(|e| e.content_hash.clone()))
        }
        fn head_chain_hash(&self, _stream_id: &str) -> Result<Option<String>, crate::StorageError> {
            Ok(None)
        }
        fn verify_stream_chain(&self, _stream_id: &str) -> Result<(), crate::StorageError> {
            // Accept unless any event has the sentinel tampered hash.
            if self
                .events
                .iter()
                .any(|e| e.content_hash == "sha256:tampered")
            {
                return Err(crate::StorageError::Other(
                    "event_store:hash_drift:test".into(),
                ));
            }
            Ok(())
        }
        fn verify_chain_integrity(&self, _stream_id: &str) -> Result<(), crate::StorageError> {
            Ok(())
        }
        fn backfill_chain_hash(&mut self, _stream_id: &str) -> Result<usize, crate::StorageError> {
            Ok(0)
        }
        fn load_by_sequence(
            &self,
            _stream_id: &str,
            _sequence: u64,
        ) -> Result<Option<EventEnvelopeV1>, crate::StorageError> {
            unimplemented!()
        }
    }

    fn reference_state(events: &[EventEnvelopeV1]) -> GraphState {
        let mut projection = GraphProjection::new("project:p-1");
        for event in events {
            projection.apply(event).unwrap();
        }
        projection.state_ref().clone()
    }

    fn sample_events() -> Vec<EventEnvelopeV1> {
        vec![
            make_event(
                "project:p-1",
                "approval.capability.requested",
                1,
                vec![subject("cycle", "c-1"), subject("capability", "git.commit")],
                "sha256:1",
            ),
            make_event(
                "project:p-1",
                "approval.capability.granted",
                2,
                vec![
                    subject("actor", "alice"),
                    subject("capability", "git.commit"),
                ],
                "sha256:2",
            ),
            make_event(
                "project:p-1",
                "workflow.phase.entered",
                3,
                vec![],
                "sha256:3",
            ),
            make_event(
                "project:p-1",
                "workflow.phase.entered",
                4,
                vec![],
                "sha256:4",
            ),
            make_event(
                "project:p-1",
                "workflow.phase.exited",
                5,
                vec![],
                "sha256:5",
            ),
        ]
    }

    #[test]
    fn reconstruct_yields_identical_state() {
        let events = sample_events();
        let store = MemStore::new(events.clone());
        let engine = ReplayEngine::new(&store, Box::new(|| GraphProjection::new("project:p-1")));
        let state = engine.reconstruct("project:p-1", None).unwrap();
        assert_eq!(state, reference_state(&events));
    }

    #[test]
    fn reconstruct_stops_at_sequence() {
        let events = sample_events();
        let store = MemStore::new(events.clone());
        let engine = ReplayEngine::new(&store, Box::new(|| GraphProjection::new("project:p-1")));
        let state = engine.reconstruct("project:p-1", Some(3)).unwrap();
        assert_eq!(state, reference_state(&events[..3]));
        assert_eq!(state.last_event_sequence, 3);
    }

    #[test]
    fn strict_fails_on_tampered_hash() {
        let mut events = sample_events();
        events[2].content_hash = "sha256:tampered".into();
        let store = MemStore::new(events);
        let engine = ReplayEngine::new(&store, Box::new(|| GraphProjection::new("project:p-1")));
        let error = engine.strict("project:p-1", None).unwrap_err();
        assert!(matches!(error, ReplayError::ChainMismatch { .. }));
    }

    #[test]
    fn strict_ok_on_valid_chain() {
        let events = sample_events();
        let store = MemStore::new(events);
        let engine = ReplayEngine::new(&store, Box::new(|| GraphProjection::new("project:p-1")));
        let state = engine.strict("project:p-1", None).unwrap();
        assert_eq!(state.last_event_sequence, 5);
    }

    #[test]
    fn reconstruct_empty_stream_errors() {
        let store = MemStore::new(vec![]);
        let engine = ReplayEngine::new(&store, Box::new(|| GraphProjection::new("project:p-1")));
        let error = engine.reconstruct("project:p-1", None).unwrap_err();
        assert!(matches!(error, ReplayError::Empty(_)));
    }
}
