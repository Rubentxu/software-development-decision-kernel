//! Correlation and causation helpers for the event bus.

use sddk_domain::EventEnvelopeV1;

use crate::EventContext;

// ── Correlation / Causation helpers ─────────────────────────────────────────────

/// Fills `correlation_id` from the [`EventContext::frame_id`] if the field
/// is not already set.
///
/// This is an additive, idempotent helper: calling it on an envelope that
/// already has a `correlation_id` is a no-op.
///
/// # Production wiring
///
/// Helpers ship as public tested API only. Production wiring (calling
/// these from the `emit_*` builders) is deferred to M6 SPEC-028,
/// when the dispatcher primitive becomes the first real consumer.
/// See `spec.md` REQ-M14-004 amendment 2026-08-22.
pub fn with_correlation_from_context(env: &mut EventEnvelopeV1, ctx: &EventContext) {
    if env.correlation_id.is_none() {
        env.correlation_id = Some(ctx.frame_id.clone());
    }
}

/// Sets `causation_id` to `cause_event_id` if the field is not already set.
///
/// This is an additive, idempotent helper: calling it on an envelope that
/// already has a `causation_id` is a no-op.
///
/// # Production wiring
///
/// Helpers ship as public tested API only. Production wiring (calling
/// these from the `emit_*` builders) is deferred to M6 SPEC-028,
/// when the dispatcher primitive becomes the first real consumer.
/// See `spec.md` REQ-M14-004 amendment 2026-08-22.
pub fn with_causation(env: &mut EventEnvelopeV1, cause_event_id: &str) {
    if env.causation_id.is_none() {
        env.causation_id = Some(cause_event_id.to_owned());
    }
}

/// Sets `correlation_id` to `correlation` if the field is not already set.
///
/// This is an additive, idempotent helper: calling it on an envelope that
/// already has a `correlation_id` is a no-op.
pub fn with_correlation_id(env: &mut EventEnvelopeV1, correlation: &str) {
    if env.correlation_id.is_none() {
        env.correlation_id = Some(correlation.to_owned());
    }
}

/// Walks the causation chain of an event by following `causation_id` links.
///
/// Returns the ordered list from the given event back to the root (the event
/// whose `causation_id` is `None`). The first element is `event_id`;
/// the last is the root event.
///
/// If `by_event_id` returns `None` for any ID in the chain, the chain
/// is truncated at that point and the partial chain is returned.
///
/// # Production wiring
///
/// Helpers ship as public tested API only. Production wiring (calling
/// these from the `emit_*` builders) is deferred to M6 SPEC-028,
/// when the dispatcher primitive becomes the first real consumer.
/// See `spec.md` REQ-M14-004 amendment 2026-08-22.
pub fn trace_causation_chain(
    event_id: &str,
    by_event_id: impl Fn(&str) -> Option<EventEnvelopeV1>,
) -> Vec<String> {
    let mut chain = Vec::new();
    let mut current = event_id.to_owned();

    // Guard against unbounded chains
    let max_len = 1000;

    while chain.len() < max_len {
        // Look up the current event
        let Some(evt) = by_event_id(&current) else {
            // Gap in chain — stop here
            break;
        };

        // Add current event to chain
        chain.push(current.clone());

        // Get the next causation_id
        let Some(next) = evt.causation_id else {
            // Root reached — no causation_id
            break;
        };

        // Cycle detection
        if chain.contains(&next) {
            break;
        }

        current = next;
    }

    chain
}

#[cfg(test)]
mod tests {
    use super::*;
    use sddk_domain::{ActorKind, ActorRef};
    use serde_json::json;

    fn minimal_env() -> EventEnvelopeV1 {
        EventEnvelopeV1 {
            event_id: "evt-test".into(),
            event_type: "workflow.phase.entered".into(),
            schema_version: 1,
            stream_id: "stream-test".into(),
            sequence: 1,
            project_id: "p-test".into(),
            occurred_at: "2026-08-22T00:00:00Z".into(),
            recorded_at: "2026-08-22T00:00:00Z".into(),
            actor: ActorRef {
                kind: ActorKind::System,
                id: "test".into(),
                definition_hash: None,
                policy_hash: None,
                model: None,
            },
            subjects: vec![],
            payload: json!({ "phase": "build" }),
            evidence_refs: vec![],
            content_hash: String::new(),
            metadata: None,
            causation_id: None,
            correlation_id: None,
            cycle_id: None,
            frame_id: None,
            fork_id: None,
        }
    }

    fn envelope_with_causation(event_id: &str, causation_id: Option<&str>) -> EventEnvelopeV1 {
        let mut env = minimal_env();
        env.event_id = event_id.into();
        env.content_hash = env.compute_content_hash();
        if let Some(cid) = causation_id {
            env.causation_id = Some(cid.into());
            env.content_hash = env.compute_content_hash();
        }
        env
    }

    // ── Correlation / Causation helper tests ──────────────────────────────────

    #[test]
    fn with_correlation_from_context_fills_frame_id() {
        let ctx = EventContext {
            command_id: "cmd-1".into(),
            frame_id: "frame-7".into(),
            event_id: "evt-1".into(),
            actor: "alice".into(),
            actor_ref: None,
            occurred_at: "2026-08-22T00:00:00Z".into(),
            correlation_id: None,
            causation_id: None,
        };
        let mut env = minimal_env();
        assert!(env.correlation_id.is_none());
        with_correlation_from_context(&mut env, &ctx);
        assert_eq!(env.correlation_id, Some("frame-7".into()));
    }

    #[test]
    fn with_correlation_from_context_is_noop_when_preset() {
        let ctx = EventContext {
            command_id: "cmd-1".into(),
            frame_id: "frame-7".into(),
            event_id: "evt-1".into(),
            actor: "alice".into(),
            actor_ref: None,
            occurred_at: "2026-08-22T00:00:00Z".into(),
            correlation_id: None,
            causation_id: None,
        };
        let mut env = minimal_env();
        env.correlation_id = Some("preset-correlation".into());
        with_correlation_from_context(&mut env, &ctx);
        assert_eq!(env.correlation_id, Some("preset-correlation".into()));
    }

    #[test]
    fn trace_causation_chain_walks_to_root() {
        // Simulate: root → e2 → e3 (current)
        fn fake_lookup(id: &str) -> Option<EventEnvelopeV1> {
            match id {
                "evt-3" => Some(envelope_with_causation("evt-3", Some("evt-2"))),
                "evt-2" => Some(envelope_with_causation("evt-2", Some("evt-1"))),
                "evt-1" => Some(envelope_with_causation("evt-1", None)),
                _ => None,
            }
        }
        let chain = trace_causation_chain("evt-3", fake_lookup);
        assert_eq!(chain, vec!["evt-3", "evt-2", "evt-1"]);
    }

    #[test]
    fn trace_causation_chain_truncates_at_missing() {
        // e2 → missing (gap in chain)
        fn fake_lookup(id: &str) -> Option<EventEnvelopeV1> {
            match id {
                "evt-3" => Some(envelope_with_causation("evt-3", Some("evt-2"))),
                "evt-2" => None, // gap
                _ => None,
            }
        }
        let chain = trace_causation_chain("evt-3", fake_lookup);
        assert_eq!(chain, vec!["evt-3"]); // stops at missing
    }

    #[test]
    fn trace_causation_chain_detects_cycle() {
        // e1 → e2 → e1 (cycle)
        fn fake_lookup(id: &str) -> Option<EventEnvelopeV1> {
            match id {
                "evt-1" => Some(envelope_with_causation("evt-1", Some("evt-2"))),
                "evt-2" => Some(envelope_with_causation("evt-2", Some("evt-1"))),
                _ => None,
            }
        }
        let chain = trace_causation_chain("evt-1", fake_lookup);
        // Stops when it sees evt-1 again
        assert_eq!(chain, vec!["evt-1", "evt-2"]);
    }

    #[test]
    fn event_bus_compat_emits_byte_equivalent_envelopes() {
        // Verify that calling helpers on an envelope with PRE-SET fields
        // does not change the bytes. This proves the helpers are truly additive
        // (noop when the field is already populated).
        use crate::emit::PhaseEventInput;
        use sddk_domain::ActorKind;

        let phase_input = PhaseEventInput {
            project_id: "p-test".into(),
            cycle_id: "c-1".into(),
            from_phase: "explore".into(),
            to_phase: "build".into(),
            transition_at: "2026-08-22T00:00:00Z".into(),
            actor_id: "alice".into(),
            actor_kind: ActorKind::Human,
            event_id_prefix: "e".into(),
            causation_id: None,
            correlation_id: None,
        };

        // Emit an envelope with correlation_id already set (preset)
        let mut env_with_preset = super::super::envelopes::build_event_envelope(
            "e-entered-c-1",
            "workflow.phase.entered",
            "build",
            &phase_input,
        );
        env_with_preset.correlation_id = Some("preset-correlation".into());
        env_with_preset.content_hash = env_with_preset.compute_content_hash();
        let bytes_preset = env_with_preset.to_canonical_json();

        // Apply helper on top of preset — should be a no-op
        let mut env_after_helper = super::super::envelopes::build_event_envelope(
            "e-entered-c-1",
            "workflow.phase.entered",
            "build",
            &phase_input,
        );
        env_after_helper.correlation_id = Some("preset-correlation".into());
        env_after_helper.content_hash = env_after_helper.compute_content_hash();
        let ctx = EventContext {
            command_id: "cmd-1".into(),
            frame_id: "frame-1".into(),
            event_id: "e-entered-c-1".into(),
            actor: "alice".into(),
            actor_ref: None,
            occurred_at: "2026-08-22T00:00:00Z".into(),
            correlation_id: None,
            causation_id: None,
        };
        with_correlation_from_context(&mut env_after_helper, &ctx);
        env_after_helper.content_hash = env_after_helper.compute_content_hash();
        let bytes_after_helper = env_after_helper.to_canonical_json();

        // Bytes must be identical when field was preset
        assert_eq!(
            bytes_preset, bytes_after_helper,
            "helper should be no-op when correlation_id is preset"
        );
    }
}
