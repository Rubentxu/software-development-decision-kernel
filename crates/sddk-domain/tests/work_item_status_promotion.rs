//! Tests for WorkItemStatus::resolve_for_projection (AC-PLN4-02).
//!
//! The PromotionBlocked variant is a projection-layer status that applies
//! when a spine-imported item has PROPOSED spine_status and missing exit_gate.

use sddk_domain::planning::WorkItemStatus;

// ── AC-PLN4-02: PromotionBlocked scenarios ─────────────────────────────────

/// Scenario: spine PROPOSED + NULL exit_gate → PromotionBlocked.
#[test]
fn resolve_proposed_null_exit_gate_promotion_blocked() {
    let result =
        WorkItemStatus::resolve_for_projection(WorkItemStatus::Draft, Some("proposed"), None);
    assert_eq!(
        result,
        WorkItemStatus::PromotionBlocked,
        "PROPOSED + NULL exit_gate must resolve to PromotionBlocked"
    );
}

/// Scenario: spine PROPOSED + empty exit_gate → PromotionBlocked.
#[test]
fn resolve_proposed_empty_exit_gate_promotion_blocked() {
    let result =
        WorkItemStatus::resolve_for_projection(WorkItemStatus::Draft, Some("proposed"), Some(""));
    assert_eq!(
        result,
        WorkItemStatus::PromotionBlocked,
        "PROPOSED + empty exit_gate must resolve to PromotionBlocked"
    );
}

/// Scenario: spine PROPOSED + present exit_gate → Draft (promotable).
#[test]
fn resolve_proposed_with_exit_gate_draft() {
    let result = WorkItemStatus::resolve_for_projection(
        WorkItemStatus::Draft,
        Some("proposed"),
        Some("Acceptance criteria met."),
    );
    assert_eq!(
        result,
        WorkItemStatus::Draft,
        "PROPOSED + exit_gate must stay Draft (promotable)"
    );
}

/// Scenario: ACTIVE status is unaffected by resolve_for_projection.
#[test]
fn resolve_active_unchanged() {
    let result =
        WorkItemStatus::resolve_for_projection(WorkItemStatus::Active, Some("active"), None);
    assert_eq!(
        result,
        WorkItemStatus::Active,
        "ACTIVE status must remain Active"
    );
}

/// Scenario: non-spine item (spine_status = None) is not PromotionBlocked.
#[test]
fn resolve_non_spine_item_not_blocked() {
    let result = WorkItemStatus::resolve_for_projection(WorkItemStatus::Draft, None, None);
    assert_eq!(
        result,
        WorkItemStatus::Draft,
        "Non-spine Draft item must stay Draft"
    );
}
