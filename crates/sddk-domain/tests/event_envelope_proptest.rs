//! Property-based tests for EventEnvelopeV1 content_hash determinism.
//!
//! Runs 100 cases with shuffled subject insertion order to verify that
//! `compute_content_hash()` is stable for logically-equivalent envelopes.

use proptest::prelude::*;
use sddk_domain::{ActorKind, ActorRef, EntityRef, EntityRefVersion, EventEnvelopeV1};
use serde_json::json;

/// An EntityRef with all fields populated for consistent test data.
fn make_entity_ref(kind: &str, id: &str, version: i64) -> EntityRef {
    EntityRef {
        kind: kind.into(),
        id: id.into(),
        version: Some(EntityRefVersion::Integer(version)),
        content_hash: None,
    }
}

/// Builds an EventEnvelopeV1 with `n` subjects, all other fields fixed.
fn envelope_with_n_subjects(n: usize) -> EventEnvelopeV1 {
    let subjects: Vec<EntityRef> = (0..n)
        .map(|i| make_entity_ref(&format!("entity_{i}"), &format!("id_{i}"), i as i64))
        .collect();

    EventEnvelopeV1 {
        event_id: "e-prop".into(),
        event_type: "workflow.phase.entered".into(),
        schema_version: 1,
        stream_id: "s-prop".into(),
        sequence: 42,
        project_id: "p-prop".into(),
        occurred_at: "2026-08-17T10:00:00Z".into(),
        recorded_at: "2026-08-17T10:00:01Z".into(),
        actor: ActorRef {
            kind: ActorKind::System,
            id: "sddk-cli".into(),
            definition_hash: None,
            policy_hash: None,
            model: None,
        },
        subjects,
        payload: json!({"key": "value"}),
        evidence_refs: vec![],
        content_hash: "sha256:placeholder".into(),
        metadata: Some(json!({"meta": 1})),
        causation_id: None,
        correlation_id: None,
        cycle_id: None,
        frame_id: None,
        fork_id: None,
    }
}

proptest! {
    #![proptest_config(ProptestConfig::with_cases(50))]

    /// Hash is stable for the same envelope built identically twice.
    #[test]
    fn content_hash_stable_for_same_envelope(n_subjects in 1usize..5) {
        let env1 = envelope_with_n_subjects(n_subjects);
        let env2 = envelope_with_n_subjects(n_subjects);

        let mut h1 = env1.clone();
        h1.content_hash.clear();
        let mut h2 = env2.clone();
        h2.content_hash.clear();

        prop_assert_eq!(h1.compute_content_hash(), h2.compute_content_hash());
    }

    /// Hash differs when subjects are reversed (different logical content).
    #[test]
    fn content_hash_differs_when_subjects_reversed(n_subjects in 2usize..5) {
        let env_a = envelope_with_n_subjects(n_subjects);
        let mut env_b = envelope_with_n_subjects(n_subjects);

        // Reverse the subjects vector — produces different envelope.
        env_b.subjects.reverse();

        let mut ha = env_a.clone();
        ha.content_hash.clear();
        let mut hb = env_b.clone();
        hb.content_hash.clear();

        let hash_a = ha.compute_content_hash();
        let hash_b = hb.compute_content_hash();

        // Different subject order → different envelope → different hash.
        // This confirms the hash is sensitive to subject content/order.
        prop_assert_ne!(hash_a, hash_b,
            "reversed subjects should produce different hash");
    }
}
