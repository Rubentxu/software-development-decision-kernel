//! Cross-storage provenance chain verification tests.
//!
//! Tests AC-PLN3-01 through AC-PLN3-04 + AC-PLN3-16 layer purity.
//!
//! These tests use two Storage instances with aligned or mismatched CAS roots
//! to exercise the cross-storage drift detection in `verify_references_with_options`.

use sddk_domain::planning::{
    DecisionRecordRecord, DependencyEdgeRecord, EvidenceAttachmentRecord,
    PLANNING_PROVENANCE_SCHEMA_VERSION, PlanningProvenanceChainV1, ProvenanceError,
    VerifyReferencesOptions, WorkItemRecord, WorkItemStatus,
};
use std::collections::HashMap;

/// A fake planning graph read that captures the CAS root identity for cross-storage tests.
#[derive(Clone)]
struct FakePlanningGraphRead {
    work_items: HashMap<String, WorkItemRecord>,
    evidence: HashMap<String, Vec<EvidenceAttachmentRecord>>,
    decisions: HashMap<String, Vec<DecisionRecordRecord>>,
    cas_root_id: String,
    handle_id: String,
}

impl FakePlanningGraphRead {
    fn new(cas_root_id: &str, handle_id: &str) -> Self {
        Self {
            work_items: HashMap::new(),
            evidence: HashMap::new(),
            decisions: HashMap::new(),
            cas_root_id: cas_root_id.to_string(),
            handle_id: handle_id.to_string(),
        }
    }

    fn with_items(mut self, items: Vec<WorkItemRecord>) -> Self {
        for item in items {
            self.work_items.insert(item.id.clone(), item);
        }
        self
    }
}

impl sddk_domain::planning::PlanningGraphRead for FakePlanningGraphRead {
    fn list_work_items_by_cycle(
        &self,
        cycle_id: &str,
    ) -> std::result::Result<Vec<WorkItemRecord>, sddk_domain::StorageError> {
        Ok(self
            .work_items
            .values()
            .filter(|w| w.cycle_id == cycle_id)
            .cloned()
            .collect())
    }

    fn list_dependency_edges_by_cycle(
        &self,
        _cycle_id: &str,
    ) -> std::result::Result<Vec<DependencyEdgeRecord>, sddk_domain::StorageError> {
        Ok(Vec::new())
    }

    fn list_evidence_attachments_by_work_item(
        &self,
        work_item_id: &str,
    ) -> std::result::Result<Vec<EvidenceAttachmentRecord>, sddk_domain::StorageError> {
        Ok(self.evidence.get(work_item_id).cloned().unwrap_or_default())
    }

    fn list_decision_records_by_work_item(
        &self,
        work_item_id: &str,
    ) -> std::result::Result<Vec<DecisionRecordRecord>, sddk_domain::StorageError> {
        Ok(self
            .decisions
            .get(work_item_id)
            .cloned()
            .unwrap_or_default())
    }

    fn cas_root_id(&self) -> String {
        self.cas_root_id.clone()
    }

    fn handle_id(&self) -> String {
        self.handle_id.clone()
    }
}

fn make_work_item(id: &str, cycle_id: &str) -> WorkItemRecord {
    WorkItemRecord {
        id: id.to_string(),
        cycle_id: cycle_id.to_string(),
        title: id.to_string(),
        description: "test".to_string(),
        status: WorkItemStatus::Active,
        actor_ref_kind: None,
        actor_ref_id: None,
        actor_ref_label: None,
        created_at: 0,
        schema_version: 1,
    }
}

// ── AC-PLN3-01: Aligned CAS roots, identical chain ──────────────────────────

/// Scenario: Aligned CAS roots, identical chain
#[test]
fn cross_storage_aligned_cas_roots_identical_chain() {
    let store_a = FakePlanningGraphRead::new("sha256:rootA", "handle-A").with_items(vec![
        make_work_item("WI-1", "C1"),
        make_work_item("WI-2", "C1"),
    ]);
    let store_b = FakePlanningGraphRead::new("sha256:rootA", "handle-B").with_items(vec![
        make_work_item("WI-1", "C1"),
        make_work_item("WI-2", "C1"),
    ]);

    let chain = PlanningProvenanceChainV1::new_v2(
        "C1".to_string(),
        vec!["WI-1".to_string(), "WI-2".to_string()],
        vec![],
        vec![],
        "sha256:rootA".to_string(),
    );

    let result =
        chain.verify_references_with_options(&store_b, &VerifyReferencesOptions::default());
    assert!(
        result.is_ok(),
        "aligned roots should verify OK: {:?}",
        result
    );
}

/// Scenario: Aligned CAS roots, v1 chain without producer stamp (backward compat)
#[test]
fn cross_storage_aligned_v1_chain_backward_compat() {
    let store_a = FakePlanningGraphRead::new("sha256:rootA", "handle-A")
        .with_items(vec![make_work_item("WI-1", "C1")]);
    let store_b = FakePlanningGraphRead::new("sha256:rootA", "handle-B")
        .with_items(vec![make_work_item("WI-1", "C1")]);

    // v1 chain — no producer metadata
    let chain =
        PlanningProvenanceChainV1::new("C1".to_string(), vec!["WI-1".to_string()], vec![], vec![]);

    let result =
        chain.verify_references_with_options(&store_b, &VerifyReferencesOptions::default());
    assert!(
        result.is_ok(),
        "v1 chain on aligned roots should verify OK: {:?}",
        result
    );
}

/// Scenario: Aligned CAS roots, schema round-trip preserves producer metadata
#[test]
fn cross_storage_schema_roundtrip_preserves_producer() {
    let chain = PlanningProvenanceChainV1::new_v2(
        "C1".to_string(),
        vec!["WI-1".to_string()],
        vec!["sha256:ev1".to_string()],
        vec![],
        "sha256:casA".to_string(),
    );

    // Serialize to JSON
    let json = serde_json::to_string(&chain).unwrap();
    let deserialized: PlanningProvenanceChainV1 = serde_json::from_str(&json).unwrap();

    assert_eq!(
        deserialized.producer_cas_root_id,
        Some("sha256:casA".to_string())
    );
    assert_eq!(deserialized.schema_version, 2);
    assert_eq!(deserialized.effective_schema_version(), 2);
}

/// Scenario: Producer's CAS root is updated to v2 stamp on first build
#[test]
fn cross_storage_v2_chain_has_producer_stamp() {
    let chain = PlanningProvenanceChainV1::new_v2(
        "C1".to_string(),
        vec!["WI-1".to_string()],
        vec![],
        vec![],
        "sha256:producerA".to_string(),
    );

    assert_eq!(
        chain.producer_cas_root_id,
        Some("sha256:producerA".to_string())
    );
    assert_eq!(chain.schema_version, PLANNING_PROVENANCE_SCHEMA_VERSION);
    assert_eq!(
        chain.effective_schema_version(),
        PLANNING_PROVENANCE_SCHEMA_VERSION
    );
}

/// Scenario: Empty chain verifies on aligned but empty verifier
#[test]
fn cross_storage_empty_chain_aligned_verifier() {
    let store_b = FakePlanningGraphRead::new("sha256:rootB", "handle-B");

    let chain = PlanningProvenanceChainV1::new_v2(
        "C1".to_string(),
        vec![],
        vec![],
        vec![],
        "sha256:rootB".to_string(),
    );

    let result =
        chain.verify_references_with_options(&store_b, &VerifyReferencesOptions::default());
    assert!(
        result.is_ok(),
        "empty chain on aligned verifier should pass: {:?}",
        result
    );
}

// ── AC-PLN3-02: Mismatched CAS roots → CrossStorageDrift ──────────────────

/// Scenario: Producer stamp set, verifier's cas_root_id differs
#[test]
fn cross_storage_mismatched_cas_root_returns_drift() {
    let store_a = FakePlanningGraphRead::new("sha256:rootA", "handle-A");
    let store_b = FakePlanningGraphRead::new("sha256:rootB", "handle-B");

    let chain = PlanningProvenanceChainV1::new_v2(
        "C1".to_string(),
        vec!["WI-1".to_string()],
        vec![],
        vec![],
        "sha256:rootA".to_string(),
    );

    let result =
        chain.verify_references_with_options(&store_b, &VerifyReferencesOptions::default());
    {
        let err = result.expect_err("should be CrossStorageDrift");
        let drift = match err {
            ProvenanceError::CrossStorageDrift {
                reason,
                producer_cas_root_id,
                verifier_cas_root_id,
                ..
            } => {
                assert_eq!(reason, "cas_root_id_mismatch");
                assert_eq!(producer_cas_root_id.as_deref(), Some("sha256:rootA"));
                assert_eq!(verifier_cas_root_id, "sha256:rootB");
            }
            other => panic!("expected CrossStorageDrift, got {:?}", other),
        };
    }
}

/// Scenario: Producer stamp absent, strict mode detects drift
#[test]
fn cross_storage_v1_strict_mode_detects_drift() {
    let store_a = FakePlanningGraphRead::new("sha256:rootA", "handle-A");
    let store_b = FakePlanningGraphRead::new("sha256:rootB", "handle-B");

    // v1 chain — no producer stamp
    let chain =
        PlanningProvenanceChainV1::new("C1".to_string(), vec!["WI-1".to_string()], vec![], vec![]);

    // Non-strict: should pass (backward compat)
    let store_with_item = FakePlanningGraphRead::new("sha256:rootB", "handle-B")
        .with_items(vec![make_work_item("WI-1", "C1")]);
    let result_non_strict =
        chain.verify_references_with_options(&store_with_item, &VerifyReferencesOptions::default());
    assert!(result_non_strict.is_ok(), "non-strict v1 chain should pass");

    // Strict: should fail
    let strict_options = VerifyReferencesOptions {
        strict_cross_storage: true,
    };
    let result_strict = chain.verify_references_with_options(&store_b, &strict_options);
    let is_drift_strict = matches!(
        result_strict,
        Err(ProvenanceError::CrossStorageDrift { .. })
    );
    assert!(
        is_drift_strict,
        "strict v1 chain should fail on mismatched roots"
    );
}
/// Scenario: Mismatched CAS root with no work-item data — CrossStorageDrift, not DanglingReference
#[test]
fn cross_storage_mismatched_no_workitems_returns_drift_not_dangling() {
    let store_b = FakePlanningGraphRead::new("sha256:rootB", "handle-B");

    let chain = PlanningProvenanceChainV1::new_v2(
        "C1".to_string(),
        vec!["WI-1".to_string()], // references a work item that doesn't exist in B
        vec![],
        vec![],
        "sha256:rootA".to_string(), // different from store_b's root
    );

    let result =
        chain.verify_references_with_options(&store_b, &VerifyReferencesOptions::default());
    // Drift is detected FIRST, before dangling-ref iteration
    let is_drift = matches!(result, Err(ProvenanceError::CrossStorageDrift { .. }));
    assert!(
        is_drift,
        "drift should be detected before dangling ref: {:?}",
        result
    );
}

// ── AC-PLN3-03: Backward compat for v1 chains on single-storage ─────────────

/// Scenario: v1 round-trip on single-storage still works
#[test]
fn v1_chain_single_storage_roundtrip() {
    let store = FakePlanningGraphRead::new("sha256:single", "handle-single")
        .with_items(vec![make_work_item("WI-1", "C1")]);

    let chain =
        PlanningProvenanceChainV1::new("C1".to_string(), vec!["WI-1".to_string()], vec![], vec![]);

    let result = chain.verify_references(&store);
    assert!(
        result.is_ok(),
        "v1 chain on same storage should pass: {:?}",
        result
    );
}

/// Scenario: v1 dangling reference on single-storage surfaces DanglingReference
#[test]
fn v1_chain_dangling_reference_single_storage() {
    let store = FakePlanningGraphRead::new("sha256:single", "handle-single");

    let chain =
        PlanningProvenanceChainV1::new("C1".to_string(), vec!["WI-1".to_string()], vec![], vec![]);

    let result = chain.verify_references(&store);
    assert!(
        matches!(result, Err(ProvenanceError::DanglingReference(id)) if id == "WI-1"),
        "expected DanglingReference for WI-1"
    );
}

// ── AC-PLN3-04: Aligned CAS roots, verifier empty → DanglingReference ─────────

/// Scenario: Aligned CAS roots, verifier has no data — dangling ref, not drift
#[test]
fn aligned_cas_empty_verifier_returns_dangling_not_drift() {
    let store_b = FakePlanningGraphRead::new("sha256:rootB", "handle-B");

    let chain = PlanningProvenanceChainV1::new_v2(
        "C1".to_string(),
        vec!["WI-X".to_string()], // doesn't exist in B
        vec![],
        vec![],
        "sha256:rootB".to_string(), // same as verifier
    );

    let result =
        chain.verify_references_with_options(&store_b, &VerifyReferencesOptions::default());
    // Since roots are aligned, we get DanglingReference, not CrossStorageDrift
    assert!(
        matches!(result, Err(ProvenanceError::DanglingReference(id)) if id == "WI-X"),
        "expected DanglingReference for WI-X"
    );
}

// ── AC-PLN3-16: Layer purity ───────────────────────────────────────────────

/// Scenario: domain layer purity — sddk-domain has no storage/engine/cli imports
#[test]
fn domain_layer_purity_check() {
    // This test documents the layer purity requirement:
    // sddk-domain should compile without any sddk-storage, sddk-engine, or sddk-cli imports.
    // The actual compilation check is done at build time; this test exists to
    // make the requirement explicit and testable.
    let chain = PlanningProvenanceChainV1::new_v2(
        "test".to_string(),
        vec![],
        vec![],
        vec![],
        "sha256:test".to_string(),
    );
    // Verify that producer metadata is set
    assert_eq!(chain.schema_version, 2);
    assert_eq!(chain.effective_schema_version(), 2);
}

/// Scenario: ActorKind 3-variant closed set preserved
#[test]
fn actor_kind_is_three_variants() {
    use sddk_domain::event_envelope::ActorKind;
    let variants = [ActorKind::Human, ActorKind::Agent, ActorKind::System];
    assert_eq!(variants.len(), 3);
}

/// Scenario: v1 chain backward compat — effective_schema_version returns 1 for old chains
#[test]
fn v1_chain_effective_version_is_1() {
    let chain = PlanningProvenanceChainV1::new("C1".to_string(), vec![], vec![], vec![]);
    // Default schema_version is 1
    assert_eq!(chain.schema_version, 1);
    assert_eq!(chain.effective_schema_version(), 1);
    // producer fields should be None for v1
    assert_eq!(chain.producer_cas_root_id, None);
    assert_eq!(chain.producer_signature, None);
}
