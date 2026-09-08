//! RED tests for typed policy evaluation with durable reasons + provenance.
//!
//! Spec: ~/.sddk-knowledge/sddk-framework/specs/engine/REQ-TypedPolicyEvaluation.md
//! ADR:  ~/.sddk-knowledge/sddk-framework/adrs/ADR-077-TYPED-POLICY-EVALUATION.md

use sddk_engine::{
    ActionKind, AdmissionRule, ApproverKind, DecisionReason, DecisionVerdict, PolicySnapshot,
    RunOrigin, RunStateView, TypedActionSurfaceView, build_action_surface_view_typed,
    empty_projection,
};

fn state_running() -> RunStateView {
    RunStateView::for_test(
        RunOrigin::Generated,
        "R-typed-001",
        vec![],
        vec![],
        vec![],
        100,
    )
}

fn policy_deny_resume() -> PolicySnapshot {
    let mut p = PolicySnapshot::default();
    p.insert(ActionKind::Resume, AdmissionRule::Deny);
    p
}

fn policy_require_approval_approve() -> PolicySnapshot {
    let mut p = PolicySnapshot::default();
    p.insert(
        ActionKind::Approve,
        AdmissionRule::RequireApproval {
            approver_kind: ApproverKind::Human,
        },
    );
    p
}

#[test]
fn typed_allow_with_provenance() {
    let state = state_running();
    let policy = PolicySnapshot::default();
    let projection = empty_projection();

    let view =
        build_action_surface_view_typed(&state, &policy, &projection).expect("build typed surface");

    let resume = view
        .decisions()
        .iter()
        .find(|d| d.kind() == ActionKind::Resume)
        .expect("Resume decision present");

    // Resume without frontier is not Allow.
    assert_ne!(*resume.verdict(), DecisionVerdict::Allow);
    assert!(!resume.reasons().is_empty());
}

#[test]
fn typed_deny_with_durable_reason() {
    let state = state_running();
    let policy = policy_deny_resume();
    let projection = empty_projection();

    let view =
        build_action_surface_view_typed(&state, &policy, &projection).expect("build typed surface");

    let resume = view
        .decisions()
        .iter()
        .find(|d| d.kind() == ActionKind::Resume)
        .expect("Resume decision present");

    assert_eq!(*resume.verdict(), DecisionVerdict::Deny);
    assert!(resume.reasons().iter().any(|r| matches!(
        r,
        DecisionReason::PolicyDenied { kind, .. } if *kind == ActionKind::Resume
    )));
    assert!(resume.provenance().iter().any(|p| p.is_policy_admission()));
}

#[test]
fn typed_require_approval_excludes_from_available_actions() {
    let state = state_running();
    let policy = policy_require_approval_approve();
    let projection = empty_projection();

    let view: TypedActionSurfaceView =
        build_action_surface_view_typed(&state, &policy, &projection).expect("build typed surface");

    assert!(!view.available_actions().contains(&ActionKind::Approve));
    let approve = view
        .decisions()
        .iter()
        .find(|d| d.kind() == ActionKind::Approve)
        .expect("Approve decision present");
    assert_eq!(
        *approve.verdict(),
        DecisionVerdict::RequireApproval {
            approver_kind: ApproverKind::Human
        }
    );
}

#[test]
fn typed_available_actions_is_allow_subset() {
    let state = state_running();
    let policy = policy_deny_resume();
    let projection = empty_projection();

    let view =
        build_action_surface_view_typed(&state, &policy, &projection).expect("build typed surface");

    // available_actions must be exactly the Allow subset of decisions
    let allow_kinds: Vec<ActionKind> = view
        .decisions()
        .iter()
        .filter(|d| *d.verdict() == DecisionVerdict::Allow)
        .map(|d| d.kind())
        .collect();
    assert_eq!(view.available_actions(), allow_kinds.as_slice());
}

#[test]
fn typed_decisions_have_seven_records_sorted_by_discriminant() {
    let state = state_running();
    let policy = PolicySnapshot::default();
    let projection = empty_projection();

    let view =
        build_action_surface_view_typed(&state, &policy, &projection).expect("build typed surface");

    assert_eq!(view.decisions().len(), 7);
    let kinds: Vec<ActionKind> = view.decisions().iter().map(|d| d.kind()).collect();
    let mut sorted = kinds.clone();
    sorted.sort_by_key(|k| sddk_engine::run_view::kind_discriminant_for_test(*k));
    assert_eq!(kinds, sorted);
}

#[test]
fn typed_reasons_and_provenance_are_non_empty() {
    let state = state_running();
    let policy = PolicySnapshot::default();
    let projection = empty_projection();

    let view =
        build_action_surface_view_typed(&state, &policy, &projection).expect("build typed surface");

    for d in view.decisions() {
        assert!(
            !d.reasons().is_empty(),
            "decision {:?} has empty reasons",
            d.kind()
        );
        assert!(
            !d.provenance().is_empty(),
            "decision {:?} has empty provenance",
            d.kind()
        );
    }
}

#[test]
fn typed_determinism_across_processes() {
    let state = state_running();
    let policy = PolicySnapshot::default();
    let projection = empty_projection();

    let v1 = build_action_surface_view_typed(&state, &policy, &projection).expect("v1");
    let v2 = build_action_surface_view_typed(&state, &policy, &projection).expect("v2");

    // Serialize both and compare bytes.
    let s1 = serde_json::to_string(&v1).expect("serialize v1");
    let s2 = serde_json::to_string(&v2).expect("serialize v2");
    assert_eq!(s1, s2);
}
