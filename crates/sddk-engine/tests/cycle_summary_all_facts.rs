//! `derive_all_cycle_facts`: the enumeration must answer for the cycles it was
//! ASKED about, with the same rule the single-cycle path uses.
//!
//! `derive_all_cycle_facts` exists because `derive_cycle_summary` is
//! "materialise the whole event log, then filter by cycle". Calling it once
//! per row of an enumeration re-reads the entire log once per row: measured on
//! `p-63676b11dc0ef88f`, 109 reads of a 651-event log to fill 188 rows, which
//! took `cycle list` from 0.05 s to 2.38 s in a release build.
//!
//! The refactor that removed those reads could have broken two things, and both
//! are pinned here because neither is visible in the happy path:
//!
//! 1. **Silence became absence.** A cheap implementation returns an entry only
//!    for cycles it saw events for. The consumer then cannot tell "this cycle
//!    has nothing pending" from "I could not derive this cycle" — the two
//!    silences, one of which is a lie. The contract is one entry per REQUESTED
//!    id, and the entry for a cycle with no events says so in its own field.
//! 2. **Two rules instead of one.** The single-cycle path and the enumeration
//!    path are the same rule stated twice the moment they are written twice,
//!    and then the operator view and `cycle status` disagree. Which is what
//!    INC-DEBT-067 was about, one layer up.

use std::collections::HashMap;

use sddk_domain::{CycleManifest, CyclePath, CycleStatus, Phase};
use sddk_engine::cycle_summary::{derive_all_cycle_facts, derive_cycle_summary};
use sddk_storage::{CycleRecord, LedgerEventInput, ProjectRecord, Storage, WorkspaceRecord};

const TIMESTAMP: &str = "2026-08-03T12:00:00Z";

fn manifest_for(cycle_id: &str) -> CycleManifest {
    CycleManifest {
        schema_version: 1,
        project_id: "project-1".into(),
        workspace_id: "workspace-1".into(),
        cycle_id: cycle_id.into(),
        display_name: "Enumeration work".into(),
        status: CycleStatus::Open,
        phase: Phase::Build,
        path: CyclePath::ALite,
        branch: "feat/enumeration".into(),
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

/// Inserts a cycle ROW without emitting an event, which is how a cycle that has
/// no ledger facts at all is built. Not a contrivance: on the real ledger those
/// rows are the majority, and they are exactly the ones a consumer must be able
/// to distinguish from a row it failed to read.
fn open_quiet_cycle(storage: &Storage, cycle_id: &str) {
    storage
        .insert_cycle(&CycleRecord {
            manifest: manifest_for(cycle_id),
            created_at: TIMESTAMP.into(),
            updated_at: TIMESTAMP.into(),
        })
        .unwrap();
}

fn approval_event(storage: &mut Storage, cycle_id: &str, event_type: &str, request_hash: &str) {
    storage
        .emit_canonical_event(&LedgerEventInput {
            event_id: format!("approval-{event_type}-{request_hash}"),
            project_id: "project-1".into(),
            cycle_id: Some(cycle_id.into()),
            frame_id: format!("frame:{cycle_id}:{request_hash}"),
            command_id: format!("command-{request_hash}"),
            actor: "test-agent".into(),
            actor_ref: None,
            event_type: event_type.into(),
            occurred_at: TIMESTAMP.into(),
            state_before: None,
            state_after: None,
            payload: serde_json::json!({
                "capability": "release.publish",
                "cycle_id": cycle_id,
                "request_hash": request_hash,
            }),
            causation_id: None,
            correlation_id: None,
        })
        .unwrap();
}

fn transition_event(storage: &mut Storage, cycle_id: &str, transition_id: &str, outcome: &str) {
    storage
        .emit_canonical_event(&LedgerEventInput {
            event_id: format!("transition-{cycle_id}-{transition_id}-{outcome}"),
            project_id: "project-1".into(),
            cycle_id: Some(cycle_id.into()),
            frame_id: format!("frame:{cycle_id}:{transition_id}"),
            command_id: format!("command-{cycle_id}-{transition_id}"),
            actor: "test-runtime".into(),
            actor_ref: None,
            event_type: "cycle.transitioned".into(),
            occurred_at: TIMESTAMP.into(),
            state_before: None,
            state_after: None,
            payload: serde_json::json!({
                "transition_id": transition_id,
                "outcome": outcome,
                "failed_gates": [],
            }),
            causation_id: None,
            correlation_id: None,
        })
        .unwrap();
}

/// One entry per requested id — INCLUDING the cycles that have no events.
///
/// The quiet row is the one this test exists for. If the producer only answers
/// for cycles it saw events for, the quiet row goes missing, and the consumer
/// that treats a missing entry as "not derivable" renders a cycle that needs
/// nothing as a cycle whose state is unknown. The output would be identical to
/// the lie INC-DEBT-067 was about, produced by an optimisation instead of an
/// oversight.
#[test]
fn every_requested_cycle_gets_an_entry_including_those_with_no_events() {
    let mut storage = storage_with_parents();
    open_quiet_cycle(&storage, "cycle-busy");
    open_quiet_cycle(&storage, "cycle-quiet");
    approval_event(
        &mut storage,
        "cycle-busy",
        "approval.capability.requested",
        "sha256:req",
    );

    let all = derive_all_cycle_facts(
        &storage,
        ["cycle-busy", "cycle-quiet", "cycle-never-opened"],
    )
    .unwrap();

    assert_eq!(
        all.len(),
        3,
        "one entry per requested id, no more and no fewer: {all:?}"
    );

    let busy = &all["cycle-busy"];
    assert!(busy.approval_waiting);
    assert_eq!(busy.derived_state, "approval-waiting");

    let quiet = &all["cycle-quiet"];
    assert!(
        !quiet.approval_waiting && quiet.derived_state.is_empty(),
        "a cycle with no events is QUIET, not undetermined, and the entry says \
         which: {quiet:?}"
    );

    // Asked about a cycle that has no ROW either. The entry exists and is
    // empty. This function derives from the event log and makes NO claim about
    // whether a cycle exists — that is the enumeration's row source, not this
    // one, and conflating them is how "no events" turns into "no cycle".
    let phantom = &all["cycle-never-opened"];
    assert!(
        phantom.derived_state.is_empty() && !phantom.approval_waiting,
        "the derivation is about events, and says nothing about existence: {phantom:?}"
    );
}

/// The enumeration and the single-cycle path cannot disagree.
///
/// Same ledger, same rule, two entry points. If this fails, `cycle list` and
/// `cycle status` are computing different truths from the same facts — the
/// divergence INC-DEBT-067 existed to stop, reappearing one level up because
/// the refactor made a second call site.
#[test]
fn the_enumeration_and_the_single_cycle_path_cannot_disagree() {
    let mut storage = storage_with_parents();
    open_quiet_cycle(&storage, "cycle-1");
    approval_event(
        &mut storage,
        "cycle-1",
        "approval.capability.requested",
        "sha256:req",
    );
    transition_event(
        &mut storage,
        "cycle-1",
        "phase.verify.uat.sync",
        "succeeded",
    );
    transition_event(&mut storage, "cycle-1", "phase.build.remediate", "failed");

    let one = derive_cycle_summary(&storage, "cycle-1").unwrap();
    let all = derive_all_cycle_facts(&storage, ["cycle-1"]).unwrap();
    let many = &all["cycle-1"];

    // Approval outranks everything else, so this ledger is simultaneously
    // approval-waiting and remediating: the assertion below is only meaningful
    // if the labels are not the only thing being compared.
    assert_eq!(many.approval_waiting, one.approval_waiting);
    assert_eq!(many.approval_waiting_on, one.approval_waiting_on);
    assert_eq!(many.uat_waiting, one.uat_waiting);
    assert_eq!(many.remediating, one.remediating);
    assert_eq!(many.remediation_rounds, one.remediation_rounds);
    assert_eq!(many.derived_state, one.derived_state);
    assert_eq!(many.derived_state, "approval-waiting");
    assert!(
        many.remediating,
        "and the losing label is still computed, not dropped"
    );
}
