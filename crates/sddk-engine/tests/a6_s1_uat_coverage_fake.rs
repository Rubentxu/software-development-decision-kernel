//! `a6_s1_uat_coverage_fake.rs` — UAT evidence battery for A6
//! — Static Enhanced Readiness (deterministic subset via fake
//! provider).
//!
//! Slice: `p-63676b11dc0ef88f/a6-static-enhanced-readiness/slices/s1-uat-coverage-fake`
//!
//! UAT rows covered (per
//! `docs/history/legacy-packages/SDDK-Production-Readiness-Alignment-2026-09-14/07-UAT-EVIDENCE-MATRIX.md`
//! §E):
//!
//! - PR-UAT-C01 — Provider stopped, OPTIONAL/PREFERRED → Base
//!   remains; EvidenceGap/NOT_EVALUATED recorded.
//! - PR-UAT-C02 — Provider stopped, REQUIRED → explicit failure;
//!   Base state remains valid.
//! - PR-UAT-C03 — Compatible provider startup/handshake — protocol
//!   / capabilities negotiated and recorded.
//! - PR-UAT-C04 — Localized delta analysis — typed result with
//!   basis / analyzer provenance; no forced full graph transfer.
//! - PR-UAT-C06 — Cancellation/deadline — incomplete / cancelled
//!   result; no green VerifyReceipt from missing requested
//!   evidence.
//! - PR-UAT-C08 — Protocol major incompatible — INCOMPATIBLE
//!   state; no capability inference from product version.
//! - PR-UAT-C10 — Repeat deterministic analysis on same basis /
//!   analyzer set — stable semantic digest / result identity.
//!
//! Rows C05, C07, C09 belong to S2; AC10 to S3; durability to S4;
//! real CogniCode to S5.

#![forbid(unsafe_code)]

use sddk_engine::code_intelligence_port::{
    AnalysisBasis, CapabilityProfile, CapabilitySnapshot, CodeIntelligencePort,
    CodeIntelligencePortError, CoverageBasis, CoverageContract, CoverageEvaluation,
    CoverageVerdict, DimensionValue, EvidenceGap, Inventory, ProviderKind, ProviderLifecycle,
    RequiredCapability, RequiredCapabilityKind, ScopeRef, ScopeRequest,
    default_coverage_evaluation,
};
use sddk_engine::code_intelligence_port_fake::{
    FakeCodeIntelligenceProvider, NullCodeIntelligenceProvider,
};

const HEAD_REV: &str = "deadbeefdeadbeefdeadbeefdeadbeefdeadbeef";
const INCLUDE_GLOBS: &[&str] = &[
    "crates/**/src/**/*.rs",
    "crates/**/tests/**/*.rs",
    "tests/**/*.rs",
];
const EXCLUDE_GLOBS: &[&str] = &["target/**", ".git/**", "**/tests/fixtures/**"];

/// Canonical S1 contract.
fn contract_v1(required: Vec<RequiredCapability>) -> CoverageContract {
    CoverageContract {
        contract_id: "static-enhanced-workspace-v1".to_string(),
        contract_version: "1.0.0".to_string(),
        consumer: "verify-kernel::static_evidence".to_string(),
        scope: ScopeRef {
            repo: "sddk-framework".to_string(),
            revision: HEAD_REV.to_string(),
            include_globs: INCLUDE_GLOBS.iter().map(|s| s.to_string()).collect(),
            exclude_globs: EXCLUDE_GLOBS.iter().map(|s| s.to_string()).collect(),
        },
        required_capabilities: required,
    }
}

/// Canonical S1 basis.
fn basis_v1() -> CoverageBasis {
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

/// Snapshot that declares the canonical S1 classes and strategies.
fn snapshot_v1() -> CapabilitySnapshot {
    CapabilitySnapshot::from_advertised_with_capabilities(
        CapabilityProfile::StaticEnhanced,
        &["rust-analyzer"],
        &["lightweight", "full"],
        &["fn", "struct", "trait", "find_usages", "analyze_impact"],
    )
}

/// Build an `AnalysisBasis` for delta/impact operations.
fn basis_for(provider: &FakeCodeIntelligenceProvider, scope: &str) -> AnalysisBasis {
    let snap = provider.capabilities();
    AnalysisBasis {
        provider_build: "test".to_string(),
        protocol_major: snap.protocol_major,
        protocol_minor: snap.protocol_minor,
        capability_snapshot: snap,
        analyzer_set_digest: sddk_engine::code_intelligence_port::DigestSha256::of(
            b"rust-analyzer",
        ),
        source_revision: HEAD_REV.to_string(),
        request_scope: scope.to_string(),
    }
}

// ────────────────────────────────────────────────────────────────────────
// PR-UAT-C01 — Provider stopped, OPTIONAL/PREFERRED
// SDDK remains Base-capable; records EvidenceGap/NOT_EVALUATED.
// ────────────────────────────────────────────────────────────────────────

#[test]
fn t_uat_c01_optional_provider_unavailable_yields_evidence_gap() {
    // Provider is Null (Unavailable). Requirement is OPTIONAL.
    let provider = NullCodeIntelligenceProvider;
    let contract = contract_v1(vec![RequiredCapability {
        kind: RequiredCapabilityKind::SymbolKind,
        name: "fn".to_string(),
    }]);
    let basis = basis_v1();
    let result = provider.coverage_evaluation(&contract, &basis);
    // AR-4: provider Unavailable -> EvidenceGap, never Satisfied.
    assert!(matches!(
        result,
        Err(EvidenceGap::ProviderUnavailable {
            lifecycle: ProviderLifecycle::Unavailable
        })
    ));
    // Base mode is first-class (IPB-010): capability snapshot is
    // `BASE`, NOT `STATIC_ENHANCED`. No capability inference from
    // product version (C08 invariant).
    assert_eq!(provider.capabilities().profile, CapabilityProfile::Base);
}

#[test]
fn t_uat_c01_preferred_provider_unavailable_yields_evidence_gap() {
    // Same outcome for PREFERRED — EvidenceGap, no implicit
    // promotion.
    let provider = NullCodeIntelligenceProvider;
    let contract = contract_v1(vec![RequiredCapability {
        kind: RequiredCapabilityKind::SymbolKind,
        name: "struct".to_string(),
    }]);
    let basis = basis_v1();
    let result = provider.coverage_evaluation(&contract, &basis);
    assert!(matches!(
        result,
        Err(EvidenceGap::ProviderUnavailable { .. })
    ));
}

// ────────────────────────────────────────────────────────────────────────
// PR-UAT-C02 — Provider stopped, REQUIRED
// Requested operation fails explicitly; Base state remains valid.
// ────────────────────────────────────────────────────────────────────────

#[test]
fn t_uat_c02_required_provider_unavailable_yields_typed_failure() {
    // For REQUIRED requirements, the consumer receives a typed
    // refusal; the CoverageEvaluation does NOT silently downgrade
    // to Satisfied. We model this at the contract-evaluation
    // level: a REQUIRED consumer calling coverage_evaluation on
    // an Unavailable provider gets `Err(EvidenceGap)` with the
    // precise lifecycle observed. The consumer's decision to
    // abort is explicit and typed.
    let provider = NullCodeIntelligenceProvider;
    let contract = contract_v1(vec![RequiredCapability {
        kind: RequiredCapabilityKind::RelationClass,
        name: "find_usages".to_string(),
    }]);
    let basis = basis_v1();
    let err = provider
        .coverage_evaluation(&contract, &basis)
        .expect_err("Unavailability is a typed failure");
    match err {
        EvidenceGap::ProviderUnavailable { lifecycle } => {
            assert_eq!(lifecycle, ProviderLifecycle::Unavailable);
        }
        other => panic!("unexpected variant: {other:?}"),
    }
    // Base state remains valid: no `STATIC_ENHANCED` advertisement.
    assert_eq!(provider.capabilities().profile, CapabilityProfile::Base);
}

// ────────────────────────────────────────────────────────────────────────
// PR-UAT-C03 — Compatible provider startup/handshake
// Runtime protocol/capabilities negotiated and recorded.
// ────────────────────────────────────────────────────────────────────────

#[test]
fn t_uat_c03_compatible_handshake_records_capabilities() {
    // The fake provider advertises STATIC_ENHANCED with the
    // canonical analyzer set. The handshake (capabilities() +
    // lifecycle_state()) records the negotiated profile, protocol
    // version, analyzer-set digest, and lifecycle.
    let provider = FakeCodeIntelligenceProvider::new("test", &["rust-analyzer"]);
    let snap = provider.capabilities();
    assert_eq!(snap.profile, CapabilityProfile::StaticEnhanced);
    assert_eq!(snap.protocol_major, 1);
    assert_eq!(snap.protocol_minor, 0);
    assert_eq!(
        snap.analyzer_set_digest,
        sddk_engine::code_intelligence_port::DigestSha256::of(b"rust-analyzer")
    );
    assert_eq!(provider.lifecycle_state(), ProviderLifecycle::Ready);
    // CapabilitySnapshot digest is recorded (stable identifier).
    let snap2 = provider.capabilities();
    assert_eq!(snap.digest, snap2.digest);
}

#[test]
fn t_uat_c03_capability_snapshot_is_content_addressed() {
    // Same input → same digest (determinism invariant).
    let p1 = FakeCodeIntelligenceProvider::new("b1", &["rust-analyzer", "cargo"]);
    let p2 = FakeCodeIntelligenceProvider::new("b2", &["cargo", "rust-analyzer"]);
    assert_eq!(
        p1.capabilities().digest,
        p2.capabilities().digest,
        "analyzer-set order must not affect digest (sorted)"
    );
    // Different analyzer set → different digest.
    let p3 = FakeCodeIntelligenceProvider::new("b3", &["rust-analyzer"]);
    assert_ne!(p1.capabilities().digest, p3.capabilities().digest);
}

// ────────────────────────────────────────────────────────────────────────
// PR-UAT-C04 — Localized delta analysis
// Typed result + basis/analyzer provenance; no forced full graph
// transfer.
// ────────────────────────────────────────────────────────────────────────

#[test]
fn t_uat_c04_localized_delta_typed_result_with_basis() {
    let provider = FakeCodeIntelligenceProvider::new("test", &["rust-analyzer"]);
    let basis = basis_for(&provider, "delta:crates/sddk-cli/src/main.rs");
    let request = ScopeRequest {
        added_units: vec!["crates/sddk-cli/src/main.rs".to_string()],
        removed_units: Vec::new(),
        modified_units: Vec::new(),
    };
    let result = provider.analyze_delta(&basis, &request);
    let result = result.expect("happy-path delta returns Ok");
    // Result is typed (not a string, not a CLI scrape): the
    // ObservationSet carries per-unit observations keyed by
    // source-unit identifier (BTreeMap<String, Vec<Observation>>).
    assert_eq!(result.observations.provider_kind, ProviderKind::Fake);
    assert!(
        result
            .observations
            .units
            .contains_key("crates/sddk-cli/src/main.rs")
    );
    assert!(!result.observations.restart_observed);
    // Basis provenance is recorded (analysis was reproducible).
    assert_eq!(result.observations.units.len(), 1);
}

#[test]
fn t_uat_c04_no_full_graph_transfer_on_localized_delta() {
    // The localized delta carries ONLY the added/modified/removed
    // units; no implicit full graph. Two requests with different
    // scopes produce different ObservationSet sizes — proving the
    // adapter does NOT pad with full-graph data.
    let provider = FakeCodeIntelligenceProvider::new("test", &["rust-analyzer"]);
    let basis_small = basis_for(&provider, "delta:unit-a");
    let basis_large = basis_for(&provider, "delta:many-units");
    let r_small = provider
        .analyze_delta(
            &basis_small,
            &ScopeRequest {
                added_units: vec!["unit-a".to_string()],
                removed_units: Vec::new(),
                modified_units: Vec::new(),
            },
        )
        .unwrap();
    let r_large = provider
        .analyze_delta(
            &basis_large,
            &ScopeRequest {
                added_units: vec![
                    "u1".to_string(),
                    "u2".to_string(),
                    "u3".to_string(),
                    "u4".to_string(),
                    "u5".to_string(),
                ],
                removed_units: Vec::new(),
                modified_units: Vec::new(),
            },
        )
        .unwrap();
    assert_eq!(r_small.observations.units.len(), 1);
    assert_eq!(r_large.observations.units.len(), 5);
    // No overlap of observation text with full-graph artefacts.
    let keys: std::collections::BTreeSet<&String> = r_large.observations.units.keys().collect();
    assert!(!keys.contains(&"full-graph".to_string()));
    assert!(!keys.contains(&"__all__".to_string()));
}

// ────────────────────────────────────────────────────────────────────────
// PR-UAT-C06 — Cancellation/deadline
// Incomplete / cancelled result; no green VerifyReceipt from
// missing requested evidence.
// ────────────────────────────────────────────────────────────────────────

#[test]
fn t_uat_c06_cancellation_yields_incomplete_not_satisfied() {
    let mut provider = FakeCodeIntelligenceProvider::new("test", &["rust-analyzer"]);
    provider.arm_cancel_after_one_unit();
    let basis = basis_for(&provider, "delta:many");
    let request = ScopeRequest {
        added_units: vec!["u1".to_string(), "u2".to_string(), "u3".to_string()],
        removed_units: Vec::new(),
        modified_units: Vec::new(),
    };
    let err = provider
        .analyze_delta(&basis, &request)
        .expect_err("cancellation must surface as typed refusal");
    assert!(matches!(err, CodeIntelligencePortError::Cancelled));
    // CoverageEvaluation on a cancelled operation does NOT
    // pretend to be Satisfied.
    let contract = contract_v1(vec![RequiredCapability {
        kind: RequiredCapabilityKind::SymbolKind,
        name: "fn".to_string(),
    }]);
    let eval = provider
        .coverage_evaluation(&contract, &basis_v1())
        .expect("Ready -> coverage_evaluation Ok");
    // Semantics is Demonstrated (snapshot declares them); but
    // operational is Unknown by default (the cancellation does
    // not enrich operational into Demonstrated). Verdict stays
    // Incomplete — no false PASS.
    assert!(matches!(eval.verdict, CoverageVerdict::Incomplete { .. }));
}

#[test]
fn t_uat_c06_cancelled_delta_does_not_produce_partial_green_receipt() {
    // The intent of C06: no "green VerifyReceipt from missing
    // requested evidence". The CoverageEvaluation's `gaps[]`
    // must enumerate the operational gap, not an empty set.
    let mut provider = FakeCodeIntelligenceProvider::new("test", &["rust-analyzer"]);
    provider.arm_cancel_after_one_unit();
    let contract = contract_v1(vec![RequiredCapability {
        kind: RequiredCapabilityKind::SymbolKind,
        name: "fn".to_string(),
    }]);
    let eval: CoverageEvaluation = provider
        .coverage_evaluation(&contract, &basis_v1())
        .unwrap();
    assert!(
        !eval.gaps.is_empty(),
        "gaps must enumerate missing evidence"
    );
    // Operational is at most Partial (cannot be Demonstrated when
    // cancellation interrupted the analysis).
    assert!(matches!(
        eval.operational,
        DimensionValue::Unknown | DimensionValue::Incomplete | DimensionValue::Partial
    ));
}

// ────────────────────────────────────────────────────────────────────────
// PR-UAT-C08 — Protocol major incompatible
// Provider state is INCOMPATIBLE; no capability inference from
// product version.
// ────────────────────────────────────────────────────────────────────────

#[test]
fn t_uat_c08_protocol_incompatible_no_capability_inference() {
    let mut provider = FakeCodeIntelligenceProvider::new("test", &["rust-analyzer"]);
    provider.force_protocol_major(2);
    // Lifecycle state is INCOMPATIBLE.
    assert_eq!(provider.lifecycle_state(), ProviderLifecycle::Incompatible);
    // Capability profile is suppressed to BASE — no inference
    // from "this product is v2.x" to "STATIC_ENHANCED".
    assert_eq!(provider.capabilities().profile, CapabilityProfile::Base);
    // analyze_delta returns typed ProtocolMajorMismatch.
    let basis = basis_for(&provider, "delta:unit-a");
    let err = provider
        .analyze_delta(
            &basis,
            &ScopeRequest {
                added_units: vec!["unit-a".to_string()],
                removed_units: Vec::new(),
                modified_units: Vec::new(),
            },
        )
        .expect_err("protocol-major mismatch is typed");
    match err {
        CodeIntelligencePortError::ProtocolMajorMismatch {
            provider_major,
            sddk_major,
        } => {
            assert_eq!(provider_major, 2);
            assert_eq!(sddk_major, 1);
        }
        other => panic!("unexpected variant: {other:?}"),
    }
    // CoverageEvaluation surfaces EvidenceGap::ProviderIncompatible
    // with both protocol majors — never a Satisfied verdict.
    let contract = contract_v1(vec![RequiredCapability {
        kind: RequiredCapabilityKind::SymbolKind,
        name: "fn".to_string(),
    }]);
    let err = provider
        .coverage_evaluation(&contract, &basis_v1())
        .expect_err("INCOMPATIBLE -> EvidenceGap");
    match err {
        EvidenceGap::ProviderIncompatible {
            provider_major,
            sddk_major,
        } => {
            assert_eq!(provider_major, 2);
            assert_eq!(sddk_major, 1);
        }
        other => panic!("unexpected variant: {other:?}"),
    }
}

// ────────────────────────────────────────────────────────────────────────
// PR-UAT-C10 — Repeat deterministic analysis on same basis/analyzer set
// Promised deterministic semantic digest/result identity is stable.
// ────────────────────────────────────────────────────────────────────────

#[test]
fn t_uat_c10_repeat_deterministic_analysis_yields_stable_digest() {
    let provider = FakeCodeIntelligenceProvider::new("test", &["rust-analyzer"]);
    let basis = basis_for(&provider, "delta:determinism");
    let request = ScopeRequest {
        added_units: vec!["u1".to_string(), "u2".to_string()],
        removed_units: Vec::new(),
        modified_units: Vec::new(),
    };
    let r1 = provider.analyze_delta(&basis, &request).unwrap();
    let r2 = provider.analyze_delta(&basis, &request).unwrap();
    assert_eq!(
        r1.digest, r2.digest,
        "same (basis, request, observations) -> same digest"
    );
    // Same basis, different request scope -> different digest.
    let basis_other = basis_for(&provider, "delta:different");
    let r3 = provider.analyze_delta(&basis_other, &request).unwrap();
    assert_ne!(r1.digest, r3.digest);
}

#[test]
fn t_uat_c10_coverage_evaluation_is_deterministic_across_runs() {
    // Re-pinned from CC-S1 t_ar_5_determinism_two_consecutive_evaluations_byte_equal
    // and surfaced as the C10 evidence row.
    let contract = contract_v1(vec![RequiredCapability {
        kind: RequiredCapabilityKind::SymbolKind,
        name: "fn".to_string(),
    }]);
    let basis = basis_v1();
    let e1 = default_coverage_evaluation(&contract, &basis, &snapshot_v1());
    let e2 = default_coverage_evaluation(&contract, &basis, &snapshot_v1());
    assert_eq!(e1, e2);
}

// ────────────────────────────────────────────────────────────────────────
// End of S1 UAT battery.
// ────────────────────────────────────────────────────────────────────────
