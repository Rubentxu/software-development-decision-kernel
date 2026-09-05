//! Spine status to WorkItemStatus mapping tests.
//!
//! Tests AC-PLN3-09 per the locked §7 mapping table.

use sddk_domain::spine::SpineStatus;
use sddk_domain::planning::WorkItemStatus;

/// Scenario: PROPOSED → Draft
#[test]
fn status_proposed_maps_to_draft() {
    assert_eq!(
        SpineStatus::Proposed.to_work_item_status().unwrap(),
        WorkItemStatus::Draft
    );
}

/// Scenario: READY → Draft
#[test]
fn status_ready_maps_to_draft() {
    assert_eq!(
        SpineStatus::Ready.to_work_item_status().unwrap(),
        WorkItemStatus::Draft
    );
}

/// Scenario: ACTIVE → Active
#[test]
fn status_active_maps_to_active() {
    assert_eq!(
        SpineStatus::Active.to_work_item_status().unwrap(),
        WorkItemStatus::Active
    );
}

/// Scenario: PARTIAL → Active
#[test]
fn status_partial_maps_to_active() {
    assert_eq!(
        SpineStatus::Partial.to_work_item_status().unwrap(),
        WorkItemStatus::Active
    );
}

/// Scenario: BLOCKED → Paused
#[test]
fn status_blocked_maps_to_paused() {
    assert_eq!(
        SpineStatus::Blocked.to_work_item_status().unwrap(),
        WorkItemStatus::Paused
    );
}

/// Scenario: SHIPPED → Done
#[test]
fn status_shipped_maps_to_done() {
    assert_eq!(
        SpineStatus::Shipped.to_work_item_status().unwrap(),
        WorkItemStatus::Done
    );
}

/// Scenario: ABSORBED → Done
#[test]
fn status_absorbed_maps_to_done() {
    assert_eq!(
        SpineStatus::Absorbed.to_work_item_status().unwrap(),
        WorkItemStatus::Done
    );
}

/// Scenario: SUPERSEDED → Superseded
#[test]
fn status_superseded_maps_to_superseded() {
    assert_eq!(
        SpineStatus::Superseded.to_work_item_status().unwrap(),
        WorkItemStatus::Superseded
    );
}

/// Scenario: All eight statuses map to the six WorkItemStatus variants correctly
#[test]
fn status_mapping_table_is_total_and_correct() {
    let table = [
        (SpineStatus::Proposed, WorkItemStatus::Draft),
        (SpineStatus::Ready, WorkItemStatus::Draft),
        (SpineStatus::Active, WorkItemStatus::Active),
        (SpineStatus::Partial, WorkItemStatus::Active),
        (SpineStatus::Blocked, WorkItemStatus::Paused),
        (SpineStatus::Shipped, WorkItemStatus::Done),
        (SpineStatus::Absorbed, WorkItemStatus::Done),
        (SpineStatus::Superseded, WorkItemStatus::Superseded),
    ];

    for (spine_status, expected_wi_status) in table {
        let result = spine_status.to_work_item_status();
        assert!(result.is_ok(), "{:?} should map to Ok", spine_status);
        assert_eq!(result.unwrap(), expected_wi_status, "{:?} should map to {:?}", spine_status, expected_wi_status);
    }
}

/// Scenario: Output variants — exactly 6 WorkItemStatus values reachable from spine import
#[test]
fn six_work_item_status_variants_from_spine() {
    use std::collections::HashSet;
    let mut reached: HashSet<WorkItemStatus> = HashSet::new();

    for status in [
        SpineStatus::Proposed,
        SpineStatus::Ready,
        SpineStatus::Active,
        SpineStatus::Partial,
        SpineStatus::Blocked,
        SpineStatus::Shipped,
        SpineStatus::Absorbed,
        SpineStatus::Superseded,
    ] {
        if let Ok(wi_status) = status.to_work_item_status() {
            reached.insert(wi_status);
        }
    }

    // 6 reachable: Draft, Active, Paused, Done, Superseded, (Cancelled is NOT reachable from spine import)
    assert!(reached.contains(&WorkItemStatus::Draft));
    assert!(reached.contains(&WorkItemStatus::Active));
    assert!(reached.contains(&WorkItemStatus::Paused));
    assert!(reached.contains(&WorkItemStatus::Done));
    assert!(reached.contains(&WorkItemStatus::Superseded));
    assert!(!reached.contains(&WorkItemStatus::Cancelled), "Cancelled is NOT reachable from spine import");
}
