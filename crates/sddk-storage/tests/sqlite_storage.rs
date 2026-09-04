use rusqlite::Connection;
use sddk_domain::{ArtifactStore, CycleId, CycleManifest, CycleStatus};
use sddk_storage::{
    ArtifactRecord, CapabilityReceiptInput, CapabilityStatus, CycleRecord, GateOutcomeStatus,
    GateReceiptInput, GateReceiptNextSeqInput, LedgerEventInput, ProjectRecord, RID_FORMAT_REGEX,
    Storage, StorageError, WorkspaceRecord,
};
use serde_json::json;
use tempfile::tempdir;

const CREATED_AT: &str = "2026-08-03T12:00:00Z";

#[test]
fn persists_canonical_records_across_reopen() {
    let directory = tempdir().unwrap();
    let database_path = directory.path().join("state/ledger.sqlite");
    let cycle = cycle_record();
    let artifact = ArtifactRecord {
        artifact_id: "artifact-1".into(),
        project_id: "project-1".into(),
        cycle_id: Some(cycle.manifest.cycle_id.clone()),
        kind: "specification".into(),
        path: "sha256/ab/spec.md".into(),
        sha256: Some(format!("sha256:{}", "a".repeat(64))),
        producer: Some("sddk-spec".into()),
        created_at: CREATED_AT.into(),
        metadata: json!({"media_type": "text/markdown"}),
    };

    {
        let storage = Storage::open(&database_path).unwrap();
        // MIGRATION_4 adds 'waived' to gate_receipts.outcome CHECK;
        // MIGRATION_5 adds events_v1; MIGRATION_6 adds projection_checkpoints_v1
        // MIGRATION_7 adds agent/behavior_version_hash to capability_receipts
        assert_eq!(storage.schema_version().unwrap(), 14);
        storage.insert_project(&project_record()).unwrap();
        storage.insert_workspace(&workspace_record()).unwrap();
        storage.insert_cycle(&cycle).unwrap();
        storage.insert_artifact(&artifact).unwrap();
    }

    let storage = Storage::open(&database_path).unwrap();
    assert_eq!(storage.get_project("project-1").unwrap(), project_record());
    assert_eq!(
        storage.get_workspace("workspace-1").unwrap(),
        workspace_record()
    );
    assert_eq!(storage.get_cycle(&cycle.manifest.cycle_id).unwrap(), cycle);
    assert_eq!(storage.get_artifact("artifact-1").unwrap(), artifact);
}

#[test]
fn adoption_registration_is_transactional_idempotent_and_conflict_safe() {
    let mut storage = Storage::open_in_memory().unwrap();
    let project = project_record();
    let workspace = workspace_record();

    storage
        .register_project_workspace(&project, &workspace)
        .unwrap();
    storage
        .register_project_workspace(
            &ProjectRecord {
                display_name: "Another checkout label".into(),
                created_at: "2026-08-04T00:00:00Z".into(),
                ..project.clone()
            },
            &WorkspaceRecord {
                created_at: "2026-08-04T00:00:00Z".into(),
                ..workspace.clone()
            },
        )
        .unwrap();

    let conflicting = ProjectRecord {
        remote_url: Some("https://example.com/other/project".into()),
        ..project
    };
    assert!(matches!(
        storage.register_project_workspace(&conflicting, &workspace),
        Err(StorageError::RegistrationConflict {
            entity: "project",
            ..
        })
    ));
    assert_eq!(storage.get_project("project-1").unwrap(), project_record());
    assert_eq!(
        storage.get_workspace("workspace-1").unwrap(),
        workspace_record()
    );
}

#[test]
fn ledger_is_hash_linked_ordered_and_append_only() {
    let directory = tempdir().unwrap();
    let database_path = directory.path().join("ledger.sqlite");
    let mut storage = Storage::open(&database_path).unwrap();
    storage.insert_project(&project_record()).unwrap();

    let first = storage.append_event(&event("event-1", None)).unwrap();
    let second = storage.append_event(&event("event-2", None)).unwrap();

    assert_eq!(first.sequence, 1);
    assert_eq!(second.sequence, 2);
    assert_eq!(
        second.previous_hash.as_deref(),
        Some(first.event_hash.as_str())
    );
    let events = storage.list_events().unwrap();
    assert_eq!(
        events
            .iter()
            .map(|event| event.sequence)
            .collect::<Vec<_>>(),
        [1, 2]
    );
    let verification = storage.verify_ledger().unwrap();
    assert_eq!(verification.event_count, 2);
    assert_eq!(verification.last_hash, Some(second.event_hash));

    drop(storage);
    let connection = Connection::open(&database_path).unwrap();
    assert!(
        connection
            .execute(
                "UPDATE ledger_events SET actor = 'tampered' WHERE sequence = 1",
                []
            )
            .is_err()
    );
    assert!(
        connection
            .execute("DELETE FROM ledger_events WHERE sequence = 1", [])
            .is_err()
    );
}

#[test]
fn cycle_event_listing_is_scoped_and_ordered() {
    let mut storage = Storage::open_in_memory().unwrap();
    storage.insert_project(&project_record()).unwrap();
    storage.insert_workspace(&workspace_record()).unwrap();
    let cycle = cycle_record();
    let cycle_id = cycle.manifest.cycle_id.clone();
    storage.insert_cycle(&cycle).unwrap();
    storage.append_event(&event("event-project", None)).unwrap();
    storage
        .append_event(&event("event-cycle-1", Some(&cycle_id)))
        .unwrap();
    storage
        .append_event(&event("event-cycle-2", Some(&cycle_id)))
        .unwrap();

    let events = storage.list_cycle_events(&cycle_id).unwrap();
    assert_eq!(
        events
            .iter()
            .map(|event| event.event_id.as_str())
            .collect::<Vec<_>>(),
        ["event-cycle-1", "event-cycle-2"]
    );
    assert!(
        storage
            .list_cycle_events("missing-cycle")
            .unwrap()
            .is_empty()
    );
}

#[test]
fn capability_receipts_begin_once_and_finalize_only_from_started() {
    let mut storage = Storage::open_in_memory().unwrap();
    storage.insert_project(&project_record()).unwrap();
    let input = capability_receipt("receipt-1", json!({"branch": "feature"}));

    let inserted = storage.begin_capability_receipt(&input).unwrap();
    assert_eq!(inserted.status, CapabilityStatus::Started);
    assert_eq!(inserted.completed_at, None);

    let replay_input = CapabilityReceiptInput {
        receipt_id: "receipt-2".into(),
        ..input.clone()
    };
    let replayed = storage.begin_capability_receipt(&replay_input).unwrap();
    assert_eq!(replayed, inserted);
    assert!(matches!(
        storage.get_capability_receipt("receipt-2"),
        Err(StorageError::NotFound { .. })
    ));

    let conflicting = CapabilityReceiptInput {
        receipt_id: "receipt-3".into(),
        capability: "git.delete_branch".into(),
        ..input
    };
    assert!(matches!(
        storage.begin_capability_receipt(&conflicting),
        Err(StorageError::IdempotencyConflict { .. })
    ));

    let finalized = storage
        .finalize_capability_receipt(
            "receipt-1",
            CapabilityStatus::Succeeded,
            Some(json!({"merged": true})),
            "2026-08-04T10:00:01Z",
        )
        .unwrap();
    assert_eq!(finalized.status, CapabilityStatus::Succeeded);
    assert_eq!(
        finalized.completed_at.as_deref(),
        Some("2026-08-04T10:00:01Z")
    );

    assert!(matches!(
        storage.finalize_capability_receipt(
            "receipt-1",
            CapabilityStatus::Failed,
            None,
            "2026-08-04T10:00:02Z"
        ),
        Err(StorageError::TerminalReceipt { .. })
    ));

    let listed = storage.list_capability_receipts("project-1").unwrap();
    assert_eq!(listed.len(), 1);
    assert_eq!(listed[0].status, CapabilityStatus::Succeeded);
}

#[test]
fn capability_receipts_reject_terminal_begins() {
    let mut storage = Storage::open_in_memory().unwrap();
    storage.insert_project(&project_record()).unwrap();
    let input = CapabilityReceiptInput {
        status: CapabilityStatus::Succeeded,
        ..capability_receipt("receipt-1", json!({}))
    };
    assert!(matches!(
        storage.begin_capability_receipt(&input),
        Err(StorageError::InvalidReceiptBegin)
    ));
}

#[test]
fn uniqueness_and_lease_conflicts_are_enforced() {
    let (mut storage, cycle) = storage_with_cycle();
    let duplicate_identity = ProjectRecord {
        project_id: "project-2".into(),
        display_name: "Duplicate".into(),
        ..project_record()
    };
    assert!(matches!(
        storage.insert_project(&duplicate_identity),
        Err(StorageError::Database(_))
    ));

    let first = storage
        .acquire_cycle_lease(&cycle.manifest.cycle_id, "runtime-a", 1_000, 2_000)
        .unwrap();
    assert_eq!(first.fencing_token, 1);
    assert!(matches!(
        storage.acquire_cycle_lease(&cycle.manifest.cycle_id, "runtime-b", 1_500, 2_500),
        Err(StorageError::LeaseConflict { .. })
    ));

    let recovered = storage
        .acquire_cycle_lease(&cycle.manifest.cycle_id, "runtime-b", 2_000, 3_000)
        .unwrap();
    assert_eq!(recovered.fencing_token, 2);
    assert_eq!(
        storage.get_cycle_lease(&cycle.manifest.cycle_id).unwrap(),
        recovered
    );
    assert!(
        !storage
            .release_cycle_lease(
                "project-1",
                &cycle.manifest.cycle_id,
                "runtime-a",
                1,
                "tester",
                "command-1",
                "2026-08-13T15:00:00Z",
            )
            .unwrap()
    );
    assert!(
        storage
            .release_cycle_lease(
                "project-1",
                &cycle.manifest.cycle_id,
                "runtime-b",
                2,
                "tester",
                "command-2",
                "2026-08-13T15:00:01Z",
            )
            .unwrap()
    );
}

#[test]
fn renew_cycle_lease_extends_expiry_preserving_token() {
    let (mut storage, cycle) = storage_with_cycle();
    let first = storage
        .acquire_cycle_lease(&cycle.manifest.cycle_id, "runtime-a", 1_000, 2_000)
        .unwrap();
    assert_eq!(first.fencing_token, 1);

    let renewed = storage
        .renew_cycle_lease(&cycle.manifest.cycle_id, "runtime-a", 1, 1_500, 5_000)
        .unwrap();
    assert_eq!(renewed.fencing_token, 1);
    assert_eq!(renewed.expires_at_ms, 5_000);
    assert_eq!(renewed.acquired_at_ms, 1_000);

    let verified = storage
        .verify_cycle_lease(&cycle.manifest.cycle_id, "runtime-a", 1, 1_500)
        .unwrap();
    assert_eq!(verified.expires_at_ms, 5_000);
}

#[test]
fn renew_cycle_lease_fails_with_wrong_owner() {
    let (mut storage, cycle) = storage_with_cycle();
    storage
        .acquire_cycle_lease(&cycle.manifest.cycle_id, "runtime-a", 1_000, 2_000)
        .unwrap();

    let error = storage
        .renew_cycle_lease(&cycle.manifest.cycle_id, "runtime-b", 1, 1_500, 5_000)
        .unwrap_err();
    assert!(matches!(
        error,
        StorageError::LeaseNotRenewable { ref current_owner, .. } if current_owner == "runtime-a"
    ));

    let verified = storage
        .verify_cycle_lease(&cycle.manifest.cycle_id, "runtime-a", 1, 1_500)
        .unwrap();
    assert_eq!(verified.expires_at_ms, 2_000);
}

#[test]
fn renew_cycle_lease_fails_with_stale_fencing_token() {
    let (mut storage, cycle) = storage_with_cycle();
    storage
        .acquire_cycle_lease(&cycle.manifest.cycle_id, "runtime-a", 1_000, 2_000)
        .unwrap();

    let error = storage
        .renew_cycle_lease(&cycle.manifest.cycle_id, "runtime-a", 99, 1_500, 5_000)
        .unwrap_err();
    assert!(matches!(
        error,
        StorageError::LeaseNotRenewable {
            current_fencing_token: 1,
            ..
        }
    ));
}

// REQ-FSI-001: lease fence rejects expired leases deterministically.
#[test]
fn verify_rejects_expired_lease_returns_lease_expired() {
    let (mut storage, cycle) = storage_with_cycle();
    storage
        .acquire_cycle_lease(&cycle.manifest.cycle_id, "runtime-a", 1_000, 2_000)
        .unwrap();

    // `now_ms` exactly equal to the expiry instant must already reject
    // (fail-closed, see design §8).
    let err = storage
        .verify_cycle_lease(&cycle.manifest.cycle_id, "runtime-a", 1, 2_000)
        .unwrap_err();
    assert!(matches!(
        err,
        StorageError::LeaseExpired { ref owner, fencing_token: 1, expires_at_ms: 2_000, now_ms: 2_000, .. } if owner == "runtime-a"
    ));

    // Past the expiry instant also rejects.
    let err = storage
        .verify_cycle_lease(&cycle.manifest.cycle_id, "runtime-a", 1, 9_999)
        .unwrap_err();
    assert!(matches!(
        err,
        StorageError::LeaseExpired { now_ms: 9_999, .. }
    ));
}

#[test]
fn verify_accepts_unexpired_lease_returns_ok() {
    let (mut storage, cycle) = storage_with_cycle();
    storage
        .acquire_cycle_lease(&cycle.manifest.cycle_id, "runtime-a", 1_000, 5_000)
        .unwrap();
    let verified = storage
        .verify_cycle_lease(&cycle.manifest.cycle_id, "runtime-a", 1, 4_999)
        .unwrap();
    assert_eq!(verified.expires_at_ms, 5_000);
}

#[test]
fn reacquire_after_expiry_preserves_acquire_semantics() {
    let (mut storage, cycle) = storage_with_cycle();
    storage
        .acquire_cycle_lease(&cycle.manifest.cycle_id, "runtime-a", 1_000, 2_000)
        .unwrap();
    // After expiry, a different agent can still acquire (existing semantic).
    let second = storage
        .acquire_cycle_lease(&cycle.manifest.cycle_id, "runtime-b", 2_500, 4_000)
        .unwrap();
    assert_eq!(second.fencing_token, 2);
    assert_eq!(second.owner, "runtime-b");
}

#[test]
fn release_cycle_lease_writes_lease_released_event() {
    let (mut storage, cycle) = storage_with_cycle();
    storage
        .acquire_cycle_lease(&cycle.manifest.cycle_id, "runtime-a", 1_000, 2_000)
        .unwrap();

    let released = storage
        .release_cycle_lease(
            "project-1",
            &cycle.manifest.cycle_id,
            "runtime-a",
            1,
            "tester-1",
            "cycle.lock.release-1",
            "2026-08-13T15:00:00Z",
        )
        .unwrap();
    assert!(released);

    let events = storage.list_events().unwrap();
    let event = events
        .iter()
        .find(|event| event.event_type == "lease.released")
        .expect("lease.released event must be appended");
    assert_eq!(event.actor, "tester-1");
    assert_eq!(event.command_id, "cycle.lock.release-1");
    assert_eq!(event.frame_id, "frame:cycle.lock.release-1");
    assert_eq!(
        event.payload,
        json!({
            "cycle_id": cycle.manifest.cycle_id.as_str(),
            "owner": "runtime-a",
            "fencing_token": 1,
            "actor": "tester-1",
        })
    );

    let miss = storage
        .release_cycle_lease(
            "project-1",
            &cycle.manifest.cycle_id,
            "runtime-a",
            1,
            "tester-1",
            "cycle.lock.release-2",
            "2026-08-13T15:00:01Z",
        )
        .unwrap();
    assert!(!miss);
}

// ─────────────────────────────────────────────────────────────────────────────
// REQ-DEBT017-1: cycle_exists helper
// ─────────────────────────────────────────────────────────────────────────────

/// REQ-DEBT017-1: cycle_exists returns true for existing and false for missing.
#[test]
fn cycle_exists_returns_true_for_existing_and_false_for_missing() {
    let (storage, cycle) = storage_with_cycle();

    // Case 1: existing cycle → true
    let found = storage
        .cycle_exists(&cycle.manifest.cycle_id)
        .expect("cycle_exists must not error");
    assert!(
        found,
        "cycle_exists should return true for existing cycle {}",
        cycle.manifest.cycle_id
    );

    // Case 2: non-existing cycle → false
    let missing = storage
        .cycle_exists("p-NONEXISTENT/never-created")
        .expect("cycle_exists must not error");
    assert!(
        !missing,
        "cycle_exists should return false for non-existing cycle"
    );
}

#[test]
fn failed_event_append_rolls_back_cycle_state_update() {
    let (mut storage, cycle) = storage_with_cycle();
    let initial_event = event("event-1", Some(&cycle.manifest.cycle_id));
    storage.append_event(&initial_event).unwrap();

    let mut blocked = cycle.manifest.clone();
    blocked.status = CycleStatus::Blocked;
    let duplicate_event = LedgerEventInput {
        state_before: Some(json!({"status": "OPEN"})),
        state_after: Some(json!({"status": "BLOCKED"})),
        ..initial_event
    };

    assert!(matches!(
        storage.update_cycle_with_event(&blocked, "2026-08-03T12:01:00Z", &duplicate_event, false),
        Err(StorageError::Database(_))
    ));
    assert_eq!(
        storage
            .get_cycle(&cycle.manifest.cycle_id)
            .unwrap()
            .manifest
            .status,
        CycleStatus::Open
    );
    assert_eq!(storage.list_events().unwrap().len(), 1);
}

fn storage_with_cycle() -> (Storage, CycleRecord) {
    let storage = Storage::open_in_memory().unwrap();
    storage.insert_project(&project_record()).unwrap();
    storage.insert_workspace(&workspace_record()).unwrap();
    let cycle = cycle_record();
    storage.insert_cycle(&cycle).unwrap();
    (storage, cycle)
}

fn project_record() -> ProjectRecord {
    ProjectRecord {
        project_id: "project-1".into(),
        display_name: "Project One".into(),
        remote_url: Some("https://example.com/owner/project".into()),
        scope: "owner".into(),
        created_at: CREATED_AT.into(),
    }
}

fn workspace_record() -> WorkspaceRecord {
    WorkspaceRecord {
        workspace_id: "workspace-1".into(),
        project_id: "project-1".into(),
        canonical_path: "/work/project".into(),
        created_at: CREATED_AT.into(),
    }
}

fn cycle_record() -> CycleRecord {
    let manifest = CycleManifest::new(
        "project-1".into(),
        "workspace-1".into(),
        CycleId::new("project-1/change").unwrap(),
        "Change".into(),
        "sddk/change".into(),
        "abc123".into(),
    );
    CycleRecord {
        manifest,
        created_at: CREATED_AT.into(),
        updated_at: CREATED_AT.into(),
    }
}

fn event(event_id: &str, cycle_id: Option<&str>) -> LedgerEventInput {
    LedgerEventInput {
        event_id: event_id.into(),
        project_id: "project-1".into(),
        cycle_id: cycle_id.map(str::to_owned),
        frame_id: "frame-1".into(),
        command_id: "command-1".into(),
        actor: "runtime".into(),
        actor_ref: None,
        event_type: "cycle.state_changed".into(),
        occurred_at: CREATED_AT.into(),
        state_before: None,
        state_after: None,
        payload: json!({"event": event_id}),
        causation_id: None,
        correlation_id: None,
    }
}

fn capability_receipt(receipt_id: &str, request: serde_json::Value) -> CapabilityReceiptInput {
    CapabilityReceiptInput {
        receipt_id: receipt_id.into(),
        project_id: "project-1".into(),
        cycle_id: None,
        capability: "git.create_branch".into(),
        idempotency_key: "create-feature-branch".into(),
        request,
        status: CapabilityStatus::Started,
        result: None,
        started_at: CREATED_AT.into(),
        completed_at: None,
        agent_version_hash: None,
        behavior_version_hash: None,
    }
}

fn gate_receipt_input_full(
    receipt_id: &str,
    gate: &str,
    plan_hash: &str,
    outcome: GateOutcomeStatus,
    cycle_id: &str,
) -> GateReceiptInput {
    GateReceiptInput {
        receipt_id: receipt_id.into(),
        project_id: "project-1".into(),
        cycle_id: Some(cycle_id.into()),
        gate: gate.into(),
        evaluator: "sddk.cli".into(),
        transition_id: "phase.explore.complete".into(),
        plan_hash: plan_hash.into(),
        outcome,
        evidence: json!({"verified": true}),
        actor: "test-runtime".into(),
        actor_ref: None,
        command_id: "cmd-1".into(),
        frame_id: "frame-1".into(),
        evaluated_at: CREATED_AT.into(),
        seq: 1,
        causation_id: None,
        correlation_id: None,
    }
}

fn gate_receipt_next_seq_input(
    gate: &str,
    plan_hash: &str,
    outcome: GateOutcomeStatus,
    cycle_id: &str,
) -> GateReceiptNextSeqInput {
    GateReceiptNextSeqInput {
        project_id: "project-1".into(),
        cycle_id: Some(cycle_id.into()),
        gate: gate.into(),
        evaluator: "sddk.cli".into(),
        transition_id: "phase.explore.complete".into(),
        plan_hash: plan_hash.into(),
        outcome,
        evidence: json!({"verified": true}),
        actor: "test-runtime".into(),
        actor_ref: None,
        command_id: "cmd-1".into(),
        frame_id: "frame-1".into(),
        evaluated_at: CREATED_AT.into(),
        causation_id: None,
        correlation_id: None,
    }
}

#[test]
fn storage_insert_gate_receipt_allocates_seq_per_group() {
    let mut storage = Storage::open_in_memory().unwrap();
    storage.insert_project(&project_record()).unwrap();
    storage.insert_workspace(&workspace_record()).unwrap();
    let cycle = cycle_record();
    let cycle_id = cycle.manifest.cycle_id.as_str();
    storage.insert_cycle(&cycle).unwrap();

    let plan_hash = "sha256:abcdef1234567890abcdef1234567890abcdef1234567890abcdef1234567890";

    // First insert for (gate, plan_hash) yields seq=1 and correct rid
    let first = storage
        .insert_gate_receipt_next_seq(&gate_receipt_next_seq_input(
            "exploration-sufficient",
            plan_hash,
            GateOutcomeStatus::Passed,
            cycle_id,
        ))
        .unwrap();
    assert_eq!(first.seq, 1);
    assert_eq!(
        first.receipt_id,
        "gate-exploration-sufficient-abcdef1234567890-1"
    );

    // Second insert yields seq=2 and correct rid
    let second = storage
        .insert_gate_receipt_next_seq(&GateReceiptNextSeqInput {
            project_id: "project-1".into(),
            cycle_id: Some(cycle_id.into()),
            gate: "exploration-sufficient".into(),
            evaluator: "sddk.cli".into(),
            transition_id: "phase.explore.complete".into(),
            plan_hash: plan_hash.into(),
            outcome: GateOutcomeStatus::Failed,
            evidence: json!({"verified": false}),
            actor: "test-runtime".into(),
            actor_ref: None,
            command_id: "cmd-2".into(),
            frame_id: "frame-2".into(),
            evaluated_at: CREATED_AT.into(),
            causation_id: None,
            correlation_id: None,
        })
        .unwrap();
    assert_eq!(second.seq, 2);
    assert_eq!(
        second.receipt_id,
        "gate-exploration-sufficient-abcdef1234567890-2"
    );

    // Both rows readable via list_gate_receipts
    let receipts = storage.list_gate_receipts(cycle_id).unwrap();
    assert_eq!(receipts.len(), 2);
    assert_eq!(receipts[0].seq, 1);
    assert_eq!(receipts[1].seq, 2);
}

#[test]
fn storage_insert_gate_receipt_concurrent_allocations_observe_distinct_seq() {
    use std::sync::{Arc, Barrier};
    use std::thread;

    // NOTE: The primary protection against the seq-split race is
    // ARCHITECTURAL: `allocate_gate_receipt_seq` was removed in this
    // branch so `seq` is always allocated inside the same IMMEDIATE
    // transaction as the INSERT in `insert_gate_receipt_next_seq`.
    // Reintroducing a separate `allocate_gate_receipt_seq` helper
    // would be visible in code review and would re-open the race.
    // This test only validates that the remaining invariant
    // (contiguous 1..=201 sequence) holds under real thread
    // contention (Barrier + file-backed SQLite + two concurrent
    // connections).

    let directory = tempfile::tempdir().unwrap();
    let database_path = directory.path().join("ledger.sqlite");

    // Setup: create storage, insert project/workspace/cycle, then drop storage
    let cycle = cycle_record();
    let cycle_id = cycle.manifest.cycle_id.as_str().to_string();
    {
        let mut storage = Storage::open(&database_path).unwrap();
        storage.insert_project(&project_record()).unwrap();
        storage.insert_workspace(&workspace_record()).unwrap();
        storage.insert_cycle(&cycle).unwrap();
        // Insert first receipt with seq=1 (using old API to bootstrap).
        // The rid uses the 16-hex slice [7..23] of the plan_hash, so it
        // matches what `build_gate_receipt_id` would produce for seq=1
        // (consistent with the concurrent inserts below).
        storage
            .insert_gate_receipt(&gate_receipt_input_full(
                "gate-exploration-sufficient-abcdef1234567890-1",
                "exploration-sufficient",
                "sha256:abcdef1234567890abcdef1234567890abcdef1234567890abcdef1234567890",
                GateOutcomeStatus::Passed,
                &cycle_id,
            ))
            .unwrap();
    }

    let plan_hash = "sha256:abcdef1234567890abcdef1234567890abcdef1234567890abcdef1234567890";
    let gate = "exploration-sufficient";
    let database_path = Arc::new(database_path.clone());
    let barrier = Arc::new(Barrier::new(2));
    let plan_hash = plan_hash.to_string();
    let gate = gate.to_string();
    let cycle_id_thread = cycle_id.clone();

    // Two threads each open their own Storage connection to the same database
    // Each thread inserts 100 times (total 200 inserts, seq 2..=201)
    let handle_a = {
        let database_path = Arc::clone(&database_path);
        let barrier = Arc::clone(&barrier);
        let plan_hash = plan_hash.clone();
        let gate = gate.clone();
        let cycle_id = cycle_id_thread.clone();
        thread::spawn(move || {
            barrier.wait();
            let mut storage = Storage::open(&*database_path).unwrap();
            for i in 0..100 {
                storage
                    .insert_gate_receipt_next_seq(&GateReceiptNextSeqInput {
                        project_id: "project-1".into(),
                        cycle_id: Some(cycle_id.clone()),
                        gate: gate.clone(),
                        evaluator: "sddk.cli".into(),
                        transition_id: "phase.explore.complete".into(),
                        plan_hash: plan_hash.clone(),
                        outcome: GateOutcomeStatus::Passed,
                        evidence: json!({"verified": true}),
                        actor: "test-runtime".into(),
                        actor_ref: None,
                        command_id: format!("cmd-a-{}", i),
                        frame_id: format!("frame-a-{}", i),
                        evaluated_at: CREATED_AT.into(),
                        causation_id: None,
                        correlation_id: None,
                    })
                    .unwrap();
            }
        })
    };

    let handle_b = {
        let database_path = Arc::clone(&database_path);
        let barrier = Arc::clone(&barrier);
        let plan_hash = plan_hash.clone();
        let gate = gate.clone();
        let cycle_id = cycle_id_thread.clone();
        thread::spawn(move || {
            barrier.wait();
            let mut storage = Storage::open(&*database_path).unwrap();
            for i in 0..100 {
                storage
                    .insert_gate_receipt_next_seq(&GateReceiptNextSeqInput {
                        project_id: "project-1".into(),
                        cycle_id: Some(cycle_id.clone()),
                        gate: gate.clone(),
                        evaluator: "sddk.cli".into(),
                        transition_id: "phase.explore.complete".into(),
                        plan_hash: plan_hash.clone(),
                        outcome: GateOutcomeStatus::Failed,
                        evidence: json!({"verified": false}),
                        actor: "test-runtime".into(),
                        actor_ref: None,
                        command_id: format!("cmd-b-{}", i),
                        frame_id: format!("frame-b-{}", i),
                        evaluated_at: CREATED_AT.into(),
                        causation_id: None,
                        correlation_id: None,
                    })
                    .unwrap();
            }
        })
    };

    handle_a.join().unwrap();
    handle_b.join().unwrap();

    // Final state has 201 rows: initial seq=1, plus 200 concurrent inserts (seq 2..=201)
    let storage = Storage::open(&*database_path).unwrap();
    let receipts = storage.list_gate_receipts(&cycle_id).unwrap();
    assert_eq!(receipts.len(), 201);
    let mut seqs: Vec<i64> = receipts.iter().map(|r| r.seq).collect();
    seqs.sort_unstable();
    // seqs must be exactly 1..=201 with no gaps and no duplicates
    let expected: Vec<i64> = (1..=201).collect();
    assert_eq!(seqs, expected);
}

#[test]
fn storage_insert_gate_receipt_next_seq_golden_rid_format() {
    // Golden byte-identical receipt_id: v1.9.17 compatible format
    // rid = "gate-{gate}-{plan_hash[7..23]}-{seq}"
    // plan_hash = "sha256:abcdef1234567890..." → [7..23] = "abcdef1234567890" (16 hex chars)
    let mut storage = Storage::open_in_memory().unwrap();
    storage.insert_project(&project_record()).unwrap();
    storage.insert_workspace(&workspace_record()).unwrap();
    let cycle = cycle_record();
    let cycle_id = cycle.manifest.cycle_id.as_str();
    storage.insert_cycle(&cycle).unwrap();

    let plan_hash = "sha256:abcdef1234567890abcdef1234567890abcdef1234567890abcdef1234567890";
    let receipt = storage
        .insert_gate_receipt_next_seq(&gate_receipt_next_seq_input(
            "exploration-sufficient",
            plan_hash,
            GateOutcomeStatus::Passed,
            cycle_id,
        ))
        .unwrap();

    // Byte-identical to v1.9.17 format
    assert_eq!(
        receipt.receipt_id,
        "gate-exploration-sufficient-abcdef1234567890-1"
    );
    // Canonical regex from sddk_storage::RID_FORMAT_REGEX
    let rid_regex = regex::Regex::new(RID_FORMAT_REGEX).unwrap();
    assert!(
        rid_regex.is_match(&receipt.receipt_id),
        "receipt_id '{}' must match RID_FORMAT_REGEX",
        receipt.receipt_id
    );
}

#[test]
fn storage_migration_3_backfills_seq_default_one() {
    let directory = tempfile::tempdir().unwrap();
    let database_path = directory.path().join("ledger.sqlite");

    // Create a v1.9.14-shaped database (schema v2, no seq column)
    {
        let conn = Connection::open(&database_path).unwrap();
        conn.execute_batch(
            r#"
            CREATE TABLE projects (
                project_id TEXT PRIMARY KEY,
                display_name TEXT NOT NULL,
                remote_url TEXT,
                scope TEXT NOT NULL,
                created_at TEXT NOT NULL
            );
            CREATE TABLE workspaces (
                workspace_id TEXT PRIMARY KEY,
                project_id TEXT NOT NULL REFERENCES projects(project_id),
                canonical_path TEXT NOT NULL,
                created_at TEXT NOT NULL
            );
            CREATE TABLE cycles (
                cycle_id TEXT PRIMARY KEY,
                project_id TEXT NOT NULL,
                workspace_id TEXT NOT NULL,
                status TEXT NOT NULL,
                phase TEXT NOT NULL,
                manifest_json TEXT NOT NULL,
                created_at TEXT NOT NULL,
                updated_at TEXT NOT NULL
            );
            CREATE TABLE gate_receipts (
                receipt_id TEXT PRIMARY KEY,
                project_id TEXT NOT NULL REFERENCES projects(project_id),
                cycle_id TEXT,
                gate TEXT NOT NULL,
                evaluator TEXT NOT NULL,
                transition_id TEXT NOT NULL,
                plan_hash TEXT NOT NULL,
                outcome TEXT NOT NULL,
                evidence TEXT NOT NULL,
                actor TEXT NOT NULL,
                command_id TEXT NOT NULL,
                frame_id TEXT NOT NULL,
                evaluated_at TEXT NOT NULL
            );
            "#,
        )
        .unwrap();
        conn.execute(
            "INSERT INTO projects VALUES ('project-1', 'Project One', 'https://example.com/owner/project', 'owner', '2026-08-03T12:00:00Z')",
            [],
        )
        .unwrap();
        conn.execute(
            "INSERT INTO workspaces VALUES ('workspace-1', 'project-1', '/work/project', '2026-08-03T12:00:00Z')",
            [],
        )
        .unwrap();
        conn.execute(
            "INSERT INTO cycles VALUES ('cycle-1', 'project-1', 'workspace-1', 'OPEN', 'explore', '{}', '2026-08-03T12:00:00Z', '2026-08-03T12:00:00Z')",
            [],
        )
        .unwrap();
        conn.execute(
            "INSERT INTO gate_receipts VALUES (
                'gate-exploration-sufficient-abcdef12',
                'project-1',
                'cycle-1',
                'exploration-sufficient',
                'sddk.cli',
                'phase.explore.complete',
                'sha256:abcdef1234567890abcdef1234567890abcdef1234567890abcdef1234567890',
                'passed',
                '{}',
                'test-runtime',
                'cmd-1',
                'frame-1',
                '2026-08-03T12:00:00Z'
            )",
            [],
        )
        .unwrap();
        // Set schema version to 2 (pre-migration)
        conn.pragma_update(None, "user_version", 2).unwrap();
    }

    // Open with current code — MIGRATION_3..MIGRATION_13 all run
    let storage = Storage::open(&database_path).unwrap();
    assert_eq!(storage.schema_version().unwrap(), 14);

    // The pre-existing row now carries seq = 1
    let receipt = storage
        .get_gate_receipt("gate-exploration-sufficient-abcdef12")
        .unwrap();
    assert_eq!(receipt.seq, 1);
}

#[test]
fn storage_get_gate_receipt_handles_v1914_id_without_seq_suffix() {
    let directory = tempfile::tempdir().unwrap();
    let database_path = directory.path().join("ledger.sqlite");

    // Create a v1.9.14-shaped database
    {
        let conn = Connection::open(&database_path).unwrap();
        conn.execute_batch(
            r#"
            CREATE TABLE projects (
                project_id TEXT PRIMARY KEY,
                display_name TEXT NOT NULL,
                remote_url TEXT,
                scope TEXT NOT NULL,
                created_at TEXT NOT NULL
            );
            CREATE TABLE workspaces (
                workspace_id TEXT PRIMARY KEY,
                project_id TEXT NOT NULL REFERENCES projects(project_id),
                canonical_path TEXT NOT NULL,
                created_at TEXT NOT NULL
            );
            CREATE TABLE cycles (
                cycle_id TEXT PRIMARY KEY,
                project_id TEXT NOT NULL,
                workspace_id TEXT NOT NULL,
                status TEXT NOT NULL,
                phase TEXT NOT NULL,
                manifest_json TEXT NOT NULL,
                created_at TEXT NOT NULL,
                updated_at TEXT NOT NULL
            );
            CREATE TABLE gate_receipts (
                receipt_id TEXT PRIMARY KEY,
                project_id TEXT NOT NULL REFERENCES projects(project_id),
                cycle_id TEXT,
                gate TEXT NOT NULL,
                evaluator TEXT NOT NULL,
                transition_id TEXT NOT NULL,
                plan_hash TEXT NOT NULL,
                outcome TEXT NOT NULL,
                evidence TEXT NOT NULL,
                actor TEXT NOT NULL,
                command_id TEXT NOT NULL,
                frame_id TEXT NOT NULL,
                evaluated_at TEXT NOT NULL
            );
            "#,
        )
        .unwrap();
        conn.execute(
            "INSERT INTO projects VALUES ('project-1', 'Project One', 'https://example.com/owner/project', 'owner', '2026-08-03T12:00:00Z')",
            [],
        )
        .unwrap();
        conn.execute(
            "INSERT INTO workspaces VALUES ('workspace-1', 'project-1', '/work/project', '2026-08-03T12:00:00Z')",
            [],
        )
        .unwrap();
        conn.execute(
            "INSERT INTO cycles VALUES ('cycle-1', 'project-1', 'workspace-1', 'OPEN', 'explore', '{}', '2026-08-03T12:00:00Z', '2026-08-03T12:00:00Z')",
            [],
        )
        .unwrap();
        conn.execute(
            "INSERT INTO gate_receipts VALUES (
                'gate-exploration-sufficient-abcdef12',
                'project-1',
                'cycle-1',
                'exploration-sufficient',
                'sddk.cli',
                'phase.explore.complete',
                'sha256:abcdef1234567890abcdef1234567890abcdef1234567890abcdef1234567890',
                'passed',
                '{}',
                'test-runtime',
                'cmd-1',
                'frame-1',
                '2026-08-03T12:00:00Z'
            )",
            [],
        )
        .unwrap();
        conn.pragma_update(None, "user_version", 2).unwrap();
    }

    // Open with v1.9.15 code
    let storage = Storage::open(&database_path).unwrap();

    // get_gate_receipt accepts the v1.9.14 id (no -{seq} suffix)
    let receipt = storage
        .get_gate_receipt("gate-exploration-sufficient-abcdef12")
        .unwrap();
    assert_eq!(receipt.receipt_id, "gate-exploration-sufficient-abcdef12");
    assert_eq!(receipt.seq, 1);
}

#[test]
fn storage_build_gate_receipt_id_rejects_plan_hash_too_short() {
    // plan_hash shorter than 23 chars must fail with PlanHashTooShort;
    // the guard at `if plan_hash.len() < REQUIRED_LEN` exists precisely
    // to prevent the `&plan_hash[7..23]` slice from panicking on inputs
    // that cannot produce a valid receipt_id suffix.
    const SHORT_PLAN_HASH: &str = "sha256:abc"; // 10 chars, < 23
    assert_eq!(SHORT_PLAN_HASH.len(), 10);
    let err =
        Storage::build_gate_receipt_id("exploration-sufficient", SHORT_PLAN_HASH, 1).unwrap_err();
    assert!(matches!(
        err,
        StorageError::PlanHashTooShort {
            actual: 10,
            required: 23,
        }
    ));
}

#[test]
fn storage_build_gate_receipt_id_accepts_exact_border_plan_hash_length() {
    // plan_hash exactly 23 chars (sha256: prefix + 16 hex digits) is the
    // minimum slice length: [7..23] consumes exactly the 16 hex digits
    // and must produce a receipt_id that matches RID_FORMAT_REGEX.
    let plan_hash = "sha256:abcdef1234567890"; // 23 chars
    assert_eq!(plan_hash.len(), 23);
    let receipt_id =
        Storage::build_gate_receipt_id("exploration-sufficient", plan_hash, 1).unwrap();
    assert_eq!(receipt_id, "gate-exploration-sufficient-abcdef1234567890-1");
    let rid_regex = regex::Regex::new(RID_FORMAT_REGEX).unwrap();
    assert!(
        rid_regex.is_match(&receipt_id),
        "receipt_id '{receipt_id}' must match RID_FORMAT_REGEX"
    );
}

#[test]
fn storage_insert_gate_receipt_next_seq_rejects_short_plan_hash() {
    // Full API path: even after the per-(gate, plan_hash) seq allocation
    // inside the IMMEDIATE transaction, a short plan_hash must fail with
    // PlanHashTooShort and roll back the transaction (no row is left
    // behind in gate_receipts).
    const SHORT_PLAN_HASH: &str = "sha256:abc"; // 10 chars, too short
    assert_eq!(SHORT_PLAN_HASH.len(), 10);
    let mut storage = Storage::open_in_memory().unwrap();
    storage.insert_project(&project_record()).unwrap();
    storage.insert_workspace(&workspace_record()).unwrap();
    let cycle = cycle_record();
    let cycle_id = cycle.manifest.cycle_id.as_str().to_string();
    storage.insert_cycle(&cycle).unwrap();

    let err = storage
        .insert_gate_receipt_next_seq(&GateReceiptNextSeqInput {
            project_id: "project-1".into(),
            cycle_id: Some(cycle_id.clone()),
            gate: "exploration-sufficient".into(),
            evaluator: "sddk.cli".into(),
            transition_id: "phase.explore.complete".into(),
            plan_hash: SHORT_PLAN_HASH.into(),
            outcome: GateOutcomeStatus::Passed,
            evidence: json!({"verified": true}),
            actor: "test-runtime".into(),
            actor_ref: None,
            command_id: "cmd-1".into(),
            frame_id: "frame-1".into(),
            evaluated_at: CREATED_AT.into(),
            causation_id: None,
            correlation_id: None,
        })
        .unwrap_err();
    assert!(matches!(
        err,
        StorageError::PlanHashTooShort {
            actual: 10,
            required: 23,
        }
    ));

    // Transaction rolled back: no gate_receipt rows for this cycle.
    assert!(storage.list_gate_receipts(&cycle_id).unwrap().is_empty());
}

// ---------------------------------------------------------------------------
// Gate-name length guards (1..=128)
// ---------------------------------------------------------------------------

#[test]
fn storage_build_gate_receipt_id_accepts_128_char_gate_border() {
    // Gate exactly 128 chars is the upper border of the allowed range and must
    // produce a receipt_id that matches RID_FORMAT_REGEX.
    let gate = "a".repeat(128);
    assert_eq!(gate.len(), 128);
    let plan_hash = "sha256:abcdef1234567890"; // 23 chars, minimum valid
    let receipt_id = Storage::build_gate_receipt_id(&gate, plan_hash, 1).unwrap();
    assert_eq!(receipt_id, format!("gate-{gate}-abcdef1234567890-1"));
    let rid_regex = regex::Regex::new(RID_FORMAT_REGEX).unwrap();
    assert!(
        rid_regex.is_match(&receipt_id),
        "receipt_id '{receipt_id}' must match RID_FORMAT_REGEX"
    );
}

#[test]
fn storage_build_gate_receipt_id_rejects_129_char_gate() {
    // Gate exceeding 128 chars must fail with GateNameInvalid.
    let gate = "a".repeat(129);
    assert_eq!(gate.len(), 129);
    let plan_hash = "sha256:abcdef1234567890";
    let err = Storage::build_gate_receipt_id(&gate, plan_hash, 1).unwrap_err();
    assert!(matches!(
        err,
        StorageError::GateNameInvalid {
            actual: 129,
            min: 1,
            max: 128,
        }
    ));
}

#[test]
fn storage_build_gate_receipt_id_rejects_empty_gate() {
    // Empty gate (0 chars) must fail with GateNameInvalid(actual=0).
    let plan_hash = "sha256:abcdef1234567890";
    let err = Storage::build_gate_receipt_id("", plan_hash, 1).unwrap_err();
    assert!(matches!(
        err,
        StorageError::GateNameInvalid {
            actual: 0,
            min: 1,
            max: 128,
        }
    ));
}

#[test]
fn storage_insert_next_seq_rejects_long_gate_without_side_effects() {
    // insert_gate_receipt_next_seq must reject a 129-char gate with
    // GateNameInvalid and leave zero rows in gate_receipts (transaction
    // rolled back).
    let gate = "a".repeat(129);
    assert_eq!(gate.len(), 129);
    let mut storage = Storage::open_in_memory().unwrap();
    storage.insert_project(&project_record()).unwrap();
    storage.insert_workspace(&workspace_record()).unwrap();
    let cycle = cycle_record();
    let cycle_id = cycle.manifest.cycle_id.as_str().to_string();
    storage.insert_cycle(&cycle).unwrap();

    let err = storage
        .insert_gate_receipt_next_seq(&GateReceiptNextSeqInput {
            project_id: "project-1".into(),
            cycle_id: Some(cycle_id.clone()),
            gate,
            evaluator: "sddk.cli".into(),
            transition_id: "phase.explore.complete".into(),
            plan_hash: "sha256:abcdef1234567890".into(),
            outcome: GateOutcomeStatus::Passed,
            evidence: json!({"verified": true}),
            actor: "test-runtime".into(),
            actor_ref: None,
            command_id: "cmd-1".into(),
            frame_id: "frame-1".into(),
            evaluated_at: CREATED_AT.into(),
            causation_id: None,
            correlation_id: None,
        })
        .unwrap_err();
    assert!(matches!(
        err,
        StorageError::GateNameInvalid {
            actual: 129,
            min: 1,
            max: 128,
        }
    ));

    // Transaction rolled back: no gate_receipt rows for this cycle.
    assert!(storage.list_gate_receipts(&cycle_id).unwrap().is_empty());
}

#[test]
fn storage_insert_gate_receipt_waived_round_trip() {
    // insert_gate_receipt_next_seq with Waived outcome persists correctly and
    // round-trips through list_gate_receipts with the Waived value preserved.
    let mut storage = Storage::open_in_memory().unwrap();
    storage.insert_project(&project_record()).unwrap();
    storage.insert_workspace(&workspace_record()).unwrap();
    let cycle = cycle_record();
    let cycle_id = cycle.manifest.cycle_id.as_str().to_string();
    storage.insert_cycle(&cycle).unwrap();

    storage
        .insert_gate_receipt_next_seq(&GateReceiptNextSeqInput {
            project_id: "project-1".into(),
            cycle_id: Some(cycle_id.clone()),
            gate: "tests-pass".into(),
            evaluator: "sddk.cli".into(),
            transition_id: "phase.explore.complete".into(),
            plan_hash: "sha256:abcdef1234567890".into(),
            outcome: GateOutcomeStatus::Waived,
            evidence: json!({"reason": "gate-not-applicable"}),
            actor: "test-runtime".into(),
            actor_ref: None,
            command_id: "cmd-1".into(),
            frame_id: "frame-1".into(),
            evaluated_at: CREATED_AT.into(),
            causation_id: None,
            correlation_id: None,
        })
        .unwrap();

    let receipts = storage.list_gate_receipts(&cycle_id).unwrap();
    assert_eq!(receipts.len(), 1);
    assert_eq!(receipts[0].outcome, GateOutcomeStatus::Waived);
}

#[test]
fn storage_insert_legacy_rejects_long_gate() {
    // insert_gate_receipt (legacy, caller-supplied seq) must also validate
    // the gate name and reject a 129-char gate with GateNameInvalid.
    let gate = "a".repeat(129);
    assert_eq!(gate.len(), 129);
    let mut storage = Storage::open_in_memory().unwrap();
    storage.insert_project(&project_record()).unwrap();
    storage.insert_workspace(&workspace_record()).unwrap();
    let cycle = cycle_record();
    let cycle_id = cycle.manifest.cycle_id.as_str().to_string();
    storage.insert_cycle(&cycle).unwrap();

    let err = storage
        .insert_gate_receipt(&GateReceiptInput {
            receipt_id: "gate-legacy-test-abcdef1234567890-1".into(),
            project_id: "project-1".into(),
            cycle_id: Some(cycle_id.clone()),
            gate,
            evaluator: "sddk.cli".into(),
            transition_id: "phase.explore.complete".into(),
            plan_hash: "sha256:abcdef1234567890".into(),
            outcome: GateOutcomeStatus::Passed,
            evidence: json!({"verified": true}),
            actor: "test-runtime".into(),
            actor_ref: None,
            command_id: "cmd-1".into(),
            frame_id: "frame-1".into(),
            evaluated_at: CREATED_AT.into(),
            seq: 1,
            causation_id: None,
            correlation_id: None,
        })
        .unwrap_err();
    assert!(matches!(
        err,
        StorageError::GateNameInvalid {
            actual: 129,
            min: 1,
            max: 128,
        }
    ));

    // Transaction rolled back: no gate_receipt rows for this cycle.
    assert!(storage.list_gate_receipts(&cycle_id).unwrap().is_empty());
}

#[test]
fn finalize_capability_receipt_with_hashes_writes_version_columns() {
    let mut storage = Storage::open_in_memory().unwrap();
    storage.insert_project(&project_record()).unwrap();

    // Begin a capability receipt
    let input = CapabilityReceiptInput {
        receipt_id: "receipt-hash-test".into(),
        project_id: "project-1".into(),
        cycle_id: None,
        capability: "git.create_branch".into(),
        idempotency_key: "create-feature-branch".into(),
        request: json!({"branch": "feature"}),
        status: CapabilityStatus::Started,
        result: None,
        started_at: CREATED_AT.into(),
        completed_at: None,
        agent_version_hash: None,
        behavior_version_hash: None,
    };
    let began = storage.begin_capability_receipt(&input).unwrap();
    assert_eq!(began.status, CapabilityStatus::Started);

    // Finalize with version hashes via the new code path
    let finalized = storage
        .finalize_capability_receipt_with_hashes(
            "receipt-hash-test",
            CapabilityStatus::Succeeded,
            Some(json!({"merged": true})),
            "2026-08-04T10:00:01Z",
            Some("agent-sha-abc123".into()),
            Some("behavior-sha-def456".into()),
        )
        .unwrap();

    assert_eq!(finalized.status, CapabilityStatus::Succeeded);
    assert_eq!(
        finalized.agent_version_hash.as_deref(),
        Some("agent-sha-abc123")
    );
    assert_eq!(
        finalized.behavior_version_hash.as_deref(),
        Some("behavior-sha-def456")
    );

    // Round-trip through get_capability_receipt also preserves the values
    let reloaded = storage.get_capability_receipt("receipt-hash-test").unwrap();
    assert_eq!(
        reloaded.agent_version_hash.as_deref(),
        Some("agent-sha-abc123")
    );
    assert_eq!(
        reloaded.behavior_version_hash.as_deref(),
        Some("behavior-sha-def456")
    );
}

#[test]
fn legacy_receipt_without_version_columns_returns_none() {
    let directory = tempfile::tempdir().unwrap();
    let database_path = directory.path().join("legacy_receipts.sqlite");

    // Create a pre-MIGRATION_7 database with capability_receipts table
    // that does NOT have the agent_version_hash / behavior_version_hash columns
    {
        let conn = Connection::open(&database_path).unwrap();
        conn.execute_batch(
            r#"
            CREATE TABLE projects (
                project_id TEXT PRIMARY KEY,
                display_name TEXT NOT NULL,
                remote_url TEXT,
                scope TEXT NOT NULL,
                created_at TEXT NOT NULL
            );
            CREATE TABLE workspaces (
                workspace_id TEXT PRIMARY KEY,
                project_id TEXT NOT NULL REFERENCES projects(project_id),
                canonical_path TEXT NOT NULL,
                created_at TEXT NOT NULL
            );
            CREATE TABLE cycles (
                cycle_id TEXT PRIMARY KEY,
                project_id TEXT NOT NULL,
                workspace_id TEXT NOT NULL,
                status TEXT NOT NULL,
                phase TEXT NOT NULL,
                manifest_json TEXT NOT NULL,
                created_at TEXT NOT NULL,
                updated_at TEXT NOT NULL
            );
            CREATE TABLE capability_receipts (
                receipt_id TEXT PRIMARY KEY,
                project_id TEXT NOT NULL,
                cycle_id TEXT,
                capability TEXT NOT NULL,
                request_hash TEXT NOT NULL,
                request_json TEXT NOT NULL,
                status TEXT NOT NULL,
                result_json TEXT,
                started_at TEXT NOT NULL,
                completed_at TEXT
            );
            CREATE TABLE idempotency_records (
                project_id TEXT NOT NULL,
                idempotency_key TEXT NOT NULL,
                request_hash TEXT NOT NULL,
                receipt_id TEXT NOT NULL,
                created_at TEXT NOT NULL,
                PRIMARY KEY (project_id, idempotency_key)
            );
            "#,
        )
        .unwrap();
        conn.execute(
            "INSERT INTO projects VALUES ('project-1', 'Project One', 'https://example.com/owner/project', 'owner', '2026-08-03T12:00:00Z')",
            [],
        )
        .unwrap();
        conn.execute(
            "INSERT INTO workspaces VALUES ('workspace-1', 'project-1', '/work/project', '2026-08-03T12:00:00Z')",
            [],
        )
        .unwrap();
        conn.execute(
            "INSERT INTO capability_receipts (receipt_id, project_id, cycle_id, capability, request_hash, request_json, status, result_json, started_at, completed_at) VALUES ('legacy-receipt-1', 'project-1', NULL, 'git.create_branch', 'hash123', '{}', 'succeeded', '{\"merged\":true}', '2026-08-03T12:00:00Z', '2026-08-03T12:01:00Z')",
            [],
        )
        .unwrap();
        // Schema version 6 (pre-MIGRATION_7)
        conn.pragma_update(None, "user_version", 6).unwrap();
    }

    // Open with current code — MIGRATION_7..MIGRATION_13 all run
    let storage = Storage::open(&database_path).unwrap();
    assert_eq!(storage.schema_version().unwrap(), 14);

    // Read back the legacy receipt — new columns must be None
    let receipt = storage.get_capability_receipt("legacy-receipt-1").unwrap();
    assert_eq!(receipt.receipt_id, "legacy-receipt-1");
    assert_eq!(receipt.agent_version_hash, None);
    assert_eq!(receipt.behavior_version_hash, None);
    assert_eq!(receipt.status, CapabilityStatus::Succeeded);
}

/// Verifies the `ArtifactStore` port implementation for `Storage`.
#[test]
fn artifact_store_port_impl() {
    let directory = tempdir().unwrap();
    let db_path = directory.path().join("ledger.sqlite");
    let mut storage = Storage::open(&db_path).unwrap();

    // Insert project, workspace, and cycle (all required for artifact FK)
    storage.insert_project(&project_record()).unwrap();
    storage.insert_workspace(&workspace_record()).unwrap();
    let cycle = cycle_record();
    storage.insert_cycle(&cycle).unwrap();

    let artifact = ArtifactRecord {
        artifact_id: "art-test-001".into(),
        project_id: "project-1".into(),
        cycle_id: Some(cycle.manifest.cycle_id.clone()),
        kind: "spec".into(),
        path: "sha256/abc123/spec.md".into(),
        sha256: Some("sha256:abc123".into()),
        producer: Some("test".into()),
        created_at: "2026-08-19T12:00:00Z".into(),
        metadata: json!({}),
    };

    // insert_artifact via port
    ArtifactStore::insert_artifact(&mut storage, &artifact).unwrap();

    // get_artifact via port — returns Option
    let found = ArtifactStore::get_artifact(&storage, "art-test-001").unwrap();
    assert_eq!(
        found.as_ref().map(|a| a.artifact_id.as_str()),
        Some("art-test-001")
    );

    // get_artifact for unknown id returns None
    let missing = ArtifactStore::get_artifact(&storage, "does-not-exist").unwrap();
    assert_eq!(missing, None);

    // list_project_artifacts via port
    let list = ArtifactStore::list_project_artifacts(&storage, "project-1").unwrap();
    assert_eq!(list.len(), 1);
    assert_eq!(list[0].artifact_id, "art-test-001");
}
