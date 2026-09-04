//! Lineage tests for operator_contract module (DW-IR-004).
//!
//! Landing site per SPEC section 8 — validates the map from Operator variant
//! to its input/output schema is total over all 12 Operator variants.

use sddk_domain::operator_contract::{
    default_input_schema, default_output_schema, variant_name,
};
use sddk_domain::{CapabilityId, GuardExpr, Operator as DomainOperator, OperatorId};

/// variant_name is non-empty for all 12 Operator variants.
#[test]
fn variant_name_is_nonempty_for_all() {
    let variants = all_variants();
    for variant in &variants {
        let name = variant_name(variant);
        assert!(!name.is_empty(), "variant_name must be non-empty for {:?}", variant);
    }
}

/// variant_name is consistent: calling twice returns the same string.
#[test]
fn variant_name_is_idempotent() {
    let variants = all_variants();
    for variant in &variants {
        let name1 = variant_name(variant);
        let name2 = variant_name(variant);
        assert_eq!(name1, name2, "variant_name must be idempotent");
    }
}

/// default_input_schema is total: all 12 variants produce a schema.
#[test]
fn input_schema_total_over_all_variants() {
    let variants = all_variants();
    for variant in &variants {
        let result = std::panic::catch_unwind(|| default_input_schema(variant));
        assert!(
            result.is_ok(),
            "default_input_schema must not panic for {:?}",
            variant
        );
    }
}

/// default_output_schema is total: all 12 variants produce a schema.
#[test]
fn output_schema_total_over_all_variants() {
    let variants = all_variants();
    for variant in &variants {
        let result = std::panic::catch_unwind(|| default_output_schema(variant));
        assert!(
            result.is_ok(),
            "default_output_schema must not panic for {:?}",
            variant
        );
    }
}

/// input and output schemas are distinct types (verified by type name).
#[test]
fn input_and_output_are_distinct_types() {
    use sddk_domain::operator_contract::{OperatorInputSchema, OperatorOutputSchema};
    assert_ne!(
        std::any::type_name::<OperatorInputSchema>(),
        std::any::type_name::<OperatorOutputSchema>(),
        "Input and output schema types must be distinct"
    );
}

/// The 12 Operator variants map to 12 distinct variant names.
#[test]
fn all_variant_names_are_unique() {
    let variants = all_variants();
    let names: Vec<_> = variants.iter().map(variant_name).collect();
    let mut sorted = names.clone();
    sorted.sort();
    sorted.dedup();
    assert_eq!(
        names.len(),
        sorted.len(),
        "all variant names must be unique, got duplicates"
    );
}

/// Map and Sequence produce different schema structures.
#[test]
fn map_and_sequence_schemas_differ() {
    let map_variant = DomainOperator::Map {
        source: OperatorId("src".into()),
        body: OperatorId("body".into()),
        max_concurrency: 4,
    };
    let seq_variant = DomainOperator::Sequence { body: vec![] };

    let map_output = default_output_schema(&map_variant);
    let seq_output = default_output_schema(&seq_variant);

    // Map has strict output (accepts_extra_fields=false), Sequence is lenient
    assert!(
        !map_output.accepts_extra_fields,
        "Map output must be strict"
    );
    assert!(
        seq_output.accepts_extra_fields,
        "Sequence output must be lenient"
    );
    // Map requires item_results, Sequence does not
    assert!(
        map_output.required_fields.contains("item_results"),
        "Map must require item_results"
    );
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
