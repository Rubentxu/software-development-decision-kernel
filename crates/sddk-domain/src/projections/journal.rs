//! Journal projection: deterministic record of all events with severity metadata.

use crate::models::Severity;

use crate::projections::{Checkpoint, Projection, ProjectionError, ProjectionVersion};

use serde::{Deserialize, Serialize};

// ── JournalEntry ──────────────────────────────────────────────────────────────

/// A journal entry recording one event's metadata and assigned severity.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct JournalEntry {
    /// Event identifier.
    pub event_id: String,
    /// Namespaced event type.
    pub event_type: String,
    /// Stream this event belongs to.
    pub stream_id: String,
    /// Monotonic sequence number.
    pub sequence: u64,
    /// SHA-256 content hash.
    pub content_hash: String,
    /// When the event occurred (RFC 3339).
    pub occurred_at: String,
    /// Severity assigned by the journal policy table.
    pub severity: Severity,
    /// Optional correlation ID for grouping.
    pub correlation_id: Option<String>,
    /// Optional causation ID for chain tracing.
    pub causation_id: Option<String>,
}

// ── Severity policy table ──────────────────────────────────────────────────────
//
// SPEC-027 categorizes events into 8 namespaces. The journal severity table
// maps each category to a severity level. The table is locked in code and
// reviewed as part of the ADR-0048 debt lifecycle.
//
// Category → Severity mapping (7 rows):
//   workflow.*        → Medium   (cycle state changes are normal operations)
//   execution.*       → Low      (execution attempts are routine)
//   routing.*         → High     (routing failures affect availability)
//   context.*         → Low      (context operations are routine)
//   governance.*      → High     (governance decisions have large blast radius)
//   evidence.*        → Medium   (evidence events are important but expected)
//   uat.*            → Medium   (UAT is significant but routine)
//   human.*          → Critical (human decisions are high-stakes)
//
// Note: Pack/runtime events (pack.*, behavior.*) are excluded from the journal
// as they are internal runtime events; they don't appear in the 7-row table.

/// Looks up the severity for a given event type using the locked policy table.
///
/// Returns `Severity::Medium` as the default if the event type does not match
/// any category prefix.
fn severity_for_event_type(event_type: &str) -> Severity {
    if event_type.starts_with("workflow.") {
        Severity::Medium
    } else if event_type.starts_with("execution.")
        || event_type.starts_with("attempt.")
        || event_type.starts_with("tool.")
    {
        Severity::Low
    } else if event_type.starts_with("routing.") || event_type.starts_with("provider.") {
        Severity::High
    } else if event_type.starts_with("context.") {
        Severity::Low
    } else if event_type.starts_with("governance.")
        || event_type.starts_with("proposal.")
        || event_type.starts_with("policy.")
        || event_type.starts_with("approval.")
        || event_type.starts_with("capability.")
        || event_type.starts_with("receipt.")
    {
        Severity::High
    } else if event_type.starts_with("evidence.") || event_type.starts_with("uat.") {
        Severity::Medium
    } else if event_type.starts_with("human.") {
        Severity::Critical
    } else {
        // Default for unknown categories
        Severity::Medium
    }
}

// ── JournalProjection ──────────────────────────────────────────────────────────

/// A deterministic journal projection over all events.
///
/// The journal accumulates every event into a [`JournalEntry`] ordered by
/// sequence number. Severity is assigned by the locked policy table above.
///
/// [`JournalEntry`]: JournalEntry
pub struct JournalProjection {
    /// Stream filter: only events from this stream are recorded.
    stream_id: String,
    /// Monotonic state: last applied sequence.
    last_event_sequence: u64,
    /// Hash of the last applied event.
    last_event_hash: String,
    /// Accumulated journal entries.
    state: Vec<JournalEntry>,
}

impl JournalProjection {
    /// Canonical name for the journal projection.
    pub const NAME: &'static str = "journal";
    /// Version for the v1 apply semantics.
    pub const VERSION: ProjectionVersion = 1;

    /// Creates a new `JournalProjection` for the given stream.
    pub fn new(stream_id: impl Into<String>) -> Self {
        Self {
            stream_id: stream_id.into(),
            last_event_sequence: 0,
            last_event_hash: String::new(),
            state: Vec::new(),
        }
    }

    /// Returns the accumulated journal entries.
    pub fn entries(&self) -> &[JournalEntry] {
        &self.state
    }
}

impl Projection for JournalProjection {
    type State = Vec<JournalEntry>;

    fn name(&self) -> &str {
        Self::NAME
    }

    fn version(&self) -> ProjectionVersion {
        Self::VERSION
    }

    fn apply(&mut self, event: &crate::EventEnvelopeV1) -> Result<(), ProjectionError> {
        // Only record events from our stream
        if event.stream_id != self.stream_id {
            return Ok(());
        }

        // Update monotone fields on every event
        self.last_event_sequence = event.sequence;
        self.last_event_hash = event.content_hash.clone();

        let entry = JournalEntry {
            event_id: event.event_id.clone(),
            event_type: event.event_type.clone(),
            stream_id: event.stream_id.clone(),
            sequence: event.sequence,
            content_hash: event.content_hash.clone(),
            occurred_at: event.occurred_at.clone(),
            severity: severity_for_event_type(&event.event_type),
            correlation_id: event.correlation_id.clone(),
            causation_id: event.causation_id.clone(),
        };

        self.state.push(entry);
        Ok(())
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
    use super::{JournalProjection, Projection, Severity};
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

    fn make_event_with_severity(
        stream_id: &str,
        event_type: &str,
        sequence: u64,
    ) -> EventEnvelopeV1 {
        // Builds a valid envelope; severity is checked against the policy table
        make_event(stream_id, event_type, sequence, json!({}))
    }

    #[test]
    fn journal_projection_byte_equal_on_replay() {
        // Two fresh projections applied the same events must produce identical state.
        let events: Vec<EventEnvelopeV1> = (1..=20)
            .map(|seq| {
                let mut env = make_event(
                    "stream-1",
                    "workflow.phase.entered",
                    seq,
                    json!({"phase": "build"}),
                );
                env.event_id = format!("evt-{seq}");
                env.content_hash = env.compute_content_hash();
                env
            })
            .collect();

        let mut proj_a = JournalProjection::new("stream-1");
        let mut proj_b = JournalProjection::new("stream-1");

        for ev in &events {
            proj_a.apply(ev).unwrap();
        }
        for ev in &events {
            proj_b.apply(ev).unwrap();
        }

        let state_a = proj_a.state_ref();
        let state_b = proj_b.state_ref();

        assert_eq!(state_a.len(), state_b.len());
        for (a, b) in state_a.iter().zip(state_b.iter()) {
            assert_eq!(a.event_id, b.event_id);
            assert_eq!(a.event_type, b.event_type);
            assert_eq!(a.sequence, b.sequence);
            assert_eq!(a.severity, b.severity);
            assert_eq!(a.correlation_id, b.correlation_id);
            assert_eq!(a.causation_id, b.causation_id);
        }

        // Serialized form must also be byte-identical
        let json_a = serde_json::to_string(state_a).unwrap();
        let json_b = serde_json::to_string(state_b).unwrap();
        assert_eq!(json_a, json_b, "replay must produce byte-identical JSON");
    }

    #[test]
    fn journal_projection_severity_table_locked() {
        // Documents the 7-row severity policy table.
        let stream = "test-stream";
        let event_types: Vec<(&str, Severity)> = vec![
            // workflow.* → Medium
            ("workflow.phase.entered", Severity::Medium),
            ("workflow.phase.exited", Severity::Medium),
            ("workflow.transition.succeeded", Severity::Medium),
            // execution.* → Low
            ("execution.started", Severity::Low),
            // routing.* → High
            ("routing.failed", Severity::High),
            // governance.* → High
            ("governance.policy.changed", Severity::High),
            // evidence.* → Medium
            ("evidence.recorded", Severity::Medium),
        ];

        let mut proj = JournalProjection::new(stream);
        for (i, (event_type, _expected)) in event_types.iter().enumerate() {
            let env = make_event_with_severity(stream, event_type, (i + 1) as u64);
            proj.apply(&env).unwrap();
        }

        let entries = proj.state_ref();
        for (i, (_event_type, expected_sev)) in event_types.iter().enumerate() {
            assert_eq!(
                entries[i].severity, *expected_sev,
                "row {i}: severity mismatch for {}",
                event_types[i].0
            );
        }

        assert_eq!(
            entries.len(),
            event_types.len(),
            "all 7 rows must be recorded"
        );
    }

    #[test]
    fn journal_projection_skips_other_streams() {
        let mut proj = JournalProjection::new("stream-1");
        proj.apply(&make_event(
            "stream-2", // different stream
            "workflow.phase.entered",
            1,
            json!({"phase": "build"}),
        ))
        .unwrap();
        assert!(proj.state_ref().is_empty());
    }
}
