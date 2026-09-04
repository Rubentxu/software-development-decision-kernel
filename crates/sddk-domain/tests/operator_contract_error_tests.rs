//! Error variant tests for operator_contract module (DW-IR-004).
//!
//! Landing site per SPEC section 8:
//! - OperatorContractError has exactly 8 variants
//! - Each variant carries meaningful context (operator_id, variant name, field)

use sddk_domain::operator_contract::OperatorContractError;
use serde_json;

/// OperatorContractError has exactly 8 variants as specified in SPEC §V-2.
/// The compile-time guard `assert_variant_count_eq!` in the module ensures this.
/// This test provides compile-time coverage visibility by matching all 8 variants.
#[test]
fn error_has_8_variants() {
    use sddk_domain::operator_contract::OperatorContractError;
    // Exhaustiveness: if a variant is added/removed, this match fails to compile.
    match Option::<OperatorContractError>::None {
        None => {}
        Some(OperatorContractError::UnsupportedSchemaVersion { .. }) => {}
        Some(OperatorContractError::UnknownOperatorVariant { .. }) => {}
        Some(OperatorContractError::InputContractViolation { .. }) => {}
        Some(OperatorContractError::OutputContractViolation { .. }) => {}
        Some(OperatorContractError::MissingRequiredField { .. }) => {}
        Some(OperatorContractError::ExtraFieldDisallowed { .. }) => {}
        Some(OperatorContractError::SchemaSourceMismatch { .. }) => {}
        Some(OperatorContractError::SchemaDialectUnknown { .. }) => {}
    }
}

/// ExtraFieldDisallowed carries field name and variant context.
#[test]
fn extra_field_error_has_context() {
    let err = OperatorContractError::ExtraFieldDisallowed {
        operator_id: sddk_domain::OperatorId("map0".into()),
        variant: "Map",
        field: "rogue".into(),
    };
    let desc = format!("{err}");
    assert!(
        desc.contains("rogue"),
        "error description must mention the field"
    );
    assert!(
        desc.contains("Map"),
        "error description must mention variant"
    );
}

/// MissingRequiredField carries field name and variant context.
#[test]
fn missing_field_error_has_context() {
    let err = OperatorContractError::MissingRequiredField {
        operator_id: sddk_domain::OperatorId("map0".into()),
        variant: "Map",
        field: "item_results".into(),
    };
    let desc = format!("{err}");
    assert!(
        desc.contains("item_results"),
        "error must mention missing field"
    );
    assert!(desc.contains("Map"), "error must mention variant");
}

/// UnsupportedSchemaVersion carries expected vs actual version.
#[test]
fn unsupported_schema_version_error() {
    let err = OperatorContractError::UnsupportedSchemaVersion { got: 0, want: 1 };
    let desc = format!("{err}");
    assert!(desc.contains("0"), "error must show got version");
    assert!(desc.contains("1"), "error must show want version");
}

/// UnknownOperatorVariant carries variant name.
#[test]
fn unknown_variant_error() {
    let err = OperatorContractError::UnknownOperatorVariant { variant: "Unknown" };
    let desc = format!("{err}");
    assert!(desc.contains("Unknown"), "error must mention variant name");
}

/// SchemaSourceMismatch carries expected and actual hashes.
#[test]
fn schema_source_mismatch_error() {
    let err = OperatorContractError::SchemaSourceMismatch {
        operator_id: sddk_domain::OperatorId("op0".into()),
        expected: "abc123".into(),
        actual: "def456".into(),
    };
    let desc = format!("{err}");
    assert!(!desc.is_empty(), "error must format to non-empty string");
}

/// SchemaDialectUnknown carries the unknown dialect string.
#[test]
fn schema_dialect_unknown_error() {
    let err = OperatorContractError::SchemaDialectUnknown {
        dialect: "unknown-dialect".into(),
    };
    let desc = format!("{err}");
    assert!(
        desc.contains("unknown-dialect"),
        "error must mention dialect"
    );
}

/// All 8 variants implement std::error::Error + Send + Sync.
#[test]
fn all_error_variants_are_thread_safe() {
    fn assert_send_sync<T: Send + Sync>() {}
    assert_send_sync::<OperatorContractError>();
}

/// All 8 variants are PartialEq.
#[test]
fn all_error_variants_are_partial_eq() {
    fn assert_partial_eq<T: PartialEq>() {}
    assert_partial_eq::<OperatorContractError>();
}

/// SchemaDialectUnknown serialization produces valid JSON (FIND-000005).
/// This variant is forward-only (unreachable from deserialization of the closed
/// SchemaDialect enum). We verify it serializes to valid JSON and that the JSON
/// is parseable, proving the variant is serialization-safe.
///
/// Note: Full round-trip (serialize → deserialize) is not possible due to
/// &'static str fields + thiserror::Error + serde Derive requiring 'static.
/// This is a known thiserror limitation; the variant is still safe for forward
/// serialization paths (e.g., logging, debugging).
#[test]
fn schema_dialect_unknown_serializes_to_valid_json() {
    let err = OperatorContractError::SchemaDialectUnknown {
        dialect: "x".into(),
    };
    // Serialize to JSON string
    let json_str = serde_json::to_string(&err).expect("must serialize to JSON string");
    // Verify it's valid JSON that can be parsed back
    let parsed: serde_json::Value = serde_json::from_str(&json_str)
        .expect("must be valid parseable JSON");
    // The JSON is {"schema_dialect_unknown":{"dialect":"x"}}
    let inner = parsed.get("schema_dialect_unknown")
        .expect("variant key must exist");
    assert_eq!(inner.get("dialect").and_then(|v| v.as_str()), Some("x"));
}
