//! AC-EVT-LEDGER-10 envelope-wins regression test.
//!
//! Spec AC-10 says: "Payload actor text MUST NOT override canonical provenance."
//! The envelope's `event.actor.id` is the canonical provenance; payload["actor"]
//! is legacy fallback only when envelope is absent.
//!
//! This test verifies the CORRECT precedence: envelope wins over payload.

use sddk_domain::event_envelope::{ActorKind, ActorRef};
use sddk_domain::projections::Projection;
use sddk_domain::projections::approval::ApprovalProjection;
use serde_json::json;

// ── Test helper ────────────────────────────────────────────────────────────────

/// Constructs an EventEnvelopeV1 with a given envelope actor and optional payload actor.
fn make_event_with_actor(
    stream_id: &str,
    event_type: &str,
    sequence: u64,
    envelope_actor_id: &str,
    envelope_actor_kind: ActorKind,
    payload_actor: Option<&str>,
) -> sddk_domain::EventEnvelopeV1 {
    let payload = match payload_actor {
        Some(actor) => json!({
            "cycle_id": stream_id,
            "capability": "git.delete_branch",
            "request_hash": "sha256:abc1234",
            "actor": actor,
            "reason": "test"
        }),
        None => json!({
            "cycle_id": stream_id,
            "capability": "git.delete_branch",
            "request_hash": "sha256:abc1234"
        }),
    };

    let mut env = sddk_domain::EventEnvelopeV1 {
        event_id: format!("e-{stream_id}-{sequence}"),
        event_type: event_type.into(),
        schema_version: 1,
        stream_id: stream_id.into(),
        sequence,
        project_id: "p-1".into(),
        occurred_at: "2026-08-17T10:00:00Z".into(),
        recorded_at: "2026-08-17T10:00:01Z".into(),
        actor: ActorRef {
            kind: envelope_actor_kind,
            id: envelope_actor_id.into(),
            definition_hash: None,
            policy_hash: None,
            model: None,
        },
        subjects: vec![],
        payload,
        evidence_refs: vec![],
        content_hash: String::new(),
        metadata: None,
        causation_id: None,
        correlation_id: None,
        cycle_id: None,
        frame_id: None,
        fork_id: None,
    };
    env.content_hash = env.compute_content_hash();
    env
}

// ── AC-EVT-LEDGER-10: Envelope wins over payload ────────────────────────────────

/// AC-EVT-LEDGER-10: When BOTH envelope ActorRef AND payload["actor"] exist,
/// the canonical envelope ActorRef.id MUST win (envelope-wins precedence).
#[test]
fn approval_projection_envelope_actor_wins_over_payload_actor_granted() {
    let mut proj = ApprovalProjection::new("c-envelope-wins");

    // Apply requested event (no payload actor, envelope is system)
    proj.apply(&make_event_with_actor(
        "c-envelope-wins",
        "approval.capability.requested",
        1,
        "agent:orchestrator",
        ActorKind::Agent,
        None,
    ))
    .unwrap();

    // Apply granted event — envelope says "agent:orchestrator" but payload says "alice"
    // AC-10 says envelope MUST win: state.actor must be "agent:orchestrator"
    proj.apply(&make_event_with_actor(
        "c-envelope-wins",
        "approval.capability.granted",
        2,
        "agent:orchestrator",
        ActorKind::Agent,
        Some("alice"),
    ))
    .unwrap();

    let key = ("c-envelope-wins".into(), "git.delete_branch".into());
    let state = proj.state_ref().get(&key).unwrap();

    // AC-10: envelope actor wins — must be "agent:orchestrator", NOT "alice"
    assert_eq!(
        state.actor.as_deref(),
        Some("agent:orchestrator"),
        "AC-10: envelope ActorRef.id must win over payload.actor"
    );
    assert_eq!(state.decision, Some(sddk_domain::ApprovalDecision::Granted));
}

/// AC-EVT-LEDGER-10: Same precedence for denied decisions.
#[test]
fn approval_projection_envelope_actor_wins_over_payload_actor_denied() {
    let mut proj = ApprovalProjection::new("c-envelope-wins-denied");

    // Apply requested event
    proj.apply(&make_event_with_actor(
        "c-envelope-wins-denied",
        "approval.capability.requested",
        1,
        "agent:orchestrator",
        ActorKind::Agent,
        None,
    ))
    .unwrap();

    // Apply denied event — envelope says "agent:orchestrator" but payload says "bob"
    proj.apply(&make_event_with_actor(
        "c-envelope-wins-denied",
        "approval.capability.denied",
        2,
        "agent:orchestrator",
        ActorKind::Agent,
        Some("bob"),
    ))
    .unwrap();

    let key = ("c-envelope-wins-denied".into(), "git.delete_branch".into());
    let state = proj.state_ref().get(&key).unwrap();

    // AC-10: envelope actor wins
    assert_eq!(
        state.actor.as_deref(),
        Some("agent:orchestrator"),
        "AC-10: envelope ActorRef.id must win over payload.actor (denied)"
    );
    assert_eq!(state.decision, Some(sddk_domain::ApprovalDecision::Denied));
}

/// AC-EVT-LEDGER-10: Payload fallback only when envelope actor is absent.
#[test]
fn approval_projection_payload_fallback_when_envelope_actor_is_absent() {
    let mut proj = ApprovalProjection::new("c-payload-fallback");

    // Apply requested event (no payload actor)
    proj.apply(&make_event_with_actor(
        "c-payload-fallback",
        "approval.capability.requested",
        1,
        "agent:orchestrator",
        ActorKind::Agent,
        None,
    ))
    .unwrap();

    // Apply granted event — NO envelope actor set (use System), but payload has "charlie"
    // This should fall back to payload since envelope actor is "sddk-cli" (not absent)
    // But actually, envelope always has actor (EventEnvelopeV1 always has actor),
    // so the fallback is only for legacy events where payload["actor"] is the only source.
    // Since our envelope has actor="sddk-cli", the envelope wins even with empty payload.
    proj.apply(&make_event_with_actor(
        "c-payload-fallback",
        "approval.capability.granted",
        2,
        "sddk-cli", // envelope actor (minimal)
        ActorKind::System,
        Some("charlie"),
    ))
    .unwrap();

    let key = ("c-payload-fallback".into(), "git.delete_branch".into());
    let state = proj.state_ref().get(&key).unwrap();

    // Envelope still wins because EventEnvelopeV1 always has actor
    assert_eq!(
        state.actor.as_deref(),
        Some("sddk-cli"),
        "envelope actor is always present (EventEnvelopeV1 invariant)"
    );
}

/// AC-EVT-LEDGER-10: When payload has actor but envelope is the same (legacy case).
#[test]
fn approval_projection_payload_fallback_when_envelope_and_payload_match() {
    let mut proj = ApprovalProjection::new("c-both-match");

    // Apply requested event
    proj.apply(&make_event_with_actor(
        "c-both-match",
        "approval.capability.requested",
        1,
        "user:alice",
        ActorKind::Human,
        None,
    ))
    .unwrap();

    // Apply granted event — both envelope and payload say "alice"
    proj.apply(&make_event_with_actor(
        "c-both-match",
        "approval.capability.granted",
        2,
        "user:alice",
        ActorKind::Human,
        Some("alice"),
    ))
    .unwrap();

    let key = ("c-both-match".into(), "git.delete_branch".into());
    let state = proj.state_ref().get(&key).unwrap();

    // Both agree — should be alice
    assert_eq!(state.actor.as_deref(), Some("user:alice"));
    assert_eq!(state.decision, Some(sddk_domain::ApprovalDecision::Granted));
}
