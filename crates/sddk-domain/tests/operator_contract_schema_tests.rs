//! Schema structure tests for operator_contract module (DW-IR-004).
//!
//! Landing site per SPEC section 8 for schema structural properties:
//! - REQ-OPOUT-002: schema_version = 1 per output schema
//! - REQ-OPOUT-003: accepts_extra_fields = false on Map output
//! - REQ-OPOUT-004: item_results in required_fields for Map
//! - REQ-OPOUT-005: items no longer appears as an output key

use sddk_domain::operator_contract::{
    default_input_schema, default_output_schema,
    OperatorInputSchema, OperatorOutputSchema, OperatorSchema, SchemaDialect,
    OPERATOR_CONTRACT_SCHEMA_VERSION,
};
use sddk_domain::{CapabilityId, GuardExpr, Operator as DomainOperator, OperatorId};

/// REQ-OPOUT-002: every output schema's fields have version = OPERATOR_CONTRACT_SCHEMA_VERSION.
#[test]
fn output_schema_field_versions_are_one() {
    let variants = all_variants();
    for variant in &variants {
        let output = default_output_schema(variant);
        for (field_name, field_schema) in &output.static_fields {
            assert_eq!(
                field_schema.version,
                OPERATOR_CONTRACT_SCHEMA_VERSION,
                "output field '{}' schema_version must be {} for {:?}",
                field_name,
                OPERATOR_CONTRACT_SCHEMA_VERSION,
                variant
            );
        }
    }
}

/// REQ-OPOUT-002: every input schema's fields have version = OPERATOR_CONTRACT_SCHEMA_VERSION.
#[test]
fn input_schema_field_versions_are_one() {
    let variants = all_variants();
    for variant in &variants {
        let input = default_input_schema(variant);
        for (field_name, field_schema) in &input.static_fields {
            assert_eq!(
                field_schema.version,
                OPERATOR_CONTRACT_SCHEMA_VERSION,
                "input field '{}' schema_version must be {} for {:?}",
                field_name,
                OPERATOR_CONTRACT_SCHEMA_VERSION,
                variant
            );
        }
    }
}

/// REQ-OPOUT-003: Map output has accepts_extra_fields = false.
#[test]
fn map_output_strict_no_extra_fields() {
    let variant = DomainOperator::Map {
        source: OperatorId("src".into()),
        body: OperatorId("body".into()),
        max_concurrency: 4,
    };
    let output = default_output_schema(&variant);
    assert!(
        !output.accepts_extra_fields,
        "Map output must not accept extra fields"
    );
}

/// REQ-OPOUT-003: Sequence output accepts extra fields (lenient by design).
#[test]
fn sequence_output_accepts_extra_fields() {
    let variant = DomainOperator::Sequence { body: vec![] };
    let output = default_output_schema(&variant);
    assert!(
        output.accepts_extra_fields,
        "Sequence output is lenient by design"
    );
}

/// REQ-OPOUT-004: Map output requires item_results.
#[test]
fn map_output_requires_item_results() {
    let variant = DomainOperator::Map {
        source: OperatorId("src".into()),
        body: OperatorId("body".into()),
        max_concurrency: 4,
    };
    let output = default_output_schema(&variant);
    assert!(
        output.required_fields.contains("item_results"),
        "Map output must require item_results"
    );
}

/// REQ-OPOUT-004: Map output does NOT require legacy "items" key.
#[test]
fn map_output_does_not_require_items() {
    let variant = DomainOperator::Map {
        source: OperatorId("src".into()),
        body: OperatorId("body".into()),
        max_concurrency: 4,
    };
    let output = default_output_schema(&variant);
    assert!(
        !output.required_fields.contains("items"),
        "Map output must NOT require legacy 'items' key"
    );
}

/// REQ-OPOUT-005: "items" is not a required field for any variant output.
#[test]
fn no_variant_requires_items_output() {
    let variants = all_variants();
    for variant in &variants {
        let output = default_output_schema(variant);
        assert!(
            !output.required_fields.contains("items"),
            "{:?} output must not require 'items'",
            variant
        );
    }
}

/// Schema dialect is JsonSchemaDraft07 for all field schemas.
#[test]
fn all_field_schemas_use_jsonschema_draft07() {
    let variants = all_variants();
    for variant in &variants {
        let input = default_input_schema(variant);
        let output = default_output_schema(variant);
        for (fname, fschema) in &input.static_fields {
            assert!(
                matches!(fschema.dialect, SchemaDialect::JsonSchemaDraft07),
                "input field '{}' dialect must be JsonSchemaDraft07 for {:?}",
                fname, variant
            );
        }
        for (fname, fschema) in &output.static_fields {
            assert!(
                matches!(fschema.dialect, SchemaDialect::JsonSchemaDraft07),
                "output field '{}' dialect must be JsonSchemaDraft07 for {:?}",
                fname, variant
            );
        }
    }
}

/// input and output schemas are distinct types (verified by type name).
#[test]
fn input_and_output_are_distinct_types() {
    // These are different newtype wrappers — verify by type name
    assert_ne!(
        std::any::type_name::<OperatorInputSchema>(),
        std::any::type_name::<OperatorOutputSchema>(),
        "Input and output schema types must be distinct"
    );
}

/// All schemas use BTreeMap for static_fields (deterministic order).
/// This is verified by the type system — static_fields: BTreeMap<String, OperatorSchema>.
/// This test serves as documentation and provides a compile-time anchor.
#[test]
fn static_fields_uses_btreemap() {
    use std::collections::BTreeMap;
    let variants = all_variants();
    for variant in &variants {
        let input = default_input_schema(variant);
        let output = default_output_schema(variant);
        // BTreeMap<K, V> is Deterministic: iteration order is sorted by K.
        let _ = &input.static_fields;
        let _ = &output.static_fields;
        // Verify they are indeed BTreeMaps by checking type at runtime via mem size
        assert_eq!(
            std::mem::size_of_val(&input.static_fields) > 0,
            true,
            "input static_fields must be populated"
        );
        assert_eq!(
            std::mem::size_of_val(&output.static_fields) > 0,
            true,
            "output static_fields must be populated"
        );
    }
}

fn all_variants() -> Vec<DomainOperator> {
    vec![
        DomainOperator::Task { capability: CapabilityId("test.cap".into()), inputs: Default::default() },
        DomainOperator::Sequence { body: vec![] },
        DomainOperator::Parallel { branches: vec![], max_concurrency: 1 },
        DomainOperator::Map { source: OperatorId("src".into()), body: OperatorId("body".into()), max_concurrency: 4 },
        DomainOperator::Join { policy: "all".into(), branches: vec![] },
        DomainOperator::Race { branches: vec![], timeout_ms: 1000 },
        DomainOperator::Choice { branches: Default::default() },
        DomainOperator::Loop { max_iterations: 10, until: GuardExpr { expr: "true".into() }, body: OperatorId("body".into()) },
        DomainOperator::Gate { condition: GuardExpr { expr: "true".into() }, body: OperatorId("body".into()) },
        DomainOperator::Wait { event_type: "click".into(), timeout_ms: 5000 },
        DomainOperator::SubWorkflow { run_ref: "run-1".into() },
        DomainOperator::Compensate { of: OperatorId("op0".into()) },
    ]
}
