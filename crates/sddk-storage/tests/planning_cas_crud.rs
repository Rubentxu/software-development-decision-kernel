//! Tests for planning CAS + CRUD integration (PLN-LEDGER-002).
//!
//! Covers AC-PLN2-03, AC-PLN2-04, AC-PLN2-10 (partial), AC-PLN2-11:
//! - Evidence body round-trip through CAS (body written to cas_root, hash ref stored, get loads body)
//! - Empty body → EmptyEvidenceBody error
//! - Decision record insert/get/list with inline rationale (Q3)
//! - build_provenance_chain over persisted records (closes FIND-PLN-007 partial)
//! - Identity stability via storage: insert then call compute_planning_graph_identity twice → equal

use sddk_domain::planning::{
    DecisionKind, DependencyEdgeKind, DependencyEdgeRecord, EvidenceAttachmentRecord,
    PlanningEvidenceKind, WorkItemRecord, WorkItemStatus, compute_planning_graph_identity,
};
use sddk_domain::{CycleId, CycleManifest};
use sddk_storage::{CycleRecord, ProjectRecord, Storage, WorkspaceRecord};
use tempfile::TempDir;

const CREATED_AT: i64 = 1_725_836_000; // 2026-09-05 00:00:00 UTC

/// Sets up project + workspace + cycle required by FK constraints.
/// Returns the full cycle_id string (e.g. "p-pln-test/evidence").
/// Uses a properly-formatted cycle_id (project/cycle-name format).
fn setup_project_workspace_cycle(storage: &mut Storage, cycle_name: &str) -> String {
    let project_id = "p-pln-test";
    let workspace_id = "ws-pln-test";
    let cycle_id = format!("{}/{}", project_id, cycle_name);
    let project = ProjectRecord {
        project_id: project_id.into(),
        display_name: "PLN Test Project".into(),
        remote_url: Some("https://example.com/test".into()),
        scope: "test".into(),
        created_at: CREATED_AT.to_string(),
    };
    let workspace = WorkspaceRecord {
        workspace_id: workspace_id.into(),
        project_id: project_id.into(),
        canonical_path: "/test".into(),
        created_at: CREATED_AT.to_string(),
    };
    let manifest = CycleManifest::new(
        project_id.into(),
        workspace_id.into(),
        CycleId::new(&cycle_id).expect("valid cycle id"),
        "PLN test cycle".into(),
        "sddk/pln-test".into(),
        "abc123def456".into(),
    );
    let cycle = CycleRecord {
        manifest,
        created_at: CREATED_AT.to_string(),
        updated_at: CREATED_AT.to_string(),
    };
    storage
        .insert_project(&project)
        .expect("insert_project must succeed");
    storage
        .insert_workspace(&workspace)
        .expect("insert_workspace must succeed");
    storage
        .insert_cycle(&cycle)
        .expect("insert_cycle must succeed");
    cycle_id
}

// ── Evidence attachment round-trip ─────────────────────────────────────────

#[test]
fn evidence_attachment_cas_round_trip() {
    let tmp = TempDir::new().unwrap();
    let db_path = tmp.path().join("ledger.sqlite");
    let mut storage = Storage::open(&db_path).expect("must open");

    let cycle_id = setup_project_workspace_cycle(&mut storage, "evidence");

    // Insert a work item
    let wi_record = WorkItemRecord {
        id: "wi-ev-test".into(),
        cycle_id: cycle_id.clone().into(),
        title: "evidence test item".into(),
        description: "desc".into(),
        status: WorkItemStatus::Draft,
        actor_ref_kind: Some("Agent".into()),
        actor_ref_id: Some("agent:test".into()),
        actor_ref_label: Some("test".into()),
        created_at: CREATED_AT,
        schema_version: 1,
    };
    storage
        .insert_work_item(&wi_record)
        .expect("insert workitem must succeed");

    // Insert evidence with a non-empty body
    let body = b"log output: build succeeded at 14:32";
    let record = EvidenceAttachmentRecord {
        id: "ev-001".into(),
        work_item_id: "wi-ev-test".into(),
        kind: PlanningEvidenceKind::Log,
        body_ref: "pending".into(),
        actor_ref_kind: Some("Agent".into()),
        actor_ref_id: Some("agent:test".into()),
        actor_ref_label: Some("test".into()),
        schema_version: 1,
    };
    storage
        .insert_evidence_attachment(&record, body)
        .expect("insert must succeed");

    // Retrieve and verify body matches
    let (loaded_record, loaded_body) = storage
        .get_evidence_attachment("ev-001")
        .expect("get must succeed")
        .expect("evidence must exist");
    assert_eq!(loaded_body, body, "loaded body must match original");
    assert_eq!(
        loaded_record.kind,
        PlanningEvidenceKind::Log,
        "kind must be preserved"
    );
}

/// Verifies get_evidence_attachment on a non-existent ID returns None (not error).
#[test]
fn get_evidence_attachment_nonexistent_returns_none() {
    let tmp = TempDir::new().unwrap();
    let db_path = tmp.path().join("ledger.sqlite");
    let storage = Storage::open(&db_path).expect("must open");

    let result = storage.get_evidence_attachment("nonexistent-ev");
    assert!(result.is_ok(), "get must not error for missing id");
    assert!(
        result.unwrap().is_none(),
        "missing evidence must return None"
    );
}

// ── Empty evidence body ─────────────────────────────────────────────────────

#[test]
fn evidence_insert_empty_body_rejected() {
    let tmp = TempDir::new().unwrap();
    let db_path = tmp.path().join("ledger.sqlite");
    let mut storage = Storage::open(&db_path).expect("must open");

    let cycle_id = setup_project_workspace_cycle(&mut storage, "empty");
    let wi_record = WorkItemRecord {
        id: "wi-empty".into(),
        cycle_id: cycle_id.clone().into(),
        title: "empty test".into(),
        description: "desc".into(),
        status: WorkItemStatus::Draft,
        actor_ref_kind: Some("Agent".into()),
        actor_ref_id: Some("agent:test".into()),
        actor_ref_label: Some("test".into()),
        created_at: CREATED_AT,
        schema_version: 1,
    };
    storage
        .insert_work_item(&wi_record)
        .expect("insert workitem must succeed");

    let record = EvidenceAttachmentRecord {
        id: "ev-empty".into(),
        work_item_id: "wi-empty".into(),
        kind: PlanningEvidenceKind::Log,
        body_ref: "pending".into(),
        actor_ref_kind: Some("Agent".into()),
        actor_ref_id: Some("agent:test".into()),
        actor_ref_label: Some("test".into()),
        schema_version: 1,
    };
    let result = storage.insert_evidence_attachment(&record, b"");
    assert!(result.is_err(), "empty body must be rejected");
    let err = result.unwrap_err();
    assert!(
        format!("{}", err).to_lowercase().contains("empty")
            || format!("{}", err).contains("non-empty"),
        "error must mention empty/non-empty: {err}"
    );
}

// ── Decision record inline storage (Q3) ─────────────────────────────────────

#[test]
fn decision_record_insert_get_round_trip() {
    let tmp = TempDir::new().unwrap();
    let db_path = tmp.path().join("ledger.sqlite");
    let mut storage = Storage::open(&db_path).expect("must open");

    let cycle_id = setup_project_workspace_cycle(&mut storage, "decision");
    let wi_record = WorkItemRecord {
        id: "wi-decision".into(),
        cycle_id: cycle_id.clone().into(),
        title: "decision test".into(),
        description: "desc".into(),
        status: WorkItemStatus::Draft,
        actor_ref_kind: Some("Human".into()),
        actor_ref_id: Some("user:bob".into()),
        actor_ref_label: Some("bob".into()),
        created_at: CREATED_AT,
        schema_version: 1,
    };
    storage
        .insert_work_item(&wi_record)
        .expect("insert workitem must succeed");

    let record = sddk_domain::planning::DecisionRecordRecord {
        id: "dec-001".into(),
        work_item_id: "wi-decision".into(),
        kind: DecisionKind::Accept,
        rationale: "Best approach after evaluating alternatives".into(),
        actor_ref_kind: Some("Human".into()),
        actor_ref_id: Some("user:bob".into()),
        actor_ref_label: Some("bob".into()),
        schema_version: 1,
    };
    storage
        .insert_decision_record(&record)
        .expect("insert decision must succeed");

    let loaded = storage
        .get_decision_record("dec-001")
        .expect("get must not error")
        .expect("decision must exist");
    assert_eq!(
        loaded.rationale, "Best approach after evaluating alternatives",
        "rationale must be preserved verbatim"
    );
    assert_eq!(loaded.kind, DecisionKind::Accept);
}

#[test]
fn decision_record_list_by_work_item() {
    let tmp = TempDir::new().unwrap();
    let db_path = tmp.path().join("ledger.sqlite");
    let mut storage = Storage::open(&db_path).expect("must open");

    let cycle_id = setup_project_workspace_cycle(&mut storage, "list");
    let wi_record = WorkItemRecord {
        id: "wi-list".into(),
        cycle_id: cycle_id.clone().into(),
        title: "list test".into(),
        description: "desc".into(),
        status: WorkItemStatus::Draft,
        actor_ref_kind: Some("Human".into()),
        actor_ref_id: Some("user:bob".into()),
        actor_ref_label: Some("bob".into()),
        created_at: CREATED_AT,
        schema_version: 1,
    };
    storage
        .insert_work_item(&wi_record)
        .expect("insert workitem must succeed");

    for i in 0..2 {
        let record = sddk_domain::planning::DecisionRecordRecord {
            id: format!("dec-list-{}", i),
            work_item_id: "wi-list".into(),
            kind: DecisionKind::Accept,
            rationale: format!("rationale {}", i),
            actor_ref_kind: Some("Human".into()),
            actor_ref_id: Some("user:bob".into()),
            actor_ref_label: Some("bob".into()),
            schema_version: 1,
        };
        storage.insert_decision_record(&record).unwrap();
    }

    let decisions = storage
        .list_decision_records_by_work_item("wi-list")
        .expect("list must succeed");
    assert_eq!(decisions.len(), 2, "must have 2 decision records");
}

// ── build_provenance_chain ───────────────────────────────────────────────────

#[test]
fn build_provenance_chain_over_persisted_records() {
    let tmp = TempDir::new().unwrap();
    let db_path = tmp.path().join("ledger.sqlite");
    let mut storage = Storage::open(&db_path).expect("must open");
    let cycle_id = setup_project_workspace_cycle(&mut storage, "provenance");

    let wi = WorkItemRecord {
        id: "wi-chain".into(),
        cycle_id: cycle_id.clone().into(),
        title: "chain test".into(),
        description: "desc".into(),
        status: WorkItemStatus::Active,
        actor_ref_kind: Some("Agent".into()),
        actor_ref_id: Some("agent:test".into()),
        actor_ref_label: Some("test".into()),
        created_at: CREATED_AT,
        schema_version: 1,
    };
    storage
        .insert_work_item(&wi)
        .expect("insert workitem must succeed");

    let ev_record = EvidenceAttachmentRecord {
        id: "ev-chain".into(),
        work_item_id: "wi-chain".into(),
        kind: PlanningEvidenceKind::Log,
        body_ref: "pending".into(),
        actor_ref_kind: Some("Agent".into()),
        actor_ref_id: Some("agent:test".into()),
        actor_ref_label: Some("test".into()),
        schema_version: 1,
    };
    storage
        .insert_evidence_attachment(&ev_record, b"log line")
        .expect("evidence must succeed");

    let dec_record = sddk_domain::planning::DecisionRecordRecord {
        id: "dec-chain".into(),
        work_item_id: "wi-chain".into(),
        kind: DecisionKind::Accept,
        rationale: "Accepted after review".into(),
        actor_ref_kind: Some("Human".into()),
        actor_ref_id: Some("user:bob".into()),
        actor_ref_label: Some("bob".into()),
        schema_version: 1,
    };
    storage
        .insert_decision_record(&dec_record)
        .expect("decision must succeed");

    let chain = storage
        .build_provenance_chain(&cycle_id)
        .expect("build_provenance_chain must succeed");

    assert_eq!(chain.cycle_id, cycle_id);
    assert!(
        chain.work_item_ids.contains(&"wi-chain".to_string()),
        "chain must contain work item id"
    );
    assert!(
        !chain.evidence_refs.is_empty(),
        "chain must contain evidence ref"
    );
    assert!(
        !chain.decision_refs.is_empty(),
        "chain must contain decision ref"
    );
}

#[test]
fn build_provenance_chain_empty_cycle() {
    let tmp = TempDir::new().unwrap();
    let db_path = tmp.path().join("ledger.sqlite");
    let storage = Storage::open(&db_path).expect("must open");

    let chain = storage
        .build_provenance_chain("c-empty-cycle")
        .expect("build_provenance_chain must succeed for empty cycle");
    assert!(chain.work_item_ids.is_empty());
    assert!(chain.evidence_refs.is_empty());
    assert!(chain.decision_refs.is_empty());
}

// ── Identity stability ────────────────────────────────────────────────────────

#[test]
fn graph_identity_deterministic_across_re_lists() {
    let tmp = TempDir::new().unwrap();
    let db_path = tmp.path().join("ledger.sqlite");
    let mut storage = Storage::open(&db_path).expect("must open");
    let cycle_id = setup_project_workspace_cycle(&mut storage, "identity");

    let wi_a = WorkItemRecord {
        id: "wi-ida".into(),
        cycle_id: cycle_id.clone().into(),
        title: "Item A".into(),
        description: "desc a".into(),
        status: WorkItemStatus::Active,
        actor_ref_kind: Some("Agent".into()),
        actor_ref_id: Some("agent:test".into()),
        actor_ref_label: Some("test".into()),
        created_at: CREATED_AT,
        schema_version: 1,
    };
    let wi_b = WorkItemRecord {
        id: "wi-idb".into(),
        cycle_id: cycle_id.clone().into(),
        title: "Item B".into(),
        description: "desc b".into(),
        status: WorkItemStatus::Draft,
        actor_ref_kind: Some("Agent".into()),
        actor_ref_id: Some("agent:test".into()),
        actor_ref_label: Some("test".into()),
        created_at: CREATED_AT,
        schema_version: 1,
    };
    storage.insert_work_item(&wi_a).unwrap();
    storage.insert_work_item(&wi_b).unwrap();

    // Build identity twice via re-listing
    let items_a = storage.list_work_items_by_cycle(&cycle_id).unwrap();
    let domain_items_a: Vec<_> = items_a.iter().map(|r| r.clone().into_domain()).collect();
    let edges_a: Vec<_> = storage
        .list_dependency_edges_by_cycle(&cycle_id)
        .unwrap()
        .iter()
        .map(|r| r.clone().into_domain())
        .collect();
    let ev_refs_a: Vec<_> = items_a
        .iter()
        .flat_map(|wi| {
            storage
                .list_evidence_attachments_by_work_item(&wi.id)
                .unwrap()
        })
        .map(|e| e.body_ref.clone())
        .collect();
    let dec_refs_a: Vec<_> = items_a
        .iter()
        .flat_map(|wi| storage.list_decision_records_by_work_item(&wi.id).unwrap())
        .map(|d| d.id.clone())
        .collect();
    let hash_a =
        compute_planning_graph_identity(&domain_items_a, &edges_a, &ev_refs_a, &dec_refs_a);

    // Re-list and recompute
    let items_b = storage.list_work_items_by_cycle(&cycle_id).unwrap();
    let domain_items_b: Vec<_> = items_b.iter().map(|r| r.clone().into_domain()).collect();
    let edges_b: Vec<_> = storage
        .list_dependency_edges_by_cycle(&cycle_id)
        .unwrap()
        .iter()
        .map(|r| r.clone().into_domain())
        .collect();
    let ev_refs_b: Vec<_> = items_b
        .iter()
        .flat_map(|wi| {
            storage
                .list_evidence_attachments_by_work_item(&wi.id)
                .unwrap()
        })
        .map(|e| e.body_ref.clone())
        .collect();
    let dec_refs_b: Vec<_> = items_b
        .iter()
        .flat_map(|wi| storage.list_decision_records_by_work_item(&wi.id).unwrap())
        .map(|d| d.id.clone())
        .collect();
    let hash_b =
        compute_planning_graph_identity(&domain_items_b, &edges_b, &ev_refs_b, &dec_refs_b);

    assert_eq!(
        hash_a, hash_b,
        "compute_planning_graph_identity must be deterministic across re-lists"
    );
}

// ── Self-loop rejection (AC-PLN2-02 / spec line 90) ──────────────────────────

/// AC-PLN2-02: insert_dependency_edge rejects self-loops (from_id == to_id).
#[test]
fn insert_dependency_edge_rejects_self_loop() {
    let tmp = TempDir::new().unwrap();
    let db_path = tmp.path().join("ledger.sqlite");
    let mut storage = Storage::open(&db_path).expect("must open");

    let cycle_id = setup_project_workspace_cycle(&mut storage, "selfloop");

    // Insert a work item
    let wi_record = WorkItemRecord {
        id: "wi-selfloop-test".into(),
        cycle_id: cycle_id.into(),
        title: "self-loop test".into(),
        description: "desc".into(),
        status: WorkItemStatus::Draft,
        actor_ref_kind: Some("Agent".into()),
        actor_ref_id: Some("agent:test".into()),
        actor_ref_label: Some("test".into()),
        created_at: CREATED_AT,
        schema_version: 1,
    };
    storage
        .insert_work_item(&wi_record)
        .expect("insert workitem must succeed");

    // Self-loop edge must be rejected
    let self_loop_edge = DependencyEdgeRecord {
        from_id: "wi-selfloop-test".into(),
        to_id: "wi-selfloop-test".into(),
        kind: DependencyEdgeKind::Blocks,
        actor_ref_kind: Some("System".into()),
        actor_ref_id: Some("system:planner".into()),
        actor_ref_label: Some("planner".into()),
        schema_version: 1,
    };
    let result = storage.insert_dependency_edge(&self_loop_edge);
    assert!(
        result.is_err(),
        "insert_dependency_edge must reject self-loop"
    );
    let err = result.unwrap_err();
    assert!(
        format!("{}", err).contains("self-loop"),
        "error must mention self-loop: {err}"
    );
}

/// Valid non-self-loop edge must still succeed after self-loop rejection.
#[test]
fn insert_dependency_edge_valid_edge_still_works() {
    let tmp = TempDir::new().unwrap();
    let db_path = tmp.path().join("ledger.sqlite");
    let mut storage = Storage::open(&db_path).expect("must open");

    let cycle_id = setup_project_workspace_cycle(&mut storage, "valid-edge");

    // Insert two work items
    let wi_a = WorkItemRecord {
        id: "wi-valid-a".into(),
        cycle_id: cycle_id.clone().into(),
        title: "A".into(),
        description: "desc".into(),
        status: WorkItemStatus::Draft,
        actor_ref_kind: Some("Agent".into()),
        actor_ref_id: Some("agent:test".into()),
        actor_ref_label: Some("test".into()),
        created_at: CREATED_AT,
        schema_version: 1,
    };
    let wi_b = WorkItemRecord {
        id: "wi-valid-b".into(),
        cycle_id: cycle_id.into(),
        title: "B".into(),
        description: "desc".into(),
        status: WorkItemStatus::Draft,
        actor_ref_kind: Some("Agent".into()),
        actor_ref_id: Some("agent:test".into()),
        actor_ref_label: Some("test".into()),
        created_at: CREATED_AT,
        schema_version: 1,
    };
    storage
        .insert_work_item(&wi_a)
        .expect("insert A must succeed");
    storage
        .insert_work_item(&wi_b)
        .expect("insert B must succeed");

    // Valid edge A → B must succeed
    let edge = DependencyEdgeRecord {
        from_id: "wi-valid-a".into(),
        to_id: "wi-valid-b".into(),
        kind: DependencyEdgeKind::Blocks,
        actor_ref_kind: Some("System".into()),
        actor_ref_id: Some("system:planner".into()),
        actor_ref_label: Some("planner".into()),
        schema_version: 1,
    };
    let result = storage.insert_dependency_edge(&edge);
    assert!(
        result.is_ok(),
        "insert_dependency_edge must accept valid edge: {}",
        result.unwrap_err()
    );
}
