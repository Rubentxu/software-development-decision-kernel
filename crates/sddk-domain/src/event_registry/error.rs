//! Error types and the [`crate::event_registry::EventSchema`] trait for event registry and validation.
//!
//! # Architecture
//!
//! 1. [`crate::event_registry::EventSchema`] — trait describing the schema of a known event payload.
//! 2. [`crate::event_registry::EventSchemaRegistry`] — thread-safe registry mapping
//!    `(event_type, schema_version)` → [`crate::event_registry::EventSchema`].
//! 3. [`crate::event_registry::CanonicalEventValidator`] — three-stage validation gate: format → hash → schema.
//!
//! # Compatibility
//!
//! Validation is additive. Unknown event types are NOT errors at the validator
//! level — they are errors at the registry lookup level, allowing forward
//! compatibility where new event types can be introduced without updating every
//! consumer.

/// Errors returned by the event registry and validator.
#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
pub enum EventRegistryError {
    /// The `(event_type, schema_version)` pair is not registered.
    #[error("event_registry:unknown_type — no schema for `{event_type}` v{schema_version}")]
    UnknownType {
        /// Event type that was looked up.
        event_type: String,
        /// Schema version that was looked up.
        schema_version: u32,
    },
}

/// Errors returned by [`crate::event_registry::CanonicalEventValidator`].
#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
pub enum EventValidatorError {
    /// The `event_type` field failed the namespacing regex check.
    #[error("event_validator:invalid_event_type_format — {0}")]
    InvalidEventTypeFormat(String),

    /// The `content_hash` field does not match the recomputed value.
    #[error("event_validator:content_hash_mismatch")]
    ContentHashMismatch,

    /// The `content_hash` field has an invalid format (missing prefix or wrong length).
    #[error("event_validator:invalid_content_hash_format")]
    InvalidContentHashFormat,

    /// The schema version is unsupported.
    #[error("event_validator:unsupported_schema_version — got {got}, want {want}")]
    UnsupportedSchemaVersion {
        /// Schema version found in the envelope.
        got: u32,
        /// Schema version the validator supports.
        want: u32,
    },

    /// The event type + schema_version is not registered in the schema registry.
    #[error("event_validator:unknown_event_schema — {0}")]
    UnknownEventSchema(String),

    /// The payload failed schema validation.
    #[error("event_validator:payload_validation_failed — {detail}")]
    PayloadValidationFailed {
        /// Human-readable description of what failed validation.
        detail: String,
    },
}

/// Descriptive metadata for an event payload schema.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct EventSchemaInfo {
    /// Canonical event type string.
    pub event_type: String,
    /// Schema version for this event type.
    pub schema_version: u32,
    /// Human-readable description of the payload structure.
    pub description: String,
}

/// Trait for describing the schema of a known event payload.
///
/// Implementations describe what fields a valid payload must contain for a given
/// `(event_type, schema_version)` pair. The trait is `Send + Sync` to allow
/// the registry to be used from multi-threaded contexts.
pub trait EventSchema: Send + Sync {
    /// Returns descriptive metadata about this schema.
    fn info(&self) -> EventSchemaInfo;

    /// Validates a JSON payload against this schema.
    ///
    /// Returns `Ok(())` if the payload is structurally valid for the schema.
    /// Returns `Err(detail)` with a human-readable explanation of the validation
    /// failure.
    ///
    /// Implementations should be cheap and deterministic — no I/O, no network.
    fn validate_payload(&self, payload: &serde_json::Value) -> Result<(), String>;
}
