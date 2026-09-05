//! Tests for WorkItemIdentityProjection stability (AC-PLN4-01 R1).
//!
//! Verifies that the four new spine columns (spine_order, spine_horizon,
//! spine_status, exit_gate) are EXCLUDED from WorkItemIdentityProjection.
//!
//! Per PLN-LEDGER-002 §8 invariant and REQ-PLN4-MDL-003.

use sddk_domain::planning::{WorkItemIdentityProjection, WorkItemV1};

/// Scenario: WorkItemIdentityProjection has exactly 7 fields (stable identity).
/// The 4 new spine columns (spine_order, spine_horizon, spine_status, exit_gate)
/// are NOT included — they are on WorkItemSnapshot, not WorkItemV1.
#[test]
fn identity_projection_excludes_spine_columns() {
    let wi = WorkItemV1::new(
        "PLN-LEDGER-004".to_string(),
        "p-63676b11dc0ef88f/pln-ledger-004".to_string(),
        "Decision-plane projections".to_string(),
        "Implement roadmap projections".to_string(),
        None,
        1234567890,
    );

    let projection = WorkItemIdentityProjection::from(&wi);

    // Verify field values from projection
    assert_eq!(projection.id, "PLN-LEDGER-004");
    assert_eq!(projection.cycle_id, "p-63676b11dc0ef88f/pln-ledger-004");
    assert_eq!(projection.title, "Decision-plane projections");
    assert_eq!(projection.description, "Implement roadmap projections");
    assert!(projection.actor_ref.is_none());
    assert_eq!(projection.schema_version, 1);

    // Verify serialized form has exactly these 7 fields (no spine columns)
    let json = serde_json::to_value(&projection).expect("must serialize");
    let obj = json.as_object().expect("must be object");

    let expected_keys = vec!["id", "cycle_id", "title", "description", "actor_ref", "schema_version"];
    let actual_keys: Vec<_> = obj.keys().collect();

    assert_eq!(
        actual_keys.len(),
        expected_keys.len(),
        "WorkItemIdentityProjection must have exactly {} fields, got {:?}",
        expected_keys.len(),
        actual_keys
    );

    for key in &expected_keys {
        assert!(
            actual_keys.iter().any(|k| *k == *key),
            "expected field '{}' not found in {:?}",
            key,
            actual_keys
        );
    }

    // Explicitly verify spine columns are NOT present
    assert!(
        !actual_keys.iter().any(|k| *k == "spine_order"),
        "spine_order must NOT be in WorkItemIdentityProjection"
    );
    assert!(
        !actual_keys.iter().any(|k| *k == "spine_horizon"),
        "spine_horizon must NOT be in WorkItemIdentityProjection"
    );
    assert!(
        !actual_keys.iter().any(|k| *k == "spine_status"),
        "spine_status must NOT be in WorkItemIdentityProjection"
    );
    assert!(
        !actual_keys.iter().any(|k| *k == "exit_gate"),
        "exit_gate must NOT be in WorkItemIdentityProjection"
    );
}
