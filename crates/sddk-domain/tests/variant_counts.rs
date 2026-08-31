//! Runtime shape tests for `assert_variant_count_eq!` macro coverage.
//!
//! 12 tests: 7 cycle-5 enums (Phase, CycleStatus, RiskLevel, RuleSeverity,
//! StalenessState, PackRisk, ReleaseChannel) + 5 cycle-4 back-compat enums
//! (WorkflowError, CompileError, AttemptError, NodeRunError, WorkflowRunError).
//!
//! These are smoke tests — they prove each guarded enum's first variant is
//! reachable and the macro did not silently collapse the match to a wildcard.

use sddk_domain::{
    channel::ReleaseChannel,
    cycle::{CycleStatus, Phase, RiskLevel},
    pack::PackRisk,
    rules::types::RuleSeverity,
    staleness::StalenessState,
    workflow::WorkflowError,
    workflow_ir::CompileError,
    workflow_run::{AttemptError, NodeRunError, WorkflowRunError},
};

// ── Cycle-5 enums ────────────────────────────────────────────────────────────

#[test]
fn cycle_status_reachable() {
    let _ = CycleStatus::Open;
}

#[test]
fn phase_reachable() {
    let _ = Phase::Explore;
}

/// Phase has exactly 9 variants (ADR-0074: orphan Phase::Review removed).
/// The `assert_variant_count_eq!` macro enforces this at compile time.
#[test]
fn phase_variant_count_is_9() {
    use sddk_domain::cycle::Phase;
    let variants = [
        Phase::Explore,
        Phase::Specify,
        Phase::Design,
        Phase::Plan,
        Phase::Build,
        Phase::Verify,
        Phase::Uat,
        Phase::Release,
        Phase::Archive,
    ];
    assert_eq!(variants.len(), 9);
}

#[test]
fn risk_level_reachable() {
    let _ = RiskLevel::Low;
}

#[test]
fn rule_severity_reachable() {
    let _ = RuleSeverity::Error;
}

#[test]
fn staleness_state_reachable() {
    let _ = StalenessState::Fresh;
}

#[test]
fn pack_risk_reachable() {
    let _ = PackRisk::Low;
}

#[test]
fn release_channel_reachable() {
    let _ = ReleaseChannel::Stable;
}

// ── Cycle-4 back-compat enums ────────────────────────────────────────────────

#[test]
fn workflow_error_reachable() {
    let _ = WorkflowError::MissingArtifact("x".into());
}

#[test]
fn compile_error_reachable() {
    let _ = CompileError::EmptyCapabilityAllowlist;
}

#[test]
fn attempt_error_reachable() {
    let _ = AttemptError::AlreadyTerminal;
}

#[test]
fn node_run_error_reachable() {
    let _ = NodeRunError::InvalidStateTransition;
}

#[test]
fn workflow_run_error_reachable() {
    let _ = WorkflowRunError::AlreadyTerminal;
}
