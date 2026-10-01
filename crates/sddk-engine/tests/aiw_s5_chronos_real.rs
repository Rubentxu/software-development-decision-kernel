//! EXT tests for the Chronos MCP runtime evidence port.
//!
//! **C3l.4 — absence is never PASS.** These tests used to read
//!
//! ```ignore
//! let bin = match mcp_bin() { Some(b) => b, None => return };
//! ```
//!
//! so with `CHRONOS_MCP_BIN` unset `cargo test` reported `3 passed` while two
//! of the three had never run (`finished in 0.00s`). They now follow the
//! convention already used by `aiw_s1_cognicode_real.rs`: `#[ignore]` by
//! default, and a hard `expect` if the EXT profile is requested without the
//! binary — so an explicit EXT run with no provider **fails loudly** instead
//! of reporting a pass.
//!
//! The honest state is resolved through
//! [`sddk_engine::ext_outcome`], whose five states are also emitted by
//! `tests/ext_provider_gate.sh`:
//!
//! ```text
//! CHRONOS_MCP_BIN=... tests/ext_provider_gate.sh chronos CHRONOS_MCP_BIN
//! ```
//!
//! Exit codes: 0 pass_observed · 1 fail_observed · 2 blocked · 3 not_run.
use sddk_engine::ext_outcome::{self, ExtOutcome};
use sddk_engine::runtime_evidence_port::{RuntimeCaptureRequest, RuntimeEvidencePort};
use sddk_engine::runtime_evidence_port_mcp::ChronosMcpAdapter;
use sddk_engine::verify_kernel::adapter_runtime_provider::{
    RuntimeProviderClaim, RuntimeProviderDomain,
};
use sddk_engine::verify_kernel::types::{EvidenceGap, VerificationClaim, VerificationResult};

/// Resolve the provider, or fail the EXT run.
///
/// A resolved-but-absent provider is NOT a pass and NOT a skip: it aborts
/// with a message naming the env var. This is the whole point of C3l.4.
fn mcp_bin() -> String {
    let (receipt, outcome) = ext_outcome::resolve_provider("chronos-mcp", "CHRONOS_MCP_BIN");
    assert!(
        !matches!(outcome, ExtOutcome::BlockedExternalDependency { .. }),
        "EXT profile requested without a usable chronos-mcp: {outcome:?} ({})",
        receipt.found_description()
    );
    receipt.path.to_string_lossy().to_string()
}

/// AT-UAT-009: with the provider absent, the resolved state is
/// `blocked_external_dependency` and is explicitly **not** a pass.
#[test]
fn absent_provider_is_blocked_not_pass() {
    let (receipt, outcome) =
        ext_outcome::resolve_provider("sddk-absent-provider-probe", "SDDK_ABSENT_PROBE_BIN");
    assert!(
        matches!(outcome, ExtOutcome::BlockedExternalDependency { .. }),
        "an absent provider must resolve to BLOCKED, got {outcome:?}"
    );
    assert!(!outcome.is_pass(), "BLOCKED must never read as a pass");
    assert!(
        outcome.is_ignored_in_ordinary_run(),
        "BLOCKED must be reported as ignored in the ordinary run"
    );
    assert!(receipt.sha256.is_none());
}

#[test]
#[ignore = "EXT profile: requires a real chronos-mcp binary"]
fn capture_produces_events_and_verified_claim() {
    let bin = mcp_bin();
    let target = std::env::var("CHRONOS_TARGET_BIN")
        .unwrap_or_else(|_| "/tmp/chronos-s5-target/t".to_string());
    let adapter =
        ChronosMcpAdapter::spawn(&bin, None, std::time::Duration::from_secs(60)).expect("spawn");
    let req = RuntimeCaptureRequest {
        program: target.clone(),
        args: vec![],
        cwd: Some("/tmp".into()),
        timeout_ms: 30_000,
    };
    let capture = adapter.capture(&req).expect("capture");
    assert!(capture.total_events > 0, "expected events");
    assert!(!capture.session_id.is_empty());
    // digest stable shape: 64 hex chars
    assert_eq!(capture.digest.0.len(), 64);

    // claim evaluation through the domain
    let canonical = capture.to_canonical_observation_set();
    let verdict = sddk_engine::verify_kernel::engine::VerifyKernel::evaluate(
        &VerificationClaim::RuntimeProvider(RuntimeProviderClaim {
            subject_tag: format!("unit:runtime:{target}"),
            contract_id: "ext-s5".into(),
            min_events: 1,
        }),
        &canonical,
        &RuntimeProviderDomain,
    );
    assert_eq!(verdict, VerificationResult::Verified);
}

#[test]
#[ignore = "EXT profile: requires a real chronos-mcp binary"]
fn events_below_threshold_is_unknown() {
    let bin = mcp_bin();
    let target = std::env::var("CHRONOS_TARGET_BIN")
        .unwrap_or_else(|_| "/tmp/chronos-s5-target/t".to_string());
    let adapter =
        ChronosMcpAdapter::spawn(&bin, None, std::time::Duration::from_secs(60)).expect("spawn");
    let capture = adapter
        .capture(&RuntimeCaptureRequest {
            program: target.clone(),
            args: vec![],
            cwd: Some("/tmp".into()),
            timeout_ms: 30_000,
        })
        .expect("capture");
    let canonical = capture.to_canonical_observation_set();
    let verdict = sddk_engine::verify_kernel::engine::VerifyKernel::evaluate(
        &VerificationClaim::RuntimeProvider(RuntimeProviderClaim {
            subject_tag: format!("unit:runtime:{target}"),
            contract_id: "ext-s5".into(),
            min_events: u64::MAX, // unsatisfiable threshold
        }),
        &canonical,
        &RuntimeProviderDomain,
    );
    assert!(matches!(
        verdict,
        VerificationResult::Unknown {
            gap: EvidenceGap::Custom(ref msg)
        } if msg.starts_with("insufficient_runtime_capture")
    ));
}

#[test]
fn spawn_fails_closed_on_missing_binary() {
    let r = ChronosMcpAdapter::spawn(
        "/nonexistent/chronos-mcp",
        None,
        std::time::Duration::from_secs(5),
    );
    assert!(r.is_err());
}
