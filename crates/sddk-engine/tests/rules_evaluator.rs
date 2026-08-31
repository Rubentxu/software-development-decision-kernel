//! Integration tests for the baseline consumer + stub evaluator.

use sddk_domain::{BaselineRef, RuleStatus};
use sddk_engine::rules::{
    Baseline, BaselineConsumer, BaselineError, CrossCrateImport, CrossCrateImportKind, evaluate_all,
};
use std::path::PathBuf;

fn make_baseline(imports: Vec<(&str, u32, &str)>) -> Baseline {
    let cross_crate_imports = imports
        .into_iter()
        .map(|(from_file, line, to_crate)| {
            let parts: Vec<&str> = from_file.split('/').collect();
            let from_crate = if parts.len() >= 2 && parts[0] == "crates" {
                parts[1].to_owned()
            } else {
                "unknown".to_owned()
            };
            let to_crate = if to_crate.starts_with("sddk-") {
                to_crate.to_owned()
            } else {
                format!("sddk-{}", to_crate)
            };
            CrossCrateImport {
                from_file: from_file.to_owned(),
                line,
                from_crate,
                to_crate_raw: to_crate.to_owned(),
                to_crate,
                kind: CrossCrateImportKind::Use,
            }
        })
        .collect();
    Baseline {
        ref_: BaselineRef {
            schema_version: "1.0.0".to_owned(),
            head_anchor: "1dd72d0".to_owned(),
            sha256: "sha256:test".to_owned(),
            cycle_id: None,
            captured_at: "2026-08-13T12:00:00Z".to_owned(),
        },
        cross_crate_imports,
    }
}

#[test]
fn baseline_consumer_rejects_unsupported_schema_version() {
    let json = r#"{"schema_version": "99.0.0", "head_anchor": "deadbeef", "captured_at": "2026-08-13T12:00:00Z", "cross_crate_coupling_baseline": {"cross_crate_imports": []}}"#;
    let tmp = tempfile::NamedTempFile::new().expect("tempfile");
    std::fs::write(tmp.path(), json).expect("write");
    let consumer = BaselineConsumer::new(tmp.path(), &["1.0.0"]).expect("constructor accepts");
    let err = consumer.load().expect_err("load should fail");
    match err {
        BaselineError::UnsupportedSchemaVersion { actual, .. } => assert_eq!(actual, "99.0.0"),
        other => panic!("expected UnsupportedSchemaVersion, got {other:?}"),
    }
}

#[test]
fn baseline_consumer_parses_and_normalizes_crates() {
    let json = r#"{"schema_version": "1.0.0", "head_anchor": "1dd72d0", "captured_at": "2026-08-13T12:00:00Z", "cross_crate_coupling_baseline": {"cross_crate_imports": [{"from_file": "crates/sddk-engine/src/lib.rs", "line": 23, "to_crate": "storage"}]}}"#;
    let tmp = tempfile::NamedTempFile::new().expect("tempfile");
    std::fs::write(tmp.path(), json).expect("write");
    let consumer = BaselineConsumer::new(tmp.path(), &["1.0.0"]).expect("constructor accepts");
    let baseline = consumer.load().expect("load should succeed");
    assert_eq!(baseline.ref_.schema_version, "1.0.0");
    assert_eq!(baseline.cross_crate_imports.len(), 1);
    let import = &baseline.cross_crate_imports[0];
    assert_eq!(import.from_crate, "sddk-engine");
    assert_eq!(import.to_crate, "sddk-storage");
    assert_eq!(import.to_crate_raw, "storage");
}

#[test]
fn evaluate_all_returns_not_applicable_for_all_rules() {
    let yaml = r#"schema_version: 1.2.0
rules:
  - id: ARCH001
    severity: error
    rule: engine_must_not_depend_on_storage
    target: dependency_graph
  - id: ARCH004
    severity: error
    rule: packs_must_declare_dependencies
    target: pack_manifest
"#;
    let registry = sddk_domain::RuleRegistry::from_yaml_str(yaml).expect("parse succeeds");
    let baseline = make_baseline(vec![]);
    let results = evaluate_all(&registry, &baseline, "2026-08-13T12:00:00Z");
    assert_eq!(results.len(), 2);
    // Phase 1: ARCH001 Pass (no violations), ARCH004 NotApplicable (kernel repo)
    let arch001 = results.iter().find(|r| r.rule_id == "ARCH001").unwrap();
    let arch004 = results.iter().find(|r| r.rule_id == "ARCH004").unwrap();
    assert_eq!(
        arch001.status,
        RuleStatus::Pass,
        "ARCH001 with empty baseline should Pass"
    );
    assert_eq!(
        arch004.status,
        RuleStatus::NotApplicable,
        "ARCH004 should be NotApplicable"
    );
    assert!(arch004.provenance.is_some(), "ARCH004 needs provenance");
}

#[test]
fn evaluate_all_applies_waiver_when_head_anchor_within_granted_sha() {
    let yaml = r#"schema_version: 1.2.0
rules:
  - id: ARCH001
    severity: error
    rule: x
    target: dependency_graph
waivers:
  - id: WV-0001
    rule_id: ARCH001
    reason: "transitive dep in flight"
    granted_until_sha: "1dd72d0"
    granted_by: "reviewer"
    granted_at: "2026-08-13T12:00:00Z"
"#;
    let registry = sddk_domain::RuleRegistry::from_yaml_str(yaml).expect("parse succeeds");
    let baseline = make_baseline(vec![]); // head_anchor = "1dd72d0"
    let results = evaluate_all(&registry, &baseline, "2026-08-13T12:00:00Z");
    assert_eq!(results.len(), 1);
    let r = &results[0];
    assert_eq!(r.status, RuleStatus::Waived);
    assert_eq!(r.waiver_id.as_deref(), Some("WV-0001"));
}

#[test]
fn evaluate_all_returns_not_applicable_when_waiver_expired() {
    let yaml = r#"schema_version: 1.2.0
rules:
  - id: ARCH001
    severity: error
    rule: x
    target: dependency_graph
waivers:
  - id: WV-0001
    rule_id: ARCH001
    reason: "old waiver"
    granted_until_sha: "00001111"
    granted_by: "reviewer"
    granted_at: "2026-08-13T12:00:00Z"
"#;
    let registry = sddk_domain::RuleRegistry::from_yaml_str(yaml).expect("parse succeeds");
    let baseline = make_baseline(vec![]); // head_anchor = "1dd72d0" > "00001111"
    let results = evaluate_all(&registry, &baseline, "2026-08-13T12:00:00Z");
    assert_eq!(results.len(), 1);
    let r = &results[0];
    assert_eq!(r.status, RuleStatus::NotApplicable);
    assert!(r.waiver_id.is_none());
    assert!(r.provenance.as_ref().unwrap().contains("expired"));
}

#[test]
fn shipped_catalog_parses_with_fifteen_rules() {
    // Phase 2: shipped architecture-rules.yaml now includes ARCH001..ARCH015 (10 rules).
    let yaml_path = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../../docs/sddk-2.0-architecture-consolidation/data/architecture-rules.yaml");
    let yaml = std::fs::read_to_string(&yaml_path).expect("shipped YAML must be readable");
    let registry =
        sddk_domain::RuleRegistry::from_yaml_str(&yaml).expect("shipped YAML must parse");
    let ids: Vec<&str> = registry.iter().map(|r| r.id.as_str()).collect();
    assert_eq!(
        ids,
        vec![
            "ARCH001", "ARCH002", "ARCH003", "ARCH004", "ARCH005", "ARCH006", "ARCH007", "ARCH008",
            "ARCH009", "ARCH010", "ARCH011", "ARCH012", "ARCH013", "ARCH014", "ARCH015",
        ]
    );
}

#[test]
fn shipped_catalog_against_baseline_produces_fifteen_evaluations() {
    // Phase 2: shipped YAML + Phase 0 baseline produces 15 evaluations:
    // ARCH001 Fail (engine→storage edges exist), ARCH002 Pass (domain clean),
    // ARCH003 Waived (WV-0015), ARCH004/005 NotApplicable, ARCH008 Pass (WV-0026 waiver).
    let yaml_path = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../../docs/sddk-2.0-architecture-consolidation/data/architecture-rules.yaml");
    let yaml = std::fs::read_to_string(&yaml_path).expect("shipped YAML must be readable");
    let registry =
        sddk_domain::RuleRegistry::from_yaml_str(&yaml).expect("shipped YAML must parse");

    let baseline_path = dirs::data_dir()
        .unwrap_or_else(|| PathBuf::from("~/.local/share"))
        .join("sddk/projects/p-52b95ef55999f9de/cycle-artifacts/p-52b95ef55999f9de/sddk-2-0-phase0-baseline/baseline-dependency-entropy.json");
    let consumer = BaselineConsumer::new(&baseline_path, &["1.0.0", "1.1.0"])
        .expect("baseline consumer must be created");
    let baseline = consumer.load().expect("baseline must load");

    let results = evaluate_all(&registry, &baseline, "2026-08-13T12:00:00Z");
    assert_eq!(
        results.len(),
        15,
        "shipped catalog must produce 15 evaluations"
    );

    let arch001 = results.iter().find(|r| r.rule_id == "ARCH001").unwrap();
    let arch002 = results.iter().find(|r| r.rule_id == "ARCH002").unwrap();
    let arch003 = results.iter().find(|r| r.rule_id == "ARCH003").unwrap();
    let arch004 = results.iter().find(|r| r.rule_id == "ARCH004").unwrap();
    let arch005 = results.iter().find(|r| r.rule_id == "ARCH005").unwrap();

    assert_eq!(
        arch001.status,
        RuleStatus::Fail,
        "ARCH001 should Fail (engine→storage exists)"
    );
    assert_eq!(
        arch002.status,
        RuleStatus::Pass,
        "ARCH002 should Pass (domain clean)"
    );
    assert_eq!(
        arch003.status,
        RuleStatus::Waived,
        "ARCH003 should be Waived (WV-0015 composition-root waiver active in shipped catalog; see ADR-0015)"
    );
    assert_eq!(
        arch003.waiver_id.as_deref(),
        Some("WV-0015-ARCH003-composition-root"),
        "ARCH003 waiver_id should point at WV-0015"
    );
    assert_eq!(
        arch004.status,
        RuleStatus::NotApplicable,
        "ARCH004 should be N/A"
    );
    assert_eq!(
        arch005.status,
        RuleStatus::NotApplicable,
        "ARCH005 should be N/A"
    );
    assert!(arch004.provenance.is_some(), "ARCH004 needs provenance");
    assert!(arch005.provenance.is_some(), "ARCH005 needs provenance");
}

// ── Phase 1 evaluator tests ───────────────────────────────────────────────────

/// ARCH001 fails when a baseline contains an engine→storage edge.
#[test]
fn arch001_fails_when_engine_depends_on_storage() {
    let yaml = r#"schema_version: 1.2.0
rules:
  - id: ARCH001
    severity: error
    rule: engine_must_not_depend_on_storage
    target: dependency_graph
"#;
    let registry = sddk_domain::RuleRegistry::from_yaml_str(yaml).expect("parse succeeds");
    let baseline = make_baseline(vec![("crates/sddk-engine/src/lib.rs", 23, "sddk-storage")]);
    let results = evaluate_all(&registry, &baseline, "2026-08-16T00:00:00Z");
    assert_eq!(results.len(), 1);
    let r = &results[0];
    assert_eq!(r.status, RuleStatus::Fail);
    let edges = r.observed.get("edges").unwrap().as_array().unwrap();
    assert!(!edges.is_empty(), "Fail must include violating edges");
}

/// ARCH002 passes when the baseline shows no domain→adapters edges.
#[test]
fn arch002_passes_when_domain_isolated() {
    let yaml = r#"schema_version: 1.2.0
rules:
  - id: ARCH002
    severity: error
    rule: domain_must_not_depend_on_adapters
    target: dependency_graph
"#;
    let registry = sddk_domain::RuleRegistry::from_yaml_str(yaml).expect("parse succeeds");
    let baseline = make_baseline(vec![]);
    let results = evaluate_all(&registry, &baseline, "2026-08-16T00:00:00Z");
    let r = &results[0];
    assert_eq!(r.status, RuleStatus::Pass);
}

/// ARCH003 reports Fail when a cli→storage edge exists; Pass otherwise.
#[test]
fn arch003_reports_imports_from_cli() {
    let yaml = r#"schema_version: 1.2.0
rules:
  - id: ARCH003
    severity: error
    rule: cli_must_not_own_persistence_logic
    target: source_imports_and_calls
"#;
    let registry = sddk_domain::RuleRegistry::from_yaml_str(yaml).expect("parse succeeds");

    // With cli→storage edge: Fail
    let baseline_fail = make_baseline(vec![("crates/sddk-cli/src/cycle.rs", 13, "sddk-storage")]);
    let results_fail = evaluate_all(&registry, &baseline_fail, "2026-08-16T00:00:00Z");
    assert_eq!(results_fail[0].status, RuleStatus::Fail);

    // Without: Pass
    let baseline_pass = make_baseline(vec![]);
    let results_pass = evaluate_all(&registry, &baseline_pass, "2026-08-16T00:00:00Z");
    assert_eq!(results_pass[0].status, RuleStatus::Pass);
}

/// ARCH004 and ARCH005 always return NotApplicable with a non-empty provenance.
#[test]
fn arch004_and_arch005_return_not_applicable() {
    let yaml = r#"schema_version: 1.2.0
rules:
  - id: ARCH004
    severity: error
    rule: packs_must_declare_dependencies
    target: pack_manifest
  - id: ARCH005
    severity: error
    rule: reactive_behaviors_must_not_execute_governed_effects_directly
    target: capability_imports
"#;
    let registry = sddk_domain::RuleRegistry::from_yaml_str(yaml).expect("parse succeeds");
    let baseline = make_baseline(vec![]);
    let results = evaluate_all(&registry, &baseline, "2026-08-16T00:00:00Z");
    assert_eq!(results.len(), 2);
    for r in &results {
        assert_eq!(r.status, RuleStatus::NotApplicable);
        assert!(
            r.provenance
                .as_ref()
                .map(|p| !p.is_empty())
                .unwrap_or(false)
        );
    }
}

/// A valid waiver (head_anchor <= granted_until_sha) supersedes a Fail.
#[test]
fn waiver_with_valid_until_supersedes_fail() {
    let yaml = r#"schema_version: 1.2.0
rules:
  - id: ARCH001
    severity: error
    rule: engine_must_not_depend_on_storage
    target: dependency_graph
waivers:
  - id: WV-0001
    rule_id: ARCH001
    reason: "orchestration dep in flight"
    granted_until_sha: "fffffffff"
    granted_by: "arch-reviewer"
    granted_at: "2026-08-16T00:00:00Z"
"#;
    let registry = sddk_domain::RuleRegistry::from_yaml_str(yaml).expect("parse succeeds");
    // Baseline head_anchor "1dd72d0" <= "fffffffff" → Waived (not Fail)
    let baseline = make_baseline(vec![("crates/sddk-engine/src/lib.rs", 23, "sddk-storage")]);
    let results = evaluate_all(&registry, &baseline, "2026-08-16T00:00:00Z");
    let r = &results[0];
    assert_eq!(
        r.status,
        RuleStatus::Waived,
        "valid waiver should supersede Fail"
    );
    assert_eq!(r.waiver_id.as_deref(), Some("WV-0001"));
}
