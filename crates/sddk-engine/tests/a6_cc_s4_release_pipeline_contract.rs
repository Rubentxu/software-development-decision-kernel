//! CC-S4 / AIW-S6 — CoverageContract for a second real consumer:
//! `release-pipeline::static_evidence`.
//!
//! Validates that per-consumer contract versioning works end-to-end:
//! two consumers can hold *different* contracts (different required
//! capabilities, different versions) over the same scope without
//! interference, and each verdict is qualified by its own
//! `contract_id` + `contract_version` (N5/N6 of CC-S1).
//!
//! This is the acceptance-side demonstration only. It does NOT
//! declare STATIC_ENHANCED for the workspace, nor does it build
//! release-pipeline infrastructure.

use sddk_engine::code_intelligence_port::{
    CapabilityProfile, CapabilitySnapshot, CoverageBasis, CoverageContract, CoverageGap,
    CoverageVerdict, DimensionValue, Inventory, RequiredCapability, RequiredCapabilityKind,
    ScopeRef, default_coverage_evaluation,
};

const HEAD_REV: &str = "cc4-test-rev";

fn scope() -> ScopeRef {
    ScopeRef {
        repo: "sddk-framework".to_string(),
        revision: HEAD_REV.to_string(),
        include_globs: vec!["crates/**/src/**/*.rs".to_string()],
        exclude_globs: vec!["target/**".to_string()],
    }
}

fn req(kind: RequiredCapabilityKind, name: &str) -> RequiredCapability {
    RequiredCapability {
        kind,
        name: name.to_string(),
    }
}

/// The verify-kernel contract (CC-S1, v1.0.0): analysis-quality focus.
/// Requires full relation analysis: find_usages + analyze_impact
/// over fn/struct symbols.
fn verify_kernel_contract_v1_0() -> CoverageContract {
    CoverageContract {
        contract_id: "static-enhanced-workspace-v1".to_string(),
        contract_version: "1.0.0".to_string(),
        consumer: "verify-kernel::static_evidence".to_string(),
        scope: scope(),
        required_capabilities: vec![
            req(RequiredCapabilityKind::SymbolKind, "fn"),
            req(RequiredCapabilityKind::SymbolKind, "struct"),
            req(RequiredCapabilityKind::RelationClass, "find_usages"),
            req(RequiredCapabilityKind::RelationClass, "analyze_impact"),
        ],
    }
}

/// The release-pipeline contract (CC-S4, v1.1.0): release-integrity
/// focus. Same scope, different consumer, *different* required
/// capabilities — release-pipeline does not need impact analysis;
/// it needs symbol presence plus import_graph for bundle integrity
/// checks.
fn release_pipeline_contract_v1_1() -> CoverageContract {
    CoverageContract {
        contract_id: "release-pipeline-static-evidence-v1".to_string(),
        contract_version: "1.1.0".to_string(),
        consumer: "release-pipeline::static_evidence".to_string(),
        scope: scope(),
        required_capabilities: vec![
            req(RequiredCapabilityKind::SymbolKind, "fn"),
            req(RequiredCapabilityKind::RelationClass, "import_graph"),
        ],
    }
}

fn basis() -> CoverageBasis {
    CoverageBasis {
        revision: HEAD_REV.to_string(),
        inventory: Inventory {
            revision: HEAD_REV.to_string(),
            rule_version: "1.0.0".to_string(),
            expected_files: vec!["crates/sddk-cli/src/main.rs".to_string()],
        },
        provider_strategy: "lightweight".to_string(),
        provider_version: "0.97.1".to_string(),
        contract_revision: "1.0.0".to_string(),
    }
}

fn snapshot_with(classes: &[&str]) -> CapabilitySnapshot {
    CapabilitySnapshot::from_advertised_with_capabilities(
        CapabilityProfile::StaticEnhanced,
        &["dummy"],
        &["lightweight"],
        classes,
    )
}

#[test]
fn t_ar_1_two_consumers_independent_satisfied_semantics() {
    // A provider declaring the union of both contracts' classes.
    let snap = snapshot_with(&[
        "fn",
        "struct",
        "find_usages",
        "analyze_impact",
        "import_graph",
    ]);
    let b = basis();

    let vk = default_coverage_evaluation(&verify_kernel_contract_v1_0(), &b, &snap);
    let rp = default_coverage_evaluation(&release_pipeline_contract_v1_1(), &b, &snap);

    // Both consumers get Demonstrated semantics over the same
    // provider snapshot — same scope, no interference.
    assert_eq!(vk.semantics, DimensionValue::Demonstrated);
    assert_eq!(rp.semantics, DimensionValue::Demonstrated);
}

#[test]
fn t_ar_2_per_consumer_gaps_not_shared() {
    // A provider that satisfies release-pipeline but NOT
    // verify-kernel (no find_usages / analyze_impact / struct).
    let snap = snapshot_with(&["fn", "import_graph"]);
    let b = basis();

    let vk = default_coverage_evaluation(&verify_kernel_contract_v1_0(), &b, &snap);
    let rp = default_coverage_evaluation(&release_pipeline_contract_v1_1(), &b, &snap);

    // verify-kernel: semantics Incomplete with its own gaps.
    assert_ne!(vk.semantics, DimensionValue::Demonstrated);
    let missing: Vec<&str> = vk
        .gaps
        .iter()
        .filter_map(|g| match g {
            CoverageGap::MissingSemanticClass(c) => Some(c.as_str()),
            _ => None,
        })
        .collect();
    assert!(missing.contains(&"struct"), "gaps: {missing:?}");
    assert!(missing.contains(&"find_usages"), "gaps: {missing:?}");
    assert!(missing.contains(&"analyze_impact"), "gaps: {missing:?}");

    // release-pipeline: Demonstrated semantics — its lighter
    // contract is fully served by the same provider.
    assert_eq!(rp.semantics, DimensionValue::Demonstrated);
    // And no missing-class gaps leak from the other consumer.
    assert!(
        rp.gaps
            .iter()
            .all(|g| !matches!(g, CoverageGap::MissingSemanticClass(_)))
    );
}

#[test]
fn t_ar_3_contract_identity_is_version_qualified() {
    // N5: the two contracts are distinct values and their
    // identity includes id + version + consumer — a bump of the
    // release-pipeline contract to v1.2.0 does not affect the
    // verify-kernel contract, and byte-equal bases with identical
    // observations still produce distinct evaluations because the
    // *contract* differs.
    let vk = verify_kernel_contract_v1_0();
    let rp = release_pipeline_contract_v1_1();
    assert_ne!(vk, rp);

    let mut rp_bumped = rp.clone();
    rp_bumped.contract_version = "1.2.0".to_string();
    assert_ne!(rp, rp_bumped);
    // Bumping the version alone (same required capabilities) does
    // not change the verdict shape: both still evaluate
    // Demonstrated semantics against a fully-declaring provider.
    let snap = snapshot_with(&[
        "fn",
        "struct",
        "find_usages",
        "analyze_impact",
        "import_graph",
    ]);
    let b = basis();
    let e1 = default_coverage_evaluation(&rp, &b, &snap);
    let e2 = default_coverage_evaluation(&rp_bumped, &b, &snap);
    assert_eq!(e1.semantics, e2.semantics);
    assert_eq!(e1.verdict, e2.verdict);
}

#[test]
fn t_ar_4_basis_carries_contract_revision_not_global_threshold() {
    // N6: no global constant. The release-pipeline contract pins
    // its own revision in the basis and the evaluation references
    // that contract only.
    let snap = snapshot_with(&["fn", "import_graph"]);
    let mut b = basis();
    b.contract_revision = "1.1.0".to_string();
    let e = default_coverage_evaluation(&release_pipeline_contract_v1_1(), &b, &snap);
    assert_eq!(e.semantics, DimensionValue::Demonstrated);
    // Inventory/operational remain Unknown at the default layer —
    // honest: the acceptance demo does not fake them.
    assert!(matches!(e.verdict, CoverageVerdict::Incomplete { .. }));
}
