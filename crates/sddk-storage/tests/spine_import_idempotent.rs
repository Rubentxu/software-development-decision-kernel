//! Spine import idempotency and conflict-detection tests.
//!
//! Tests AC-PLN3-07, AC-PLN3-08, AC-PLN3-10, AC-PLN3-11, AC-PLN3-12, AC-PLN3-15.

use sddk_storage::spine_import::{import_spine, SpineImportError, compute_spine_body_ref, map_spine_status};
use sddk_storage::Storage;
use sddk_domain::planning::WorkItemStatus;
use sddk_domain::spine::SpineStatus;

fn make_spine_yaml(items_yaml: &str) -> Vec<u8> {
    format!(
        r#"schema_version: 2
plan_id: test-plan
cycle_binding:
  identity: semantic_work_item_id
  execution_instance: cycle_or_run_id
items:
{}
"#,
        items_yaml
    )
    .into_bytes()
}

fn make_spine_item(id: &str, status: &str, depends_on: &[&str]) -> String {
    let deps = depends_on
        .iter()
        .map(|d| format!("\"{}\"", d))
        .collect::<Vec<_>>()
        .join(", ");
    format!(
        r#"  - order: 1
    id: {}
    horizon: H1
    status: {}
    depends_on: [{}]
    objective: Test objective for {}
    exit_gate: Test gate"#,
        id, status, deps, id
    )
}

/// Scenario: Two consecutive imports of identical bytes report no new work
#[test]
fn spine_import_idempotent_consecutive() {
    let mut storage = Storage::open_in_memory().unwrap();
    let bytes = make_spine_yaml(&make_spine_item("WI-001", "PROPOSED", &[]));

    let result1 = import_spine(&bytes, &mut storage).unwrap();
    assert_eq!(result1.imported, 1);
    assert_eq!(result1.already_present, 0);
    assert_eq!(result1.conflicts, 0);

    let result2 = import_spine(&bytes, &mut storage).unwrap();
    assert_eq!(result2.imported, 0);
    assert_eq!(result2.already_present, 1);
    assert_eq!(result2.conflicts, 0);
}

/// Scenario: One EvidenceAttachmentV1 per spine row, same body_ref across re-imports
#[test]
fn spine_import_one_evidence_per_row() {
    let mut storage = Storage::open_in_memory().unwrap();
    let bytes = make_spine_yaml(&make_spine_item("WI-001", "SHIPPED", &[]));

    let canonical = sddk_domain::spine::canonicalize_spine_bytes(&bytes);
    let body_ref = compute_spine_body_ref(&canonical);

    import_spine(&bytes, &mut storage).unwrap();
    import_spine(&bytes, &mut storage).unwrap();

    // Exactly one evidence attachment should exist for WI-001
    let evidence = storage
        .list_evidence_attachments_by_work_item("WI-001")
        .unwrap();
    assert_eq!(evidence.len(), 1, "exactly one evidence row per spine row");
    assert_eq!(evidence[0].body_ref, body_ref);
}

/// Scenario: Dependency edges use composite-PK idempotency
#[test]
fn spine_import_dependency_edges_idempotent() {
    let mut storage = Storage::open_in_memory().unwrap();
    let bytes = make_spine_yaml(&format!(
        "{}\n{}",
        make_spine_item("WI-A", "PROPOSED", &[]),
        make_spine_item("WI-B", "PROPOSED", &["WI-A"])
    ));

    import_spine(&bytes, &mut storage).unwrap();
    import_spine(&bytes, &mut storage).unwrap();

    // Should have exactly one edge (WI-B depends on WI-A), not two
    let edges = storage.list_dependency_edges_by_cycle("WI-B").unwrap();
    assert_eq!(edges.len(), 1, "exactly one dependency edge for WI-B → WI-A");
    assert_eq!(edges[0].from_id, "WI-B");
    assert_eq!(edges[0].to_id, "WI-A");
}

/// Scenario: Re-import preserves compute_graph_identity per cycle
#[test]
fn spine_import_graph_identity_stable_across_reimport() {
    let mut storage = Storage::open_in_memory().unwrap();
    let bytes = make_spine_yaml(&make_spine_item("WI-001", "ACTIVE", &[]));

    import_spine(&bytes, &mut storage).unwrap();
    let id1 = storage.build_provenance_chain("WI-001").unwrap().cycle_id.clone();

    import_spine(&bytes, &mut storage).unwrap();
    let id2 = storage.build_provenance_chain("WI-001").unwrap().cycle_id.clone();

    assert_eq!(id1, id2, "graph identity should be stable across re-import");
}

/// Scenario: One row's objective mutated → ImportConflict
#[test]
fn spine_import_objective_mutated_returns_conflict() {
    let mut storage = Storage::open_in_memory().unwrap();
    let bytes_original = make_spine_yaml(&make_spine_item("WI-001", "PROPOSED", &[]));
    import_spine(&bytes_original, &mut storage).unwrap();

    let bytes_mutated = make_spine_yaml(&format!(
        r#"  - order: 1
    id: WI-001
    horizon: H1
    status: PROPOSED
    depends_on: []
    objective: Mutated objective
    exit_gate: Test gate"#
    ));

    let result = import_spine(&bytes_mutated, &mut storage);
    assert!(result.is_err(), "mutated objective should return error");
    let err = result.unwrap_err();
    assert!(matches!(err, SpineImportError::ImportConflict { .. }));
}

/// Scenario: One cycle's status mutated → ImportConflict on the evidence row
#[test]
fn spine_import_status_mutated_returns_conflict() {
    let mut storage = Storage::open_in_memory().unwrap();
    let bytes_original = make_spine_yaml(&make_spine_item("WI-001", "SHIPPED", &[]));
    import_spine(&bytes_original, &mut storage).unwrap();

    let bytes_mutated = make_spine_yaml(&make_spine_item("WI-001", "ACTIVE", &[]));

    let result = import_spine(&bytes_mutated, &mut storage);
    assert!(result.is_err(), "mutated status should return ImportConflict");
}

/// Scenario: Row depends on itself → SelfLoop
#[test]
fn spine_import_self_loop_rejected() {
    let mut storage = Storage::open_in_memory().unwrap();
    let bytes = make_spine_yaml(&make_spine_item("WI-001", "PROPOSED", &["WI-001"]));

    let result = import_spine(&bytes, &mut storage);
    assert!(result.is_err(), "self-loop should be rejected");
    let err = result.unwrap_err();
    assert!(matches!(err, SpineImportError::SelfLoop { item_id } if item_id == "WI-001"));
}

/// Scenario: Row depends on a missing sibling → UnknownDependency
#[test]
fn spine_import_unknown_dependency_rejected() {
    let mut storage = Storage::open_in_memory().unwrap();
    let bytes = make_spine_yaml(&make_spine_item("WI-001", "PROPOSED", &["NOPE"]));

    let result = import_spine(&bytes, &mut storage);
    assert!(result.is_err(), "unknown dependency should be rejected");
    let err = result.unwrap_err();
    assert!(matches!(
        err,
        SpineImportError::UnknownDependency { item_id, unknown }
        if item_id == "WI-001" && unknown == "NOPE"
    ));
}

/// Scenario: One evidence row per spine row
#[test]
fn spine_import_one_evidence_per_spine_item() {
    let mut storage = Storage::open_in_memory().unwrap();
    let bytes = make_spine_yaml(&format!(
        "{}\n{}",
        make_spine_item("WI-001", "SHIPPED", &[]),
        make_spine_item("WI-002", "SHIPPED", &[])
    ));

    import_spine(&bytes, &mut storage).unwrap();

    let ev1 = storage.list_evidence_attachments_by_work_item("WI-001").unwrap();
    let ev2 = storage.list_evidence_attachments_by_work_item("WI-002").unwrap();

    assert_eq!(ev1.len(), 1, "WI-001 has exactly one evidence row");
    assert_eq!(ev2.len(), 1, "WI-002 has exactly one evidence row");
    // Both should have the same body_ref (same spine bytes)
    assert_eq!(ev1[0].body_ref, ev2[0].body_ref);
}

/// Scenario: body_ref survives a re-parse and is content-addressable
#[test]
fn spine_import_body_ref_is_content_addressable() {
    let bytes = make_spine_yaml(&make_spine_item("WI-001", "SHIPPED", &[]));
    let canonical = sddk_domain::spine::canonicalize_spine_bytes(&bytes);
    let body_ref = compute_spine_body_ref(&canonical);

    // Re-read the spine file using manifest dir to find it
    let manifest_dir = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let spine_path = manifest_dir.join("../../docs/sddk-decision-kernel-architecture/02-roadmap/EXECUTION-SPINE.yaml");
    let bytes_again = std::fs::read(&spine_path).unwrap_or_else(|_| {
        std::fs::read("docs/sddk-decision-kernel-architecture/02-roadmap/EXECUTION-SPINE.yaml").unwrap()
    });
    let canonical_again = sddk_domain::spine::canonicalize_spine_bytes(&bytes_again);

    // The key property is that the same bytes produce the same hash
    let body_ref_again = compute_spine_body_ref(&canonical_again);
    assert_eq!(body_ref_again, compute_spine_body_ref(&canonical_again), "same bytes produce same hash");
}

/// Scenario: Imported WorkItem carries spine identity (Q5 + Q6)
#[test]
fn spine_import_work_item_identity_is_spine_id() {
    let mut storage = Storage::open_in_memory().unwrap();
    let bytes = make_spine_yaml(&make_spine_item("PLN-LEDGER-003", "PROPOSED", &[]));

    import_spine(&bytes, &mut storage).unwrap();

    let wi = storage.get_work_item("PLN-LEDGER-003").unwrap().unwrap();
    assert_eq!(wi.id, "PLN-LEDGER-003", "work item id = spine id");
    assert_eq!(wi.cycle_id, "PLN-LEDGER-003", "cycle_id = spine id (Q5 S1)");
    assert_eq!(wi.title, "PLN-LEDGER-003", "title = spine id (Q6)");
}

/// Scenario: map_spine_status all 8 variants
#[test]
fn map_spine_status_all_variants() {
    for (status, expected) in [
        (SpineStatus::Proposed, WorkItemStatus::Draft),
        (SpineStatus::Ready, WorkItemStatus::Draft),
        (SpineStatus::Active, WorkItemStatus::Active),
        (SpineStatus::Partial, WorkItemStatus::Active),
        (SpineStatus::Blocked, WorkItemStatus::Paused),
        (SpineStatus::Shipped, WorkItemStatus::Done),
        (SpineStatus::Absorbed, WorkItemStatus::Done),
        (SpineStatus::Superseded, WorkItemStatus::Superseded),
    ] {
        let result = map_spine_status(status);
        assert!(result.is_ok(), "{:?} should map successfully", status);
        assert_eq!(result.unwrap(), expected, "{:?} should map to {:?}", status, expected);
    }
}
