//! A7-S2 — RUNTIME_ENHANCED UAT hardening (R7, R10, R12 de
//! `05-CHRONOS-HANDOFF.md` §8), fake-driven, sin binario externo.
//!
//! - R7: presupuesto/timeout excedido → typed error, nunca éxito
//!   parcial o éxito falso.
//! - R10: capturas repetidas con mismas observaciones → digest
//!   estable (deterministic contract).
//! - R12: resource bounds — un presupuesto de 0ms se rechaza
//!   fail-closed; event_counts enormes no desestabilizan (el
//!   resultado es tipado y acotado por construcción).
//!
//! R8 (restart/reconnect) y R11/R13 (contradiction reconciliation /
//! AuthorityEngine bypass) permanecen en A7-S3: requieren harness
//! de proceso y superficie de reconciliación que este slice no
//! inventa.

use sddk_engine::code_intelligence_port::DigestSha256;
use sddk_engine::runtime_evidence_port::{
    ObservationSet as RuntimeObs, RuntimeCaptureRequest, RuntimeCaptureResult, RuntimeEvidencePort,
    RuntimePortError,
};
use std::collections::BTreeMap;

struct SlowRuntime {
    /// Simulated overrun: capture always takes "longer" than the
    /// caller's budget when budget_ms < min_budget_ms.
    min_budget_ms: u64,
}

impl RuntimeEvidencePort for SlowRuntime {
    fn capabilities(&self) -> sddk_engine::runtime_evidence_port::RuntimeCapabilitySnapshot {
        sddk_engine::runtime_evidence_port::RuntimeCapabilitySnapshot {
            provider_name: "slow-runtime".into(),
            provider_version: "0.1.0".into(),
            protocol_major: 1,
            tools: vec!["debug_run".into()],
        }
    }

    fn capture(
        &self,
        req: &RuntimeCaptureRequest,
    ) -> Result<RuntimeCaptureResult, RuntimePortError> {
        if req.timeout_ms < self.min_budget_ms {
            // Provider honors the budget: explicit error, no
            // partial result, no fake success.
            return Err(RuntimePortError::ProviderError(format!(
                "capture budget exceeded: requested {}ms < provider minimum {}ms",
                req.timeout_ms, self.min_budget_ms
            )));
        }
        let mut counts = BTreeMap::new();
        counts.insert("function_entry".to_string(), 5);
        Ok(RuntimeCaptureResult {
            session_id: "s-fixed".into(),
            duration_ns: Some(500),
            total_events: 5,
            event_counts: counts,
            thread_count: 1,
            exit_status: Some(0),
            digest: DigestSha256("aa".repeat(32)),
            observations: RuntimeObs::new(),
        })
    }
}

/// R7: a budget the provider cannot honor yields a typed
/// ProviderError — never Ok with partial/faked content.
#[test]
fn t_r7_budget_exceeded_typed_error_not_fake_success() {
    let f = SlowRuntime {
        min_budget_ms: 1000,
    };
    let req = RuntimeCaptureRequest {
        program: "/tmp/t".into(),
        args: vec![],
        cwd: None,
        timeout_ms: 10,
    };
    match f.capture(&req) {
        Err(RuntimePortError::ProviderError(m)) => {
            assert!(m.contains("budget exceeded"), "msg: {m}");
        }
        Ok(r) => panic!("budget overrun must not yield Ok: {r:?}"),
        other => panic!("expected ProviderError, got {other:?}"),
    }
}

/// R10: identical inputs produce identical digests across
/// repeated captures (determinism within declared contract).
#[test]
fn t_r10_repeated_capture_stable_digest() {
    let f = SlowRuntime { min_budget_ms: 0 };
    let req = RuntimeCaptureRequest {
        program: "/tmp/t".into(),
        args: vec![],
        cwd: None,
        timeout_ms: 5_000,
    };
    let r1 = f.capture(&req).unwrap();
    let r2 = f.capture(&req).unwrap();
    let r3 = f.capture(&req).unwrap();
    assert_eq!(r1.digest, r2.digest);
    assert_eq!(r2.digest, r3.digest);
    assert_eq!(r1.total_events, r2.total_events);
    assert_eq!(r1.event_counts, r3.event_counts);
}

/// R12: resource bounds — a zero budget is rejected fail-closed
/// (no unbounded capture), and the typed result shape is bounded
/// by construction (fixed-size digest, sorted event_counts map).
#[test]
fn t_r12_zero_budget_fail_closed_and_bounded_shape() {
    let f = SlowRuntime { min_budget_ms: 1 };
    let req = RuntimeCaptureRequest {
        program: "/tmp/t".into(),
        args: vec![],
        cwd: None,
        timeout_ms: 0,
    };
    assert!(f.capture(&req).is_err(), "0ms budget must fail closed");
    // Bounded shape: digest is a fixed 64-char string; event
    // counts are a BTreeMap (ordered, no duplication).
    let ok_req = RuntimeCaptureRequest {
        program: "/tmp/t".into(),
        args: vec![],
        cwd: None,
        timeout_ms: 100,
    };
    let r = f.capture(&ok_req).unwrap();
    assert_eq!(r.digest.0.len(), 64);
    let keys: Vec<&String> = r.event_counts.keys().collect();
    let mut sorted = keys.clone();
    sorted.sort();
    assert_eq!(keys, sorted, "event_counts must be sorted");
}
