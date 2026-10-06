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
    let results = evaluate_all(&registry, &baseline, "2026-08-13T12:00:00Z", None);
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
    let results = evaluate_all(&registry, &baseline, "2026-08-13T12:00:00Z", None);
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
    let results = evaluate_all(&registry, &baseline, "2026-08-13T12:00:00Z", None);
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
        .join("../../docs/history/legacy-packages/sddk-2.0-architecture-consolidation/data/architecture-rules.yaml");
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
        .join("../../docs/history/legacy-packages/sddk-2.0-architecture-consolidation/data/architecture-rules.yaml");
    let yaml = std::fs::read_to_string(&yaml_path).expect("shipped YAML must be readable");
    let registry =
        sddk_domain::RuleRegistry::from_yaml_str(&yaml).expect("shipped YAML must parse");

    let baseline_path = dirs::data_dir()
        .unwrap_or_else(|| PathBuf::from("~/.local/share"))
        .join("sddk/projects/p-52b95ef55999f9de/cycle-artifacts/p-52b95ef55999f9de/sddk-2-0-phase0-baseline/baseline-dependency-entropy.json");
    let consumer = BaselineConsumer::new(&baseline_path, &["1.0.0", "1.1.0"])
        .expect("baseline consumer must be created");
    let baseline = consumer.load().expect("baseline must load");

    let results = evaluate_all(&registry, &baseline, "2026-08-13T12:00:00Z", None);
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
    let results = evaluate_all(&registry, &baseline, "2026-08-16T00:00:00Z", None);
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
    let results = evaluate_all(&registry, &baseline, "2026-08-16T00:00:00Z", None);
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
    let results_fail = evaluate_all(&registry, &baseline_fail, "2026-08-16T00:00:00Z", None);
    assert_eq!(results_fail[0].status, RuleStatus::Fail);

    // Without: Pass
    let baseline_pass = make_baseline(vec![]);
    let results_pass = evaluate_all(&registry, &baseline_pass, "2026-08-16T00:00:00Z", None);
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
    let results = evaluate_all(&registry, &baseline, "2026-08-16T00:00:00Z", None);
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
    let results = evaluate_all(&registry, &baseline, "2026-08-16T00:00:00Z", None);
    let r = &results[0];
    assert_eq!(
        r.status,
        RuleStatus::Waived,
        "valid waiver should supersede Fail"
    );
    assert_eq!(r.waiver_id.as_deref(), Some("WV-0001"));
}

// ── INC-DEBT-018: git-ancestry waiver expiry resolver ─────────────────────────

mod resolver {
    use super::*;
    use sddk_engine::rules::{evaluate_all_with_resolver, git_ancestry_resolver};
    use std::sync::Arc;

    const YAML: &str = r#"schema_version: 1.2.0
rules:
  - id: ARCH001
    severity: error
    rule: engine_must_not_depend_on_storage
    target: dependency_graph
waivers:
  - id: WV-TEST
    rule_id: ARCH001
    reason: "resolver semantics test"
    granted_until_sha: "%SHA%"
    granted_by: "test"
    granted_at: "2026-09-12T00:00:00Z"
"#;

    fn registry_with_until(until: &str) -> sddk_domain::RuleRegistry {
        let yaml = YAML.replace("%SHA%", until);
        sddk_domain::RuleRegistry::from_yaml_str(&yaml).expect("parse succeeds")
    }

    fn status_with(until: &str, resolver: sddk_domain::WaiverExpiryResolver) -> RuleStatus {
        let registry = registry_with_until(until);
        let baseline = make_baseline(vec![("crates/sddk-engine/src/lib.rs", 23, "sddk-storage")]);
        let results = evaluate_all_with_resolver(&registry, &baseline, "t", resolver, None);
        results[0].status
    }

    #[test]
    fn lexicographic_fallback_preserves_legacy_behavior() {
        let resolver: sddk_domain::WaiverExpiryResolver =
            Arc::new(|head: &str, until: &str| head <= until);
        // head "1dd72d0" > "00001111" → expired
        assert_eq!(
            status_with("00001111", resolver.clone()),
            RuleStatus::NotApplicable
        );
        // head "1dd72d0" <= "fffffffff" → active
        assert_eq!(status_with("fffffffff", resolver), RuleStatus::Waived);
    }

    #[test]
    fn sentinel_never_expires_regardless_of_resolver() {
        // Even a resolver that says "always expired" must honor the sentinel.
        let resolver: sddk_domain::WaiverExpiryResolver = Arc::new(|_h, _u| false);
        assert_eq!(
            status_with("9999999999999999999999999999999999999999", resolver),
            RuleStatus::Waived
        );
    }

    #[test]
    fn ancestry_resolver_falls_back_when_sha_unknown() {
        // Neither anchor is a git object → lexicographic fallback: head
        // "1dd72d0" vs "fffffffff" → lex active (Waived), vs "0000000" → expired.
        let resolver = git_ancestry_resolver(&PathBuf::from("."));
        assert_eq!(
            status_with("fffffffff", resolver.clone()),
            RuleStatus::Waived
        );
        assert_eq!(status_with("0000000", resolver), RuleStatus::NotApplicable);
    }

    #[test]
    fn ancestry_resolver_treats_descendant_head_as_active() {
        // This test file runs inside the sddk-framework repo; HEAD is a
        // descendant of the initial commit. Use the initial commit as the
        // granted SHA: ancestry says ACTIVE, lexicographic said expired.
        let root = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../..");
        let first = std::process::Command::new("git")
            .args(["rev-list", "--max-parents=0", "HEAD"])
            .current_dir(&root)
            .output()
            .expect("git available");
        let first_sha = String::from_utf8_lossy(&first.stdout).trim().to_owned();
        assert!(!first_sha.is_empty(), "need root commit");
        let resolver = git_ancestry_resolver(&root);
        assert_eq!(status_with(&first_sha, resolver), RuleStatus::Waived);
    }
}

// ── ARCH004 / ARCH005: falsadores de las dos leyes que eran stubs ─────────────
//
// El defecto que estos tests cierran: los dos evaluadores anteriores devolvian
// `NotApplicable` con un motivo FALSO ("no hay pack", "el runtime reactivo no
// ha llegado"), y un `NotApplicable` no es un verde — pero tampoco es una
// medida. Estos tests comprueban las dos propiedades que separan "miré y no
// hay" de "no miré":
//
//   1. cuando hay violacion, la ley CAE (direccionality);
//   2. cuando no hay arbol, la ley NO se declara conforme (fail-closed).
//
// Un test que solo comprobara (1) pasaria con el stub viejo, porque el stub
// nunca cae: por eso (2) es la mitad del contrato.

mod tree_reading_rules {
    use super::*;
    use std::path::Path;

    const CATALOG: &str = r#"schema_version: 1.2.0
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

    fn write(path: &Path, body: &str) {
        std::fs::create_dir_all(path.parent().expect("has parent")).expect("mkdir");
        std::fs::write(path, body).expect("write");
    }

    fn verdict(rule_id: &str, root: Option<&Path>) -> sddk_domain::RuleEvaluation {
        let registry = sddk_domain::RuleRegistry::from_yaml_str(CATALOG).expect("catalog parses");
        let baseline = make_baseline(vec![]);
        let results = evaluate_all(&registry, &baseline, "t", root);
        results
            .into_iter()
            .find(|r| r.rule_id == rule_id)
            .unwrap_or_else(|| panic!("{rule_id} missing from the evaluation"))
    }

    // ── ARCH004 ─────────────────────────────────────────────────────────────

    #[test]
    fn arch004_fails_on_a_declaration_naming_a_crate_that_does_not_exist() {
        let tmp = tempfile::tempdir().expect("tempdir");
        let root = tmp.path();
        write(
            &root.join("crates/sddk-domain/Cargo.toml"),
            "[package]\nname = \"sddk-domain\"\n",
        );
        write(
            &root.join("packs/demo/manifest.toml"),
            "[pack]\nid = \"demo\"\n\n[dependencies]\nrequires = [\"sddk-ghost\"]\n",
        );

        let v = verdict("ARCH004", Some(root));
        assert_eq!(
            v.status,
            RuleStatus::Fail,
            "a manifest naming sddk-ghost, which no crate provides, is a violation"
        );
        assert_eq!(v.observed["count"], 1);
        assert_eq!(v.observed["violations"][0]["kind"], "dangling_declaration");
    }

    #[test]
    fn arch004_fails_on_a_real_dependency_the_manifest_never_names() {
        let tmp = tempfile::tempdir().expect("tempdir");
        let root = tmp.path();
        write(
            &root.join("crates/sddk-domain/Cargo.toml"),
            "[package]\nname = \"sddk-domain\"\n",
        );
        write(
            &root.join("crates/demo/Cargo.toml"),
            "[package]\nname = \"demo\"\n\n[dependencies]\nsddk-domain = { path = \"../sddk-domain\" }\n",
        );
        // The manifest declares nothing at all.
        write(
            &root.join("packs/demo/manifest.toml"),
            "[pack]\nid = \"demo\"\n\n[dependencies]\nrequires = []\n",
        );

        let v = verdict("ARCH004", Some(root));
        assert_eq!(
            v.status,
            RuleStatus::Fail,
            "the crate depends on sddk-domain and [dependencies] omits it"
        );
        assert_eq!(v.observed["violations"][0]["kind"], "undeclared_dependency");
        assert_eq!(v.observed["violations"][0]["actual"], "sddk-domain");
    }

    #[test]
    fn arch004_passes_when_the_manifest_and_the_crate_agree() {
        let tmp = tempfile::tempdir().expect("tempdir");
        let root = tmp.path();
        write(
            &root.join("crates/sddk-domain/Cargo.toml"),
            "[package]\nname = \"sddk-domain\"\n",
        );
        write(
            &root.join("crates/demo/Cargo.toml"),
            "[package]\nname = \"demo\"\n\n[dependencies]\nsddk-domain = { path = \"../sddk-domain\" }\n",
        );
        write(
            &root.join("packs/demo/manifest.toml"),
            "[pack]\nid = \"demo\"\n\n[dependencies]\nrequires = [\"sddk-domain\"]\n",
        );

        let v = verdict("ARCH004", Some(root));
        assert_eq!(
            v.status,
            RuleStatus::Pass,
            "declared and actual are the same set"
        );
        assert_eq!(v.observed["count"], 0);
    }

    #[test]
    fn arch004_does_not_flag_an_optional_capability_that_is_absent() {
        let tmp = tempfile::tempdir().expect("tempdir");
        let root = tmp.path();
        write(
            &root.join("crates/sddk-domain/Cargo.toml"),
            "[package]\nname = 'sddk-domain'\n",
        );
        write(
            &root.join("crates/demo/Cargo.toml"),
            "[package]\nname = 'demo'\n\n[dependencies]\nsddk-domain = { path = '../sddk-domain' }\n",
        );
        // `integrates_with` names an OPTIONAL capability, and the contract says
        // its absence degrades gracefully. A rule that failed here would be
        // failing a pack for using the field the way the field is defined.
        write(
            &root.join("packs/demo/manifest.toml"),
            "[pack]\nid = 'demo'\n\n[dependencies]\nrequires = ['sddk-domain']\nintegrates_with = ['optional-bridge']\n",
        );

        let v = verdict("ARCH004", Some(root));
        assert_eq!(
            v.status,
            RuleStatus::Pass,
            "an absent optional capability is the field working, not a violation: {:?}",
            v.observed["violations"]
        );
    }

    #[test]
    fn arch004_treats_an_unreadable_manifest_as_a_violation_not_as_absence() {
        let tmp = tempfile::tempdir().expect("tempdir");
        let root = tmp.path();
        write(
            &root.join("packs/demo/manifest.toml"),
            "[dependencies\nrequires = broken\n",
        );

        let v = verdict("ARCH004", Some(root));
        assert_eq!(
            v.status,
            RuleStatus::Fail,
            "a manifest that cannot be parsed must not read as 'declared nothing, therefore clean'"
        );
        assert_eq!(v.observed["violations"][0]["kind"], "unreadable_manifest");
    }

    // ── ARCH005 ─────────────────────────────────────────────────────────────

    #[test]
    fn arch005_fails_when_a_reactive_module_writes_directly() {
        let tmp = tempfile::tempdir().expect("tempdir");
        let root = tmp.path();
        write(
            &root.join("crates/sddk-engine/src/reactive_verify.rs"),
            "pub fn go() { std::fs::write(\"/tmp/x\", b\"y\").unwrap(); }\n",
        );

        let v = verdict("ARCH005", Some(root));
        assert_eq!(
            v.status,
            RuleStatus::Fail,
            "a governed effect executed straight from a reactive behavior is the violation"
        );
        assert_eq!(v.observed["count"], 1);
        assert_eq!(v.observed["violations"][0]["effect"], "durable_write");
    }

    #[test]
    fn arch005_fails_when_a_reactive_module_reaches_an_adapter() {
        let tmp = tempfile::tempdir().expect("tempdir");
        let root = tmp.path();
        write(
            &root.join("crates/sddk-engine/src/reactive.rs"),
            "use sddk_storage::SqliteXStore;\n",
        );

        let v = verdict("ARCH005", Some(root));
        assert_eq!(v.status, RuleStatus::Fail);
        assert_eq!(v.observed["violations"][0]["effect"], "storage_adapter");
    }

    #[test]
    fn arch005_passes_on_a_pure_reactive_module() {
        let tmp = tempfile::tempdir().expect("tempdir");
        let root = tmp.path();
        write(
            &root.join("crates/sddk-engine/src/reactive_verify.rs"),
            "pub fn decide() -> bool { let v = std::fs::read_to_string(\"/x\").ok(); v.is_some() }\n",
        );

        let v = verdict("ARCH005", Some(root));
        assert_eq!(
            v.status,
            RuleStatus::Pass,
            "reading the tree is `Read`, not a governed effect"
        );
        assert_eq!(v.observed["count"], 0);
        assert_eq!(
            v.observed["subjects"][0]["file"],
            "crates/sddk-engine/src/reactive_verify.rs"
        );
    }

    #[test]
    fn arch005_does_not_flag_a_module_documenting_the_rule_it_obeys() {
        let tmp = tempfile::tempdir().expect("tempdir");
        let root = tmp.path();
        write(
            &root.join("crates/sddk-engine/src/reactive_verify.rs"),
            "// This module must never call fs::write or Command::new itself.\n\
             //! Governed effects go through the capability layer.\n\
             pub fn decide() -> bool { true }\n",
        );

        let v = verdict("ARCH005", Some(root));
        assert_eq!(
            v.status,
            RuleStatus::Pass,
            "a module stating the constraint in its own docs is not violating it"
        );
    }

    #[test]
    fn arch005_without_a_subject_is_measured_but_never_conformant() {
        let tmp = tempfile::tempdir().expect("tempdir");
        let v = verdict("ARCH005", Some(tmp.path()));

        assert_ne!(
            v.status,
            RuleStatus::Pass,
            "no subject means the law says nothing, which is not a conformance claim"
        );
        assert_eq!(
            v.observed["measured"], true,
            "it looked, and there was nothing to look at"
        );
    }

    // ── La mitad fail-closed: sin arbol, NINGUNA de las dos se declara verde ──

    #[test]
    fn no_repository_root_is_reported_as_not_measured_never_as_a_pass() {
        for rule_id in ["ARCH004", "ARCH005"] {
            let v = verdict(rule_id, None);
            assert_ne!(
                v.status,
                RuleStatus::Pass,
                "{rule_id} with no root must not claim a clean measurement"
            );
            assert_eq!(v.status, RuleStatus::NotApplicable);
            assert_eq!(v.observed["measured"], false);
            let provenance = v.provenance.expect("provenance states what was not done");
            assert!(
                provenance.contains("NOT MEASURED"),
                "{rule_id} provenance must say the measurement was skipped, got: {provenance}"
            );
        }
    }

    #[test]
    fn the_summary_names_the_violation_kind_rather_than_borrowing_another_rules_noun() {
        let tmp = tempfile::tempdir().expect("tempdir");
        let root = tmp.path();
        write(
            &root.join("packs/demo/manifest.toml"),
            "[dependencies]\nrequires = [\"sddk-ghost\"]\n",
        );

        let v = verdict("ARCH004", Some(root));
        let summary = v.observed["summary"].as_str().expect("summary present");
        assert!(
            summary.contains("dangling"),
            "the rendered detail must name what it found, got: {summary}"
        );
        assert!(
            !summary.contains("edge"),
            "'edge' is ARCH001's noun; a manifest rule borrowing it hides what it measured: {summary}"
        );
    }
}
