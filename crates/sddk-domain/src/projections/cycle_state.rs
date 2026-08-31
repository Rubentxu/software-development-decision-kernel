//! Cycle-state projection: tracks the current phase of a cycle workflow.

use crate::projections::{Checkpoint, Projection, ProjectionError, ProjectionVersion};

use serde::{Deserialize, Serialize};

// ── CycleState ────────────────────────────────────────────────────────────────

/// Tracks the current phase of a single cycle's workflow.
///
/// The projection's `apply` method handles `workflow.phase.entered` and
/// `workflow.phase.exited` event types and ignores others.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct CycleState {
    /// Current phase label, or `"unknown"` before any `phase.entered` event.
    pub phase: String,
    /// Monotonic sequence number of the last event applied to this projection.
    pub last_event_sequence: u64,
    /// Hash of the last event applied (for [`Checkpoint.last_event_hash`]).
    pub last_event_hash: String,
    /// RFC 3339 wall-clock time of the most recent `workflow.phase.entered` event.
    pub entered_at: Option<String>,
}

impl Default for CycleState {
    fn default() -> Self {
        Self {
            phase: "unknown".into(),
            last_event_sequence: 0,
            last_event_hash: String::new(),
            entered_at: None,
        }
    }
}

/// Concrete projection for the `cycle_state` read-model.
///
/// Listens for `workflow.phase.entered` and `workflow.phase.exited` events
/// on the cycle's stream and updates [`CycleState::phase`] accordingly.
pub struct CycleStateProjection {
    /// Stream this projection is subscribed to.
    cycle_id: String,
    /// Mutable projection state.
    state: CycleState,
}

impl CycleStateProjection {
    /// Canonical name for the `cycle_state` projection.
    pub const NAME: &'static str = "cycle_state";
    /// Version for the v1 `apply` semantics.
    pub const VERSION: ProjectionVersion = 1;

    /// Creates a new `CycleStateProjection` for the given cycle stream.
    pub fn new(stream_id: impl Into<String>) -> Self {
        Self {
            cycle_id: stream_id.into(),
            state: CycleState::default(),
        }
    }

    /// Returns the cycle stream ID this projection consumes from.
    pub fn cycle_id(&self) -> &str {
        &self.cycle_id
    }
}

impl Projection for CycleStateProjection {
    type State = CycleState;

    fn name(&self) -> &str {
        Self::NAME
    }

    fn version(&self) -> ProjectionVersion {
        Self::VERSION
    }

    fn apply(&mut self, event: &crate::EventEnvelopeV1) -> Result<(), ProjectionError> {
        // Only process events from our stream.
        if event.stream_id != self.cycle_id {
            return Ok(());
        }

        // Update monotone fields regardless of event type.
        self.state.last_event_sequence = event.sequence;
        self.state.last_event_hash = event.content_hash.clone();

        match event.event_type.as_str() {
            "workflow.phase.entered" => {
                let phase = event
                    .payload
                    .get("phase")
                    .and_then(|v| v.as_str())
                    .ok_or_else(|| ProjectionError::InvalidPayload {
                        event_type: event.event_type.clone(),
                        detail: format!(
                            "event {} missing 'phase' string in payload",
                            event.event_id
                        ),
                    })?
                    .to_string();
                self.state.phase = phase;
                self.state.entered_at = Some(event.occurred_at.clone());
                Ok(())
            }
            "workflow.phase.exited" => {
                self.state.phase = "exited".into();
                Ok(())
            }
            _ => Ok(()), // Ignore other event types per spec.
        }
    }

    fn checkpoint(&self) -> Checkpoint {
        Checkpoint {
            projection_name: Self::NAME.to_string(),
            version: self.version(),
            last_event_sequence: self.state.last_event_sequence,
            last_event_hash: self.state.last_event_hash.clone(),
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
    use super::{CycleStateProjection, Projection};
    use crate::event_envelope::{ActorKind, ActorRef, EventEnvelopeV1};
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
    fn apply_workflow_phase_entered_sets_phase() {
        let mut proj = CycleStateProjection::new("cycle-1");
        proj.apply(&make_event(
            "cycle-1",
            "workflow.phase.entered",
            1,
            json!({ "phase": "build" }),
        ))
        .unwrap();
        assert_eq!(proj.state_ref().phase, "build");
        assert_eq!(proj.state_ref().last_event_sequence, 1);
        assert!(!proj.state_ref().last_event_hash.is_empty());
        assert!(proj.state_ref().last_event_hash.starts_with("sha256:"));
    }

    #[test]
    fn apply_workflow_phase_exited_marks_exited() {
        let mut proj = CycleStateProjection::new("cycle-1");
        proj.apply(&make_event(
            "cycle-1",
            "workflow.phase.entered",
            1,
            json!({ "phase": "build" }),
        ))
        .unwrap();
        proj.apply(&make_event(
            "cycle-1",
            "workflow.phase.exited",
            2,
            json!({}),
        ))
        .unwrap();
        assert_eq!(proj.state_ref().phase, "exited");
    }

    #[test]
    fn apply_other_event_types_ignored() {
        let mut proj = CycleStateProjection::new("cycle-1");
        proj.apply(&make_event("cycle-1", "uat.scenario.started", 1, json!({})))
            .unwrap();
        assert_eq!(proj.state_ref().phase, "unknown");
    }

    #[test]
    fn apply_skips_other_streams() {
        let mut proj = CycleStateProjection::new("cycle-1");
        proj.apply(&make_event(
            "cycle-2",
            "workflow.phase.entered",
            1,
            json!({ "phase": "build" }),
        ))
        .unwrap();
        assert_eq!(proj.state_ref().phase, "unknown");
    }

    #[test]
    fn checkpoint_includes_last_event_hash() {
        let mut proj = CycleStateProjection::new("cycle-1");
        proj.apply(&make_event(
            "cycle-1",
            "workflow.phase.entered",
            3,
            json!({ "phase": "test" }),
        ))
        .unwrap();
        let cp = proj.checkpoint();
        assert_eq!(cp.last_event_sequence, 3);
        assert!(cp.last_event_hash.starts_with("sha256:"));
    }
}
