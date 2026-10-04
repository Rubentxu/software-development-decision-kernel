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
    use super::EventRegistryError;

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
        // We register 27 event types (22 prior + 4 backlog ledger events added in
        // cycle p-63676b11dc0ef88f/backlog-ledger-substrate + 1 static-evidence
        // observation.set.appended v1 added in cycle p-63676b11dc0ef88f/
        // a6-static-enhanced-readiness/slices/s4-durability).
        assert_eq!(registry.len(), 27);
    }

    /// `backlog.item.discarded` must validate **membership** in the
    /// closed set, not merely that `reason` is a string.
    ///
    /// This is the gate for payloads that reach the log without passing
    /// through the typed `BacklogEvent` — a replay, an import, a hand-
    /// written row. Before, the schema accepted any string, so a closed
    /// set enforced only by clap was one `String` away from being a
    /// suggestion.
    #[test]
    fn discarded_schema_admits_every_member_and_nothing_else() {
        use crate::backlog::BacklogDiscardReason;

        let registry = std_registry();
        let schema = registry
            .get("backlog.item.discarded", 1)
            .expect("backlog.item.discarded v1 must be registered");

        let payload = |reason: &str| {
            serde_json::json!({
                "item_id": "B-001",
                "reason": reason,
                "discarded_at": "2026-10-04T00:00:00Z",
            })
        };

        for r in BacklogDiscardReason::ALL {
            let p = payload(&r.to_string());
            assert!(
                schema.validate_payload(&p).is_ok(),
                "member {r:?} must validate: {:?}",
                schema.validate_payload(&p)
            );
        }

        for bad in [
            "",
            "won't fix",
            "done",
            "x",
            "banana",
            "Resolved",
            "RESOLVED",
            " resolved",
        ] {
            let p = payload(bad);
            assert!(
                schema.validate_payload(&p).is_err(),
                "{bad:?} must NOT validate, but it did"
            );
        }
    }

    /// The reason member list and the schema's notion of it cannot be
    /// two different sets. A member added to the enum that the schema
    /// would reject would make the type unwritable; one the schema would
    /// accept but the enum lacks would reopen the hole.
    #[test]
    fn discarded_schema_and_the_domain_enum_describe_the_same_set() {
        use crate::backlog::BacklogDiscardReason;

        let registry = std_registry();
        let schema = registry
            .get("backlog.item.discarded", 1)
            .expect("backlog.item.discarded v1 must be registered");

        // Anything the schema admits must parse back to a member...
        for candidate in [
            "superseded",
            "wontfix",
            "duplicate",
            "resolved",
            "done",
            "x",
            "",
            "banana",
        ] {
            let p = serde_json::json!({
                "item_id": "B-001",
                "reason": candidate,
                "discarded_at": "2026-10-04T00:00:00Z",
            });
            if schema.validate_payload(&p).is_ok() {
                assert!(
                    candidate.parse::<BacklogDiscardReason>().is_ok(),
                    "the schema admits {candidate:?} but the domain parser refuses it"
                );
            }
        }
        // ...and the schema's description must point at the authority
        // rather than transcribe it. Demanding that the description
        // enumerate the members would force a hand-written second copy
        // of the set back into the source — the very defect the typed
        // reason removed from the error message — so the assertion is
        // that the description *names the type*, not the values.
        let described = format!("{:?}", schema.info()).to_lowercase();
        assert!(
            described.contains("backlogdiscardreason"),
            "the schema description must name the authority type instead of \
             listing the set by hand: {described}"
        );
    }
}
