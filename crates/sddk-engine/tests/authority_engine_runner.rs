// Copyright (c) SDDK contributors.
// SPDX-License-Identifier: MIT
//
// authority_engine_runner.rs — SPEC-M5: 8 integration scenarios over the
// `AuthorityEngineRunner` facade.

use sddk_engine::authority_engine::{
    ActionKind, AdmissionDecision, AuthorityEngineError, AuthorityEngineRunner, Facts,
    RunnerVerdict,
};

fn runner() -> AuthorityEngineRunner {
    AuthorityEngineRunner::new()
}

// SC-M5-1: High band → RequireApproval.
#[test]
fn sc_m5_1_high_band_requires_approval() {
    let v: RunnerVerdict = runner()
        .admit_surface(
            "user:alice",
            "cycle_state",
            ActionKind::CycleTransition,
            "c1",
            Facts::default(),
        )
        .expect("ok");
    assert!(matches!(
        v.decision,
        AdmissionDecision::RequireApproval { .. }
    ));
    assert_eq!(v.capability, "surface.cycle_state");
}

// SC-M5-2: Low band + matching capability → Allow.
#[test]
fn sc_m5_2_low_band_allows() {
    let v = runner()
        .admit_surface(
            "system",
            "ledger_events",
            ActionKind::CliRun,
            "evt-1",
            Facts::default(),
        )
        .expect("ok");
    assert!(matches!(v.decision, AdmissionDecision::Allow { .. }));
}

// SC-M5-3: Unknown surface → Err(InternalContractBug).
#[test]
fn sc_m5_3_unknown_surface_returns_error() {
    let r = runner().admit_surface(
        "user:alice",
        "does_not_exist",
        ActionKind::CycleStart,
        "x",
        Facts::default(),
    );
    assert!(matches!(
        r,
        Err(AuthorityEngineError::InternalContractBug { .. })
    ));
}

// SC-M5-4: Human on Medium band with `plan.write` capability → Allow.
#[test]
fn sc_m5_4_medium_band_allows_human() {
    let v = runner()
        .admit_surface(
            "user:alice",
            "evidence_attachment",
            ActionKind::PlanEvidence,
            "p1",
            Facts::default(),
        )
        .expect("ok");
    assert!(matches!(v.decision, AdmissionDecision::Allow { .. }));
}

// SC-M5-5: RunnerVerdict always includes AdmissionExplanation with non-empty digest.
#[test]
fn sc_m5_5_explanation_digest_non_empty() {
    let v = runner()
        .admit_surface(
            "system",
            "ledger_events",
            ActionKind::CliRun,
            "evt-1",
            Facts::default(),
        )
        .expect("ok");
    assert!(!v.explanation.decision_digest.0.is_empty());
}

// SC-M5-6: Legacy AuthorityContext::validate stays working (compat mirror).
// We verify the runner does NOT replace the legacy path — both can coexist.
// WU-C1.3 (C1.3 hard-disable): `WritableSurface::LedgerEvents` now denies
// EVERY actor kind — domain events go through the canonical events_v1
// stream, so any new legacy-ledger write must fail closed. The other
// surfaces keep their legacy matrix behavior.
#[test]
fn sc_m5_6_runner_coexists_with_legacy_validate() {
    use sddk_engine::authority::{AuthorityContext, WritableSurface};
    let ctx = AuthorityContext::for_test(sddk_domain::ActorKind::System, "sys");
    // Deny contract for the retired surface (C1-HARDDISABLE-3).
    assert!(
        ctx.validate(WritableSurface::LedgerEvents).is_err(),
        "LedgerEvents must deny every actor kind since WU-C1.3"
    );
    // The legacy validate path itself still works for an admitted surface.
    assert!(ctx.validate(WritableSurface::CycleState).is_ok());
}

// WU-C1.3: LedgerEvents denies all three actor kinds (fail-closed surface).
#[test]
fn ledger_events_surface_is_denied_for_every_actor_kind() {
    use sddk_engine::authority::{AuthorityContext, WritableSurface};
    for (kind, id) in [
        (sddk_domain::ActorKind::Human, "user:alice"),
        (sddk_domain::ActorKind::Agent, "agent:sddk"),
        (sddk_domain::ActorKind::System, "sys"),
    ] {
        let ctx = AuthorityContext::for_test(kind, id);
        let outcome = ctx.validate(WritableSurface::LedgerEvents);
        assert!(outcome.is_err(), "{id} must be denied on ledger_events");
        let err = outcome.unwrap_err().to_string();
        assert!(
            err.contains("ledger_events"),
            "rejection must name the surface: {err}"
        );
    }
}

// SC-M5-7: Deterministic decision_id for same inputs.
#[test]
fn sc_m5_7_decision_id_deterministic() {
    let v = runner()
        .admit_surface(
            "user:alice",
            "cycle_state",
            ActionKind::CycleTransition,
            "c1",
            Facts::default(),
        )
        .expect("ok");
    assert_eq!(v.decision.decision_id(), "approval-human-cycle_transition");
}

// SC-M5-8: Runner is safe under non-panicking error paths.
#[test]
fn sc_m5_8_runner_never_panics() {
    for surface in [
        "cycle_state",
        "ledger_events",
        "gate_receipts",
        "plan_revisions",
        "transition_records",
        "framework_bundle",
        "github_releases",
        "knowledge_graph_vault",
        "plan_item",
        "dependency_edge",
        "evidence_attachment",
        "decision_record",
    ] {
        let _ = runner().admit_surface(
            "user:alice",
            surface,
            ActionKind::CliRun,
            "x",
            Facts::default(),
        );
    }
    // Unknown surfaces should never panic — they return Err.
    let r = runner().admit_surface(
        "user:alice",
        "no_such_surface",
        ActionKind::CliRun,
        "x",
        Facts::default(),
    );
    assert!(r.is_err());
}
