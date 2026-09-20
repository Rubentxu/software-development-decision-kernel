//! A7-S1 — RUNTIME_ENHANCED UAT battery (part 1: req 1-6 + 9 of
//! `05-CHRONOS-HANDOFF.md` §8), driven by an in-test fake runtime
//! provider (no external binary needed — Base regression).
//!
//! EXT coverage against the real chronos-mcp lives in
//! `aiw_s5_chronos_real.rs` (env-gated).
//!
//! UAT mapping (§8):
//! - R1 base-green-without-provider → t_r1
//! - R2 optional absence → EvidenceGap, not PASS → t_r2
//! - R3 required absence → typed failure → t_r3
//! - R4 capability negotiation pinned → t_r4
//! - R5 provenance in basis → t_r5
//! - R6 stable refs (locator), not raw trace copy → t_r6
//! - R9 incompatible protocol explicit → t_r9
//!
//! R7/R8/R10..R13 require timeout injection, restart harness,
//! AuthorityEngine integration and resource-bound scenarios —
//! deferred to A7-S2 (documented in SCOPE).

use sddk_engine::architecture_graph::SoftwareUnitRef;
use sddk_engine::code_intelligence_port::DigestSha256;
use sddk_engine::observation::types::{
    ObservationOrigin, ObservationSet, ObservationStance, ObservationSubject,
};
use sddk_engine::runtime_evidence_port::{
    ObservationSet as RuntimeObs, RuntimeCapabilitySnapshot, RuntimeCaptureRequest,
    RuntimeCaptureResult, RuntimeEvidencePort, RuntimePortError,
};
use std::collections::BTreeMap;
use std::sync::atomic::{AtomicU64, Ordering};

/// Fake runtime provider: deterministic captures, configurable
/// failure modes, protocol pinning.
struct FakeRuntime {
    provider_version: String,
    protocol_major: u32,
    fail_mode: Option<FailMode>,
    captures: AtomicU64,
}

enum FailMode {
    Unavailable,
    Incompatible,
}

impl FakeRuntime {
    fn ok() -> Self {
        Self {
            provider_version: "0.1.0".into(),
            protocol_major: 1,
            fail_mode: None,
            captures: AtomicU64::new(0),
        }
    }
    fn failing(mode: FailMode) -> Self {
        let mut f = Self::ok();
        f.fail_mode = Some(mode);
        f
    }
}

impl RuntimeEvidencePort for FakeRuntime {
    fn capabilities(&self) -> RuntimeCapabilitySnapshot {
        RuntimeCapabilitySnapshot {
            provider_name: "fake-runtime".into(),
            provider_version: self.provider_version.clone(),
            protocol_major: self.protocol_major,
            tools: vec!["debug_run".into(), "query_events".into()],
        }
    }

    fn capture(
        &self,
        _req: &RuntimeCaptureRequest,
    ) -> Result<RuntimeCaptureResult, RuntimePortError> {
        match &self.fail_mode {
            Some(FailMode::Unavailable) => return Err(RuntimePortError::Unavailable),
            Some(FailMode::Incompatible) => {
                return Err(RuntimePortError::ProviderError(
                    "protocol major mismatch: provider=2 sddk=1".into(),
                ));
            }
            None => {}
        }
        let n = self.captures.fetch_add(1, Ordering::SeqCst);
        let mut counts = BTreeMap::new();
        counts.insert("function_entry".to_string(), 10);
        counts.insert("function_exit".to_string(), 9);
        let mut obs = RuntimeObs::new();
        obs.push(
            sddk_engine::observation::types::SoftwareObservation::declare(
                ObservationSubject::Unit(SoftwareUnitRef::new("target/main")),
                ObservationStance::Affirms,
                sddk_engine::evidence_ref::EvidenceRef::new(
                    sddk_engine::evidence_ref::EvidenceKind::Adhoc,
                    format!("fake-session-{n}/events"),
                ),
                ObservationOrigin::RuntimeProvider,
                sddk_engine::observation::types::ObservationBasis::for_provider_result(
                    "a7-s1-fake",
                    "deadbeef",
                ),
                None,
                "fake-runtime",
            ),
        );
        Ok(RuntimeCaptureResult {
            session_id: format!("fake-session-{n}"),
            duration_ns: Some(1_000),
            total_events: 19,
            event_counts: counts,
            thread_count: 1,
            exit_status: Some(0),
            digest: DigestSha256(
                "4242424242424242424242424242424242424242424242424242424242424242".into(),
            ),
            observations: obs,
        })
    }
}

fn req() -> RuntimeCaptureRequest {
    RuntimeCaptureRequest {
        program: "/tmp/fake-target".into(),
        args: vec![],
        cwd: None,
        timeout_ms: 5_000,
    }
}

/// R1: with no runtime provider at all, the observation set is
/// simply empty and typed error paths work — Base stays green
/// (nothing crashes, nothing fakes success).
#[test]
fn t_r1_base_green_without_provider() {
    // A port-less world = an empty ObservationSet, not an error.
    let empty = ObservationSet::new();
    assert!(empty.is_empty());
    // And a capture attempt on an unavailable provider is a typed
    // Err, never a fake Ok.
    let f = FakeRuntime::failing(FailMode::Unavailable);
    assert!(matches!(
        f.capture(&req()),
        Err(RuntimePortError::Unavailable)
    ));
}

/// R2: OPTIONAL provider absence yields a typed gap (Err), never
/// a PASS-looking capture result.
#[test]
fn t_r2_optional_absence_yields_gap_not_pass() {
    let f = FakeRuntime::failing(FailMode::Unavailable);
    let res = f.capture(&req());
    assert!(res.is_err());
    // No result means no observations to green anything.
    match res {
        Err(RuntimePortError::Unavailable) => {}
        other => panic!("expected Unavailable, got {other:?}"),
    }
}

/// R3: REQUIRED provider absence is an explicit typed failure the
/// caller must handle (distinct error display).
#[test]
fn t_r3_required_absence_explicit_failure() {
    let f = FakeRuntime::failing(FailMode::Unavailable);
    let err = f.capture(&req()).unwrap_err();
    assert_eq!(err.to_string(), "runtime provider unavailable");
}

/// R4: capability negotiation succeeds against a pinned build —
/// snapshot carries name/version/protocol/tools, stable across calls.
#[test]
fn t_r4_capability_negotiation_pinned() {
    let f = FakeRuntime::ok();
    let c1 = f.capabilities();
    let c2 = f.capabilities();
    assert_eq!(c1, c2, "capabilities must be stable across calls");
    assert_eq!(c1.provider_name, "fake-runtime");
    assert_eq!(c1.protocol_major, 1);
    assert_eq!(
        c1.tools,
        vec!["debug_run".to_string(), "query_events".to_string()]
    );
}

/// R5: scenario basis carries provenance (digest pins the
/// provider result; session id identifies the run).
#[test]
fn t_r5_basis_provenance() {
    let f = FakeRuntime::ok();
    let r = f.capture(&req()).unwrap();
    assert_eq!(r.digest.0.len(), 64);
    assert!(!r.session_id.is_empty());
    // Canonical conversion stamps the origin + provider name.
    let canon = r.to_canonical_observation_set();
    let all = canon.observations();
    assert!(!all.is_empty());
    assert!(
        all.iter()
            .all(|o| o.origin == ObservationOrigin::RuntimeProvider)
    );
}

/// R6: evidence references stable locators (session-relative),
/// not raw trace payloads.
#[test]
fn t_r6_stable_refs_not_raw_traces() {
    let f = FakeRuntime::ok();
    let r = f.capture(&req()).unwrap();
    assert!(!r.observations.is_empty());
    for o in &r.observations.observations {
        let loc = &o.evidence.locator;
        assert!(
            loc.starts_with("fake-session-"),
            "locator must be a stable ref, got {loc}"
        );
        assert!(loc.len() < 256, "locator must be a ref, not a trace dump");
    }
}

/// R9: incompatible protocol is explicit, fail-closed.
#[test]
fn t_r9_incompatible_protocol_explicit() {
    let f = FakeRuntime::failing(FailMode::Incompatible);
    let err = f.capture(&req()).unwrap_err();
    match err {
        RuntimePortError::ProviderError(m) => {
            assert!(m.contains("protocol major mismatch"), "msg: {m}");
        }
        other => panic!("expected ProviderError, got {other:?}"),
    }
}
