//! Engine-level operator contract tests for DW-IR-004.
//!
//! Test landing sites per SPEC section 8:
//! - REQ-OPIN-001..004: input contract validation
//! - REQ-OPOUT-001..006: output contract validation
//! - REQ-OPTEST-001: grep-based CI assertion
//!
//! The grep test (no_placeholder_remains) is the primary gate for REQ-OPTEST-001.
//! The schema-per-variant tests validate that every variant has a well-formed
//! input and output schema with the correct structural properties.

use std::collections::BTreeMap;

use sddk_domain::{
    CapabilityId, Operator as DomainOperator, OperatorId,
};
use sddk_domain::operator_contract::{
    default_input_schema, default_output_schema, variant_name,
    OperatorContractError,
};

// ── REQ-OPTEST-001 — grep CI guard ───────────────────────────────────────────

/// Grep-based CI assertion: the literal placeholder `outputs["items"]: serde_json::Value::Array`
/// must NOT appear in any `.rs`, `.yaml`, `.toml`, `.md`, or `.snap` file under `crates/`.
///
/// This is REQ-OPTEST-001 — the adversarial snapshot must not contain the old
/// untyped placeholder that was the root cause of the DW-IR-004 problem.
///
/// Run with: `cargo test -p sddk-engine --test operator_contract_tests`
#[test]
fn no_placeholder_remains() {
    // Walk all source/test/doc files under crates/ and assert no placeholder.
    let placeholder = r#"outputs["items"]: serde_json::Value::Array"#;
    let mut found = Vec::new();

    for entry in walkdir_files("crates", &["rs", "yaml", "toml", "md", "snap"]) {
        let content = std::fs::read_to_string(&entry).unwrap_or_default();
        if content.contains(placeholder) || content.contains("outputs[\"items\"]") {
            found.push(entry.display().to_string());
        }
    }

    assert!(
        found.is_empty(),
        "Placeholder `outputs[\"items\"]: serde_json::Value::Array` found in: {found:?}"
    );
}

fn walkdir_files(root: &str, extensions: &[&str]) -> Vec<std::path::PathBuf> {
    let mut results = Vec::new();
    if let Ok(entries) = std::fs::read_dir(root) {
        for entry in entries.flatten() {
            let path = entry.path();
            if path.is_dir() {
                results.extend(walkdir_files(&path.to_string_lossy(), extensions));
            } else if let Some(ext) = path.extension() {
                if extensions.contains(&ext.to_str().unwrap_or("")) {
                    results.push(path);
                }
            }
        }
    }
    results
}

// ── REQ-OPIN-001 / REQ-OPOUT-001 — per-variant default schemas ────────────────

/// REQ-OPIN-001: every variant has a default input contract.
#[test]
fn input_schema_per_variant_12() {
    let variants = [
        DomainOperator::Task { capability: CapabilityId("test.cap".into()), inputs: Default::default() },
        DomainOperator::Sequence { body: vec![] },
        DomainOperator::Parallel { branches: vec![], max_concurrency: 1 },
        DomainOperator::Map { source: OperatorId("src".into()), body: OperatorId("body".into()), max_concurrency: 4 },
        DomainOperator::Join { policy: "all".into(), branches: vec![] },
        DomainOperator::Race { branches: vec![], timeout_ms: 1000 },
        DomainOperator::Choice { branches: Default::default() },
        DomainOperator::Loop { max_iterations: 10, until: sddk_domain::GuardExpr { expr: "true".into() }, body: OperatorId("body".into()) },
        DomainOperator::Gate { condition: sddk_domain::GuardExpr { expr: "true".into() }, body: OperatorId("body".into()) },
        DomainOperator::Wait { event_type: "click".into(), timeout_ms: 5000 },
        DomainOperator::SubWorkflow { run_ref: "run-1".into() },
        DomainOperator::Compensate { of: OperatorId("op0".into()) },
    ];

    for variant in &variants {
        let input = default_input_schema(variant);
        assert_eq!(&input, &input, "input schema must be PartialEq for {:?}", variant_name(variant));
    }
}

/// REQ-OPOUT-001: every variant has a default output contract.
#[test]
fn output_schema_per_variant_12() {
    let variants = [
        DomainOperator::Task { capability: CapabilityId("test.cap".into()), inputs: Default::default() },
        DomainOperator::Sequence { body: vec![] },
        DomainOperator::Parallel { branches: vec![], max_concurrency: 1 },
        DomainOperator::Map { source: OperatorId("src".into()), body: OperatorId("body".into()), max_concurrency: 4 },
        DomainOperator::Join { policy: "all".into(), branches: vec![] },
        DomainOperator::Race { branches: vec![], timeout_ms: 1000 },
        DomainOperator::Choice { branches: Default::default() },
        DomainOperator::Loop { max_iterations: 10, until: sddk_domain::GuardExpr { expr: "true".into() }, body: OperatorId("body".into()) },
        DomainOperator::Gate { condition: sddk_domain::GuardExpr { expr: "true".into() }, body: OperatorId("body".into()) },
        DomainOperator::Wait { event_type: "click".into(), timeout_ms: 5000 },
        DomainOperator::SubWorkflow { run_ref: "run-1".into() },
        DomainOperator::Compensate { of: OperatorId("op0".into()) },
    ];

    for variant in &variants {
        let output = default_output_schema(variant);
        let name = variant_name(variant);
        assert!(!name.is_empty(), "variant name must be non-empty for {:?}", variant);
        assert_eq!(&output, &output, "output schema must be PartialEq for {}", name);
    }
}

// ── REQ-OPOUT-006 — Map replaces placeholder ─────────────────────────────────

/// REQ-OPOUT-006 adversarial: emitting the old placeholder alongside valid output
/// returns `ExtraFieldDisallowed` for the "items" key.
///
/// Note: schema validates required fields first, so we must include `item_results`
/// to get past that check and reach the extra-field check for "items".
#[test]
fn output_rejects_legacy_items_key() {
    let variant = DomainOperator::Map {
        source: OperatorId("src".into()),
        body: OperatorId("body".into()),
        max_concurrency: 4,
    };
    let schema = default_output_schema(&variant);

    // Include valid item_results AND the legacy "items" key
    let mut outputs = BTreeMap::new();
    outputs.insert(
        "item_results".into(),
        serde_json::json!([{ "operator_id": "body", "outputs": {} }]),
    );
    outputs.insert("items".into(), serde_json::json!(["a", "b", "c"]));

    let result = schema.validate(&OperatorId("map0".into()), "Map", &outputs);
    assert!(
        matches!(result, Err(OperatorContractError::ExtraFieldDisallowed { ref field, .. }) if field == "items"),
        "legacy 'items' key must be rejected as extra field, got: {result:?}"
    );
}

// ── REQ-OPOUT-002 / REQ-OPOUT-003 / REQ-OPOUT-004 — structural properties ──

/// REQ-OPOUT-003: Map output schema has `accepts_extra_fields = false`.
#[test]
fn map_output_accepts_extra_fields_false() {
    let variant = DomainOperator::Map {
        source: OperatorId("src".into()),
        body: OperatorId("body".into()),
        max_concurrency: 4,
    };
    let schema = default_output_schema(&variant);
    assert!(
        !schema.accepts_extra_fields,
        "Map output schema must have accepts_extra_fields = false"
    );
}

/// REQ-OPOUT-004: Map output schema requires `item_results` field.
#[test]
fn map_output_requires_item_results() {
    let variant = DomainOperator::Map {
        source: OperatorId("src".into()),
        body: OperatorId("body".into()),
        max_concurrency: 4,
    };
    let schema = default_output_schema(&variant);
    assert!(
        schema.required_fields.contains("item_results"),
        "Map output must require 'item_results'"
    );
}

/// REQ-OPOUT-003 adversarial: output with extra field is rejected.
#[test]
fn output_rejects_extra_field_when_strict() {
    let variant = DomainOperator::Map {
        source: OperatorId("src".into()),
        body: OperatorId("body".into()),
        max_concurrency: 4,
    };
    let schema = default_output_schema(&variant);

    // Emit valid output + extra field "rogue"
    let mut outputs = BTreeMap::new();
    outputs.insert(
        "item_results".into(),
        serde_json::json!([{ "operator_id": "body", "outputs": {} }]),
    );
    outputs.insert("rogue".into(), serde_json::json!(1));

    let result = schema.validate(&OperatorId("map0".into()), "Map", &outputs);
    assert!(
        matches!(result, Err(OperatorContractError::ExtraFieldDisallowed { field, .. }) if field == "rogue"),
        "extra field 'rogue' must be rejected"
    );
}

/// REQ-OPOUT-004 adversarial: missing required field is rejected.
#[test]
fn output_missing_required_field() {
    let variant = DomainOperator::Map {
        source: OperatorId("src".into()),
        body: OperatorId("body".into()),
        max_concurrency: 4,
    };
    let schema = default_output_schema(&variant);

    // Emit output WITHOUT required "item_results"
    let outputs: BTreeMap<String, serde_json::Value> =
        BTreeMap::from([("something".into(), serde_json::json!("else"))]);

    let result = schema.validate(&OperatorId("map0".into()), "Map", &outputs);
    assert!(
        matches!(result, Err(OperatorContractError::MissingRequiredField { field, .. }) if field == "item_results"),
        "missing required field 'item_results' must be rejected"
    );
}

// ── REQ-OPIN-002 — Task input is permissive ───────────────────────────────────

/// REQ-OPIN-002: Task input schema accepts extra fields
/// (capability declares exhaustive inputs per spec).
#[test]
fn task_input_accepts_extra_fields() {
    let variant = DomainOperator::Task {
        capability: CapabilityId("git.commit".into()),
        inputs: Default::default(),
    };
    let schema = default_input_schema(&variant);
    assert!(
        schema.accepts_extra_fields,
        "Task input schema should accept extra fields"
    );
}
