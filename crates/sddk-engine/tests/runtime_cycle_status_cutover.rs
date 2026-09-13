//! WU-C3 cutover tests (DELTA-CONF-004) — cycle/run lifecycle separation.
//!
//! Covers:
//! 1. `runtime_cycle_status_writes_are_rejected` — storage and workflow
//!    manifest layers reject runtime-derived statuses as canonical truth.
//! 2. `cycle_summary_derived_from_run_facts` — the derived runtime summary
//!    reconstructs approval-wait / UAT-wait / remediation state from ledger
//!    facts (scenario 5).
//! 3. `legacy_cycle_view_rendering_parity` — decode of a pre-cutover cycle
//!    keeps all 11 variants and the derived summary coexists with the
//!    persisted delivery status (scenario 7).

use std::collections::HashMap;

use sddk_domain::{CycleManifest, CyclePath, CycleStatus, Phase};
use sddk_engine::{
    WorkflowLoadError, WorkflowValidationError,
    cycle_summary::{CycleRuntimeSummary, derive_cycle_summary},
    load_workflow_str,
};
use sddk_storage::{CycleRecord, LedgerEventInput, ProjectRecord, Storage, WorkspaceRecord};
use serde_json::json;

const WORKFLOW_YAML: &str = include_str!("../../../workflow/workflow.yaml");
const TIMESTAMP: &str = "2026-08-03T12:00:00Z";

// ── helpers ──────────────────────────────────────────────────────────────────

fn manifest_with_status(status: CycleStatus) -> CycleManifest {
    CycleManifest {
        schema_version: 1,
        project_id: "project-1".into(),
        workspace_id: "workspace-1".into(),
        cycle_id: "cycle-1".into(),
        display_name: "Cutover work".into(),
        status,
        phase: Phase::Build,
        path: CyclePath::ALite,
        branch: "feat/cutover".into(),
        base: "abc123".into(),
        head: None,
        artifacts: HashMap::new(),
        release: None,
        delivery_kind: None,
        remediation_round: 0,
        remote_url: Some("https://example.com/owner/project".into()),
        scope: Some("owner".into()),
        pause_at: None,
        review_at: None,
        last_pause_reason: None,
        replan_count: 0,
    }
}

fn storage_with_parents() -> Storage {
    let storage = Storage::open_in_memory().unwrap();
    storage
        .insert_project(&ProjectRecord {
            project_id: "project-1".into(),
            display_name: "Project One".into(),
            remote_url: Some("https://example.com/owner/project".into()),
            scope: "owner".into(),
            created_at: TIMESTAMP.into(),
        })
        .unwrap();
    storage
        .insert_workspace(&WorkspaceRecord {
            workspace_id: "workspace-1".into(),
            project_id: "project-1".into(),
            canonical_path: "/work/project".into(),
            created_at: TIMESTAMP.into(),
        })
        .unwrap();
    storage
}

fn open_cycle(storage: &Storage, status: CycleStatus) {
    storage
        .insert_cycle(&CycleRecord {
            manifest: manifest_with_status(status),
            created_at: TIMESTAMP.into(),
            updated_at: TIMESTAMP.into(),
        })
        .unwrap();
}

fn transition_event(storage: &mut Storage, event_id: &str, transition_id: &str, outcome: &str) {
    let record = storage.get_cycle("cycle-1").unwrap();
    storage
        .emit_canonical_event(&LedgerEventInput {
            event_id: event_id.into(),
            project_id: record.manifest.project_id.clone(),
            cycle_id: Some("cycle-1".into()),
            frame_id: format!("frame:{event_id}"),
            command_id: format!("command-{event_id}"),
            actor: "test-runtime".into(),
            actor_ref: None,
            event_type: "cycle.transitioned".into(),
            occurred_at: TIMESTAMP.into(),
            state_before: None,
            state_after: None,
            payload: json!({
                "transition_id": transition_id,
                "outcome": outcome,
                "failed_gates": [],
            }),
            causation_id: None,
            correlation_id: None,
        })
        .unwrap();
}

fn approval_event(storage: &mut Storage, event_id: &str, event_type: &str, request_hash: &str) {
    let record = storage.get_cycle("cycle-1").unwrap();
    storage
        .emit_canonical_event(&LedgerEventInput {
            event_id: event_id.into(),
            project_id: record.manifest.project_id.clone(),
            cycle_id: Some("cycle-1".into()),
            frame_id: format!("frame:{event_id}"),
            command_id: format!("command-{event_id}"),
            actor: "test-agent".into(),
            actor_ref: None,
            event_type: event_type.into(),
            occurred_at: TIMESTAMP.into(),
            state_before: None,
            state_after: None,
            payload: json!({
                "capability": "release.publish",
                "cycle_id": "cycle-1",
                "request_hash": request_hash,
            }),
            causation_id: None,
            correlation_id: None,
        })
        .unwrap();
}

// ── 1. runtime status writes are rejected ────────────────────────────────────

#[test]
fn runtime_cycle_status_writes_are_rejected() {
    let storage = storage_with_parents();

    for status in [
        CycleStatus::Remediating,
        CycleStatus::Recovering,
        CycleStatus::UatWaiting,
        CycleStatus::ApprovalPending,
    ] {
        let error = storage
            .insert_cycle(&CycleRecord {
                manifest: manifest_with_status(status),
                created_at: TIMESTAMP.into(),
                updated_at: TIMESTAMP.into(),
            })
            .expect_err("runtime-derived status insert must be rejected");
        assert!(
            error.to_string().contains("decode-only"),
            "unexpected error for {status:?}: {error}"
        );
    }
}

#[test]
fn workflow_manifest_rejects_runtime_status_targets() {
    // UAT_WAITING as a `to` target (legacy manifest shape, pre-cutover).
    let yaml = WORKFLOW_YAML.replacen(
        "  - id: phase.verify.uat.sync
    from:
      status: OPEN
      phase: verify
    to:
      status: OPEN
      phase: uat",
        "  - id: phase.verify.uat.sync
    from:
      status: OPEN
      phase: verify
    to:
      status: UAT_WAITING
      phase: uat",
        1,
    );
    assert_ne!(yaml, WORKFLOW_YAML, "replacement must apply");
    let error = load_workflow_str(&yaml).expect_err("UAT_WAITING target must be rejected");
    match error {
        WorkflowLoadError::Validation(WorkflowValidationError::RuntimeStatusInManifest {
            transition_id,
            field,
            status,
        }) => {
            assert_eq!(transition_id, "phase.verify.uat.sync");
            assert_eq!(field, "to");
            assert_eq!(status, CycleStatus::UatWaiting);
        }
        other => panic!("unexpected error: {other:?}"),
    }
}

#[test]
fn runtime_status_partition_covers_exactly_four_variants() {
    assert!(CycleStatus::Remediating.is_runtime_derived());
    assert!(CycleStatus::Recovering.is_runtime_derived());
    assert!(CycleStatus::UatWaiting.is_runtime_derived());
    assert!(CycleStatus::ApprovalPending.is_runtime_derived());
    assert!(!CycleStatus::Open.is_runtime_derived());
    assert!(!CycleStatus::Blocked.is_runtime_derived());
    assert!(!CycleStatus::ReleasePending.is_runtime_derived());
    assert!(!CycleStatus::Released.is_runtime_derived());
    assert!(!CycleStatus::Closed.is_runtime_derived());
    assert!(!CycleStatus::Abandoned.is_runtime_derived());
    assert!(!CycleStatus::Paused.is_runtime_derived());
}

// ── 2. cycle summary derived from Run facts (scenario 5) ─────────────────────

#[test]
fn cycle_summary_derived_from_run_facts() {
    let mut storage = storage_with_parents();
    open_cycle(&storage, CycleStatus::Open);
    let summary = |storage: &Storage| derive_cycle_summary(storage, "cycle-1").unwrap();

    // No facts: no derived state.
    let clean = summary(&storage);
    assert_eq!(clean.derived_state, "");
    assert_eq!(clean.persisted_status, CycleStatus::Open);
    assert!(!clean.approval_waiting);
    assert!(!clean.uat_waiting);
    assert!(!clean.remediating);
    assert_eq!(clean.remediation_rounds, 0);

    // A failed verify transition opens a remediation round (fact on ledger;
    // cycle stays OPEN).
    transition_event(
        &mut storage,
        "evt-fail-1",
        "phase.verify.complete",
        "failed",
    );
    let remediation = summary(&storage);
    assert_eq!(remediation.derived_state, "remediating");
    assert!(remediation.remediating);
    assert_eq!(remediation.remediation_rounds, 1);
    assert_eq!(remediation.persisted_status, CycleStatus::Open);

    // Remediation completion closes the round.
    transition_event(
        &mut storage,
        "evt-fix-1",
        "phase.verify.remediate",
        "succeeded",
    );
    let closed = summary(&storage);
    assert!(!closed.remediating);
    assert_eq!(closed.derived_state, "");
    assert_eq!(closed.remediation_rounds, 1);

    // UAT wait: started by a succeeded uat.sync, closed by uat.complete.
    transition_event(
        &mut storage,
        "evt-uat-sync",
        "phase.verify.uat.sync",
        "succeeded",
    );
    let uat = summary(&storage);
    assert_eq!(uat.derived_state, "uat-waiting");
    assert!(uat.uat_waiting);
    transition_event(
        &mut storage,
        "evt-uat-complete",
        "phase.uat.complete",
        "succeeded",
    );
    let uat_done = summary(&storage);
    assert!(!uat_done.uat_waiting);

    // Approval wait: requested without a decision; decision closes it.
    approval_event(
        &mut storage,
        "evt-appr-req",
        "approval.capability.requested",
        "h1",
    );
    let waiting = summary(&storage);
    assert_eq!(waiting.derived_state, "approval-waiting");
    assert!(waiting.approval_waiting);
    assert_eq!(waiting.approval_waiting_on, ["release.publish"]);
    approval_event(
        &mut storage,
        "evt-appr-ok",
        "approval.capability.granted",
        "h1",
    );
    let granted = summary(&storage);
    assert!(!granted.approval_waiting);
    assert_eq!(granted.derived_state, "");
}

#[test]
fn approval_precedence_beats_uat_and_remediation() {
    let mut storage = storage_with_parents();
    open_cycle(&storage, CycleStatus::Open);
    transition_event(&mut storage, "evt-fail", "phase.verify.complete", "failed");
    transition_event(
        &mut storage,
        "evt-uat-sync",
        "phase.verify.uat.sync",
        "succeeded",
    );
    approval_event(
        &mut storage,
        "evt-appr",
        "approval.capability.requested",
        "h1",
    );
    let summary = derive_cycle_summary(&storage, "cycle-1").unwrap();
    assert_eq!(summary.derived_state, "approval-waiting");
}

// ── 3. legacy decode parity (scenario 7) ─────────────────────────────────────

#[test]
fn legacy_cycle_view_rendering_parity() {
    let mut storage = storage_with_parents();
    // A pre-cutover row persisted REMEDIATING; decode must keep working.
    // insert_cycle now refuses, so simulate the legacy row by opening with a
    // delivery status and rewriting through the decode path (raw SQL).
    open_cycle(&storage, CycleStatus::Open);
    storage
        .connection_for_tests()
        .execute(
            "UPDATE cycles SET status = 'REMEDIATING', manifest_json = json_set(manifest_json, '$.status', 'REMEDIATING') WHERE cycle_id = 'cycle-1'",
            [],
        )
        .unwrap();

    let record = storage.get_cycle("cycle-1").unwrap();
    assert_eq!(record.manifest.status, CycleStatus::Remediating);

    // The derived summary coexists with the legacy persisted status.
    transition_event(&mut storage, "evt-fail", "phase.verify.complete", "failed");
    let summary: CycleRuntimeSummary = derive_cycle_summary(&storage, "cycle-1").unwrap();
    assert_eq!(summary.persisted_status, CycleStatus::Remediating);
    assert!(summary.remediating);
    assert_eq!(summary.derived_state, "remediating");
}
