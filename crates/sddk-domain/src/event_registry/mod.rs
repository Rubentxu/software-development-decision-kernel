//! Event schema registry and canonical event validator.
//!
//! This module provides the canonical validation substrate for `EventEnvelopeV1`
//! events. It is pure (no I/O, no side effects): same input always yields same
//! output.
//!
//! # Architecture
//!
//! 1. [`crate::event_registry::EventSchema`] — trait describing the schema of a known event payload.
//! 2. [`crate::event_registry::EventSchemaRegistry`] — thread-safe registry mapping
//!    `(event_type, schema_version)` → [`crate::event_registry::EventSchema`].
//! 3. [`crate::event_registry::CanonicalEventValidator`] — three-stage validation gate: format → hash → schema.
//!
//! # Submodules
//!
//! - [`error`] — [`crate::event_registry::EventRegistryError`], [`crate::event_registry::EventValidatorError`],
//!   [`crate::event_registry::EventSchemaInfo`], [`crate::event_registry::EventSchema`] trait.
//! - [`registry`] — [`crate::event_registry::EventSchemaRegistry`] implementation.
//! - [`validator`] — [`crate::event_registry::CanonicalEventValidator`] implementation.
//! - [`schemas`] — the `schema_struct` macro, all registered schemas, and [`crate::event_registry::schemas::std_registry()`].

pub mod error;
pub mod registry;
pub mod schemas;
pub mod validator;

// Re-export all public types for external consumers.
// External consumers (e.g. crates/sddk-cli/src/approval.rs) use:
//   sddk_domain::event_registry::{CanonicalEventValidator, EventRegistryError, ...}
pub use error::EventSchema;
pub use error::{EventRegistryError, EventSchemaInfo, EventValidatorError};
pub use registry::EventSchemaRegistry;
pub use validator::CanonicalEventValidator;
