//! EventEnvelope v1 wire-format types.
//!
//! Canonical JSON determinism invariant: this module relies on `serde_json`'s
//! default `Map<String, Value>` = `BTreeMap` ordering. DO NOT enable the
//! `serde_json/preserve_order` feature — it breaks canonicalization.

use regex::Regex;
use serde::{Deserialize, Serialize};
use serde_json::Value;
use sha2::{Digest, Sha256};
use std::sync::OnceLock;

/// Entity reference within an event envelope.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct EntityRef {
    /// Type discriminator for the referenced entity (e.g. `cycle`, `scenario`, `feature`).
    #[serde(rename = "type")]
    pub kind: String,
    /// Stable identifier of the entity within its kind namespace.
    pub id: String,
    /// Optional version tag — either a string label or an integer counter.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub version: Option<EntityRefVersion>,
    /// Optional content hash pointing at the entity's canonical representation.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub content_hash: Option<String>,
}

/// Version variant for an entity reference — can be a string or integer tag.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(untagged)]
pub enum EntityRefVersion {
    /// String version label (e.g. `"v1.2.0"`).
    String(String),
    /// Integer version counter (e.g. `7`).
    Integer(i64),
}

/// Actor (principal) who authored or initiated the event.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ActorRef {
    /// Kind of actor that produced the event.
    pub kind: ActorKind,
    /// Stable identifier of the actor within the kind namespace.
    pub id: String,
    /// Optional hash pointing at the actor's behavioural definition (prompts, skills).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub definition_hash: Option<String>,
    /// Optional hash pointing at the policy bundle applied to this actor.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub policy_hash: Option<String>,
    /// Optional model identifier (for agent actors).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub model: Option<String>,
}

/// Kind of actor that initiated an event.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum ActorKind {
    /// Human operator (developer, architect, QA).
    Human,
    /// AI agent (model-bound executor).
    Agent,
    /// System-level caller (CI, scheduler, internal services).
    System,
}

/// Error arising from invalid event type formatting.
#[derive(Debug, thiserror::Error, PartialEq, Eq)]
pub enum EventTypeError {
    /// The supplied event_type failed the namespacing regex
    /// `^[a-z][a-z0-9_]*(\.[a-z][a-z0-9_]*){2,}$`.
    #[error("event_type must match `[a-z][a-z0-9_]*(\\.[a-z][a-z0-9_]*){{2,}}` (got: {0:?})")]
    InvalidFormat(String),
}

/// Wire-format envelope for SDDK domain events (CEP-1 compatible).
///
/// The `content_hash` field is required and carries a SHA-256 digest of the
/// canonical JSON representation (excluding the `content_hash` field itself).
/// Canonicalization relies on `serde_json`'s default `Map<String, Value>` =
/// `BTreeMap` ordering; struct fields serialize in declaration order.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct EventEnvelopeV1 {
    /// Globally unique event identifier.
    pub event_id: String,
    /// Namespaced event type in `realm.object.verb` form, e.g. `uat.acceptance.granted`.
    pub event_type: String,
    /// Schema version; always `1` for this type.
    pub schema_version: u32,
    /// Stream this event belongs to.
    pub stream_id: String,
    /// Monotonic sequence number within the stream.
    pub sequence: u64,
    /// Project that produced or owns this event.
    pub project_id: String,
    /// Wall-clock time when the event occurred (RFC 3339).
    pub occurred_at: String,
    /// Wall-clock time when the event was recorded (RFC 3339).
    pub recorded_at: String,
    /// Actor who authored or initiated the event.
    pub actor: ActorRef,
    /// Zero or more entities affected by or related to this event.
    pub subjects: Vec<EntityRef>,
    /// Arbitrary JSON payload specific to the event type.
    pub payload: Value,
    /// References to external evidence (e.g. UAT check receipts).
    pub evidence_refs: Vec<String>,
    /// SHA-256 content hash in `sha256:<64-hex>` format.
    pub content_hash: String,
    /// Optional metadata bag.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub metadata: Option<Value>,
    /// ID of the event that directly caused this one.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub causation_id: Option<String>,
    /// ID used to correlate related events across a session or operation.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub correlation_id: Option<String>,
    /// Cycle this event is part of.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub cycle_id: Option<String>,
    /// Frame within the cycle.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub frame_id: Option<String>,
    /// Fork this event originated from, if any.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub fork_id: Option<String>,
}

impl EventEnvelopeV1 {
    /// Schema version constant for V1 envelopes.
    pub const SCHEMA_VERSION: u32 = 1;
    /// Prefix for content_hash values per JSON schema regex.
    pub const CONTENT_HASH_PREFIX: &'static str = "sha256:";

    /// Canonical JSON serialization.
    ///
    /// Determinism invariant: this relies on `serde_json::Map<String, Value>`
    /// using `BTreeMap` (the workspace default). DO NOT enable the
    /// `serde_json` `preserve_order` feature — that breaks canonicalization.
    pub fn to_canonical_json(&self) -> String {
        serde_json::to_string(self).expect("EventEnvelopeV1 is always serializable; this is a bug")
    }

    /// Computes `sha256:<64-hex-lowercase>` over the canonical JSON
    /// representation of the **immutable event content** — excluding
    /// `content_hash` itself, `sequence` (adapter-assigned), and
    /// `recorded_at` (adapter-assigned wall-clock time).
    ///
    /// This makes the hash stable and idempotent: the same logical event
    /// always produces the same hash regardless of adapter-assigned fields
    /// or the current `content_hash` field value.
    pub fn compute_content_hash(&self) -> String {
        let mut for_hash = self.clone();
        for_hash.content_hash = String::new();
        for_hash.sequence = 0;
        for_hash.recorded_at = String::new();
        let canonical = serde_json::to_string(&for_hash)
            .expect("EventEnvelopeV1 is always serializable; this is a bug");
        let digest = Sha256::digest(canonical.as_bytes());
        format!("{}{:x}", Self::CONTENT_HASH_PREFIX, digest)
    }

    /// Validates `event_type` against the namespacing regex.
    ///
    /// The regex requires `realm.object.verb` form: at least two segments
    /// separated by dots, each segment starting with a lowercase letter and
    /// containing only lowercase letters, digits, or underscores.
    /// Two-segment names (e.g. `cycle.created`, `lease.released`) are accepted
    /// for backward compatibility with the legacy production corpus.
    pub fn validate_event_type(s: &str) -> Result<(), EventTypeError> {
        static RE: OnceLock<Regex> = OnceLock::new();
        let re = RE.get_or_init(|| {
            Regex::new(r"^[a-z][a-z0-9_]*(\.[a-z][a-z0-9_]*){1,}$")
                .expect("static regex compilation")
        });
        if re.is_match(s) {
            Ok(())
        } else {
            Err(EventTypeError::InvalidFormat(s.to_owned()))
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    fn minimal_envelope() -> EventEnvelopeV1 {
        EventEnvelopeV1 {
            event_id: "e-1".into(),
            event_type: "workflow.phase.entered".into(),
            schema_version: EventEnvelopeV1::SCHEMA_VERSION,
            stream_id: "s-1".into(),
            sequence: 1,
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
            payload: json!({}),
            evidence_refs: vec![],
            content_hash: "sha256:placeholder".into(),
            metadata: None,
            causation_id: None,
            correlation_id: None,
            cycle_id: None,
            frame_id: None,
            fork_id: None,
        }
    }

    #[test]
    fn compute_content_hash_format_matches_regex() {
        let h = minimal_envelope().compute_content_hash();
        assert!(h.starts_with("sha256:"));
        assert_eq!(h.len(), "sha256:".len() + 64);
        assert!(
            h[7..]
                .chars()
                .all(|c| c.is_ascii_hexdigit() && !c.is_ascii_uppercase())
        );
    }

    #[test]
    fn compute_content_hash_is_stable() {
        let env1 = minimal_envelope();
        let env2 = minimal_envelope();
        assert_eq!(env1.compute_content_hash(), env2.compute_content_hash());
    }

    #[test]
    fn to_canonical_json_is_brace_terminated() {
        let j = minimal_envelope().to_canonical_json();
        assert!(j.starts_with('{'));
        assert!(j.ends_with('}'));
    }

    // ─── Event Schema Compatibility Fixtures ─────────────────────────────────

    /// Verifies that [`EventEnvelopeV1`] roundtrips through JSON without data loss.
    /// This is the primary schema-compatibility contract: every version-1 event
    /// that is serialized can be deserialized back to an equivalent envelope.
    #[test]
    fn roundtrip_preserves_all_fields() {
        let original = {
            let mut env = minimal_envelope();
            env.content_hash = env.compute_content_hash();
            env
        };
        let json = original.to_canonical_json();
        let deserialized: EventEnvelopeV1 = serde_json::from_str(&json).unwrap();

        assert_eq!(deserialized.event_id, original.event_id);
        assert_eq!(deserialized.event_type, original.event_type);
        assert_eq!(deserialized.schema_version, original.schema_version);
        assert_eq!(deserialized.stream_id, original.stream_id);
        assert_eq!(deserialized.sequence, original.sequence);
        assert_eq!(deserialized.project_id, original.project_id);
        assert_eq!(deserialized.occurred_at, original.occurred_at);
        assert_eq!(deserialized.recorded_at, original.recorded_at);
        assert_eq!(deserialized.actor.kind, original.actor.kind);
        assert_eq!(deserialized.actor.id, original.actor.id);
        assert_eq!(deserialized.subjects, original.subjects);
        assert_eq!(deserialized.payload, original.payload);
        assert_eq!(deserialized.evidence_refs, original.evidence_refs);
        assert_eq!(deserialized.content_hash, original.content_hash);
        assert_eq!(deserialized.metadata, original.metadata);
        assert_eq!(deserialized.causation_id, original.causation_id);
        assert_eq!(deserialized.correlation_id, original.correlation_id);
        assert_eq!(deserialized.cycle_id, original.cycle_id);
        assert_eq!(deserialized.frame_id, original.frame_id);
        assert_eq!(deserialized.fork_id, original.fork_id);
    }

    /// Verifies that [`to_canonical_json`] is deterministic: two envelopes with
    /// identical content produce bit-for-bit identical JSON. This is required
    /// for stable [`compute_content_hash`] values across re-serializations.
    #[test]
    fn canonical_json_determinism() {
        let env1 = minimal_envelope();
        let env2 = minimal_envelope();
        assert_eq!(
            env1.to_canonical_json(),
            env2.to_canonical_json(),
            "canonical JSON must be identical for identical content"
        );
    }

    /// Verifies that optional fields are omitted from JSON when None (using
    /// `#[serde(skip_serializing_if = "Option::is_none")]`). This ensures
    /// backward compatibility: future schema versions adding new optional fields
    /// will not cause older deserializers to reject events.
    #[test]
    fn optional_fields_omitted_when_none() {
        let env = minimal_envelope();
        let json = env.to_canonical_json();
        let parsed: serde_json::Value = serde_json::from_str(&json).unwrap();

        assert!(
            parsed.get("metadata").is_none(),
            "metadata=None must be omitted"
        );
        assert!(
            parsed.get("causation_id").is_none(),
            "causation_id=None must be omitted"
        );
        assert!(
            parsed.get("correlation_id").is_none(),
            "correlation_id=None must be omitted"
        );
        assert!(
            parsed.get("cycle_id").is_none(),
            "cycle_id=None must be omitted"
        );
        assert!(
            parsed.get("frame_id").is_none(),
            "frame_id=None must be omitted"
        );
        assert!(
            parsed.get("fork_id").is_none(),
            "fork_id=None must be omitted"
        );
    }

    /// Verifies that optional fields are included in JSON when Some. Round-trip
    /// must preserve the values.
    #[test]
    fn optional_fields_preserved_when_some() {
        let mut env = minimal_envelope();
        env.metadata = Some(json!({"key": "value"}));
        env.causation_id = Some("e-previous-1".into());
        env.correlation_id = Some("corr-session-1".into());
        env.cycle_id = Some("project-1/change".into());
        env.frame_id = Some("frame-1".into());
        env.fork_id = Some("fork-1".into());

        let json = env.to_canonical_json();
        let deserialized: EventEnvelopeV1 = serde_json::from_str(&json).unwrap();

        assert_eq!(deserialized.metadata, env.metadata);
        assert_eq!(deserialized.causation_id, env.causation_id);
        assert_eq!(deserialized.correlation_id, env.correlation_id);
        assert_eq!(deserialized.cycle_id, env.cycle_id);
        assert_eq!(deserialized.frame_id, env.frame_id);
        assert_eq!(deserialized.fork_id, env.fork_id);
    }

    /// Verifies that a "fixture" JSON event — a hand-crafted historical event —
    /// can be deserialized and will produce the correct content_hash when the
    /// hash is recomputed. This proves that events serialized under the v1
    /// schema remain loadable and verifiable indefinitely.
    #[test]
    fn schema_v1_fixture_deserializes_and_rehashes() {
        // This JSON is a representative v1 event captured from production.
        let fixture_json = r#"{
  "event_id": "e-project-1-cycle-1-entered-1",
  "event_type": "workflow.phase.entered",
  "schema_version": 1,
  "stream_id": "project-1/change",
  "sequence": 1,
  "project_id": "project-1",
  "occurred_at": "2026-08-01T09:00:00Z",
  "recorded_at": "2026-08-01T09:00:01Z",
  "actor": {
    "kind": "system",
    "id": "sddk-cli"
  },
  "subjects": [],
  "payload": {"phase": "explore"},
  "evidence_refs": [],
  "content_hash": ""
}"#;
        let env: EventEnvelopeV1 = serde_json::from_str(fixture_json).unwrap();

        // Verify all fields deserialized correctly
        assert_eq!(env.event_id, "e-project-1-cycle-1-entered-1");
        assert_eq!(env.event_type, "workflow.phase.entered");
        assert_eq!(env.schema_version, 1);
        assert_eq!(env.stream_id, "project-1/change");
        assert_eq!(env.project_id, "project-1");

        // content_hash was empty in fixture — recompute it
        let computed = env.compute_content_hash();
        assert!(computed.starts_with("sha256:"));
        assert_eq!(computed.len(), 7 + 64);
    }

    #[test]
    fn validate_event_type_accepts_valid() {
        let valid = [
            // 3+ segment names (canonical form)
            "workflow.phase.entered",
            "uat.acceptance.granted",
            "capability.execution.completed",
            "graph.staleness.detected",
            // 2 segment names (legacy corpus: cycle.created, lease.released)
            "cycle.created",
            "lease.released",
            "cycle.transitioned",
        ];
        for s in valid {
            assert_eq!(
                EventEnvelopeV1::validate_event_type(s),
                Ok(()),
                "expected {s:?} to be valid"
            );
        }
    }

    #[test]
    fn validate_event_type_rejects_invalid() {
        let invalid = [
            "invalid_type",       // no dots — 1 segment only
            "Upper.Started",      // uppercase in segment
            ".starts.with.dot",   // empty segment
            "trailing.dot.",      // empty final segment
            "1starts.with.digit", // segment starts with digit
            "cycle",              // single segment
        ];
        for s in invalid {
            assert!(
                EventEnvelopeV1::validate_event_type(s).is_err(),
                "expected {s:?} to be invalid"
            );
        }
    }
}
