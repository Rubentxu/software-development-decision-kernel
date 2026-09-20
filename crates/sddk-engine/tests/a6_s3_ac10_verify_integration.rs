//! `a6_s3_ac10_verify_integration.rs` — AC10 acceptance battery for
//! slice S3.
//!
//! Slice: `p-63676b11dc0ef88f/a6-static-enhanced-readiness/slices/s3-ac10-verify-integration`
//!
//! AC10 closed end-to-end: a CogniCode-shaped `AnalysisResult` from
//! `CodeIntelligencePort` is bridged into the SDDK Verify substrate
//! and consumed by `VerifyKernel::evaluate` via the existing
//! `StaticProviderDomain`.
//!
//! UAT rows:
//! - AC10 (custom, mini-roadmap §A6) — static evidence is consumed by
//!   Verify.
//! - PR-UAT-C04 (re-pinned) — localized delta, no full graph.
//! - PR-UAT-019 — Verify is delta-scoped, no full repo scan, and
//!   never produces a false PASS on missing evidence.

#![forbid(unsafe_code)]

use std::collections::BTreeMap;

use sddk_engine::code_intelligence_port::{
    AnalysisBasis, CapabilityProfile, CapabilitySnapshot, CodeIntelligencePort, DigestSha256,
    Observation, ProviderKind, ScopeRequest,
};
use sddk_engine::code_intelligence_port_fake::FakeCodeIntelligenceProvider;
use sddk_engine::evidence_ref::{EvidenceKind, EvidenceRef};
use sddk_engine::observation::types::ObservationSet as SubstrateObservationSet;
use sddk_engine::verify_kernel::adapter_static_provider::{
    StaticProviderClaim, StaticProviderDomain,
};
use sddk_engine::verify_kernel::engine::VerifyKernel;
use sddk_engine::verify_kernel::evidence_source_static_provider::{
    BridgedObservationSet, LOCATOR_PREFIX,
};
use sddk_engine::verify_kernel::types::{EvidenceGap, VerificationClaim, VerificationResult};

const HEAD_REV: &str = "deadbeefdeadbeefdeadbeefdeadbeefdeadbeef";

fn basis_for(provider: &FakeCodeIntelligenceProvider, scope: &str) -> AnalysisBasis {
    let snap = provider.capabilities();
    AnalysisBasis {
        provider_build: "test".to_string(),
        protocol_major: snap.protocol_major,
        protocol_minor: snap.protocol_minor,
        capability_snapshot: snap,
        analyzer_set_digest: DigestSha256::of(b"rust-analyzer"),
        source_revision: HEAD_REV.to_string(),
        request_scope: scope.to_string(),
    }
}

// ────────────────────────────────────────────────────────────────────────
// AC10 — CogniCode ObservationSet bridges into VerifyKernel.
// ────────────────────────────────────────────────────────────────────────

#[test]
fn t_ac10_cognicode_observation_set_bridges_into_verify_kernel() {
    // Step 1: drive `analyze_delta` on a localized delta (1 unit).
    let provider = FakeCodeIntelligenceProvider::new("test", &["rust-analyzer"]);
    let basis = basis_for(&provider, "delta:crates/sddk-cli/src/main.rs");
    let request = ScopeRequest {
        added_units: vec!["crates/sddk-cli/src/main.rs".to_string()],
        removed_units: Vec::new(),
        modified_units: Vec::new(),
    };
    let analysis = provider.analyze_delta(&basis, &request).unwrap();

    // Step 2: bridge into the Verify substrate.
    let bridged: BridgedObservationSet =
        sddk_engine::verify_kernel::evidence_source_static_provider::build(
            &analysis,
            HEAD_REV,
            Some("0.97.1"),
        );
    assert_eq!(bridged.count, 1, "1 unit, 1 fake-observation");
    assert_eq!(bridged.provider_kind, ProviderKind::Fake);

    // Step 3: feed the substrate to VerifyKernel via StaticProviderDomain.
    // The static_provider domain matches by `obs.subject.canonical_tag()`,
    // which for `ObservationSubject::Unit(SoftwareUnitRef(...))` is
    // `"unit:<locator>"`.
    let claim = VerificationClaim::StaticProvider(StaticProviderClaim {
        subject_tag: "unit:crates/sddk-cli/src/main.rs".to_string(),
        contract_id: "static-enhanced-workspace-v1".to_string(),
    });
    let result = VerifyKernel::evaluate(&claim, &bridged.observations, &StaticProviderDomain);
    // The fake produced a single observation for the requested unit
    // (text: "fake:crates/sddk-cli/src/main.rs:ok"), so the
    // subject_tag matches -> Verified.
    match result {
        VerificationResult::Verified => {}
        other => panic!("expected Verified, got {other:?}"),
    }
}

// ────────────────────────────────────────────────────────────────────────
// PR-UAT-C04 (re-pinned) — Localized delta, no full graph.
// ────────────────────────────────────────────────────────────────────────

#[test]
fn t_ac10_localized_delta_no_full_graph_in_verify() {
    // Same as CC-S0/C04: a localized delta of N units produces a
    // substrate with exactly N observations. No full-graph padding.
    let provider = FakeCodeIntelligenceProvider::new("test", &["rust-analyzer"]);
    let basis = basis_for(&provider, "delta:three-units");
    let request = ScopeRequest {
        added_units: vec!["u1".to_string(), "u2".to_string(), "u3".to_string()],
        removed_units: Vec::new(),
        modified_units: Vec::new(),
    };
    let analysis = provider.analyze_delta(&basis, &request).unwrap();
    let bridged = sddk_engine::verify_kernel::evidence_source_static_provider::build(
        &analysis,
        HEAD_REV,
        Some("0.97.1"),
    );
    assert_eq!(bridged.count, 3, "3 units, 3 observations; no full graph");
    // The substrate size is pinned; if the bridge padded with
    // full-graph data, count would be larger.
    assert_eq!(bridged.observations.len(), 3);
}

// ────────────────────────────────────────────────────────────────────────
// PR-UAT-019 — Verify delta-scoped; no false PASS on missing evidence.
// ────────────────────────────────────────────────────────────────────────

#[test]
fn t_ac10_verify_receipt_marks_observed_static_evidence() {
    // Step 1: build a substrate manually (so we don't depend on
    // the fake) with three observations: two `Affirms` and one
    // `Denies` for the same subject_tag. The Denies wins
    // (contradiction preserved, never averaged — AC10 + S2 C09
    // invariant).
    let substrate = build_substrate_with_3_obs("symbol-42:usages", "cognicode-mcp@0.97.1");

    let claim = VerificationClaim::StaticProvider(StaticProviderClaim {
        subject_tag: "unit:symbol-42:usages".to_string(),
        contract_id: "static-enhanced-workspace-v1".to_string(),
    });
    let result = VerifyKernel::evaluate(&claim, &substrate, &StaticProviderDomain);
    match result {
        VerificationResult::Contradicted { reason: _ } => {}
        other => panic!("expected Contradicted when Affirms+Denies both present, got {other:?}"),
    }
}

#[test]
fn t_ac10_no_evidence_for_subject_yields_unknown_not_pass() {
    // Substrate is empty: no observation matches the claim's
    // subject_tag. Verify must NOT produce a Verified result;
    // it must produce Unknown with an explicit EvidenceGap.
    let substrate = SubstrateObservationSet::new();
    let claim = VerificationClaim::StaticProvider(StaticProviderClaim {
        subject_tag: "unit:symbol-not-in-substrate".to_string(),
        contract_id: "static-enhanced-workspace-v1".to_string(),
    });
    let result = VerifyKernel::evaluate(&claim, &substrate, &StaticProviderDomain);
    match result {
        VerificationResult::Unknown { gap } => {
            // The gap reason is provider-specific; we only assert
            // that it is NOT Verified and that a gap is reported.
            match gap {
                EvidenceGap::MissingForSubject(_)
                | EvidenceGap::NoEvidenceProvided
                | EvidenceGap::InsufficientEvidence
                | EvidenceGap::Custom(_) => {}
                EvidenceGap::Sentinel => panic!("Sentinel must not appear"),
            }
        }
        other => panic!("expected Unknown, got {other:?}"),
    }
}

#[test]
fn t_ac10_observed_static_marker_is_locator_prefix() {
    // The bridge stamps `EvidenceRef.locator` with the
    // `cognicode://` prefix. A VerifyReceipt consumer can
    // distinguish `OBSERVED_STATIC` from `INFERRED` by reading
    // the prefix without extending the closed `EvidenceKind`
    // vocabulary. This test pins the prefix as a public constant.
    assert_eq!(LOCATOR_PREFIX, "cognicode://");
    let sample = EvidenceRef::new(EvidenceKind::Adhoc, format!("{LOCATOR_PREFIX}u1"));
    assert!(sample.locator.starts_with(LOCATOR_PREFIX));
    assert_eq!(sample.kind, EvidenceKind::Adhoc);
}

// ────────────────────────────────────────────────────────────────────────
// Helpers.
// ────────────────────────────────────────────────────────────────────────

fn build_substrate_with_3_obs(subject_tag: &str, producer: &str) -> SubstrateObservationSet {
    use sddk_engine::architecture_graph::types::SoftwareUnitRef;
    use sddk_engine::observation::types::{
        ObservationBasis, ObservationOrigin, ObservationStance, ObservationSubject,
        SoftwareObservation,
    };

    let basis = ObservationBasis::for_provider_result(HEAD_REV, "test-digest");
    let make = |text: &str, stance: ObservationStance| -> SoftwareObservation {
        // `text` is used to derive distinct observation bodies; we
        // do not propagate it as a field because the substrate
        // dedupes by `ObservationId`. Keep the parameter named for
        // documentation.
        let _ = text;
        SoftwareObservation::declare(
            ObservationSubject::Unit(SoftwareUnitRef::new(subject_tag.to_string())),
            stance,
            EvidenceRef::new(
                EvidenceKind::Adhoc,
                format!("{LOCATOR_PREFIX}{subject_tag}"),
            ),
            ObservationOrigin::StaticProvider,
            basis.clone(),
            None,
            producer.to_string(),
        )
    };

    let mut substrate = SubstrateObservationSet::new();
    substrate.insert(make("affirms-1", ObservationStance::Affirms));
    substrate.insert(make("affirms-2", ObservationStance::Affirms));
    substrate.insert(make("denies", ObservationStance::Denies));
    substrate
}

// Anchor imports so the test file's use list is preserved even if a
// future test reorganisation drops a direct reference.
#[allow(dead_code)]
fn _unused_anchors(
    _b: BTreeMap<String, Vec<Observation>>,
    _cap: CapabilitySnapshot,
    _profile: CapabilityProfile,
    _kind: ProviderKind,
) {
}
