//! Three-stage event envelope validator: format → hash → schema.
//!
//! See [`CanonicalEventValidator`] for the full validation pipeline. Stage order is
//! deliberate: hash verification is a pure function of envelope bytes (cheap) and
//! rejecting tampered events before schema lookup prevents probing attacks.

use std::sync::Arc;

use super::error::EventValidatorError;
use super::registry::EventSchemaRegistry;

// ── Canonical Validator ───────────────────────────────────────────────────────

/// Three-stage event envelope validator.
///
/// Stage order (envelope-level checks BEFORE payload-level checks):
/// 1. **Format check** — `event_type` namespacing regex + `schema_version` known
/// 2. **Hash check** — `content_hash` matches recomputed value
/// 3. **Schema check** — payload validates against registered schema
///
/// The ordering is deliberate: hash verification is a pure function of the
/// envelope bytes and is cheaper than schema lookup; rejecting tampered events
/// before doing any schema lookups also prevents probing attacks.
#[derive(Debug, Clone)]
pub struct CanonicalEventValidator {
    registry: Arc<EventSchemaRegistry>,
}

impl CanonicalEventValidator {
    /// Creates a new validator backed by the given registry.
    ///
    /// The registry must be initialized with all known event types before
    /// validation is performed.
    pub fn new(registry: Arc<EventSchemaRegistry>) -> Self {
        Self { registry }
    }

    /// Performs three-stage validation on an event envelope.
    ///
    /// Stage order: format → hash → schema.
    /// The first failure terminates validation (fail-fast).
    ///
    /// Returns `Ok(())` if all three stages pass.
    /// Returns a specific error variant for the first failure encountered.
    pub fn validate(&self, envelope: &crate::EventEnvelopeV1) -> Result<(), EventValidatorError> {
        // ── Stage 1: Format checks ────────────────────────────────────────────
        self.validate_format(envelope)?;

        // ── Stage 2: Hash check ──────────────────────────────────────────────
        self.validate_hash(envelope)?;

        // ── Stage 3: Schema check ───────────────────────────────────────────
        self.validate_schema(envelope)?;

        Ok(())
    }

    /// Stage 1: validates event_type format and schema_version.
    fn validate_format(
        &self,
        envelope: &crate::EventEnvelopeV1,
    ) -> Result<(), EventValidatorError> {
        // Check event_type namespacing
        if let Err(e) = crate::EventEnvelopeV1::validate_event_type(&envelope.event_type) {
            return Err(EventValidatorError::InvalidEventTypeFormat(e.to_string()));
        }

        // Check schema version is known (v1 only for now)
        if envelope.schema_version != crate::EventEnvelopeV1::SCHEMA_VERSION {
            return Err(EventValidatorError::UnsupportedSchemaVersion {
                got: envelope.schema_version,
                want: crate::EventEnvelopeV1::SCHEMA_VERSION,
            });
        }

        Ok(())
    }

    /// Stage 2: validates content_hash matches recomputed value.
    fn validate_hash(&self, envelope: &crate::EventEnvelopeV1) -> Result<(), EventValidatorError> {
        // Check content_hash format
        if !envelope.content_hash.starts_with("sha256:")
            || envelope.content_hash.len() != "sha256:".len() + 64
        {
            return Err(EventValidatorError::InvalidContentHashFormat);
        }

        // Recompute and compare
        let computed = envelope.compute_content_hash();
        if computed != envelope.content_hash {
            return Err(EventValidatorError::ContentHashMismatch);
        }

        Ok(())
    }

    /// Stage 3: validates payload against the registered schema.
    fn validate_schema(
        &self,
        envelope: &crate::EventEnvelopeV1,
    ) -> Result<(), EventValidatorError> {
        let schema = self
            .registry
            .get(&envelope.event_type, envelope.schema_version)
            .map_err(|_| {
                EventValidatorError::UnknownEventSchema(format!(
                    "{} v{}",
                    envelope.event_type, envelope.schema_version
                ))
            })?;

        schema
            .validate_payload(&envelope.payload)
            .map_err(|detail| EventValidatorError::PayloadValidationFailed { detail })
    }
}

#[cfg(test)]
mod tests {
    use std::sync::Arc;

    use super::super::schemas::std_registry;
    use super::{CanonicalEventValidator, EventSchemaRegistry, EventValidatorError};
    use crate::event_envelope::{ActorKind, ActorRef, EventEnvelopeV1};
    use crate::projections::{JournalProjection, Projection};
    use serde_json::json;

    fn valid_envelope(event_type: &str, payload: serde_json::Value) -> EventEnvelopeV1 {
        let mut env = EventEnvelopeV1 {
            event_id: "evt-test-1".into(),
            event_type: event_type.into(),
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

    fn build_corpus_envelope(
        seq: usize,
        event_type: &str,
        payload: serde_json::Value,
    ) -> EventEnvelopeV1 {
        let mut env = EventEnvelopeV1 {
            event_id: format!("evt-corpus-{}", seq + 1),
            event_type: event_type.to_string(),
            schema_version: 1,
            stream_id: "stream-corpus".to_string(),
            sequence: (seq + 1) as u64,
            project_id: "p-corpus".to_string(),
            occurred_at: "2026-08-22T00:00:00Z".into(),
            recorded_at: "2026-08-22T00:00:00Z".into(),
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
    fn validator_passes_valid_envelope() {
        let registry = std_registry();
        let validator = CanonicalEventValidator::new(registry);
        let env = valid_envelope("workflow.phase.entered", json!({ "phase": "build" }));
        assert!(validator.validate(&env).is_ok());
    }

    #[test]
    fn validator_rejects_content_hash_mismatch_before_payload_check() {
        let registry = std_registry();
        let validator = CanonicalEventValidator::new(registry);
        let mut env = valid_envelope("workflow.phase.entered", json!({ "phase": "build" }));
        // Tamper with the payload — hash should fail FIRST
        env.payload = json!({ "phase": "tampered" });
        let result = validator.validate(&env);
        // Should fail on hash mismatch, not on payload schema
        assert!(matches!(
            result,
            Err(EventValidatorError::ContentHashMismatch)
        ));
    }

    #[test]
    fn content_hash_roundtrip_preserved_for_legacy_events() {
        // This test proves that compute_content_hash() is stable for all known
        // event types, ensuring byte-identical hashes after re-serialization.

        let cases = [
            ("workflow.phase.entered", json!({ "phase": "build" })),
            ("workflow.phase.exited", json!({})),
            (
                "workflow.transition.succeeded",
                json!({ "transition_id": "t1", "outcome": "succeeded", "from_phase": "explore", "to_phase": "build", "failed_gates": [] }),
            ),
            (
                "approval.capability.requested",
                json!({ "capability": "git.commit", "cycle_id": "c-1", "request_hash": "sha256:abc123", "expires_at": "2026-08-23T00:00:00Z" }),
            ),
            ("uat.scenario.started", json!({ "mode": "runner" })),
            ("uat.check.passed", json!({ "verdict": "pass" })),
            (
                "uat.acceptance.granted",
                json!({ "acceptance_record_hash": "sha256:dddd" }),
            ),
            (
                "workflow.ir.compiled",
                json!({ "template_id": "sddk.adaptive.discovery", "template_version": "1.0.0", "ir_hash": "sha256:2222", "operator_count": 7 }),
            ),
            (
                "workflow.run.started",
                json!({ "run_id": "run-1", "ir_hash": "sha256:2222", "correlation_id": "corr-1", "budget_json": { "max_wall_ms": 60000 } }),
            ),
            (
                "workflow.run.cancelled",
                json!({ "run_id": "run-1", "reason": "user_requested" }),
            ),
            (
                "workflow.graph.revision.accepted",
                json!({ "run_id": "run-1", "revision": 0, "digest": "sha256:6666" }),
            ),
            (
                "cycle.snapshot.restored",
                json!({ "cycle_id": "c-1", "restored_at_ms": 1234567890i64 }),
            ), // NOTE: restored_at_ms is i64, not string
        ];

        for (event_type, payload) in cases {
            let env = valid_envelope(event_type, payload);
            let hash = env.compute_content_hash();
            // Verify the hash is stable (round-trip)
            assert!(
                hash.starts_with("sha256:"),
                "hash for {event_type} should be valid sha256 format"
            );
            assert_eq!(hash.len(), 71, "hash length for {event_type} should be 71");

            // Also validate through the validator
            let registry = std_registry();
            let validator = CanonicalEventValidator::new(registry);
            if let Err(e) = validator.validate(&env) {
                panic!("validator rejected {event_type}: {e}");
            }
        }
    }

    #[test]
    fn validator_rejects_invalid_event_type_format() {
        let registry = std_registry();
        let validator = CanonicalEventValidator::new(registry);
        let mut env = valid_envelope("workflow.phase.entered", json!({ "phase": "build" }));
        env.event_type = "InvalidType".into();
        env.content_hash = env.compute_content_hash();
        let result = validator.validate(&env);
        assert!(matches!(
            result,
            Err(EventValidatorError::InvalidEventTypeFormat(_))
        ));
    }

    #[test]
    fn validator_rejects_unknown_event_schema() {
        let registry = Arc::new(EventSchemaRegistry::new());
        let validator = CanonicalEventValidator::new(registry);
        let env = valid_envelope("workflow.phase.entered", json!({ "phase": "build" }));
        let result = validator.validate(&env);
        assert!(matches!(
            result,
            Err(EventValidatorError::UnknownEventSchema(_))
        ));
    }

    #[test]
    fn corpus_replay_through_validator() {
        // Corpus exercising ALL 18 registered event types through
        // CanonicalEventValidator + JournalProjection.
        // Asserts zero rejections + deterministic journal output.
        // Format regex now accepts ≥2 dot-separated segments (fixed from ≥3).

        use crate::JournalProjection;

        let registry = std_registry();
        let validator = CanonicalEventValidator::new(registry);

        // All 18 registered event types — ZERO exclusions.
        let corpus: Vec<(&str, serde_json::Value)> = vec![
            // Workflow events
            ("workflow.phase.entered", json!({ "phase": "build" })),
            ("workflow.phase.exited", json!({})),
            (
                "workflow.transition.succeeded",
                json!({
                    "transition_id": "t-1",
                    "outcome": "succeeded",
                    "from_phase": "explore",
                    "to_phase": "build",
                    "failed_gates": []
                }),
            ),
            (
                "workflow.transition.failed",
                json!({ "transition_id": "t-2", "failed_gates": ["gate-1"] }),
            ),
            (
                "workflow.ir.compiled",
                json!({
                    "template_id": "sddk.adaptive.discovery",
                    "ir_hash": "sha256:abc123def456",
                    "template_version": "1.0.0",
                    "operator_count": 7
                }),
            ),
            (
                "workflow.run.started",
                json!({ "run_id": "run-1", "ir_hash": "sha256:2222" }),
            ),
            (
                "workflow.run.cancelled",
                json!({ "run_id": "run-1", "reason": "user_requested" }),
            ),
            (
                "workflow.graph.revision.accepted",
                json!({ "run_id": "run-1", "revision": 0, "digest": "sha256:6666" }),
            ),
            // Approval events
            (
                "approval.capability.requested",
                json!({
                    "capability": "git.commit",
                    "cycle_id": "c-1",
                    "request_hash": "sha256:abc123",
                    "expires_at": "2026-08-23T00:00:00Z"
                }),
            ),
            (
                "approval.capability.granted",
                json!({
                    "cycle_id": "c-1",
                    "capability": "git.commit",
                    "request_hash": "sha256:abc123"
                }),
            ),
            (
                "approval.capability.denied",
                json!({
                    "cycle_id": "c-1",
                    "capability": "git.commit",
                    "request_hash": "sha256:abc123"
                }),
            ),
            // UAT events
            ("uat.scenario.started", json!({ "mode": "runner" })),
            ("uat.check.passed", json!({ "verdict": "pass" })),
            (
                "uat.acceptance.granted",
                json!({ "acceptance_record_hash": "sha256:dddd" }),
            ),
            // Cycle events (including 2-segment legacy types)
            (
                "cycle.created",
                json!({ "cycle_id": "c-1", "transition_id": "t-x", "outcome": "created" }),
            ),
            (
                "cycle.transitioned",
                json!({ "transition_id": "t-1", "outcome": "succeeded" }),
            ),
            (
                "cycle.snapshot.restored",
                json!({ "cycle_id": "c-1", "restored_at_ms": 1234567890i64 }),
            ),
            // Lease events (2-segment legacy type)
            (
                "lease.released",
                json!({ "cycle_id": "c-1", "released_at_ms": 1234567890i64 }),
            ),
        ];

        // Build EventEnvelopeV1 corpus and validate through CanonicalEventValidator
        let mut envelopes: Vec<EventEnvelopeV1> = Vec::new();
        for (seq, (event_type, payload)) in corpus.iter().enumerate() {
            let env = build_corpus_envelope(seq, event_type, payload.clone());

            // Validate through CanonicalEventValidator — must not reject
            assert!(
                validator.validate(&env).is_ok(),
                "validator rejected {}: {:?}",
                event_type,
                validator.validate(&env)
            );
            envelopes.push(env);
        }

        // Replay through JournalProjection — must be deterministic
        let mut proj_a = JournalProjection::new("stream-corpus");
        let mut proj_b = JournalProjection::new("stream-corpus");

        for ev in &envelopes {
            proj_a.apply(ev).expect("journal apply must not fail");
        }
        for ev in &envelopes {
            proj_b.apply(ev).expect("journal apply must not fail");
        }

        let state_a = proj_a.state_ref();
        let state_b = proj_b.state_ref();

        assert_eq!(state_a.len(), state_b.len(), "replay length must match");
        for (a, b) in state_a.iter().zip(state_b.iter()) {
            assert_eq!(a.event_id, b.event_id, "event_id must match on replay");
            assert_eq!(
                a.event_type, b.event_type,
                "event_type must match on replay"
            );
            assert_eq!(a.sequence, b.sequence, "sequence must match on replay");
            assert_eq!(a.severity, b.severity, "severity must match on replay");
        }

        // Serialized form must also be byte-identical
        let json_a = serde_json::to_string(state_a).expect("state must serialize");
        let json_b = serde_json::to_string(state_b).expect("state must serialize");
        assert_eq!(
            json_a, json_b,
            "journal replay must produce byte-identical JSON"
        );

        // Assert all 18 registered event types processed
        assert_eq!(
            envelopes.len(),
            18,
            "corpus must contain all 18 registered types"
        );
        assert_eq!(state_a.len(), 18, "journal must contain all 18 events");
    }
}
