//! RED tests for `FrontierProjection`-driven next-action derivation.
//!
//! Spec: ~/.sddk-knowledge/sddk-framework/specs/engine/REQ-NextActionDerivation.md
//! ADR:  ~/.sddk-knowledge/sddk-framework/adrs/ADR-076-NEXT-ACTION-DERIVATION.md
//!
//! These tests assert the projection rules from the persisted frontier onto
//! the closed `ActionKind` taxonomy. They use the `*_with_frontier` builder
//! and are paired with the existing heuristic tests in `action_surface_tests.rs`,
//! which continue to exercise the deprecated fallback path.

use sddk_engine::{
    ActionKind, DeclaredTransitionRef, FrontierEntryRef, FrontierProjection, PolicySnapshot,
    RunOrigin, RunStateView, build_action_surface_view_with_frontier, empty_projection,
};

fn state_running() -> RunStateView {
    RunStateView::for_test(
        RunOrigin::Generated,
        "R-dec-001",
        vec![],
        vec![],
        vec![],
        100,
    )
}

fn state_pending() -> RunStateView {
    RunStateView::for_test(
        RunOrigin::Declared,
        "R-dec-002",
        vec![],
        vec![],
        vec![],
        100,
    )
}

fn state_failed() -> RunStateView {
    RunStateView::for_test(
        RunOrigin::Generated,
        "R-dec-003",
        vec![],
        vec![],
        vec![],
        100,
    )
}

fn trans(id: &str, from: &str, to: &str) -> DeclaredTransitionRef {
    DeclaredTransitionRef::for_test(
        String::from(id),
        String::from(from),
        String::from(to),
        None,
        false,
        false,
        false,
        vec!["default".into()],
    )
}

fn entry(transition_id: &str, requires_met: bool) -> FrontierEntryRef {
    FrontierEntryRef::for_test(String::from(transition_id), requires_met)
}

#[test]
fn frontier_projection_empty_only_abort() {
    // Scenario: Empty frontier -> only Abort
    let state = state_running();
    let policy = PolicySnapshot::default();
    let projection = empty_projection();

    let surface = build_action_surface_view_with_frontier(&state, &policy, &projection)
        .expect("build surface should succeed");

    assert_eq!(surface.available_actions(), &[ActionKind::Abort]);
}

#[test]
fn frontier_projection_resume_from_running() {
    // Scenario: Resume derives from a single ready frontier entry
    let state = state_running();
    let policy = PolicySnapshot::default();
    let projection = FrontierProjection::for_test(
        vec![entry("T-resume", true)],
        vec![trans("T-resume", "Running", "Inspecting")],
    );

    let surface = build_action_surface_view_with_frontier(&state, &policy, &projection)
        .expect("build surface should succeed");

    assert!(surface.available_actions().contains(&ActionKind::Resume));
    assert!(surface.available_actions().contains(&ActionKind::Abort));
    assert!(!surface.available_actions().contains(&ActionKind::Start));
}

#[test]
fn frontier_projection_start_from_pending() {
    let state = state_pending();
    let policy = PolicySnapshot::default();
    let projection = FrontierProjection::for_test(
        vec![entry("T-start", true)],
        vec![trans("T-start", "Pending", "Running")],
    );

    let surface = build_action_surface_view_with_frontier(&state, &policy, &projection)
        .expect("build surface should succeed");

    assert!(surface.available_actions().contains(&ActionKind::Start));
    assert!(surface.available_actions().contains(&ActionKind::Abort));
    assert!(!surface.available_actions().contains(&ActionKind::Resume));
}

#[test]
fn frontier_projection_approve_only() {
    let state = state_running();
    let policy = PolicySnapshot::default();
    let mut t = trans("T-approve", "Running", "Approved");
    t.has_approval_gate = true;
    let projection = FrontierProjection::for_test(vec![entry("T-approve", true)], vec![t]);

    let surface = build_action_surface_view_with_frontier(&state, &policy, &projection)
        .expect("build surface should succeed");

    assert!(surface.available_actions().contains(&ActionKind::Approve));
    assert!(!surface.available_actions().contains(&ActionKind::Escalate));
}

#[test]
fn frontier_projection_escalate_only() {
    let state = state_running();
    let policy = PolicySnapshot::default();
    let mut t = trans("T-escalate", "Running", "Escalated");
    t.has_escalation_gate = true;
    let projection = FrontierProjection::for_test(vec![entry("T-escalate", true)], vec![t]);

    let surface = build_action_surface_view_with_frontier(&state, &policy, &projection)
        .expect("build surface should succeed");

    assert!(surface.available_actions().contains(&ActionKind::Escalate));
    assert!(!surface.available_actions().contains(&ActionKind::Approve));
}

#[test]
fn frontier_projection_retry_failed() {
    let state = state_failed();
    let policy = PolicySnapshot::default();
    let mut t = trans("T-retry", "Failed", "Pending");
    t.has_retry_policy = true;
    let projection = FrontierProjection::for_test(vec![entry("T-retry", true)], vec![t]);

    let surface = build_action_surface_view_with_frontier(&state, &policy, &projection)
        .expect("build surface should succeed");

    assert!(surface.available_actions().contains(&ActionKind::Retry));
}

#[test]
fn frontier_projection_reconcile_drifted() {
    let state = state_running();
    let policy = PolicySnapshot::default();
    let projection = FrontierProjection::for_test(
        vec![entry("T-reconcile", true)],
        vec![trans("T-reconcile", "Running", "Drifted")],
    );

    let surface = build_action_surface_view_with_frontier(&state, &policy, &projection)
        .expect("build surface should succeed");

    assert!(surface.available_actions().contains(&ActionKind::Reconcile));
}

#[test]
fn frontier_projection_multi_action_coexist() {
    // Scenario: Approve + Escalate coexist
    let state = state_running();
    let policy = PolicySnapshot::default();
    let mut t_app = trans("T-app", "Running", "Approved");
    t_app.has_approval_gate = true;
    let mut t_esc = trans("T-esc", "Running", "Escalated");
    t_esc.has_escalation_gate = true;
    let projection = FrontierProjection::for_test(
        vec![entry("T-app", true), entry("T-esc", true)],
        vec![t_app, t_esc],
    );

    let surface = build_action_surface_view_with_frontier(&state, &policy, &projection)
        .expect("build surface should succeed");

    assert!(surface.available_actions().contains(&ActionKind::Approve));
    assert!(surface.available_actions().contains(&ActionKind::Escalate));
}

#[test]
fn frontier_projection_determinism() {
    // Scenario: Determinism across processes
    let state = state_running();
    let policy = PolicySnapshot::default();
    let mut t_app = trans("T-app", "Running", "Approved");
    t_app.has_approval_gate = true;
    let mut t_esc = trans("T-esc", "Running", "Escalated");
    t_esc.has_escalation_gate = true;
    let projection = FrontierProjection::for_test(
        vec![entry("T-app", true), entry("T-esc", true)],
        vec![t_app, t_esc],
    );

    let s1 =
        build_action_surface_view_with_frontier(&state, &policy, &projection).expect("first build");
    let s2 = build_action_surface_view_with_frontier(&state, &policy, &projection)
        .expect("second build");

    assert_eq!(s1.available_actions(), s2.available_actions());
}

#[test]
fn frontier_projection_supersedes_heuristic() {
    // Scenario: Frontier-derived supersedes heuristic
    let state = RunStateView::for_test(
        RunOrigin::Generated,
        "R-dec-supersede",
        vec![],
        vec![],
        vec!["approval-X".into()], // heuristic would emit Approve
        100,
    );
    let policy = PolicySnapshot::default();
    let projection = FrontierProjection::for_test(
        vec![entry("T-other", true)],
        vec![trans("T-other", "Running", "Next")], // no approval gate
    );

    let surface = build_action_surface_view_with_frontier(&state, &policy, &projection)
        .expect("build surface should succeed");

    assert!(!surface.available_actions().contains(&ActionKind::Approve));
}
