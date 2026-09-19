//! EXT tests for the Chronos MCP runtime evidence port, gated on
//! `CHRONOS_MCP_BIN` (skipped otherwise), mirroring
//! `aiw_s1_cognicode_real.rs`. Requires a target binary for probes;
//! `CHRONOS_TARGET_BIN` (default /tmp/chronos-s5-target/t).
use sddk_engine::runtime_evidence_port::{RuntimeCaptureRequest, RuntimeEvidencePort};
use sddk_engine::runtime_evidence_port_mcp::ChronosMcpAdapter;
use sddk_engine::verify_kernel::adapter_runtime_provider::{
    RuntimeProviderClaim, RuntimeProviderDomain,
};
use sddk_engine::verify_kernel::types::{EvidenceGap, VerificationClaim, VerificationResult};

fn mcp_bin() -> Option<String> {
    std::env::var("CHRONOS_MCP_BIN")
        .ok()
        .filter(|s| !s.is_empty())
}

#[test]
fn capture_produces_events_and_verified_claim() {
    let bin = match mcp_bin() {
        Some(b) => b,
        None => return,
    };
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
fn events_below_threshold_is_unknown() {
    let bin = match mcp_bin() {
        Some(b) => b,
        None => return,
    };
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
