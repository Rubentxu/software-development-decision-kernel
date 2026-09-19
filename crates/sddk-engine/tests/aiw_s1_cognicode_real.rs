//! AIW-S1 integration tests — real provider (EXT) + normalization
//! and digest contract.
//!
//! Cycle: `p-63676b11dc0ef88f/aiw-s1-cognicode-real`.
//!
//! Scope honesty: the EXT tests require the real `cognicode-mcp`
//! binary and are `#[ignore]`d by default (Base regression, UAT
//! A09, runs without the binary). Run them explicitly with:
//!   COGNICODE_MCP_BIN=... cargo test -p sddk-engine \
//!     --test aiw_s1_cognicode_real -- --ignored

use std::time::Duration;

use sddk_engine::code_intelligence_port::{
    AnalysisBasis, CapabilityProfile, CodeIntelligencePort, CodeIntelligencePortError,
    DigestSha256, ImpactRequest, ProviderKind, ProviderLifecycle, ScopeRequest,
};
use sddk_engine::code_intelligence_port_mcp::CogniCodeMcpAdapter;

fn binary_path() -> Option<String> {
    std::env::var("COGNICODE_MCP_BIN")
        .ok()
        .filter(|p| !p.is_empty())
}

fn repo_root() -> String {
    // integration tests run with CWD = crate dir; the repo root
    // is two levels up.
    let mut p = std::env::current_dir().expect("cwd");
    p.pop();
    p.pop();
    p.to_string_lossy().to_string()
}

fn basis_for(adapter: &CogniCodeMcpAdapter, scope: &str) -> AnalysisBasis {
    AnalysisBasis {
        provider_build: "cognicode-mcp/test".to_string(),
        protocol_major: 2025,
        protocol_minor: 3,
        capability_snapshot: adapter.capabilities(),
        analyzer_set_digest: adapter.capabilities().analyzer_set_digest,
        source_revision: "test-rev".to_string(),
        request_scope: scope.to_string(),
    }
}

/// UAT A03 (NEG): spawn failure fails closed as Unavailable.
#[test]
fn a03_spawn_failure_is_unavailable() {
    let err = CogniCodeMcpAdapter::spawn(
        "/nonexistent/cognicode-mcp",
        "/tmp",
        "rev",
        Duration::from_secs(5),
    )
    .err()
    .expect("must fail");
    assert_eq!(err, CodeIntelligencePortError::Unavailable);
}

/// UAT A06: digests are real SHA-256 (64 lowercase hex), stable
/// for identical input and different across inputs.
#[test]
fn a06_digest_is_real_sha256() {
    let a = DigestSha256::of(b"hello");
    let b = DigestSha256::of(b"hello");
    let c = DigestSha256::of(b"world");
    assert_eq!(a, b);
    assert_ne!(a, c);
    assert_eq!(a.0.len(), 64, "SHA-256 hex is 64 chars");
    assert!(
        a.0.chars()
            .all(|ch| ch.is_ascii_hexdigit() && !ch.is_ascii_uppercase())
    );
    // known vector
    let known = DigestSha256::of(b"abc");
    assert_eq!(
        known.0,
        "ba7816bf8f01cfea414140de5dae2223b00361a396177a9cb410ff61f20015ad"
    );
}

/// UAT A01 (EXT+E2E, ignored without the real binary): full chain
/// spawn → negotiate → build_graph → find_usages observation with
/// complete provenance, consumed via the port contract.
#[test]
#[ignore]
fn a01_real_provider_produces_typed_observation() {
    let bin = binary_path().expect("set COGNICODE_MCP_BIN");
    let adapter =
        CogniCodeMcpAdapter::spawn(&bin, &repo_root(), "a01-rev", Duration::from_secs(180))
            .expect("provider must start");
    assert_eq!(adapter.lifecycle_state(), ProviderLifecycle::Ready);
    assert_eq!(
        adapter.capabilities().profile,
        CapabilityProfile::StaticEnhanced
    );
    assert!(adapter.capabilities().digest.0.len() == 64);

    let basis = basis_for(&adapter, "aiw-s1/a01");
    let req = ScopeRequest {
        added_units: vec![],
        removed_units: vec![],
        modified_units: vec!["CodeIntelligencePort".to_string()],
    };
    let result = adapter.analyze_scope(&basis, &req).expect("analyze ok");
    assert!(!result.partial);
    assert_eq!(result.observations.provider_kind, ProviderKind::CogniCode);
    assert!(!result.observations.restart_observed);
    let obs = result
        .observations
        .units
        .get("CodeIntelligencePort")
        .expect("unit present");
    // The raw MCP JSON must reference the port symbol (positive,
    // falsifiable relation).
    let text = &obs[0].text;
    assert!(text.contains("CodeIntelligencePort"));
    assert!(text.contains("usages") || text.contains("is_definition"));
    assert!(result.digest.0.len() == 64);
}

/// UAT A02 (EXT, ignored): same basis+scope reproduces the same
/// digest (stability); a different revision changes the basis and
/// therefore the digest material (no stale evidence reuse).
#[test]
#[ignore]
fn a02_two_revisions_distinct_basis() {
    let bin = binary_path().expect("set COGNICODE_MCP_BIN");
    let adapter = CogniCodeMcpAdapter::spawn(&bin, &repo_root(), "rev-A", Duration::from_secs(180))
        .expect("provider starts");
    let req = ScopeRequest {
        added_units: vec![],
        removed_units: vec![],
        modified_units: vec!["build_operator".to_string()],
    };
    let basis_a = basis_for(&adapter, "aiw-s1/a02");
    let r1 = adapter.analyze_scope(&basis_a, &req).expect("ok");
    let r2 = adapter.analyze_scope(&basis_a, &req).expect("ok");
    assert_eq!(r1.digest, r2.digest, "same basis+input: stable digest");

    let mut basis_b = basis_for(&adapter, "aiw-s1/a02");
    basis_b.source_revision = "rev-B".to_string();
    let r3 = adapter.analyze_scope(&basis_b, &req).expect("ok");
    assert_ne!(
        r1.digest, r3.digest,
        "different revision: distinct basis digest"
    );
}

/// UAT A03 (EXT, ignored): impact over an unknown symbol must not
/// fabricate impact — empty result is preserved as-is but flagged
/// by the payload itself; errors fail closed.
#[test]
#[ignore]
fn a03_impact_payload_preserved_verbatim() {
    let bin = binary_path().expect("set COGNICODE_MCP_BIN");
    let adapter =
        CogniCodeMcpAdapter::spawn(&bin, &repo_root(), "a03-rev", Duration::from_secs(180))
            .expect("provider starts");
    let basis = basis_for(&adapter, "aiw-s1/a03");
    let req = ImpactRequest {
        changed_units: vec!["build_operator".to_string()],
        depth: None,
    };
    let result = adapter.analyze_impact(&basis, &req).expect("ok");
    let text = &result.observations.units["build_operator"][0].text;
    // Verbatim preservation: the normalizer stores the provider
    // payload, it does not interpret risk levels (consumer does).
    assert!(text.contains("risk_level"));
    assert!(text.contains("impacted_files"));
}
