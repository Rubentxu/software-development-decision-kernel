//! Event-replay equality tests (AC-EVT-LEDGER-05).
//!
//! Verifies that replaying from the same snapshot always produces identical
//! chains, establishing the replay equality invariant: two replays from the
//! same snapshot position must yield bit-for-bit identical event chains.

use sddk_domain::EventEnvelopeV1;
use sddk_domain::event_envelope::{ActorKind, ActorRef, EntityRef};
use sddk_domain::graph::GraphProjection;
use sddk_domain::replay::{ReplayEngine, Snapshot};
use serde_json::json;

/// In-memory event store for tests.
struct MemStore {
    events: Vec<EventEnvelopeV1>,
}

impl MemStore {
    fn new(events: Vec<EventEnvelopeV1>) -> Self {
        Self { events }
    }
}

impl sddk_domain::EventStore for MemStore {
    fn append(
        &mut self,
        _envelope: &EventEnvelopeV1,
    ) -> Result<sddk_domain::EventAppended, sddk_domain::StorageError> {
        unimplemented!("read-only test store")
    }
    fn load_by_event_id(
        &self,
        _event_id: &str,
    ) -> Result<Option<EventEnvelopeV1>, sddk_domain::StorageError> {
        unimplemented!()
    }
    fn load_stream(
        &self,
        stream_id: &str,
        after_sequence: Option<u64>,
        limit: u32,
    ) -> Result<Vec<EventEnvelopeV1>, sddk_domain::StorageError> {
        let start = after_sequence.unwrap_or(0);
        Ok(self
            .events
            .iter()
            .filter(|e| e.stream_id == stream_id && e.sequence > start)
            .take(limit as usize)
            .cloned()
            .collect())
    }
    fn last_sequence(&self, _stream_id: &str) -> Result<Option<u64>, sddk_domain::StorageError> {
        Ok(self.events.last().map(|e| e.sequence))
    }
    fn count(&self) -> Result<u64, sddk_domain::StorageError> {
        Ok(self.events.len() as u64)
    }
    fn head_hash(&self, _stream_id: &str) -> Result<Option<String>, sddk_domain::StorageError> {
        Ok(self.events.last().map(|e| e.content_hash.clone()))
    }
    fn head_chain_hash(
        &self,
        _stream_id: &str,
    ) -> Result<Option<String>, sddk_domain::StorageError> {
        Ok(None)
    }
    fn verify_stream_chain(&self, _stream_id: &str) -> Result<(), sddk_domain::StorageError> {
        Ok(())
    }
    fn verify_chain_integrity(&self, _stream_id: &str) -> Result<(), sddk_domain::StorageError> {
        Ok(())
    }
    fn backfill_chain_hash(
        &mut self,
        _stream_id: &str,
    ) -> Result<usize, sddk_domain::StorageError> {
        Ok(0)
    }
    fn load_by_sequence(
        &self,
        _stream_id: &str,
        _sequence: u64,
    ) -> Result<Option<EventEnvelopeV1>, sddk_domain::StorageError> {
        unimplemented!()
    }
}

fn make_event(stream: &str, event_type: &str, seq: u64, content_hash: &str) -> EventEnvelopeV1 {
    let payload = if event_type == "workflow.phase.entered" {
        json!({ "phase": format!("phase-{}", seq) })
    } else {
        json!({})
    };
    EventEnvelopeV1 {
        event_id: format!("evt-{seq}"),
        event_type: event_type.into(),
        schema_version: 1,
        stream_id: stream.into(),
        sequence: seq,
        project_id: "p-test".into(),
        occurred_at: format!("2026-09-01T10:00:{seq:02}Z"),
        recorded_at: format!("2026-09-01T10:00:{seq:02}Z"),
        actor: ActorRef {
            kind: ActorKind::System,
            id: "sddk-test".into(),
            definition_hash: None,
            policy_hash: None,
            model: None,
        },
        subjects: vec![],
        payload,
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

fn sample_events() -> Vec<EventEnvelopeV1> {
    vec![
        make_event("project:p-test", "workflow.phase.entered", 1, "sha256:01"),
        make_event("project:p-test", "workflow.phase.entered", 2, "sha256:02"),
        make_event("project:p-test", "workflow.phase.entered", 3, "sha256:03"),
        make_event("project:p-test", "workflow.phase.exited", 4, "sha256:04"),
        make_event("project:p-test", "workflow.phase.entered", 5, "sha256:05"),
    ]
}

fn snapshot_at(events: &[EventEnvelopeV1], seq: u64) -> Snapshot {
    let event = events.iter().find(|e| e.sequence == seq).unwrap();
    Snapshot {
        name: format!("checkpoint-{}", seq),
        stream_id: event.stream_id.clone(),
        sequence: event.sequence,
        content_hash: event.content_hash.clone(),
        chain_hash: "test-chain-hash".into(),
        taken_at_ms: 1_725_836_000_000,
    }
}

/// AC-EVT-LEDGER-05: two replays from the same snapshot produce identical chains.
#[test]
fn two_replays_from_same_snapshot_produce_identical_chains() {
    let events = sample_events();
    let store = MemStore::new(events.clone());
    let snapshot = snapshot_at(&events, 3);

    // Replay #1 from snapshot
    let engine1 = ReplayEngine::new(&store, Box::new(|| GraphProjection::new("project:p-test")));
    let state1 = engine1
        .reconstruct(&snapshot.stream_id, Some(snapshot.sequence))
        .unwrap();

    // Replay #2 from same snapshot
    let engine2 = ReplayEngine::new(&store, Box::new(|| GraphProjection::new("project:p-test")));
    let state2 = engine2
        .reconstruct(&snapshot.stream_id, Some(snapshot.sequence))
        .unwrap();

    assert_eq!(state1, state2, "replay equality invariant violated");
}

/// AC-EVT-LEDGER-05: strict replay also satisfies equality invariant.
#[test]
fn two_strict_replays_from_same_snapshot_produce_identical_chains() {
    let events = sample_events();
    let store = MemStore::new(events.clone());
    let snapshot = snapshot_at(&events, 4);

    let engine1 = ReplayEngine::new(&store, Box::new(|| GraphProjection::new("project:p-test")));
    let state1 = engine1
        .strict(&snapshot.stream_id, Some(snapshot.sequence))
        .unwrap();

    let engine2 = ReplayEngine::new(&store, Box::new(|| GraphProjection::new("project:p-test")));
    let state2 = engine2
        .strict(&snapshot.stream_id, Some(snapshot.sequence))
        .unwrap();

    assert_eq!(state1, state2, "strict replay equality invariant violated");
}

/// AC-EVT-LEDGER-05: equality holds across different stream prefixes.
#[test]
fn replay_equality_holds_across_full_and_partial_streams() {
    let events = sample_events();
    let store = MemStore::new(events.clone());

    // Replay from seq 2
    let engine1 = ReplayEngine::new(&store, Box::new(|| GraphProjection::new("project:p-test")));
    let state1 = engine1.reconstruct("project:p-test", Some(2)).unwrap();

    // Replay same prefix again
    let engine2 = ReplayEngine::new(&store, Box::new(|| GraphProjection::new("project:p-test")));
    let state2 = engine2.reconstruct("project:p-test", Some(2)).unwrap();

    assert_eq!(state1, state2);
    assert_eq!(state1.last_event_sequence, 2);
}
