//! RED tests for `RunStateView` and `build_run_state_view`.
//!
//! Spec: ~/.sddk-knowledge/sddk-framework/specs/engine/REQ-CurrentRunView-Shape.md
//! ADR:  ~/.sddk-knowledge/sddk-framework/adrs/ADR-075-CURRENT-RUN-VIEW-SHAPE.md

use sddk_engine::{RunOrigin, RunStateView, build_run_state_view};

#[test]
fn run_state_view_for_declared_run_in_open_has_frontier() {
    // Scenario: RunStateView for a declared cycle run in OPEN
    let view = build_run_state_view(
        "R-declared-001",
        42,
        RunOrigin::Declared,
        vec!["node-a".to_string()],
        vec![],
        vec![],
        42,
    )
    .expect("build_run_state_view should succeed");

    assert_eq!(view.origin(), RunOrigin::Declared);
    assert_eq!(view.run_id(), "R-declared-001");
    assert_eq!(view.frontier(), &["node-a".to_string()]);
    assert!(view.blockers().is_empty());
    assert!(view.pending_decisions().is_empty());
    assert_eq!(view.evaluated_at(), 42);
}

#[test]
fn run_state_view_for_generated_run_with_pending_choice_has_decision() {
    // Scenario: RunStateView for a generated run in Running with a pending Choice
    let view = build_run_state_view(
        "R-gen-002",
        17,
        RunOrigin::Generated,
        vec![],
        vec!["blocker-from-choice".to_string()],
        vec!["decision-choice-1".to_string()],
        17,
    )
    .expect("build should succeed");

    assert_eq!(view.origin(), RunOrigin::Generated);
    assert!(view.frontier().is_empty());
    assert_eq!(view.blockers(), &["blocker-from-choice".to_string()]);
    assert_eq!(
        view.pending_decisions(),
        &["decision-choice-1".to_string()]
    );
}

#[test]
fn run_state_view_is_deterministic_across_rebuilds() {
    // Scenario: Determinism across rebuilds
    let a = build_run_state_view(
        "R-deterministic",
        100,
        RunOrigin::Generated,
        vec!["n1".to_string(), "n2".to_string()],
        vec!["b1".to_string()],
        vec![],
        100,
    )
    .expect("build a");

    let b = build_run_state_view(
        "R-deterministic",
        100,
        RunOrigin::Generated,
        vec!["n1".to_string(), "n2".to_string()],
        vec!["b1".to_string()],
        vec![],
        100,
    )
    .expect("build b");

    // Byte-equal via serde round-trip
    let ja = serde_json::to_string(&a).expect("serialize a");
    let jb = serde_json::to_string(&b).expect("serialize b");
    assert_eq!(ja, jb);
}

#[test]
fn run_state_view_for_terminal_run_has_empty_frontier() {
    // Scenario: Terminal run has empty frontier and no actions
    let view = build_run_state_view(
        "R-terminal-003",
        1,
        RunOrigin::Generated,
        vec![],
        vec![],
        vec![],
        1,
    )
    .expect("build terminal view");

    assert!(view.frontier().is_empty());
    assert!(view.blockers().is_empty());
    assert!(view.pending_decisions().is_empty());
}
