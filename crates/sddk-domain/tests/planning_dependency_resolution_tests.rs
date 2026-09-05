//! Tests for DependencyResolutionService.
//!
//! Verifies DependencyEdgeKind semantics per ADR-073 §3.2:
//! - Blocks: blocks any non-terminal transition
//! - BlocksOnClosure: blocks only terminal transitions

use sddk_domain::planning::service::{DependencyResolutionError, DependencyResolutionService};
use sddk_domain::planning::{
    DependencyEdgeKind, DependencyEdgeV1, WORK_ITEM_SCHEMA_VERSION, WorkItemV1,
};

/// Helper: build a WorkItemV1 with the given id, cycle, and status.
fn work_item(id: &str, cycle: &str, status: sddk_domain::planning::WorkItemStatus) -> WorkItemV1 {
    WorkItemV1 {
        id: id.to_string(),
        cycle_id: cycle.to_string(),
        title: format!("title-{}", id),
        description: format!("description-{}", id),
        status,
        actor_ref: None,
        created_at: 0,
        schema_version: WORK_ITEM_SCHEMA_VERSION,
    }
}

/// Helper: build a DependencyEdgeV1.
fn edge(from_id: &str, to_id: &str, kind: DependencyEdgeKind) -> DependencyEdgeV1 {
    DependencyEdgeV1::new(from_id.to_string(), to_id.to_string(), kind, None)
}

// ── Blocks semantics ─────────────────────────────────────────────────────────

#[test]
fn blocks_blocks_draft_to_active() {
    // a (Draft) -> b via Blocks
    let target = work_item("b", "cycle-1", sddk_domain::planning::WorkItemStatus::Draft);
    let edges = vec![edge("a", "b", DependencyEdgeKind::Blocks)];
    let status_lookup = |id: &sddk_domain::planning::WorkItemId| match id.as_str() {
        "a" => Some(sddk_domain::planning::WorkItemStatus::Draft),
        _ => None,
    };

    let result = DependencyResolutionService::resolve_can_activate(&target, &edges, &status_lookup);
    assert!(matches!(
        result,
        Err(DependencyResolutionError::BlockedBy { .. })
    ));
}

#[test]
fn blocks_blocks_active_to_paused() {
    // a (Active) -> b via Blocks
    let target = work_item("b", "cycle-1", sddk_domain::planning::WorkItemStatus::Draft);
    let edges = vec![edge("a", "b", DependencyEdgeKind::Blocks)];
    let status_lookup = |id: &sddk_domain::planning::WorkItemId| match id.as_str() {
        "a" => Some(sddk_domain::planning::WorkItemStatus::Active),
        _ => None,
    };

    let result = DependencyResolutionService::resolve_can_activate(&target, &edges, &status_lookup);
    assert!(matches!(
        result,
        Err(DependencyResolutionError::BlockedBy { .. })
    ));
}

#[test]
fn blocks_allows_when_no_predecessors() {
    // no incoming edges to b
    let target = work_item("b", "cycle-1", sddk_domain::planning::WorkItemStatus::Draft);
    let edges: Vec<DependencyEdgeV1> = vec![];
    let status_lookup = |_: &sddk_domain::planning::WorkItemId| None;

    let result = DependencyResolutionService::resolve_can_activate(&target, &edges, &status_lookup);
    assert!(result.is_ok());
}

#[test]
fn blocks_allows_when_predecessor_is_terminal() {
    // a (Done) -> b via Blocks — terminal predecessor allows
    let target = work_item("b", "cycle-1", sddk_domain::planning::WorkItemStatus::Draft);
    let edges = vec![edge("a", "b", DependencyEdgeKind::Blocks)];
    let status_lookup = |id: &sddk_domain::planning::WorkItemId| match id.as_str() {
        "a" => Some(sddk_domain::planning::WorkItemStatus::Done),
        _ => None,
    };

    let result = DependencyResolutionService::resolve_can_activate(&target, &edges, &status_lookup);
    assert!(result.is_ok());
}

// ── BlocksOnClosure semantics ─────────────────────────────────────────────────

#[test]
fn blocks_on_closure_allows_non_terminal_to_non_terminal() {
    // a (Active) -> b via BlocksOnClosure — non-terminal predecessor does NOT block
    let target = work_item("b", "cycle-1", sddk_domain::planning::WorkItemStatus::Draft);
    let edges = vec![edge("a", "b", DependencyEdgeKind::BlocksOnClosure)];
    let status_lookup = |id: &sddk_domain::planning::WorkItemId| match id.as_str() {
        "a" => Some(sddk_domain::planning::WorkItemStatus::Active),
        _ => None,
    };

    let result = DependencyResolutionService::resolve_can_activate(&target, &edges, &status_lookup);
    assert!(result.is_ok());
}

#[test]
fn blocks_on_closure_blocks_terminal_transition() {
    // a (Active) -> b via BlocksOnClosure — non-terminal predecessor blocks terminalize
    let target = work_item(
        "b",
        "cycle-1",
        sddk_domain::planning::WorkItemStatus::Active,
    );
    let edges = vec![edge("a", "b", DependencyEdgeKind::BlocksOnClosure)];
    let status_lookup = |id: &sddk_domain::planning::WorkItemId| match id.as_str() {
        "a" => Some(sddk_domain::planning::WorkItemStatus::Active),
        _ => None,
    };

    let result = DependencyResolutionService::resolve_can_terminalize(
        &target,
        sddk_domain::planning::WorkItemStatus::Done,
        &edges,
        &status_lookup,
    );
    assert!(matches!(
        result,
        Err(DependencyResolutionError::BlockedBy { .. })
    ));
}

#[test]
fn blocks_on_closure_allows_when_predecessor_is_terminal() {
    // a (Done) -> b via BlocksOnClosure — terminal predecessor allows terminalize
    let target = work_item(
        "b",
        "cycle-1",
        sddk_domain::planning::WorkItemStatus::Active,
    );
    let edges = vec![edge("a", "b", DependencyEdgeKind::BlocksOnClosure)];
    let status_lookup = |id: &sddk_domain::planning::WorkItemId| match id.as_str() {
        "a" => Some(sddk_domain::planning::WorkItemStatus::Done),
        _ => None,
    };

    let result = DependencyResolutionService::resolve_can_terminalize(
        &target,
        sddk_domain::planning::WorkItemStatus::Done,
        &edges,
        &status_lookup,
    );
    assert!(result.is_ok());
}

// ── Cycle detection ───────────────────────────────────────────────────────────

#[test]
fn self_loop_is_detected() {
    // a -> a (self-loop)
    let target = work_item("a", "cycle-1", sddk_domain::planning::WorkItemStatus::Draft);
    let edges = vec![edge("a", "a", DependencyEdgeKind::Blocks)];
    let status_lookup = |id: &sddk_domain::planning::WorkItemId| match id.as_str() {
        "a" => Some(sddk_domain::planning::WorkItemStatus::Draft),
        _ => None,
    };

    let result = DependencyResolutionService::resolve_can_activate(&target, &edges, &status_lookup);
    assert!(matches!(
        result,
        Err(DependencyResolutionError::SelfLoop { .. })
    ));
}

// ── Edge cases ────────────────────────────────────────────────────────────────

#[test]
fn empty_edges_always_ok() {
    let target = work_item("b", "cycle-1", sddk_domain::planning::WorkItemStatus::Draft);
    let edges: Vec<DependencyEdgeV1> = vec![];
    let status_lookup = |_: &sddk_domain::planning::WorkItemId| None;

    let result = DependencyResolutionService::resolve_can_activate(&target, &edges, &status_lookup);
    assert!(result.is_ok());
}

#[test]
fn mixed_blocks_and_blocks_on_closure() {
    // a (Draft) -> b via Blocks, c (Done) -> b via BlocksOnClosure
    // a is non-terminal so it blocks b's activation
    let target = work_item("b", "cycle-1", sddk_domain::planning::WorkItemStatus::Draft);
    let edges = vec![
        edge("a", "b", DependencyEdgeKind::Blocks),
        edge("c", "b", DependencyEdgeKind::BlocksOnClosure),
    ];
    let status_lookup = |id: &sddk_domain::planning::WorkItemId| match id.as_str() {
        "a" => Some(sddk_domain::planning::WorkItemStatus::Draft),
        "c" => Some(sddk_domain::planning::WorkItemStatus::Done),
        _ => None,
    };

    let result = DependencyResolutionService::resolve_can_activate(&target, &edges, &status_lookup);
    assert!(matches!(
        result,
        Err(DependencyResolutionError::BlockedBy { .. })
    ));
}
