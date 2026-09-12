//! Secretary authority closed-set validator (stage 1).
//!
//! Enforces ADR-0073 §"Authority prohibida" + ADR-0073-AMENDMENT-1 at the
//! event-bus boundary: an actor bound to the Secretary role
//! (`ActorRef { kind: Agent, role: Some("secretary") }`, per the EVT-LEDGER-001
//! six-field contract) MUST NOT emit events whose type falls under the
//! orchestrator-/runtime-exclusive prefixes `release.*`, `gate.*`, `lease.*`,
//! `receipt.*`. Any such attempt fails closed with
//! [`SecretaryClosedSetError::ProhibitedEventType`].
//!
//! Escalation path (INC-HX-AUTH-004 path 7): when the actor is a Secretary and
//! the requested event type is outside the closed set, callers use
//! [`is_escalation_required`] and MUST emit `escalation.requested` instead of
//! mutating state. A Secretary emitting any *state-mutating* event type not in
//! the closed set is rejected by [`validate_secretary_event`] with
//! [`SecretaryClosedSetError::EscalationRequired`].
//!
//! Backlog boundary (REQ-SecretaryL1ClosedSetProposals): "Secretary is never a
//! second orchestrator, memory owner, or authority path."

use sddk_domain::{ActorKind, ActorRef};

/// Event-type prefixes the Secretary may never emit (ADR-0073, verbatim).
pub const SECRETARY_PROHIBITED_PREFIXES: [&str; 4] = ["release.", "gate.", "lease.", "receipt."];

/// The role string binding a Secretary agent (ADR-0073-AMENDMENT-1).
pub const SECRETARY_ROLE: &str = "secretary";

/// Returns `true` when this actor is bound to the Secretary role
/// (`kind == Agent && role == "secretary"`).
pub fn is_secretary(actor: &ActorRef) -> bool {
    actor.kind == ActorKind::Agent && actor.role.as_deref() == Some(SECRETARY_ROLE)
}

/// Returns `true` when the event type falls under an orchestrator-/runtime-
/// exclusive prefix (`release.*`, `gate.*`, `lease.*`, `receipt.*`).
pub fn is_prohibited_event_type(event_type: &str) -> bool {
    SECRETARY_PROHIBITED_PREFIXES
        .iter()
        .any(|prefix| event_type.starts_with(prefix))
}

/// Returns `true` when a Secretary actor must escalate instead of emitting
/// this event type directly (i.e. the type is not prohibited but also not in
/// the Secretary's admissible closed set of proposal/acknowledgement kinds).
pub fn is_escalation_required(event_type: &str) -> bool {
    !is_prohibited_event_type(event_type) && !is_closed_set_event_type(event_type)
}

/// Error taxonomy for the Secretary closed-set validator.
#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
#[non_exhaustive]
pub enum SecretaryClosedSetError {
    /// Secretary attempted to emit an orchestrator-/runtime-exclusive event.
    #[error(
        "secretary actor is prohibited from emitting event type '{event_type}' (ADR-0073: release.*/gate.*/lease.*/receipt.* are orchestrator/runtime-exclusive)"
    )]
    ProhibitedEventType { event_type: String },
    /// Secretary attempted a state mutation outside its closed set; must
    /// emit `escalation.requested` instead.
    #[error(
        "secretary actor must escalate (emit 'escalation.requested') for event type '{event_type}' instead of emitting it directly (ADR-0073 closed set)"
    )]
    EscalationRequired { event_type: String },
}

/// Validate that the given actor may emit `event_type`. Fails closed:
/// - Secretary + prohibited prefix → [`SecretaryClosedSetError::ProhibitedEventType`].
/// - Secretary + any non-closed-set type → [`SecretaryClosedSetError::EscalationRequired`].
/// - Non-Secretary actors → `Ok(())` (their authority is governed by
///   `AuthorityContext` / `AuthorityEngineRunner`, not this validator).
pub fn validate_secretary_event(
    actor: &ActorRef,
    event_type: &str,
) -> Result<(), SecretaryClosedSetError> {
    if !is_secretary(actor) {
        return Ok(());
    }
    if is_prohibited_event_type(event_type) {
        return Err(SecretaryClosedSetError::ProhibitedEventType {
            event_type: event_type.to_string(),
        });
    }
    if is_escalation_required(event_type) {
        return Err(SecretaryClosedSetError::EscalationRequired {
            event_type: event_type.to_string(),
        });
    }
    Ok(())
}

/// The Secretary's admissible closed set (stage 1): proposal/acknowledgement
/// event types routed through existing seams, plus the mandatory escalation
/// path. The closed set widens only via explicit ADR (ADR-0073).
fn is_closed_set_event_type(event_type: &str) -> bool {
    matches!(
        event_type,
        "secretary.proposal.suggested"
            | "secretary.acknowledgement.recorded"
            | "secretary.escalation.requested"
            | "escalation.requested"
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    fn secretary() -> ActorRef {
        ActorRef {
            kind: ActorKind::Agent,
            id: "agent:secretary-1".into(),
            definition_hash: None,
            policy_hash: None,
            model: None,
            role: Some("secretary".into()),
        }
    }

    fn plain_agent() -> ActorRef {
        ActorRef {
            kind: ActorKind::Agent,
            id: "agent:worker-1".into(),
            definition_hash: None,
            policy_hash: None,
            model: None,
            role: None,
        }
    }

    #[test]
    fn secretary_role_binding_detected() {
        assert!(is_secretary(&secretary()));
        assert!(!is_secretary(&plain_agent()));
        let human = ActorRef {
            kind: ActorKind::Human,
            id: "user:alice".into(),
            definition_hash: None,
            policy_hash: None,
            model: None,
            role: Some("secretary".into()),
        };
        // Human with role=secretary is NOT the binding (must be Agent kind).
        assert!(!is_secretary(&human));
    }

    #[test]
    fn prohibited_prefixes_reject_secretary() {
        for et in [
            "release.complete",
            "gate.passed",
            "lease.released",
            "receipt.replay",
        ] {
            assert_eq!(
                validate_secretary_event(&secretary(), et),
                Err(SecretaryClosedSetError::ProhibitedEventType {
                    event_type: et.into()
                }
                .map_event_type()),
                "secretary must be blocked on {et}"
            );
        }
    }

    #[test]
    fn non_secretary_actors_pass_validator() {
        assert_eq!(
            validate_secretary_event(&plain_agent(), "release.complete"),
            Ok(())
        );
        assert_eq!(
            validate_secretary_event(&plain_agent(), "lease.released"),
            Ok(())
        );
    }

    #[test]
    fn outside_closed_set_requires_escalation() {
        assert_eq!(
            validate_secretary_event(&secretary(), "cycle.transitioned"),
            Err(SecretaryClosedSetError::EscalationRequired {
                event_type: "cycle.transitioned".into()
            }
            .map_event_type())
        );
        // The escalation event itself is admissible.
        assert_eq!(
            validate_secretary_event(&secretary(), "escalation.requested"),
            Ok(())
        );
        assert_eq!(
            validate_secretary_event(&secretary(), "secretary.proposal.suggested"),
            Ok(())
        );
    }

    impl SecretaryClosedSetError {
        // Test helper to compare errors without constructing payloads twice.
        fn map_event_type(self) -> Self {
            self
        }
    }
}
