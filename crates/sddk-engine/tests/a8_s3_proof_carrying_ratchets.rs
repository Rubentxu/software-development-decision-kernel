//! A8-S3 — Proof-carrying changes + immune-system ratchets (AC14).
//!
//! Un defecto resuelto se convierte en protección durable:
//!
//! - PC-1: suite de mutaciones con digest — el "proof" viaja con el
//!   cambio (`MutationSuiteReceipt.digest`, determinista).
//! - PC-2: ratchet monótono — subir strictness está permitido,
//!   bajarla es rechazado (el defecto resuelto no vuelve).
//! - PC-3: firma válida requerida — política sin firma conocida es
//!   InvalidSignature/UnknownSigner; sin inyección de gate.
//! - PC-4: override expirado denegado — un waiver caducado no
//!   reabre la puerta.

use sddk_engine::architecture_mutation::run::{run_mutation_suite, suite_digest};
use sddk_engine::architecture_mutation::sandbox::MutationSandbox;
use sddk_engine::architecture_mutation::types::{
    GuardCheck, GuardId, GuardScope, MutationGuard, MutationId, MutationInjection, MutationKind,
    MutationSpec,
};
use sddk_engine::signed_gates::{
    DefaultSignedGateAuthority, GateOverride, GatePolicy, PolicyRatchet, SignedGateAuthority,
    Strictness,
};

fn policy(strictness: Strictness) -> GatePolicy {
    GatePolicy::new(
        "ac14-policy",
        strictness,
        "architect",
        "ab".repeat(32),
        "2026-09-20T00:00:00Z",
    )
}

/// PC-1: el receipt de la suite es el proof que acompaña al cambio;
/// mismo delta → mismo digest (evidencia portable y verificable).
#[test]
fn t_pc1_suite_digest_is_portable_proof() {
    let mk = || MutationSandbox::from_sources([("src/domain/service.rs", "pub fn handle() {}\n")]);
    let spec = MutationSpec {
        id: MutationId::new("pc-proof"),
        kind: MutationKind::ProviderTypeLeak,
        target_path: "src/domain/service.rs".into(),
        injection: MutationInjection::AppendLine("use tonic::Channel;".into()),
        expected_guard: GuardId::new("g"),
        expected_contract: None,
    };
    let guard = MutationGuard {
        id: GuardId::new("g"),
        scope: GuardScope::all(),
        check: GuardCheck::ForbiddenLines {
            forbidden: vec!["tonic".into()],
        },
    };
    let r1 = run_mutation_suite(
        &mk(),
        std::slice::from_ref(&spec),
        std::slice::from_ref(&guard),
    )
    .unwrap();
    let r2 = run_mutation_suite(&mk(), &[spec], &[guard]).unwrap();
    assert!(r1.all_detected, "solved defect → immune guard detects it");
    assert_eq!(r1.digest, r2.digest, "proof is deterministic");
    assert_eq!(suite_digest(&r1.probes), r1.digest);
}

/// PC-2: ratchet monótono — endurecer pasa, relajar se rechaza.
#[test]
fn t_pc2_ratchet_monotonic_no_regression() {
    let auth = DefaultSignedGateAuthority;
    let now = "2026-09-20T12:00:00Z";
    let signers = vec!["architect".to_string()];
    // Tighten: Standard → Strict is allowed.
    let tighten = auth
        .evaluate(
            &policy(Strictness::Strict),
            Some(PolicyRatchet::new(Strictness::Standard, Strictness::Strict)),
            None,
            &signers,
            Some(Strictness::Standard),
            "gate-ac14",
            now,
        )
        .expect("tightening allowed");
    assert!(tighten.ratchet.is_some(), "ratchet recorded");
    // Loosen: Strict → Lax is rejected.
    let loosen = auth.evaluate(
        &policy(Strictness::Lax),
        Some(PolicyRatchet::new(Strictness::Strict, Strictness::Lax)),
        None,
        &signers,
        Some(Strictness::Strict),
        "gate-ac14",
        now,
    );
    match loosen {
        Err(sddk_engine::signed_gates::GateError::NonMonotonicStrictness { from, to }) => {
            assert_eq!((from, to), (Strictness::Strict, Strictness::Lax));
        }
        other => panic!("expected NonMonotonicStrictness, got {other:?}"),
    }
}

/// PC-3: la política debe estar firmada por un signer conocido —
/// firma vacía o signer desconocido se rechazan.
#[test]
fn t_pc3_signature_required_known_signer() {
    let auth = DefaultSignedGateAuthority;
    let now = "2026-09-20T12:00:00Z";
    let signers = vec!["architect".to_string()];
    // Empty signature.
    let mut empty = policy(Strictness::Standard);
    empty.signature = String::new();
    let r = auth.evaluate(&empty, None, None, &signers, None, "g", now);
    assert!(matches!(
        r,
        Err(sddk_engine::signed_gates::GateError::EmptySignature)
    ));
    // Unknown signer.
    let mut stranger = policy(Strictness::Standard);
    stranger.signed_by = "attacker".into();
    let r2 = auth.evaluate(&stranger, None, None, &signers, None, "g", now);
    assert!(matches!(
        r2,
        Err(sddk_engine::signed_gates::GateError::UnknownSigner)
    ));
    // Valid signer passes.
    assert!(
        auth.evaluate(
            &policy(Strictness::Standard),
            None,
            None,
            &signers,
            None,
            "g",
            now
        )
        .is_ok()
    );
}

/// PC-4: un override (waiver) caducado no admite el cambio.
#[test]
fn t_pc4_expired_override_denied() {
    let auth = DefaultSignedGateAuthority;
    let now = "2026-09-20T12:00:00Z";
    let signers = vec!["architect".to_string()];
    let expired = GateOverride::Allow {
        reason: "legacy waiver".into(),
        expires_at: "2026-09-19T00:00:00Z".into(),
    };
    let r = auth.evaluate(
        &policy(Strictness::Standard),
        None,
        Some(expired),
        &signers,
        None,
        "gate-ac14",
        now,
    );
    assert!(
        matches!(
            r,
            Err(sddk_engine::signed_gates::GateError::ExpiredOverride)
        ),
        "expired waiver must not reopen the gate"
    );
}
