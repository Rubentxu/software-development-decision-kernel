//! Spine parsing tests.
//!
//! Tests AC-PLN3-05 and AC-PLN3-06.

use sddk_domain::spine::{
    ExecutionSpineV1, SpineParseError, SpineStatus, canonicalize_spine_bytes, parse_spine_yaml,
};

/// Scenario: Current EXECUTION-SPINE.yaml parses cleanly
#[test]
fn spine_parses_current_execution_spine_file() {
    let manifest_dir = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let spine_path = manifest_dir
        .join("../../docs/sddk-decision-kernel-architecture/02-roadmap/EXECUTION-SPINE.yaml");
    let bytes = std::fs::read(&spine_path).unwrap_or_else(|_| {
        // Fallback: try relative path from CWD
        std::fs::read("docs/sddk-decision-kernel-architecture/02-roadmap/EXECUTION-SPINE.yaml")
            .unwrap()
    });

    let spine = parse_spine_yaml(&bytes).expect("EXECUTION-SPINE.yaml should parse");
    assert_eq!(spine.schema_version, 2);
    assert!(!spine.items.is_empty(), "spine should have items");
    assert_eq!(spine.cycle_binding.identity, "semantic_work_item_id");
    assert_eq!(spine.cycle_binding.execution_instance, "cycle_or_run_id");
}

/// Scenario: `status: SHIPPED # reconciled ...` is parsed as `Shipped`
#[test]
fn spine_status_comment_tolerance() {
    // A minimal spine YAML fragment with a trailing comment on status
    let yaml = r#"
schema_version: 2
plan_id: test-plan
cycle_binding:
  identity: semantic_work_item_id
  execution_instance: cycle_or_run_id
items:
  - order: 1
    id: TEST-001
    horizon: H0
    status: SHIPPED # reconciled at 2026-09-05
    depends_on: []
    objective: Test objective
    exit_gate: Test gate
"#;
    let spine = parse_spine_yaml(yaml.as_bytes()).expect("should parse with comment");
    assert_eq!(spine.items.len(), 1);
    assert_eq!(spine.items[0].status, SpineStatus::Shipped);
}

/// Scenario: All eight YAML statuses parse to the right enum variant
#[test]
fn spine_all_eight_statuses_parse() {
    for (yaml_status, expected) in [
        ("PROPOSED", SpineStatus::Proposed),
        ("READY", SpineStatus::Ready),
        ("ACTIVE", SpineStatus::Active),
        ("PARTIAL", SpineStatus::Partial),
        ("BLOCKED", SpineStatus::Blocked),
        ("SHIPPED", SpineStatus::Shipped),
        ("ABSORBED", SpineStatus::Absorbed),
        ("SUPERSEDED", SpineStatus::Superseded),
    ] {
        let yaml = format!(
            r#"
schema_version: 2
plan_id: test
cycle_binding:
  identity: semantic_work_item_id
  execution_instance: cycle_or_run_id
items:
  - order: 1
    id: TEST-001
    horizon: H0
    status: {}
    depends_on: []
    objective: Test
    exit_gate: Test
"#,
            yaml_status
        );
        let spine =
            parse_spine_yaml(yaml.as_bytes()).expect(&format!("{yaml_status} should parse"));
        assert_eq!(
            spine.items[0].status, expected,
            "status {yaml_status} should map to {:?}",
            expected
        );
    }
}

/// Scenario: Round-trip — parse → canonical serialize → parse yields identical spine
#[test]
fn spine_roundtrip_is_stable() {
    let manifest_dir = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let spine_path = manifest_dir
        .join("../../docs/sddk-decision-kernel-architecture/02-roadmap/EXECUTION-SPINE.yaml");
    let bytes = std::fs::read(&spine_path).unwrap_or_else(|_| {
        std::fs::read("docs/sddk-decision-kernel-architecture/02-roadmap/EXECUTION-SPINE.yaml")
            .unwrap()
    });

    let spine1 = parse_spine_yaml(&bytes).expect("first parse should succeed");
    let canonical = canonicalize_spine_bytes(&bytes);
    let spine2 = parse_spine_yaml(&canonical).expect("round-trip parse should succeed");

    // Items should be identical (sorted by id for determinism)
    assert_eq!(spine1.items.len(), spine2.items.len());
    for (a, b) in spine1.items.iter().zip(spine2.items.iter()) {
        assert_eq!(a.id, b.id, "item id should be stable");
        assert_eq!(a.status, b.status, "item status should be stable");
        assert_eq!(
            a.depends_on, b.depends_on,
            "item depends_on should be stable"
        );
    }
}

/// Scenario: `cycle_binding` block is preserved at top-level
#[test]
fn spine_cycle_binding_preserved() {
    let manifest_dir = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let spine_path = manifest_dir
        .join("../../docs/sddk-decision-kernel-architecture/02-roadmap/EXECUTION-SPINE.yaml");
    let bytes = std::fs::read(&spine_path).unwrap_or_else(|_| {
        std::fs::read("docs/sddk-decision-kernel-architecture/02-roadmap/EXECUTION-SPINE.yaml")
            .unwrap()
    });

    let spine = parse_spine_yaml(&bytes).expect("should parse");
    assert_eq!(spine.cycle_binding.identity, "semantic_work_item_id");
    assert_eq!(spine.cycle_binding.execution_instance, "cycle_or_run_id");
}

/// Scenario: Missing schema_version is rejected
#[test]
fn spine_missing_schema_version_rejected() {
    let yaml = r#"
plan_id: test-plan
items: []
"#;
    let result = parse_spine_yaml(yaml.as_bytes());
    assert!(result.is_err(), "missing schema_version should be rejected");
    let err = result.unwrap_err();
    assert_eq!(err.reason, "missing_schema_version");
    assert_eq!(err.line, 1);
    assert_eq!(err.column, 1);
}

/// Scenario: Unknown top-level field is rejected with line + column
#[test]
fn spine_unknown_field_rejected() {
    let yaml = r#"
schema_version: 2
bogus_field: true
plan_id: test
cycle_binding:
  identity: semantic_work_item_id
  execution_instance: cycle_or_run_id
items: []
"#;
    let result = parse_spine_yaml(yaml.as_bytes());
    assert!(result.is_err(), "unknown field should be rejected");
    let err = result.unwrap_err();
    assert!(
        err.reason.contains("unknown_field"),
        "should mention unknown_field: {}",
        err.reason
    );
}

/// Scenario: Empty items[] array is accepted
#[test]
fn spine_empty_items_accepted() {
    let yaml = r#"
schema_version: 2
plan_id: test
cycle_binding:
  identity: semantic_work_item_id
  execution_instance: cycle_or_run_id
items: []
"#;
    let spine = parse_spine_yaml(yaml.as_bytes()).expect("empty items should be accepted");
    assert_eq!(spine.items.len(), 0);
}
