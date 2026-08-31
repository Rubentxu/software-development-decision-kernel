//! Envelope builder helpers for the event bus.

use sddk_domain::{ActorRef, EntityRef, EventEnvelopeV1};

use crate::TransitionOutcome;

use super::emit::OutcomeEventInput;

// ── Envelope builders ──────────────────────────────────────────────────────────

/// Builds an `EventEnvelopeV1` for a transition-outcome event.
pub fn build_outcome_envelope(
    event_id: String,
    event_type: &str,
    input: &OutcomeEventInput,
) -> EventEnvelopeV1 {
    let payload = serde_json::json!({
        "transition_id": input.transition_id,
        "outcome": outcome_from_enum(event_type),
        "from_phase": input.from_phase,
        "to_phase": input.to_phase,
        "failed_gates": input.failed_gates,
    });
    let mut env = EventEnvelopeV1 {
        event_id,
        event_type: event_type.to_string(),
        schema_version: 1,
        stream_id: input.cycle_id.clone(),
        sequence: 0,
        project_id: input.project_id.clone(),
        occurred_at: input.transition_at.clone(),
        recorded_at: input.transition_at.clone(),
        actor: ActorRef {
            kind: input.actor_kind,
            id: input.actor_id.clone(),
            definition_hash: None,
            policy_hash: None,
            model: None,
        },
        subjects: vec![EntityRef {
            kind: "cycle".into(),
            id: input.cycle_id.clone(),
            version: None,
            content_hash: None,
        }],
        payload,
        evidence_refs: vec![],
        content_hash: String::new(),
        metadata: None,
        causation_id: None,
        correlation_id: None,
        cycle_id: Some(input.cycle_id.clone()),
        frame_id: None,
        fork_id: None,
    };
    env.content_hash = env.compute_content_hash();
    env
}

/// Converts an event_type string to a TransitionOutcome enum value for the payload.
/// `workflow.transition.succeeded` → Succeeded, `workflow.transition.failed` → Failed.
fn outcome_from_enum(event_type: &str) -> TransitionOutcome {
    match event_type {
        "workflow.transition.succeeded" => TransitionOutcome::Succeeded,
        "workflow.transition.failed" => TransitionOutcome::Failed,
        _ => TransitionOutcome::Failed,
    }
}

/// Builds an `EventEnvelopeV1` for a phase transition event.
pub fn build_event_envelope(
    event_id: &str,
    event_type: &str,
    phase_label: &str,
    input: &super::emit::PhaseEventInput,
) -> EventEnvelopeV1 {
    let payload = serde_json::json!({ "phase": phase_label });
    let mut env = EventEnvelopeV1 {
        event_id: event_id.to_string(),
        event_type: event_type.to_string(),
        schema_version: 1,
        stream_id: input.cycle_id.clone(),
        sequence: 0,
        project_id: input.project_id.clone(),
        occurred_at: input.transition_at.clone(),
        recorded_at: input.transition_at.clone(),
        actor: ActorRef {
            kind: input.actor_kind,
            id: input.actor_id.clone(),
            definition_hash: None,
            policy_hash: None,
            model: None,
        },
        subjects: vec![EntityRef {
            kind: "cycle".into(),
            id: input.cycle_id.clone(),
            version: None,
            content_hash: None,
        }],
        payload,
        evidence_refs: vec![],
        content_hash: String::new(),
        metadata: None,
        causation_id: None,
        correlation_id: None,
        cycle_id: Some(input.cycle_id.clone()),
        frame_id: None,
        fork_id: None,
    };
    env.content_hash = env.compute_content_hash();
    env
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::emit::PhaseEventInput;
    use sddk_domain::ActorKind;

    #[test]
    fn build_event_envelope_produces_valid_envelope() {
        let input = PhaseEventInput {
            project_id: "p-1".into(),
            cycle_id: "c-1".into(),
            from_phase: "build".into(),
            to_phase: "test".into(),
            transition_at: "2026-08-17T10:00:00Z".into(),
            actor_id: "user:test".into(),
            actor_kind: ActorKind::Human,
            event_id_prefix: "e-c-1".into(),
        };
        let env = build_event_envelope(
            "e-c-1-entered-c-1",
            "workflow.phase.entered",
            "test",
            &input,
        );

        assert_eq!(env.event_type, "workflow.phase.entered");
        assert_eq!(env.stream_id, "c-1");
        assert_eq!(env.payload.get("phase").unwrap().as_str().unwrap(), "test");
        assert_eq!(env.actor.id, "user:test");
        assert!(!env.content_hash.is_empty());
        assert!(env.content_hash.starts_with("sha256:"));
        assert_eq!(env.subjects.len(), 1);
        assert_eq!(env.subjects[0].kind, "cycle");
        assert_eq!(env.subjects[0].id, "c-1");
    }
}
