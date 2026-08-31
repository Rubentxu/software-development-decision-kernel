//! Approval projection: tracks approval decisions per cycle and capability.

use crate::models::ApprovalDecision;

use crate::projections::{Checkpoint, Projection, ProjectionError, ProjectionVersion};

use serde::{Deserialize, Serialize};

// ── ApprovalState ─────────────────────────────────────────────────────────────

/// Approval state for one `(cycle_id, capability)` pair.
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
pub struct ApprovalState {
    /// SHA-256 hash of the structured request.
    pub request_hash: String,
    /// RFC 3339 timestamp of the latest event.
    pub last_event_at: String,
    /// Event identifier of the latest event.
    pub last_event_id: String,
    /// Decision outcome, if resolved.
    pub decision: Option<ApprovalDecision>,
    /// Human operator who made the decision, if resolved.
    pub actor: Option<String>,
    /// Justification, if resolved.
    pub reason: Option<String>,
}

/// Concrete projection for the `approval` read-model.
///
/// Listens for `approval.capability.requested`, `approval.capability.granted`,
/// and `approval.capability.denied` events and tracks the latest decision
/// per `(cycle_id, capability)` pair.
pub struct ApprovalProjection {
    /// Stream this projection is subscribed to (used as the cycle context).
    cycle_stream: String,
    /// Monotonic sequence of the last event applied (global, not per-capability).
    last_event_sequence: u64,
    /// Content hash of the last event applied.
    last_event_hash: String,
    /// Mutable projection state keyed by `(cycle_id, capability)`.
    state: std::collections::HashMap<(String, String), ApprovalState>,
}

impl ApprovalProjection {
    /// Canonical name for the `approval` projection.
    pub const NAME: &'static str = "approval";
    /// Version for the v1 `apply` semantics.
    pub const VERSION: ProjectionVersion = 1;

    /// Creates a new `ApprovalProjection` for the given cycle stream.
    pub fn new(cycle_stream: impl Into<String>) -> Self {
        Self {
            cycle_stream: cycle_stream.into(),
            last_event_sequence: 0,
            last_event_hash: String::new(),
            state: std::collections::HashMap::new(),
        }
    }

    /// Returns the current approval states as a map keyed by `(cycle_id, capability)`.
    pub fn states(&self) -> &std::collections::HashMap<(String, String), ApprovalState> {
        &self.state
    }
}

impl Projection for ApprovalProjection {
    type State = std::collections::HashMap<(String, String), ApprovalState>;

    fn name(&self) -> &str {
        Self::NAME
    }

    fn version(&self) -> ProjectionVersion {
        Self::VERSION
    }

    fn apply(&mut self, event: &crate::EventEnvelopeV1) -> Result<(), ProjectionError> {
        // Only process events from our stream.
        if event.stream_id != self.cycle_stream {
            return Ok(());
        }

        // Update monotone fields on every call regardless of event type.
        self.last_event_sequence = event.sequence;
        self.last_event_hash = event.content_hash.clone();

        // Only process approval event types; ignore all others.
        match event.event_type.as_str() {
            "approval.capability.requested"
            | "approval.capability.granted"
            | "approval.capability.denied" => {}
            _ => return Ok(()),
        }

        let cycle_id = event
            .payload
            .get("cycle_id")
            .and_then(|v| v.as_str())
            .ok_or_else(|| ProjectionError::InvalidPayload {
                event_type: event.event_type.clone(),
                detail: format!(
                    "event {} missing 'cycle_id' string in payload",
                    event.event_id
                ),
            })?
            .to_string();

        let capability = event
            .payload
            .get("capability")
            .and_then(|v| v.as_str())
            .ok_or_else(|| ProjectionError::InvalidPayload {
                event_type: event.event_type.clone(),
                detail: format!(
                    "event {} missing 'capability' string in payload",
                    event.event_id
                ),
            })?
            .to_string();

        let request_hash = event
            .payload
            .get("request_hash")
            .and_then(|v| v.as_str())
            .ok_or_else(|| ProjectionError::InvalidPayload {
                event_type: event.event_type.clone(),
                detail: format!(
                    "event {} missing 'request_hash' string in payload",
                    event.event_id
                ),
            })?
            .to_string();

        let key = (cycle_id.clone(), capability.clone());
        let state = self.state.entry(key.clone()).or_default();

        match event.event_type.as_str() {
            "approval.capability.requested" => {
                state.request_hash = request_hash;
                state.last_event_at = event.occurred_at.clone();
                state.last_event_id = event.event_id.clone();
                state.decision = None;
                state.actor = None;
                state.reason = None;
                Ok(())
            }
            "approval.capability.granted" => {
                state.request_hash = request_hash;
                state.last_event_at = event.occurred_at.clone();
                state.last_event_id = event.event_id.clone();
                state.decision = Some(ApprovalDecision::Granted);
                state.actor = event
                    .payload
                    .get("actor")
                    .and_then(|v| v.as_str())
                    .map(String::from);
                state.reason = event
                    .payload
                    .get("reason")
                    .and_then(|v| v.as_str())
                    .map(String::from);
                Ok(())
            }
            "approval.capability.denied" => {
                state.request_hash = request_hash;
                state.last_event_at = event.occurred_at.clone();
                state.last_event_id = event.event_id.clone();
                state.decision = Some(ApprovalDecision::Denied);
                state.actor = event
                    .payload
                    .get("actor")
                    .and_then(|v| v.as_str())
                    .map(String::from);
                state.reason = event
                    .payload
                    .get("reason")
                    .and_then(|v| v.as_str())
                    .map(String::from);
                Ok(())
            }
            _ => unreachable!(),
        }
    }

    fn checkpoint(&self) -> Checkpoint {
        Checkpoint {
            projection_name: Self::NAME.to_string(),
            version: self.version(),
            last_event_sequence: self.last_event_sequence,
            last_event_hash: self.last_event_hash.clone(),
            updated_at: time::OffsetDateTime::now_utc()
                .format(&time::format_description::well_known::Rfc3339)
                .expect("RFC 3339 formatting cannot fail"),
        }
    }

    fn state_ref(&self) -> &Self::State {
        &self.state
    }
}

#[cfg(test)]
mod tests {
    use super::{ApprovalProjection, Projection};
    use crate::event_envelope::{ActorKind, ActorRef, EventEnvelopeV1};
    use crate::models::ApprovalDecision;
    use serde_json::json;

    fn make_event(
        stream_id: &str,
        event_type: &str,
        sequence: u64,
        payload: serde_json::Value,
    ) -> EventEnvelopeV1 {
        let mut env = EventEnvelopeV1 {
            event_id: format!("e-{stream_id}-{sequence}"),
            event_type: event_type.into(),
            schema_version: 1,
            stream_id: stream_id.into(),
            sequence,
            project_id: "p-1".into(),
            occurred_at: "2026-08-17T10:00:00Z".into(),
            recorded_at: "2026-08-17T10:00:01Z".into(),
            actor: ActorRef {
                kind: ActorKind::System,
                id: "sddk-cli".into(),
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

    #[test]
    fn approval_projection_requested_then_granted_has_decision() {
        let mut proj = ApprovalProjection::new("c-1");
        proj.apply(&make_event(
            "c-1",
            "approval.capability.requested",
            1,
            json!({
                "cycle_id": "c-1",
                "capability": "git.delete_branch",
                "request_hash": "sha256:abc1234",
                "expires_at": "2026-08-18T18:00:00Z"
            }),
        ))
        .unwrap();

        // State is pending after requested.
        let key = ("c-1".into(), "git.delete_branch".into());
        assert!(proj.state_ref().contains_key(&key));
        let state = proj.state_ref().get(&key).unwrap();
        assert!(state.decision.is_none());
        assert_eq!(state.request_hash, "sha256:abc1234");

        // Apply granted decision.
        proj.apply(&make_event(
            "c-1",
            "approval.capability.granted",
            2,
            json!({
                "cycle_id": "c-1",
                "capability": "git.delete_branch",
                "request_hash": "sha256:abc1234",
                "actor": "alice",
                "reason": "ok, reversible via reflog"
            }),
        ))
        .unwrap();

        let state = proj.state_ref().get(&key).unwrap();
        assert_eq!(state.decision, Some(ApprovalDecision::Granted));
        assert_eq!(state.actor, Some("alice".into()));
        assert_eq!(state.reason, Some("ok, reversible via reflog".into()));
        assert_eq!(proj.checkpoint().last_event_sequence, 2);
    }

    #[test]
    fn approval_projection_denied_has_decision() {
        let mut proj = ApprovalProjection::new("c-1");
        proj.apply(&make_event(
            "c-1",
            "approval.capability.requested",
            1,
            json!({
                "cycle_id": "c-1",
                "capability": "git.delete_branch",
                "request_hash": "sha256:def5678"
            }),
        ))
        .unwrap();
        proj.apply(&make_event(
            "c-1",
            "approval.capability.denied",
            2,
            json!({
                "cycle_id": "c-1",
                "capability": "git.delete_branch",
                "request_hash": "sha256:def5678",
                "actor": "bob",
                "reason": "too risky"
            }),
        ))
        .unwrap();

        let key = ("c-1".into(), "git.delete_branch".into());
        let state = proj.state_ref().get(&key).unwrap();
        assert_eq!(state.decision, Some(ApprovalDecision::Denied));
        assert_eq!(state.actor, Some("bob".into()));
    }

    #[test]
    fn approval_projection_skips_other_streams() {
        let mut proj = ApprovalProjection::new("c-1");
        proj.apply(&make_event(
            "c-2",
            "approval.capability.requested",
            1,
            json!({
                "cycle_id": "c-2",
                "capability": "git.delete_branch",
                "request_hash": "sha256:abc1234"
            }),
        ))
        .unwrap();
        assert!(proj.state_ref().is_empty());
        assert_eq!(proj.checkpoint().last_event_sequence, 0);
    }

    #[test]
    fn approval_projection_ignores_other_event_types() {
        let mut proj = ApprovalProjection::new("c-1");
        proj.apply(&make_event(
            "c-1",
            "uat.scenario.started",
            1,
            json!({ "cycle_id": "c-1", "capability": "git.delete_branch", "request_hash": "sha256:abc" }),
        ))
        .unwrap();
        assert!(proj.state_ref().is_empty());
    }

    #[test]
    fn approval_projection_multiple_capabilities() {
        let mut proj = ApprovalProjection::new("c-1");
        proj.apply(&make_event(
            "c-1",
            "approval.capability.requested",
            1,
            json!({
                "cycle_id": "c-1",
                "capability": "git.delete_branch",
                "request_hash": "sha256:aaa"
            }),
        ))
        .unwrap();
        proj.apply(&make_event(
            "c-1",
            "approval.capability.requested",
            2,
            json!({
                "cycle_id": "c-1",
                "capability": "git.merge",
                "request_hash": "sha256:bbb"
            }),
        ))
        .unwrap();

        assert_eq!(proj.state_ref().len(), 2);
        assert!(
            proj.state_ref()
                .contains_key(&("c-1".into(), "git.delete_branch".into()))
        );
        assert!(
            proj.state_ref()
                .contains_key(&("c-1".into(), "git.merge".into()))
        );
    }

    #[test]
    fn approval_projection_checkpoint_sequence_tracks_global() {
        let mut proj = ApprovalProjection::new("c-1");
        proj.apply(&make_event(
            "c-1",
            "approval.capability.requested",
            5,
            json!({
                "cycle_id": "c-1",
                "capability": "git.delete_branch",
                "request_hash": "sha256:abc1234"
            }),
        ))
        .unwrap();
        assert_eq!(proj.checkpoint().last_event_sequence, 5);
        assert!(proj.checkpoint().last_event_hash.starts_with("sha256:"));
    }
}
