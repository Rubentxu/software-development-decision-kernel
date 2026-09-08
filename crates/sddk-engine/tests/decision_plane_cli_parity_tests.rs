//! RED tests for Decision Plane CLI parity (typed_action_command + parity_check).
//!
//! Spec: ~/.sddk-knowledge/sddk-framework/specs/engine/REQ-DecisionPlaneCLIParity.md
//! ADR:  ~/.sddk-knowledge/sddk-framework/adrs/ADR-078-DECISION-PLANE-CLI-PARITY.md

use sddk_engine::{
    ActionCommandContext, ActionKind, DecisionReason, DecisionVerdict, ParityError,
    build_typed_action_command, check_decision_plane_parity,
};

fn ctx() -> ActionCommandContext {
    ActionCommandContext::for_test(
        "c-1",
        "alice",
        1,
        None,
        Some(String::from("t-explore.complete")),
    )
}

fn allow_record(kind: ActionKind) -> sddk_engine::DecisionRecord {
    sddk_engine::DecisionRecord::for_test(
        kind,
        DecisionVerdict::Allow,
        vec![DecisionReason::Closed {
            reason: "test".into(),
        }],
        vec![],
    )
}

#[test]
fn resume_produces_transition_command() {
    let decision = allow_record(ActionKind::Resume);
    let cmd = build_typed_action_command(&decision, &ctx())
        .expect("build command")
        .expect("Resume should be actionable");
    assert_eq!(
        cmd,
        "sddk cycle transition --cycle c-1 --transition t-explore.complete --lease-owner alice --fencing-token 1"
    );
}

#[test]
fn approve_adds_gate_approval_hint() {
    let decision = allow_record(ActionKind::Approve);
    let cmd = build_typed_action_command(&decision, &ctx())
        .expect("build command")
        .expect("Approve should be actionable");
    assert!(cmd.contains("--gate approval"), "command: {cmd}");
}

#[test]
fn escalate_adds_gate_escalation_hint() {
    let decision = allow_record(ActionKind::Escalate);
    let cmd = build_typed_action_command(&decision, &ctx())
        .expect("build command")
        .expect("Escalate should be actionable");
    assert!(cmd.contains("--gate escalation"), "command: {cmd}");
}

#[test]
fn deny_returns_none() {
    let decision = sddk_engine::DecisionRecord::for_test(
        ActionKind::Resume,
        DecisionVerdict::Deny,
        vec![DecisionReason::PolicyDenied {
            policy_id: "p".into(),
            kind: ActionKind::Resume,
        }],
        vec![],
    );
    let cmd = build_typed_action_command(&decision, &ctx()).expect("build command");
    assert!(cmd.is_none(), "Deny must not produce a command");
}

#[test]
fn require_approval_returns_none() {
    let decision = sddk_engine::DecisionRecord::for_test(
        ActionKind::Approve,
        DecisionVerdict::RequireApproval {
            approver_kind: sddk_engine::ApproverKind::Human,
        },
        vec![DecisionReason::ApprovalRequired {
            approver_kind: sddk_engine::ApproverKind::Human,
        }],
        vec![],
    );
    let cmd = build_typed_action_command(&decision, &ctx()).expect("build command");
    assert!(
        cmd.is_none(),
        "RequireApproval must not produce a direct command"
    );
}

#[test]
fn missing_transition_id_is_error() {
    let decision = allow_record(ActionKind::Resume);
    let mut c = ctx();
    c.clear_transition_id();
    let err =
        build_typed_action_command(&decision, &c).expect_err("missing transition_id must error");
    assert!(matches!(
        err,
        sddk_engine::CommandBuildError::MissingContext { .. }
    ));
}

#[test]
fn parity_check_passes_on_match() {
    let decision = allow_record(ActionKind::Resume);
    let expected = build_typed_action_command(&decision, &ctx())
        .unwrap()
        .unwrap();
    check_decision_plane_parity(&expected, &decision, &ctx()).expect("parity must pass");
}

#[test]
fn parity_check_fails_on_drift() {
    let decision = allow_record(ActionKind::Resume);
    let expected = build_typed_action_command(&decision, &ctx())
        .unwrap()
        .unwrap();
    // Remove the --fencing-token flag to simulate drift.
    let drifted: String = expected
        .split_whitespace()
        .filter(|w| !w.starts_with("--fencing-token") && *w != "1")
        .collect::<Vec<_>>()
        .join(" ");
    let err = check_decision_plane_parity(&drifted, &decision, &ctx())
        .expect_err("parity must fail on drift");
    assert!(matches!(err, ParityError::CommandMismatch { .. }));
}

#[test]
fn parity_check_rejects_unrelated_command() {
    // Resume is actionable (verdict=Allow), so an unrelated command
    // must surface as CommandMismatch (expected vs actual).
    let decision = allow_record(ActionKind::Resume);
    let err = check_decision_plane_parity("rm -rf /", &decision, &ctx())
        .expect_err("parity must reject unrelated command");
    assert!(matches!(err, ParityError::CommandMismatch { .. }));
}
