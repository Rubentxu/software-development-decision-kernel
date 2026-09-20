//! A8-S2 — Counterfactual architecture (AC13,
//! `arch-spec-038` parte counterfactual; ADR-0116).
//!
//! Un plan de refactor contrafactual evalúa un delta candidato
//! sobre un sandbox EFÍMERO (clon), nunca sobre la base real:
//!
//! - CF-1: la evaluación del delta NO muta la base (aislamiento).
//! - CF-2: delta detectado por guard → el plan contrafactual se
//!   RECHAZA con la evidencia del guard (contrafactual con
//!   protección arquitectural).
//! - CF-3: delta no detectado (guard pasando) → candidato viable,
//!   recibido como MutationProbe con `detected=false`, decisión
//!   explícita del planificador, no un error.
//! - CF-4: determinismo — re-evaluar el mismo delta sobre la misma
//!   base produce idéntico resultado (mismo receipt digest).

use sddk_engine::architecture_mutation::run::run_mutation_probe;
use sddk_engine::architecture_mutation::sandbox::MutationSandbox;
use sddk_engine::architecture_mutation::types::{
    GuardCheck, GuardId, GuardScope, MutationGuard, MutationId, MutationInjection, MutationKind,
    MutationSpec,
};

fn base_sandbox() -> MutationSandbox {
    MutationSandbox::from_sources([
        ("src/domain/service.rs", "pub fn handle() {}\n"),
        ("src/domain/mod.rs", "mod service;\n"),
    ])
}

fn forbidden_guard(sub: &str) -> MutationGuard {
    MutationGuard {
        id: GuardId::new("cf-guard"),
        scope: GuardScope::all(),
        check: GuardCheck::ForbiddenLines {
            forbidden: vec![sub.to_string()],
        },
    }
}

fn spec(id: &str, injection: MutationInjection) -> MutationSpec {
    MutationSpec {
        id: MutationId::new(id),
        kind: MutationKind::ProviderTypeLeak,
        target_path: "src/domain/service.rs".into(),
        injection,
        expected_guard: GuardId::new("cf-guard"),
        expected_contract: None,
    }
}

/// CF-1: la evaluación contrafactual es aislada — la base original
/// no cambia tras evaluar un delta sobre su clon.
#[test]
fn t_cf1_counterfactual_isolation_base_untouched() {
    let base = base_sandbox();
    let delta = spec(
        "cf-leak",
        MutationInjection::AppendLine("use tonic::transport::Channel; // provider leak".into()),
    );
    let guard = forbidden_guard("tonic");
    let _probe = run_mutation_probe(&base, &delta, &guard);
    // Base is untouched: re-running the same probe on the base
    // yields the same first-run result (no accumulated mutation).
    let probe2 = run_mutation_probe(&base, &delta, &guard);
    assert!(probe2.mutation_applied);
    assert!(probe2.detected, "base unchanged → same detection");
}

/// CF-2: un delta que viola la arquitectura se detecta con
/// evidencia — el contrafactual se rechaza informado.
#[test]
fn t_cf2_violating_delta_detected_with_evidence() {
    let base = base_sandbox();
    let delta = spec(
        "cf-violation",
        MutationInjection::AppendLine("use tonic::transport::Channel;".into()),
    );
    let probe = run_mutation_probe(&base, &delta, &forbidden_guard("tonic"));
    assert!(probe.detected);
    assert!(
        !probe.evidence.is_empty(),
        "rejection carries guard evidence"
    );
}

/// CF-3: un delta compatible pasa los guards — `detected=false`
/// es un resultado válido del plan contrafactual, no un error.
#[test]
fn t_cf3_compatible_delta_viable_not_error() {
    let base = base_sandbox();
    let delta = spec(
        "cf-compatible",
        MutationInjection::AppendLine("pub fn pure_helper() -> u32 { 1 }".into()),
    );
    let probe = run_mutation_probe(&base, &delta, &forbidden_guard("tonic"));
    assert!(
        probe.mutation_applied,
        "candidate applied to ephemeral clone"
    );
    assert!(!probe.detected, "no violation → viable counterfactual");
    assert!(probe.evidence.is_empty());
}

/// CF-4: determinismo contrafactual — misma base + mismo delta →
/// idéntico resultado (aplicación, detección y evidencia).
#[test]
fn t_cf4_counterfactual_deterministic() {
    let base = base_sandbox();
    let delta = spec(
        "cf-det",
        MutationInjection::AppendLine("use tonic::Channel;".into()),
    );
    let guard = forbidden_guard("tonic");
    let a = run_mutation_probe(&base, &delta, &guard);
    let b = run_mutation_probe(&base, &delta, &guard);
    assert_eq!(a, b, "counterfactual evaluation is pure/deterministic");
}
