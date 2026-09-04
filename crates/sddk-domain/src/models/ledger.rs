//! Ledger event records and verification.
use serde::{Deserialize, Serialize};
use serde_json::Value;

use crate::event_envelope::{ActorKind, ActorRef};

/// Data required to append one ledger event.
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub struct LedgerEventInput {
    pub event_id: String,
    pub project_id: String,
    pub cycle_id: Option<String>,
    pub frame_id: String,
    pub command_id: String,
    /// Deprecated: use `actor_ref` instead. Preserved for legacy corpus replay.
    #[serde(default)]
    pub actor: String,
    /// Canonical actor provenance (per ADR-069 §5 and ADR-071 §5).
    /// Additive: not present in pre-EVT-LEDGER-001 events.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub actor_ref: Option<ActorRef>,
    pub event_type: String,
    pub occurred_at: String,
    pub state_before: Option<Value>,
    pub state_after: Option<Value>,
    pub payload: Value,
    /// Causation chain: event_id of the immediate predecessor in the same stream.
    /// Additive: not present in pre-EVT-LEDGER-001 events.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub causation_id: Option<String>,
    /// Correlation group: frame_id of the command that triggered this event.
    /// Additive: not present in pre-EVT-LEDGER-001 events.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub correlation_id: Option<String>,
}

/// An immutable hash-linked ledger event.
#[derive(Debug, Clone, PartialEq, Default, Serialize)]
pub struct LedgerEvent {
    pub sequence: i64,
    pub event_id: String,
    pub project_id: String,
    pub cycle_id: Option<String>,
    pub frame_id: String,
    pub command_id: String,
    /// Deprecated: use `actor_ref` instead. Preserved for legacy corpus replay.
    pub actor: String,
    /// Canonical actor provenance (per ADR-069 §5 and ADR-071 §5).
    /// Additive: not present in pre-EVT-LEDGER-001 events.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub actor_ref: Option<ActorRef>,
    pub event_type: String,
    pub occurred_at: String,
    pub state_before: Option<Value>,
    pub state_after: Option<Value>,
    pub payload: Value,
    pub previous_hash: Option<String>,
    pub event_hash: String,
    /// Causation chain: event_id of the immediate predecessor in the same stream.
    /// Additive: not present in pre-EVT-LEDGER-001 events.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub causation_id: Option<String>,
    /// Correlation group: frame_id of the command that triggered this event.
    /// Additive: not present in pre-EVT-LEDGER-001 events.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub correlation_id: Option<String>,
}

impl LedgerEvent {
    pub fn as_input(&self) -> LedgerEventInput {
        LedgerEventInput {
            event_id: self.event_id.clone(),
            project_id: self.project_id.clone(),
            cycle_id: self.cycle_id.clone(),
            frame_id: self.frame_id.clone(),
            command_id: self.command_id.clone(),
            actor: self.actor.clone(),
            actor_ref: self.actor_ref.clone(),
            event_type: self.event_type.clone(),
            occurred_at: self.occurred_at.clone(),
            state_before: self.state_before.clone(),
            state_after: self.state_after.clone(),
            payload: self.payload.clone(),
            causation_id: self.causation_id.clone(),
            correlation_id: self.correlation_id.clone(),
        }
    }
}

/// Metadata for an artifact stored outside SQLite.
#[derive(Debug, Clone, PartialEq)]
pub struct ArtifactRecord {
    pub artifact_id: String,
    pub project_id: String,
    pub cycle_id: Option<String>,
    pub kind: String,
    pub path: String,
    pub sha256: Option<String>,
    pub producer: Option<String>,
    pub created_at: String,
    pub metadata: Value,
}

/// Result of verifying the complete ledger chain.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LedgerVerification {
    pub event_count: usize,
    pub last_hash: Option<String>,
}

#[cfg(test)]
mod tests {
    use super::*;

    // REQ-IRDT-RT-06: LedgerEvent JSON payloads round-trip byte-exactly.
    #[test]
    fn ledger_event_input_roundtrip_preserves_payload() {
        let input = LedgerEventInput {
            event_id: "evt-001".to_string(),
            project_id: "proj-001".to_string(),
            cycle_id: Some("cycle-001".to_string()),
            frame_id: "frame-001".to_string(),
            command_id: "cmd-001".to_string(),
            actor: "test-actor".to_string(),
            actor_ref: Some(ActorRef {
                kind: ActorKind::Human,
                id: "user:test-actor".to_string(),
                definition_hash: None,
                policy_hash: None,
                model: None,
            }),
            event_type: "test.event".to_string(),
            occurred_at: "2026-09-04T12:00:00Z".to_string(),
            state_before: Some(serde_json::json!({"before": "value"})),
            state_after: Some(serde_json::json!({"after": "value"})),
            payload: serde_json::json!({"key": "value", "nested": {"a": 1}}),
            causation_id: Some("evt-previous-001".to_string()),
            correlation_id: Some("frame-001".to_string()),
        };

        // Byte-exact round-trip via serde_json
        let bytes = serde_json::to_vec(&input).expect("must serialize");
        let round_tripped: LedgerEventInput =
            serde_json::from_slice(&bytes).expect("must deserialize");

        assert_eq!(input, round_tripped);

        // Verify byte equality for each Value field individually
        let before_bytes = serde_json::to_vec(&input.state_before).unwrap();
        let after_bytes = serde_json::to_vec(&input.state_after).unwrap();
        let payload_bytes = serde_json::to_vec(&input.payload).unwrap();

        let rt_before_bytes = serde_json::to_vec(&round_tripped.state_before).unwrap();
        let rt_after_bytes = serde_json::to_vec(&round_tripped.state_after).unwrap();
        let rt_payload_bytes = serde_json::to_vec(&round_tripped.payload).unwrap();

        assert_eq!(
            before_bytes, rt_before_bytes,
            "state_before bytes must be byte-equal after round-trip"
        );
        assert_eq!(
            after_bytes, rt_after_bytes,
            "state_after bytes must be byte-equal after round-trip"
        );
        assert_eq!(
            payload_bytes, rt_payload_bytes,
            "payload bytes must be byte-equal after round-trip"
        );
    }
}
