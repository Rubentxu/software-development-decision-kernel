//! RED tests for `ActionSurfaceView`, `ActionKind`, `PolicySnapshot` and
//! `build_action_surface_view`.
//!
//! Spec: ~/.sddk-knowledge/sddk-framework/specs/engine/REQ-CurrentRunView-Actions.md
//! ADR:  ~/.sddk-knowledge/sddk-framework/adrs/ADR-075-CURRENT-RUN-VIEW-SHAPE.md

#![allow(deprecated)]

use sddk_engine::run_view::build_action_surface_view;
use sddk_engine::{ActionKind, PolicySnapshot, RunOrigin, RunStateView};

fn state_with_frontier(frontier: Vec<String>) -> RunStateView {
    RunStateView::for_test(
        RunOrigin::Generated,
        "R-act-001",
        frontier,
        vec![],
        vec![],
        7,
    )
}

fn state_with_pending(pending: Vec<String>) -> RunStateView {
    RunStateView::for_test(
        RunOrigin::Generated,
        "R-act-002",
        vec![],
        vec![],
        pending,
        7,
    )
}

#[test]
fn resume_is_admitted_on_non_terminal_frontier_with_default_policy() {
    // Scenario: Resume admitted on a non-terminal frontier
    let state = state_with_frontier(vec!["n1".into()]);
    let policy = PolicySnapshot::default();
    let surface = build_action_surface_view(&state, &policy).expect("build surface should succeed");

    assert!(surface.available_actions().contains(&ActionKind::Resume));
    assert!(!surface.available_actions().contains(&ActionKind::Approve));
    assert!(!surface.available_actions().contains(&ActionKind::Escalate));
}

#[test]
fn approve_only_with_pending_approval() {
    // Scenario: Approve only with a pending approval
    let state = state_with_pending(vec!["approval-d".into()]);
    let policy = PolicySnapshot::default();
    let surface = build_action_surface_view(&state, &policy).expect("build surface should succeed");

    assert!(surface.available_actions().contains(&ActionKind::Approve));
    assert!(!surface.available_actions().contains(&ActionKind::Escalate));
}

#[test]
fn closed_taxonomy_no_free_form_actions() {
    // Scenario: Closed taxonomy — no free-form actions
    let state = state_with_frontier(vec!["n1".into()]);
    let policy = PolicySnapshot::default();
    let surface = build_action_surface_view(&state, &policy).expect("build surface should succeed");

    // Every element must be a recognized variant
    for action in surface.available_actions() {
        assert!(
            matches!(
                action,
                ActionKind::Start
                    | ActionKind::Resume
                    | ActionKind::Abort
                    | ActionKind::Approve
                    | ActionKind::Escalate
                    | ActionKind::Retry
                    | ActionKind::Reconcile
            ),
            "ActionKind must belong to closed taxonomy: {action:?}"
        );
    }
}

#[test]
fn policy_deny_overrides_preconditions() {
    // Scenario: Policy deny overrides preconditions
    let state = state_with_frontier(vec!["n1".into()]);
    let policy = PolicySnapshot::deny_resume();
    let surface = build_action_surface_view(&state, &policy).expect("build surface should succeed");

    assert!(!surface.available_actions().contains(&ActionKind::Resume));
}

#[test]
fn eager_no_work_on_read() {
    // Scenario: Eager — no work on read
    let state = state_with_frontier(vec!["n1".into()]);
    let policy = PolicySnapshot::default();
    let surface = build_action_surface_view(&state, &policy).expect("build surface should succeed");

    let r1 = surface.available_actions().as_ptr();
    let r2 = surface.available_actions().as_ptr();
    assert_eq!(r1, r2, "Eager: same Vec reference on repeated reads");
}

#[test]
fn two_policies_produce_distinct_surfaces() {
    // Scenario: Two policies produce distinct surfaces
    let state = state_with_frontier(vec!["n1".into()]);
    let s1 = build_action_surface_view(&state, &PolicySnapshot::default()).expect("build s1");
    let s2 = build_action_surface_view(&state, &PolicySnapshot::deny_resume()).expect("build s2");

    assert_ne!(s1.policy_digest(), s2.policy_digest());
    assert!(s1.available_actions().contains(&ActionKind::Resume));
    assert!(!s2.available_actions().contains(&ActionKind::Resume));
}
