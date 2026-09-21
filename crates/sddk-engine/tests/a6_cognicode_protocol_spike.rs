//! Falsification battery for A6 CogniCode STATIC_ENHANCED — CC-S0.
//!
//! Proves the SDDK-side seam for static-intelligence providers
//! exists and is contract-clean. The cycle's scope is narrow: the
//! `CodeIntelligencePort` trait, an in-process deterministic fake
//! provider, and 6 RED→GREEN tests covering negotiation,
//! determinism, INCOMPATIBLE handling, cancellation, restart, and
//! Base-mode first-class.
//!
//! These tests are pinned against the pre-fix code first (RED)
//! and become GREEN after the trait + fake land.
//!
//! Spec authority: `docs/architecture/specs/arch-spec-021-intelligence-provider-boundary.md`
//! Cycle plan: `docs/history/proposals/all-proposals/2026-09-19-a6-cognicode-cc-s0-protocol-spike-PROPOSAL.md`
//! ADR: `docs/architecture/adrs/ADR-0137-CODE-INTELLIGENCE-PORT-SEAM.md`

use sddk_engine::code_intelligence_port::analyzer_token;
use sddk_engine::code_intelligence_port::{
    AnalysisBasis, CapabilityProfile, CapabilitySnapshot, CodeIntelligencePort, DigestSha256,
    ImpactRequest, ObservationSet, ProviderLifecycle, ScopeRequest,
};

// ── helpers ─────────────────────────────────────────────────────

fn fixture_basis(provider_build: &str, analyzer_set: &[&str]) -> AnalysisBasis {
    AnalysisBasis {
        provider_build: provider_build.into(),
        protocol_major: 1,
        protocol_minor: 0,
        capability_snapshot: CapabilitySnapshot::from_advertised(
            CapabilityProfile::StaticEnhanced,
            analyzer_set,
        ),
        analyzer_set_digest: DigestSha256::of(analyzer_set.join("|").as_bytes()),
        source_revision: "rev-fixture-0001".into(),
        request_scope: "fixture://analyze-delta".into(),
    }
}

// ── T1 — connect / negotiate returns the advertised profile ─────

#[test]
fn t1_negotiate_returns_static_enhanced_snapshot_when_advertised() {
    // Fake provider advertises STATIC_ENHANCED. The adapter must
    // report it back via `capabilities()`, NOT silently degrade
    // to BASE.
    let fake = sddk_engine::code_intelligence_port_fake::FakeCodeIntelligenceProvider::new(
        "fake-cognicode-0.0.0",
        &[analyzer_token("rust"), analyzer_token("deps")],
    );
    let advertised = fake.capabilities();
    assert_eq!(
        advertised.profile,
        CapabilityProfile::StaticEnhanced,
        "T1 RED: fake advertises StaticEnhanced; adapter must report it (not BASE)"
    );
    assert_eq!(advertised.protocol_major, 1);
}

// ── T2 — analyze_delta is deterministic across runs ─────────────

#[test]
fn t2_analyze_delta_is_deterministic_across_runs() {
    let fake = sddk_engine::code_intelligence_port_fake::FakeCodeIntelligenceProvider::new(
        "fake-cognicode-0.0.0",
        &[analyzer_token("rust")],
    );
    let basis = fixture_basis("fake-cognicode-0.0.0", &["rust"]);
    let delta = ScopeRequest {
        added_units: vec!["crates/sddk-engine/src/lib.rs".into()],
        removed_units: vec![],
        modified_units: vec!["crates/sddk-engine/src/code_intelligence_port.rs".into()],
    };
    let result_a = fake
        .analyze_delta(&basis, &delta)
        .expect("T2: first analyze_delta");
    let result_b = fake
        .analyze_delta(&basis, &delta)
        .expect("T2: second analyze_delta");
    assert_eq!(
        result_a.digest, result_b.digest,
        "T2 RED: same basis + delta MUST yield the same digest"
    );
}

// ── T3 — protocol-major mismatch → INCOMPATIBLE (not BASE) ─────

#[test]
fn t3_protocol_major_mismatch_yields_incompatible() {
    let mut fake = sddk_engine::code_intelligence_port_fake::FakeCodeIntelligenceProvider::new(
        "fake-cognicode-0.0.0",
        &[analyzer_token("rust")],
    );
    fake.force_protocol_major(99);
    let state = fake.lifecycle_state();
    assert_eq!(
        state,
        ProviderLifecycle::Incompatible,
        "T3 RED: protocol-major mismatch MUST be INCOMPATIBLE, not silently degraded to BASE"
    );
    // The provider's advertised profile is suppressed when
    // INCOMPATIBLE; capabilities() reports BASE because the
    // spike has no usable enhanced profile to offer.
    assert_eq!(fake.capabilities().profile, CapabilityProfile::Base);
}

// ── T4 — cancellation yields typed refusal (no false PASS) ──────

#[test]
fn t4_cancellation_yields_typed_refusal_no_false_evidence() {
    let mut fake = sddk_engine::code_intelligence_port_fake::FakeCodeIntelligenceProvider::new(
        "fake-cognicode-0.0.0",
        &[analyzer_token("rust")],
    );
    fake.arm_cancel_after_one_unit();
    let basis = fixture_basis("fake-cognicode-0.0.0", &["rust"]);
    let delta = ScopeRequest {
        added_units: vec![
            "a.rs".into(),
            "b.rs".into(),
            "c.rs".into(),
            "d.rs".into(),
            "e.rs".into(),
        ],
        removed_units: vec![],
        modified_units: vec![],
    };
    let outcome = fake.analyze_delta(&basis, &delta);
    assert!(
        outcome.is_err(),
        "T4 RED: cancellation after one unit MUST yield a typed error, not a partial ObservationSet"
    );
    let err = outcome.unwrap_err();
    assert!(
        matches!(
            err,
            sddk_engine::code_intelligence_port::CodeIntelligencePortError::Cancelled
        ),
        "T4 RED: error variant must be Cancelled (got: {err:?})"
    );
}

// ── T5 — restart mid-request yields replay OR Partial marker ────

#[test]
fn t5_restart_mid_request_yields_replay_or_partial_marker() {
    let mut fake = sddk_engine::code_intelligence_port_fake::FakeCodeIntelligenceProvider::new(
        "fake-cognicode-0.0.0",
        &[analyzer_token("rust")],
    );
    fake.arm_restart_after_one_unit();
    let basis = fixture_basis("fake-cognicode-0.0.0", &["rust"]);
    let delta = ScopeRequest {
        added_units: vec!["a.rs".into(), "b.rs".into(), "c.rs".into()],
        removed_units: vec![],
        modified_units: vec![],
    };
    // First attempt: provider restarts after one unit; result is
    // either a replay (same digest) OR an explicit Partial
    // marker. Anything else is a contract violation.
    let first = fake.analyze_delta(&basis, &delta);
    let outcome = first.expect("T5: result must be Ok with Partial marker, not Err");
    assert!(
        outcome.partial,
        "T5 RED: restart mid-request MUST yield a Partial marker (or full replay); not silent PASS"
    );
    // The observation set must carry a `restart_observed: true`
    // signal so downstream consumers can reconcile.
    assert!(
        outcome.observations.restart_observed,
        "T5 RED: restart signal MUST be propagated to the ObservationSet"
    );
}

// ── T6 — Base mode first-class with provider UNAVAILABLE ────────

#[test]
fn t6_base_mode_first_class_with_provider_unavailable() {
    // A NULL provider (UNAVAILABLE) MUST yield CapabilityProfile::Base
    // and a no-op observe() — SDDK's Verify path is unchanged.
    let null = sddk_engine::code_intelligence_port_fake::NullCodeIntelligenceProvider;
    let advertised = null.capabilities();
    assert_eq!(
        advertised.profile,
        CapabilityProfile::Base,
        "T6 RED: NULL provider MUST advertise Base; not StaticEnhanced"
    );
    let basis = fixture_basis("none", &[]);
    let delta = ScopeRequest::default();
    let outcome = null.analyze_delta(&basis, &delta);
    assert!(
        outcome.is_ok(),
        "T6 RED: NULL provider MUST be observable, not error"
    );
    let result = outcome.unwrap();
    assert!(
        result.observations.units.is_empty(),
        "T6 RED: NULL provider's ObservationSet must be empty"
    );
    assert!(
        result.observations.provider_kind
            == sddk_engine::code_intelligence_port::ProviderKind::Null,
        "T6 RED: NULL provider's ObservationSet must carry ProviderKind::Null"
    );
}

// ── Reference to module-level types used above ──────────────────
// The `analyzer_token` helper is provided by the trait module
// (test-only re-export under `#[cfg(test)]`); see
// `code_intelligence_port.rs`. Listed here so the imports remain
// stable if the trait module is later reorganised.
#[allow(unused_imports)]
use sddk_engine::code_intelligence_port::CodeIntelligencePortError;

// Avoid an unused-import warning when `DigestSha256` /
// `ObservationSet` / `ImpactRequest` are not directly named in
// the bodies (they are used through trait method returns).
#[allow(dead_code)]
fn _types_used(_: DigestSha256, _: ObservationSet, _: ImpactRequest) {}
