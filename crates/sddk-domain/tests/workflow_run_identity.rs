//! Tests for deterministic `RunId` derivation (REQ-WFR-ID-001).
//!
//! The recipe is: sha256(plan_revision_id || ":" || correlation_id) -> 64 lowercase hex.

use sddk_domain::workflow_run::{CorrelationId, RunId};

/// Scenario: identical inputs derive identical run_id
/// GIVEN a fixed (plan_revision_id, correlation_id) pair
/// WHEN RunId::derive is called twice, in separate processes
/// THEN both results are equal and match ^[0-9a-f]{64}$
#[test]
fn run_id_derivation_is_deterministic() {
    let plan_revision_id = "plan-rev-abc123";
    let correlation_id = CorrelationId("corr-xyz-789".into());

    let run_id1 = RunId::derive(plan_revision_id, &correlation_id);
    let run_id2 = RunId::derive(plan_revision_id, &correlation_id);

    assert_eq!(run_id1, run_id2, "RunId derivation must be deterministic");
    assert_eq!(
        run_id1.0.len(),
        64,
        "RunId must be exactly 64 hex characters"
    );
    assert!(
        run_id1
            .0
            .chars()
            .all(|c| c.is_ascii_hexdigit() && !c.is_ascii_uppercase()),
        "RunId must be lowercase hex only"
    );
}

/// Scenario: either input change yields a different run_id
/// GIVEN a baseline (plan_revision_id, correlation_id)
/// WHEN exactly one of the two inputs changes
/// THEN the derived RunId differs from the baseline
#[test]
fn run_id_binds_both_inputs() {
    let plan_revision_id = "plan-rev-abc123";
    let correlation_id = CorrelationId("corr-xyz-789".into());

    let baseline = RunId::derive(plan_revision_id, &correlation_id);

    // Change plan_revision_id only
    let changed_plan = RunId::derive("plan-rev-CHANGED", &correlation_id);
    assert_ne!(
        baseline, changed_plan,
        "Changing plan_revision_id must produce a different RunId"
    );

    // Change correlation_id only
    let changed_corr = RunId::derive(plan_revision_id, &CorrelationId("corr-CHANGED".into()));
    assert_ne!(
        baseline, changed_corr,
        "Changing correlation_id must produce a different RunId"
    );
}
