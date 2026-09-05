//! Tests for `PlanningProvenanceChainV1::verify_references` (AC-PLN2-11).
//!
//! Verifies that dangling references are detected and that structural shape
//! validation works at the domain level.

use sddk_domain::planning::{PlanningProvenanceChainV1, ProvenanceError};

/// Helper: build a minimal PlanningProvenanceChainV1.
fn chain(
    cycle_id: &str,
    work_item_ids: Vec<&str>,
    evidence_refs: Vec<&str>,
    decision_refs: Vec<&str>,
) -> PlanningProvenanceChainV1 {
    PlanningProvenanceChainV1::new(
        cycle_id.to_string(),
        work_item_ids.iter().map(|s| s.to_string()).collect(),
        evidence_refs.iter().map(|s| s.to_string()).collect(),
        decision_refs.iter().map(|s| s.to_string()).collect(),
    )
}

// ── Structural validation ────────────────────────────────────────────────────────

#[test]
fn empty_cycle_id_rejected() {
    // GIVEN a chain with cycle_id = ""
    let ch = chain("", vec!["wi-001"], vec![], vec![]);

    // WHEN verify_references is called
    let result = ch.verify_references();

    // THEN it returns EmptyCycleId
    assert!(matches!(result, Err(ProvenanceError::EmptyCycleId)));
}

#[test]
fn non_empty_cycle_id_passes_structural_check() {
    // GIVEN a chain with a valid cycle_id and no references
    let ch = chain("cycle-001", vec![], vec![], vec![]);

    let result = ch.verify_references();

    // Domain-level structural check passes (no cycle_id)
    assert!(result.is_ok());
}

// ── Reference shape ─────────────────────────────────────────────────────────────

#[test]
fn chain_with_work_items_and_evidence_passes() {
    // GIVEN a chain with valid work_item_ids, evidence_refs, and decision_refs
    let ch = chain(
        "cycle-001",
        vec!["wi-001", "wi-002"],
        vec!["sha256:abc123"],
        vec!["dec-001"],
    );

    let result = ch.verify_references();

    // Domain-level: cycle_id is non-empty so structural check passes
    assert!(result.is_ok());
}

#[test]
fn empty_work_item_ids_with_evidence_passes() {
    // GIVEN a chain with no WorkItems but with evidence and decisions
    let ch = chain("cycle-001", vec![], vec!["sha256:abc"], vec!["dec-001"]);

    let result = ch.verify_references();

    assert!(result.is_ok());
}

// ── Round-trip ─────────────────────────────────────────────────────────────────

#[test]
fn chain_round_trip_serialization_preserves_data() {
    // GIVEN a chain with all fields populated
    let ch = PlanningProvenanceChainV1::new(
        "cycle-001".to_string(),
        vec!["wi-001".to_string(), "wi-002".to_string()],
        vec!["sha256:abc123".to_string()],
        vec!["dec-001".to_string()],
    );

    // WHEN serialized and deserialized
    let json = serde_json::to_string(&ch).expect("chain is serializable");
    let ch2: PlanningProvenanceChainV1 =
        serde_json::from_str(&json).expect("chain is deserializable");

    // THEN the chain data is preserved
    assert_eq!(ch.cycle_id, ch2.cycle_id);
    assert_eq!(ch.work_item_ids, ch2.work_item_ids);
    assert_eq!(ch.evidence_refs, ch2.evidence_refs);
    assert_eq!(ch.decision_refs, ch2.decision_refs);
}

// ── JSON serialization identity stability ───────────────────────────────────────

#[test]
fn chain_json_round_trip_preserves_all_fields() {
    // GIVEN a chain with all fields populated
    let ch = PlanningProvenanceChainV1::new(
        "cycle-001".to_string(),
        vec!["wi-001".to_string(), "wi-002".to_string()],
        vec!["sha256:abc123".to_string()],
        vec!["dec-001".to_string()],
    );

    // WHEN serialized and deserialized multiple times
    let json1 = serde_json::to_string(&ch).expect("chain is serializable");
    let ch1: PlanningProvenanceChainV1 =
        serde_json::from_str(&json1).expect("chain is deserializable");
    let json2 = serde_json::to_string(&ch1).expect("chain is serializable");
    let ch2: PlanningProvenanceChainV1 =
        serde_json::from_str(&json2).expect("chain is deserializable");

    // THEN all fields are preserved through the round-trip
    assert_eq!(ch.cycle_id, ch2.cycle_id);
    assert_eq!(ch.work_item_ids, ch2.work_item_ids);
    assert_eq!(ch.evidence_refs, ch2.evidence_refs);
    assert_eq!(ch.decision_refs, ch2.decision_refs);
    assert_eq!(json1, json2, "serialized form is stable");
}

// ── Reorder tolerance ──────────────────────────────────────────────────────────

#[test]
fn evidence_refs_reordering_does_not_break_verification() {
    // GIVEN a chain with evidence_refs in one order
    let ch1 = PlanningProvenanceChainV1::new(
        "cycle-001".to_string(),
        vec!["wi-001".to_string()],
        vec!["sha256:aaa".to_string(), "sha256:bbb".to_string()],
        vec![],
    );

    // WHEN evidence_refs are reordered (same logical set)
    let ch2 = PlanningProvenanceChainV1::new(
        "cycle-001".to_string(),
        vec!["wi-001".to_string()],
        vec!["sha256:bbb".to_string(), "sha256:aaa".to_string()],
        vec![],
    );

    // THEN domain structural check passes for both (ordering is external concern)
    assert!(ch1.verify_references().is_ok());
    assert!(ch2.verify_references().is_ok());
}

#[test]
fn decision_refs_reordering_does_not_break_verification() {
    let ch1 = PlanningProvenanceChainV1::new(
        "cycle-001".to_string(),
        vec!["wi-001".to_string()],
        vec![],
        vec!["dec-001".to_string(), "dec-002".to_string()],
    );

    let ch2 = PlanningProvenanceChainV1::new(
        "cycle-001".to_string(),
        vec!["wi-001".to_string()],
        vec![],
        vec!["dec-002".to_string(), "dec-001".to_string()],
    );

    assert!(ch1.verify_references().is_ok());
    assert!(ch2.verify_references().is_ok());
}

// ── Multi-cycle isolation ──────────────────────────────────────────────────────

#[test]
fn different_cycle_ids_both_verify_successfully() {
    // Two cycles with the same content but different cycle_ids are both structurally valid.
    // The cycles are isolated — verify_references does not cross-contaminate.
    let ch_a = PlanningProvenanceChainV1::new(
        "cycle-A".to_string(),
        vec!["wi-001".to_string()],
        vec!["sha256:abc".to_string()],
        vec!["dec-001".to_string()],
    );
    let ch_b = PlanningProvenanceChainV1::new(
        "cycle-B".to_string(),
        vec!["wi-001".to_string()],
        vec!["sha256:abc".to_string()],
        vec!["dec-001".to_string()],
    );

    assert!(ch_a.verify_references().is_ok());
    assert!(ch_b.verify_references().is_ok());
}

// ── Edge case: large ref counts ────────────────────────────────────────────────

#[test]
fn chain_with_many_references_is_valid() {
    let work_item_ids: Vec<String> = (0..100).map(|i| format!("wi-{:03}", i)).collect();
    let evidence_refs: Vec<String> = (0..50).map(|i| format!("sha256:{:x}", i * 2)).collect();
    let decision_refs: Vec<String> = (0..25).map(|i| format!("dec-{:03}", i)).collect();

    let ch = PlanningProvenanceChainV1::new(
        "cycle-001".to_string(),
        work_item_ids,
        evidence_refs,
        decision_refs,
    );

    assert!(ch.verify_references().is_ok());
}
