//! Thread-safe, in-memory registry of event schemas keyed by `(event_type, schema_version)`.
//!
//! The registry is built once at startup (or test initialization) and is thereafter
//! read-only. All lookups are O(1) via an internal `Arc`-based map.
use std::sync::Arc;

use super::error::{EventRegistryError, EventSchema};

// ── Registry implementation ────────────────────────────────────────────────────

/// Thread-safe, in-memory registry of event schemas.
///
/// The registry is keyed by `(event_type, schema_version)` pairs. It is built
/// once at startup (or test initialization) and is thereafter read-only.
/// All lookups are O(1) via an internal `Arc`-based map.
#[derive(Default)]
pub struct EventSchemaRegistry {
    entries: std::collections::HashMap<(String, u32), Arc<dyn EventSchema>>,
}

impl std::fmt::Debug for EventSchemaRegistry {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("EventSchemaRegistry")
            .field("entries", &self.entries.len())
            .finish()
    }
}

impl EventSchemaRegistry {
    /// Creates a new empty registry.
    pub fn new() -> Self {
        Self {
            entries: std::collections::HashMap::new(),
        }
    }

    /// Registers a schema for a given `(event_type, schema_version)` pair.
    ///
    /// If a schema is already registered for this pair, it is replaced.
    pub fn register<S: EventSchema + 'static>(&mut self, schema: S) {
        let info = schema.info();
        self.entries.insert(
            (info.event_type.clone(), info.schema_version),
            Arc::new(schema),
        );
    }

    /// Looks up the schema for a given `(event_type, schema_version)` pair.
    ///
    /// Returns `Ok(Arc<dyn EventSchema>)` if found.
    /// Returns `Err(EventRegistryError::UnknownType)` if not registered.
    pub fn get(
        &self,
        event_type: &str,
        schema_version: u32,
    ) -> Result<Arc<dyn EventSchema>, EventRegistryError> {
        self.entries
            .get(&(event_type.to_owned(), schema_version))
            .cloned()
            .ok_or_else(|| EventRegistryError::UnknownType {
                event_type: event_type.to_owned(),
                schema_version,
            })
    }

    /// Returns `true` if the registry has a schema for the given pair.
    pub fn contains(&self, event_type: &str, schema_version: u32) -> bool {
        self.entries
            .contains_key(&(event_type.to_owned(), schema_version))
    }

    /// Returns the number of registered schemas.
    pub fn len(&self) -> usize {
        self.entries.len()
    }

    /// Returns `true` if the registry is empty.
    pub fn is_empty(&self) -> bool {
        self.entries.is_empty()
    }
}

#[cfg(test)]
mod tests {
    use super::super::schemas::std_registry;
    use super::{Arc, EventRegistryError, EventSchemaRegistry};
    use crate::event_envelope::{ActorKind, ActorRef, EventEnvelopeV1};
    use crate::projections::Projection;
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

    #[test]
    fn registry_resolves_known_types() {
        let registry = std_registry();
        assert!(registry.contains("workflow.phase.entered", 1));
        assert!(registry.contains("approval.capability.requested", 1));
        assert!(registry.contains("uat.scenario.started", 1));
    }

    #[test]
    fn registry_unknown_type_errors_without_panic() {
        let registry = std_registry();
        let result = registry.get("nonexistent.type.foo", 1);
        assert!(matches!(
            result,
            Err(EventRegistryError::UnknownType { .. })
        ));
    }

    #[test]
    fn registry_len_matches_expected_count() {
        let registry = std_registry();
        // We register 22 event types (17 original + 5 workflow events added in cycle-16)
        assert_eq!(registry.len(), 22);
    }
}
