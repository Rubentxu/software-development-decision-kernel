//! Tests for `WorkItemStatus` state machine transitions and `DependencyResolutionService`.
//!
//! Covers per ADR-073 §3.2:
//! - All 7 legal transitions
//! - Illegal transition rejection
//! - Terminal-state rejection of outgoing transitions
//! - DependencyResolutionService interaction for Draft→Active and terminal transitions

use sddk_domain::planning::service::{DependencyResolutionError, DependencyResolutionService};
use sddk_domain::planning::{
    DependencyEdgeKind, DependencyEdgeV1, WORK_ITEM_SCHEMA_VERSION, WorkItemStatus, WorkItemV1,
};

/// Helper: build a WorkItemV1 with the given id and status.
fn work_item(id: &str, status: WorkItemStatus) -> WorkItemV1 {
    WorkItemV1 {
        id: id.to_string(),
        cycle_id: "cycle-1".to_string(),
        title: format!("title-{}", id),
        description: format!("description-{}", id),
        status,
        actor_ref: None,
        created_at: 0,
        schema_version: WORK_ITEM_SCHEMA_VERSION,
    }
}

/// Helper: status lookup closure from a map.
fn status_lookup_from<'a>(
    map: &'a [(&'static str, WorkItemStatus)],
) -> impl Fn(&sddk_domain::planning::WorkItemId) -> Option<WorkItemStatus> + 'a {
    move |id: &sddk_domain::planning::WorkItemId| {
        map.iter().find(|(k, _)| *k == id.as_str()).map(|(_, v)| *v)
    }
}

/// Helper: build a DependencyEdgeV1.
fn edge(from_id: &str, to_id: &str, kind: DependencyEdgeKind) -> DependencyEdgeV1 {
    DependencyEdgeV1::new(from_id.to_string(), to_id.to_string(), kind, None)
}

// ── Legal transitions ───────────────────────────────────────────────────────────

#[test]
fn draft_to_active_is_legal() {
    assert!(WorkItemStatus::Draft.can_transition_to(WorkItemStatus::Active));
}

#[test]
fn active_to_paused_is_legal() {
    assert!(WorkItemStatus::Active.can_transition_to(WorkItemStatus::Paused));
}

#[test]
fn paused_to_active_is_legal() {
    assert!(WorkItemStatus::Paused.can_transition_to(WorkItemStatus::Active));
}

#[test]
fn active_to_done_is_legal() {
    assert!(WorkItemStatus::Active.can_transition_to(WorkItemStatus::Done));
}

#[test]
fn active_to_superseded_is_legal() {
    assert!(WorkItemStatus::Active.can_transition_to(WorkItemStatus::Superseded));
}

#[test]
fn active_to_cancelled_is_legal() {
    assert!(WorkItemStatus::Active.can_transition_to(WorkItemStatus::Cancelled));
}

#[test]
fn paused_to_cancelled_is_legal() {
    assert!(WorkItemStatus::Paused.can_transition_to(WorkItemStatus::Cancelled));
}

// ── Illegal transitions ────────────────────────────────────────────────────────

#[test]
fn terminal_states_reject_all_outgoing_transitions() {
    // Done rejects everything
    assert!(!WorkItemStatus::Done.can_transition_to(WorkItemStatus::Active));
    assert!(!WorkItemStatus::Done.can_transition_to(WorkItemStatus::Paused));
    assert!(!WorkItemStatus::Done.can_transition_to(WorkItemStatus::Draft));
    assert!(!WorkItemStatus::Done.can_transition_to(WorkItemStatus::Done));
    assert!(!WorkItemStatus::Done.can_transition_to(WorkItemStatus::Superseded));
    assert!(!WorkItemStatus::Done.can_transition_to(WorkItemStatus::Cancelled));

    // Superseded rejects everything
    assert!(!WorkItemStatus::Superseded.can_transition_to(WorkItemStatus::Active));
    assert!(!WorkItemStatus::Superseded.can_transition_to(WorkItemStatus::Paused));
    assert!(!WorkItemStatus::Superseded.can_transition_to(WorkItemStatus::Done));
    assert!(!WorkItemStatus::Superseded.can_transition_to(WorkItemStatus::Draft));

    // Cancelled rejects everything
    assert!(!WorkItemStatus::Cancelled.can_transition_to(WorkItemStatus::Active));
    assert!(!WorkItemStatus::Cancelled.can_transition_to(WorkItemStatus::Paused));
    assert!(!WorkItemStatus::Cancelled.can_transition_to(WorkItemStatus::Done));
    assert!(!WorkItemStatus::Cancelled.can_transition_to(WorkItemStatus::Draft));
}

#[test]
fn draft_cannot_skip_directly_to_terminal() {
    // Draft cannot go directly to Done/Superseded/Cancelled
    assert!(!WorkItemStatus::Draft.can_transition_to(WorkItemStatus::Done));
    assert!(!WorkItemStatus::Draft.can_transition_to(WorkItemStatus::Superseded));
    assert!(!WorkItemStatus::Draft.can_transition_to(WorkItemStatus::Cancelled));
}

#[test]
fn paused_cannot_go_to_done_or_superseded() {
    // Paused cannot go to terminal (must go through Active)
    assert!(!WorkItemStatus::Paused.can_transition_to(WorkItemStatus::Done));
    assert!(!WorkItemStatus::Paused.can_transition_to(WorkItemStatus::Superseded));
}

#[test]
fn active_cannot_go_back_to_draft() {
    assert!(!WorkItemStatus::Active.can_transition_to(WorkItemStatus::Draft));
}

// ── DependencyResolutionService integration ─────────────────────────────────────

#[test]
fn resolve_transition_draft_to_active_blocks_when_pred_is_non_terminal() {
    // A (Draft) blocks B from Draft→Active via Blocks edge
    let target = work_item("B", WorkItemStatus::Draft);
    let edges = vec![edge("A", "B", DependencyEdgeKind::Blocks)];
    let lookup = status_lookup_from(&[("A", WorkItemStatus::Draft)]);

    let result = DependencyResolutionService::resolve_transition(
        &edges,
        &target,
        WorkItemStatus::Active,
        &lookup,
    );

    assert!(matches!(
        result,
        Err(DependencyResolutionError::BlockedBy { .. })
    ));
}

#[test]
fn resolve_transition_draft_to_active_allows_when_pred_is_terminal() {
    // A (Done) does not block B from Draft→Active
    let target = work_item("B", WorkItemStatus::Draft);
    let edges = vec![edge("A", "B", DependencyEdgeKind::Blocks)];
    let lookup = status_lookup_from(&[("A", WorkItemStatus::Done)]);

    let result = DependencyResolutionService::resolve_transition(
        &edges,
        &target,
        WorkItemStatus::Active,
        &lookup,
    );

    assert!(result.is_ok());
}

#[test]
fn resolve_transition_to_terminal_blocks_with_blocks_on_closure() {
    // A (Active) → B via BlocksOnClosure blocks B's terminal transition
    let target = work_item("B", WorkItemStatus::Active);
    let edges = vec![edge("A", "B", DependencyEdgeKind::BlocksOnClosure)];
    let lookup = status_lookup_from(&[("A", WorkItemStatus::Active)]);

    let result = DependencyResolutionService::resolve_transition(
        &edges,
        &target,
        WorkItemStatus::Done,
        &lookup,
    );

    assert!(matches!(
        result,
        Err(DependencyResolutionError::BlockedBy {
            kind: DependencyEdgeKind::BlocksOnClosure,
            ..
        })
    ));
}

#[test]
fn resolve_transition_to_terminal_allows_when_all_preds_terminal() {
    // Both preds (Done) do not block B's terminal transition
    let target = work_item("B", WorkItemStatus::Active);
    let edges = vec![
        edge("A", "B", DependencyEdgeKind::Blocks),
        edge("C", "B", DependencyEdgeKind::BlocksOnClosure),
    ];
    let lookup = status_lookup_from(&[
        ("A", WorkItemStatus::Done),
        ("C", WorkItemStatus::Superseded),
    ]);

    let result = DependencyResolutionService::resolve_transition(
        &edges,
        &target,
        WorkItemStatus::Done,
        &lookup,
    );

    assert!(result.is_ok());
}

#[test]
fn self_loop_via_resolve_transition_rejected() {
    // A → A self-loop is detected
    let target = work_item("A", WorkItemStatus::Draft);
    let edges = vec![edge("A", "A", DependencyEdgeKind::Blocks)];
    let lookup = status_lookup_from(&[("A", WorkItemStatus::Draft)]);

    let result = DependencyResolutionService::resolve_transition(
        &edges,
        &target,
        WorkItemStatus::Active,
        &lookup,
    );

    assert!(matches!(
        result,
        Err(DependencyResolutionError::SelfLoop { .. })
    ));
}

#[test]
fn terminal_status_rejects_transition_regardless_of_dependencies() {
    // Done should not transition even with no incoming edges
    let target = work_item("B", WorkItemStatus::Done);
    let edges: Vec<DependencyEdgeV1> = vec![];
    let lookup = status_lookup_from(&[]);

    let result = DependencyResolutionService::resolve_transition(
        &edges,
        &target,
        WorkItemStatus::Active,
        &lookup,
    );

    // Terminal states reject outgoing transitions at the state-machine level
    assert!(!WorkItemStatus::Done.can_transition_to(WorkItemStatus::Active));
    // Service returns Ok(()) because the transition is legal from service's perspective
    // when there are no edges to check — the state machine enforces terminal rejection
    // separately (tested above). This test confirms the service does not override.
    assert!(result.is_ok());
}

// ── valid_transitions helper ────────────────────────────────────────────────────

#[test]
fn valid_transitions_returns_all_legal_outgoing_for_each_status() {
    // Draft
    assert_eq!(
        WorkItemStatus::Draft.valid_transitions(),
        vec![WorkItemStatus::Active]
    );

    // Active
    let active_transitions = WorkItemStatus::Active.valid_transitions();
    assert!(active_transitions.contains(&WorkItemStatus::Paused));
    assert!(active_transitions.contains(&WorkItemStatus::Done));
    assert!(active_transitions.contains(&WorkItemStatus::Superseded));
    assert!(active_transitions.contains(&WorkItemStatus::Cancelled));
    assert_eq!(active_transitions.len(), 4);

    // Paused
    let paused_transitions = WorkItemStatus::Paused.valid_transitions();
    assert!(paused_transitions.contains(&WorkItemStatus::Active));
    assert!(paused_transitions.contains(&WorkItemStatus::Cancelled));
    assert_eq!(paused_transitions.len(), 2);

    // Terminal states have no valid transitions
    assert!(WorkItemStatus::Done.valid_transitions().is_empty());
    assert!(WorkItemStatus::Superseded.valid_transitions().is_empty());
    assert!(WorkItemStatus::Cancelled.valid_transitions().is_empty());
}
