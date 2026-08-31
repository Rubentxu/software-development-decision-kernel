//! Unit tests for the architecture-rule registry.

use sddk_domain::{
    ARCHITECTURE_RULES_SCHEMA_VERSION, EvaluatorKind, RuleRegistry, RuleSeverity, RuleStatus,
    RuleTarget,
};

const SAMPLE_YAML: &str = r#"schema_version: 1.2.0
rules:
  - id: ARCH001
    severity: error
    rule: engine_must_not_depend_on_storage
    target: dependency_graph
  - id: ARCH002
    severity: error
    rule: domain_must_not_depend_on_adapters
    target: dependency_graph
  - id: ARCH003
    severity: error
    rule: cli_must_not_own_persistence_logic
    target: source_imports_and_calls
  - id: ARCH004
    severity: error
    rule: packs_must_declare_dependencies
    target: pack_manifest
  - id: ARCH005
    severity: error
    rule: reactive_behaviors_must_not_execute_governed_effects_directly
    target: capability_imports
"#;

#[test]
fn parse_minimal_yaml() {
    let yaml = r#"schema_version: 1.2.0
rules:
  - id: ARCH001
    severity: error
    rule: engine_must_not_depend_on_storage
    target: dependency_graph
"#;
    let registry = RuleRegistry::from_yaml_str(yaml).expect("parse should succeed");
    let first = registry.iter().next().expect("at least one rule");
    assert_eq!(first.id, "ARCH001");
    assert_eq!(first.severity, RuleSeverity::Error);
    assert_eq!(first.target, RuleTarget::DependencyGraph);
}

#[test]
fn parse_five_rules() {
    let registry = RuleRegistry::from_yaml_str(SAMPLE_YAML).expect("parse should succeed");
    let ids: Vec<&str> = registry.iter().map(|r| r.id.as_str()).collect();
    assert_eq!(
        ids,
        vec!["ARCH001", "ARCH002", "ARCH003", "ARCH004", "ARCH005"]
    );
}

#[test]
fn parse_rejects_unsupported_schema_version() {
    let yaml = r#"schema_version: 99.0.0
rules: []
"#;
    let err = RuleRegistry::from_yaml_str(yaml).expect_err("unsupported version must fail");
    assert!(err.to_string().contains("99.0.0"));
}

#[test]
fn parse_rejects_duplicate_rule_id() {
    let yaml = r#"schema_version: 1.2.0
rules:
  - id: ARCH001
    severity: error
    rule: x
    target: dependency_graph
  - id: ARCH001
    severity: error
    rule: y
    target: dependency_graph
"#;
    let err = RuleRegistry::from_yaml_str(yaml).expect_err("duplicate must fail");
    assert!(err.to_string().contains("duplicate"));
}

#[test]
fn parse_rejects_missing_rule_id() {
    let yaml = r#"schema_version: 1.2.0
rules:
  - id: ""
    severity: error
    rule: x
    target: dependency_graph
"#;
    let err = RuleRegistry::from_yaml_str(yaml).expect_err("empty rule id must fail");
    assert!(err.to_string().contains("missing id") || err.to_string().contains("MissingRuleId"));
}

#[test]
fn waiver_lookup_returns_waiver() {
    let yaml = r#"schema_version: 1.2.0
rules:
  - id: ARCH001
    severity: error
    rule: x
    target: dependency_graph
waivers:
  - id: WV-0001
    rule_id: ARCH001
    reason: "transitive dep"
    granted_until_sha: "1dd72d0"
    granted_by: "reviewer"
    granted_at: "2026-08-13T12:00:00Z"
"#;
    let registry = RuleRegistry::from_yaml_str(yaml).expect("parse should succeed");
    let w = registry.waiver_for("ARCH001").expect("waiver present");
    assert_eq!(w.id, "WV-0001");
}

#[test]
fn waiver_lookup_returns_none_when_absent() {
    let registry = RuleRegistry::from_yaml_str(SAMPLE_YAML).expect("parse should succeed");
    assert!(registry.waiver_for("ARCH001").is_none());
}

#[test]
fn evaluation_serializes_without_cep_fields() {
    let evaluation = sddk_domain::RuleEvaluation {
        rule_id: "ARCH001".to_owned(),
        status: RuleStatus::Pass,
        observed: serde_json::json!({"violations": 0}),
        baseline_sha256: "sha256:deadbeef".to_owned(),
        evaluated_at: "2026-08-13T12:00:00Z".to_owned(),
        evaluated_by: "sddk-rules-cli@0.1.0".to_owned(),
        waiver_id: None,
        evaluator_kind: EvaluatorKind::Schema,
        evaluator_version: "0.1.0".to_owned(),
        provenance: None,
    };
    let json = serde_json::to_string(&evaluation).expect("serialize");
    for forbidden in ["event_id", "caused_by", "frame_id", "correlation_token"] {
        assert!(
            !json.contains(forbidden),
            "CEP field {forbidden} must not appear"
        );
    }
}

#[test]
fn schema_version_constant_is_stable() {
    assert_eq!(ARCHITECTURE_RULES_SCHEMA_VERSION, "1.2.0");
}
