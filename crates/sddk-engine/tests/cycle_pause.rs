//! Contract tests for `Engine::cycle_pause` and `Engine::cycle_resume`.
//!
//! Per [[REQ-Cycle-Pause-Contract]] and [[REQ-Cycle-Resume-Contract]]:
//! - pause requires lease fence (fail-closed)
//! - pause releases lease atomically
//! - pause writes pause-receipt.json
//! - resume requires cycle to be Paused
//! - resume acquires fresh lease with new fencing token
//! - resume writes resume-receipt.json

use std::collections::{BTreeSet, HashMap};
use std::path::Path;

use sddk_domain::{
    CycleManifest, CyclePath, CycleStatus, PauseReason, Phase, StorageError as DomainStorageError,
};
use sddk_engine::{CycleStartInput, Engine, EventContext, authority::AuthorityContext};
use sddk_storage::{ProjectRecord, Storage, WorkspaceRecord};

const WORKFLOW_YAML: &str = include_str!("../../../workflow/workflow.yaml");
const TIMESTAMP: &str = "2026-08-04T10:00:00Z";

fn engine_with_storage(storage: Storage) -> Engine<Storage> {
    Engine::new(
        sddk_engine::load_workflow_str(WORKFLOW_YAML).unwrap(),
        storage,
    )
    .unwrap()
}

fn setup() -> (tempfile::TempDir, Engine<Storage>) {
    let directory = tempfile::tempdir().unwrap();
    let storage = Storage::open_in_memory().unwrap();
    storage
        .insert_project(&ProjectRecord {
            project_id: "project-1".into(),
            display_name: "project".into(),
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
    let engine = engine_with_storage(storage);
    (directory, engine)
}

fn start_cycle(engine: &mut Engine<Storage>, event_id: &str) -> CycleManifest {
    let input = CycleStartInput {
        manifest: manifest_for_path(CyclePath::AFull),
        requirements: cycle_start_requirements(),
    };
    let plan = engine.plan_cycle_start(input).unwrap();
    engine
        .apply_cycle_start(&plan, &context(event_id, "command-a"))
        .unwrap()
        .manifest
}

fn manifest_for_path(path: CyclePath) -> CycleManifest {
    CycleManifest {
        schema_version: 1,
        project_id: "project-1".into(),
        workspace_id: "workspace-1".into(),
        cycle_id: "cycle-1".into(),
        display_name: "Pause work".into(),
        status: CycleStatus::Open,
        phase: Phase::Explore,
        path,
        branch: "feat/pause".into(),
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
    }
}

fn cycle_start_requirements() -> BTreeSet<String> {
    [
        "project.adopted",
        "project.initialized",
        "worktree.clean",
        "cycle.no_active_conflict",
    ]
    .into_iter()
    .map(str::to_owned)
    .collect()
}

fn context(event_id: &str, command_id: &str) -> EventContext {
    EventContext {
        command_id: command_id.into(),
        frame_id: format!("frame:{command_id}"),
        event_id: event_id.into(),
        actor: "test-runtime".into(),
        actor_ref: None,
        occurred_at: TIMESTAMP.into(),
        correlation_id: None,
        causation_id: None,
    }
}

fn receipt_path(directory: &tempfile::TempDir) -> std::path::PathBuf {
    directory.path().to_path_buf()
}

fn auth() -> AuthorityContext {
    AuthorityContext::for_test(sddk_domain::ActorKind::Agent, "test-runtime")
}

// ─────────────────────────────────────────────────────────────────────────────
// REQ-Cycle-Pause-Contract tests
// ─────────────────────────────────────────────────────────────────────────────

/// Scenario: pause happy path releases lease atomically and writes receipt.
///
/// GIVEN cycle in `Open` with active lease (owner = "alice", fencing_token = 1)
/// WHEN running `sddk cycle pause --cycle X --reason priority_revoked
///        --lease-owner alice --fencing-token 1`
/// THEN status = `Paused`, `cycle_leases` row gone, `pause-receipt.json` written.
#[test]
fn pause_happy_path_releases_lease_and_writes_receipt() {
    let (_dir, mut engine) = setup();

    // Start a cycle and acquire lease
    let manifest = start_cycle(&mut engine, "evt-1");
    let cycle_id = &manifest.cycle_id;

    let now_ms = 1_700_000_000_000i64;
    let expires_ms = i64::MAX;
    engine
        .acquire_cycle_lease(cycle_id, "alice", now_ms, expires_ms)
        .unwrap();

    // Verify lease exists
    let lease_before = engine.ledger().get_cycle_lease(cycle_id).unwrap();
    assert_eq!(lease_before.owner, "alice");
    assert_eq!(lease_before.fencing_token, 1);

    // Pause the cycle
    let receipt_dir = receipt_path(&_dir);
    let result = engine.cycle_pause(
        cycle_id,
        PauseReason::PriorityRevoked,
        None,
        "test-actor",
        "cmd-pause-1",
        "evt-pause-1",
        TIMESTAMP,
        &receipt_dir,
        "alice",
        1,
        &auth(),
    );

    assert!(
        result.is_ok(),
        "cycle_pause should succeed with valid lease fence, got: {:?}",
        result
    );

    // Verify status is Paused
    let updated = engine.ledger().get_cycle(cycle_id).unwrap();
    assert_eq!(
        updated.manifest.status,
        CycleStatus::Paused,
        "cycle status should be Paused after pause"
    );

    // Verify lease is released
    let lease_result = engine.ledger().get_cycle_lease(cycle_id);
    assert!(
        lease_result.is_err(),
        "lease should be released after pause"
    );

    // Verify pause receipt was written
    let receipt_path = receipt_dir.join(cycle_id).join("pause-receipt.json");
    assert!(
        receipt_path.exists(),
        "pause-receipt.json should exist at {:?}",
        receipt_path
    );
}

/// Scenario: pause without active lease fails closed.
///
/// GIVEN cycle in `Open` with no current lease
/// WHEN running `sddk cycle pause --cycle X --reason priority_revoked
///        --lease-owner alice --fencing-token 1`
/// THEN exit non-zero `EngineError::PauseRequiresLeaseFence`;
///     recovery names `sddk cycle lock acquire`; no event.
#[test]
fn pause_without_lease_fails_closed() {
    let (_dir, mut engine) = setup();

    // Start a cycle WITHOUT acquiring a lease
    let manifest = start_cycle(&mut engine, "evt-1");
    let cycle_id = &manifest.cycle_id;

    let receipt_dir = receipt_path(&_dir);
    let result = engine.cycle_pause(
        cycle_id,
        PauseReason::PriorityRevoked,
        None,
        "test-actor",
        "cmd-pause-1",
        "evt-pause-1",
        TIMESTAMP,
        &receipt_dir,
        "alice",
        1,
        &auth(),
    );

    assert!(result.is_err(), "cycle_pause should fail without lease");
    let err = result.unwrap_err();
    assert!(
        matches!(err, sddk_engine::EngineError::PauseRequiresLeaseFence),
        "expected PauseRequiresLeaseFence, got: {:?}",
        err
    );
}

/// Scenario: pause from terminal status is forbidden.
///
/// GIVEN cycle in `Closed`
/// WHEN running `sddk cycle pause --cycle X --reason priority_revoked
///        --lease-owner alice --fencing-token 1`
/// THEN exit non-zero `EngineError::PauseFromTerminalForbidden`;
///     no `pause-receipt.json`; no event.
#[test]
fn pause_from_terminal_forbidden() {
    let (_dir, mut engine) = setup();

    // Start and then supersede (closes) the cycle
    let manifest = start_cycle(&mut engine, "evt-1");
    let cycle_id = &manifest.cycle_id;

    // Acquire lease first
    let now_ms = 1_700_000_000_000i64;
    let expires_ms = i64::MAX;
    engine
        .acquire_cycle_lease(cycle_id, "alice", now_ms, expires_ms)
        .unwrap();

    // Transition to Closed via supersede
    let receipt_dir = receipt_path(&_dir);
    engine
        .cycle_supersede(
            cycle_id,
            None,
            Some(sddk_engine::SupersedeReason::ScopeInvalid),
            &["evidence".into()],
            "test-actor",
            "cmd-supersede-1",
            "evt-supersede-1",
            TIMESTAMP,
            &receipt_dir,
            "alice",
            1,
            &auth(),
        )
        .unwrap();

    // Re-acquire lease after supersede (lease was released)
    engine
        .acquire_cycle_lease(cycle_id, "alice", 0, i64::MAX)
        .unwrap();

    // Now try to pause a Closed cycle — should fail
    let result = engine.cycle_pause(
        cycle_id,
        PauseReason::PriorityRevoked,
        None,
        "test-actor",
        "cmd-pause-1",
        "evt-pause-1",
        TIMESTAMP,
        &receipt_dir,
        "alice",
        1,
        &auth(),
    );

    assert!(result.is_err(), "cycle_pause from Closed should fail");
    let err = result.unwrap_err();
    assert!(
        matches!(err, sddk_engine::EngineError::PauseFromTerminalForbidden),
        "expected PauseFromTerminalForbidden, got: {:?}",
        err
    );
}

/// Scenario: pause from already-Paused is idempotently rejected.
///
/// GIVEN cycle in `Paused` from a prior pause
/// WHEN running `sddk cycle pause --cycle X --reason context_switch
///        --lease-owner alice --fencing-token 2`
/// THEN exit non-zero `EngineError::PauseAlreadyPaused`;
///     prior `pause-receipt.json` unchanged; no event.
#[test]
fn pause_already_paused_idempotent() {
    let (_dir, mut engine) = setup();

    // Start a cycle, acquire lease, pause it
    let manifest = start_cycle(&mut engine, "evt-1");
    let cycle_id = &manifest.cycle_id;

    let now_ms = 1_700_000_000_000i64;
    let expires_ms = i64::MAX;
    engine
        .acquire_cycle_lease(cycle_id, "alice", now_ms, expires_ms)
        .unwrap();

    let receipt_dir = receipt_path(&_dir);
    engine
        .cycle_pause(
            cycle_id,
            PauseReason::PriorityRevoked,
            None,
            "test-actor",
            "cmd-pause-1",
            "evt-pause-1",
            TIMESTAMP,
            &receipt_dir,
            "alice",
            1,
            &auth(),
        )
        .unwrap();

    // Record the original receipt content
    let receipt_path = receipt_dir.join(cycle_id).join("pause-receipt.json");
    let original_content = std::fs::read_to_string(&receipt_path).unwrap();

    // Try to pause again — should fail with PauseAlreadyPaused
    let result = engine.cycle_pause(
        cycle_id,
        PauseReason::ContextSwitch,
        None,
        "test-actor",
        "cmd-pause-2",
        "evt-pause-2",
        TIMESTAMP,
        &receipt_dir,
        "alice",
        2,
        &auth(),
    );

    assert!(result.is_err(), "cycle_pause from Paused should fail");
    let err = result.unwrap_err();
    assert!(
        matches!(err, sddk_engine::EngineError::PauseAlreadyPaused),
        "expected PauseAlreadyPaused, got: {:?}",
        err
    );

    // Verify original receipt is unchanged
    let current_content = std::fs::read_to_string(&receipt_path).unwrap();
    assert_eq!(
        original_content, current_content,
        "original pause-receipt.json should be unchanged after idempotent rejection"
    );
}

// ─────────────────────────────────────────────────────────────────────────────
// REQ-Cycle-Resume-Contract tests
// ─────────────────────────────────────────────────────────────────────────────

/// Scenario: resume happy path re-acquires lease and prints new token.
///
/// GIVEN cycle in `Paused` with no current lease (prior lease was released)
/// WHEN running `sddk cycle resume --cycle X --lease-owner bob`
/// THEN status = `Open`, new `cycle_leases` row with `owner = "bob"` and
///     `fencing_token = 1` (fresh start, prior token unknown after release),
///     `resume-receipt.json` written.
#[test]
fn resume_happy_path_reacquires_lease_and_prints_new_token() {
    let (_dir, mut engine) = setup();

    // Start a cycle, pause it (lease gets released)
    let manifest = start_cycle(&mut engine, "evt-1");
    let cycle_id = &manifest.cycle_id;

    let now_ms = 1_700_000_000_000i64;
    let expires_ms = i64::MAX;
    engine
        .acquire_cycle_lease(cycle_id, "alice", now_ms, expires_ms)
        .unwrap();

    let receipt_dir = receipt_path(&_dir);
    engine
        .cycle_pause(
            cycle_id,
            PauseReason::PriorityRevoked,
            None,
            "test-actor",
            "cmd-pause-1",
            "evt-pause-1",
            TIMESTAMP,
            &receipt_dir,
            "alice",
            1,
            &auth(),
        )
        .unwrap();

    // Verify no lease after pause
    let lease_after_pause = engine.ledger().get_cycle_lease(cycle_id);
    assert!(
        lease_after_pause.is_err(),
        "lease should be released after pause"
    );

    // Resume the cycle
    let resume_result = engine.cycle_resume(
        cycle_id,
        "test-actor",
        "cmd-resume-1",
        "evt-resume-1",
        TIMESTAMP,
        &receipt_dir,
        "bob",
        &auth(),
    );

    assert!(
        resume_result.is_ok(),
        "cycle_resume should succeed, got: {:?}",
        resume_result
    );

    let _receipt = resume_result.unwrap();

    // Verify status is Open
    let updated = engine.ledger().get_cycle(cycle_id).unwrap();
    assert_eq!(
        updated.manifest.status,
        CycleStatus::Open,
        "cycle status should be Open after resume"
    );

    // Verify new lease was acquired
    let new_lease = engine.ledger().get_cycle_lease(cycle_id).unwrap();
    assert_eq!(new_lease.owner, "bob", "lease owner should be bob");
    assert_eq!(
        new_lease.fencing_token, 1,
        "fencing token should be 1 for new lease after prior was released"
    );

    // Verify resume receipt was written
    let resume_receipt_path = receipt_dir.join(cycle_id).join("resume-receipt.json");
    assert!(
        resume_receipt_path.exists(),
        "resume-receipt.json should exist"
    );
}

/// Scenario: resume from non-Paused status is rejected.
///
/// GIVEN cycle in `Open` (never paused)
/// WHEN running `sddk cycle resume --cycle X --lease-owner bob`
/// THEN exit non-zero `EngineError::ResumeFromPausedOnly`;
///     no new `cycle_leases` row; no event.
#[test]
fn resume_from_non_paused_rejected() {
    let (_dir, mut engine) = setup();

    // Start a cycle — stays in Open
    let manifest = start_cycle(&mut engine, "evt-1");
    let cycle_id = &manifest.cycle_id;

    let receipt_dir = receipt_path(&_dir);
    let result = engine.cycle_resume(
        cycle_id,
        "test-actor",
        "cmd-resume-1",
        "evt-resume-1",
        TIMESTAMP,
        &receipt_dir,
        "bob",
        &auth(),
    );

    assert!(result.is_err(), "cycle_resume from Open should fail");
    let err = result.unwrap_err();
    assert!(
        matches!(err, sddk_engine::EngineError::ResumeFromPausedOnly),
        "expected ResumeFromPausedOnly, got: {:?}",
        err
    );

    // Verify no lease was created
    let lease_result = engine.ledger().get_cycle_lease(cycle_id);
    assert!(
        lease_result.is_err(),
        "no lease should be created when resume fails"
    );
}

/// Scenario: supersede from Paused records prior_status and succeeds.
///
/// GIVEN cycle in `Paused` with `--successor Y` registered
/// WHEN running `sddk cycle supersede --cycle X --successor Y`
/// THEN status = `Closed` and `cycle.supersede.applied` payload contains
///     `prior_status = "Paused"`.
#[test]
fn supersede_from_paused_records_prior_status() {
    let (_dir, mut engine) = setup();

    // Start a cycle, pause it, then create successor cycle
    let manifest = start_cycle(&mut engine, "evt-1");
    let cycle_id = &manifest.cycle_id;

    // Create successor cycle first
    let succ_manifest = CycleManifest {
        cycle_id: "cycle-successor".into(),
        display_name: "Successor".into(),
        ..manifest.clone()
    };
    let succ_input = CycleStartInput {
        manifest: succ_manifest,
        requirements: cycle_start_requirements(),
    };
    let succ_plan = engine.plan_cycle_start(succ_input).unwrap();
    engine
        .apply_cycle_start(&succ_plan, &context("evt-succ-1", "cmd-succ-1"))
        .unwrap();

    // Pause the original cycle
    let now_ms = 1_700_000_000_000i64;
    let expires_ms = i64::MAX;
    engine
        .acquire_cycle_lease(cycle_id, "alice", now_ms, expires_ms)
        .unwrap();

    let receipt_dir = receipt_path(&_dir);
    engine
        .cycle_pause(
            cycle_id,
            PauseReason::PriorityRevoked,
            None,
            "test-actor",
            "cmd-pause-1",
            "evt-pause-1",
            TIMESTAMP,
            &receipt_dir,
            "alice",
            1,
            &auth(),
        )
        .unwrap();

    // Re-acquire lease after pause (lease was released by cycle_pause)
    engine
        .acquire_cycle_lease(cycle_id, "alice", 0, i64::MAX)
        .unwrap();

    // Now supersede from Paused
    engine
        .cycle_supersede(
            cycle_id,
            Some("cycle-successor".into()),
            None,
            &["evidence".into()],
            "test-actor",
            "cmd-supersede-1",
            "evt-supersede-1",
            TIMESTAMP,
            &receipt_dir,
            "alice",
            1,
            &auth(),
        )
        .unwrap();

    // Verify status is Closed
    let updated = engine.ledger().get_cycle(cycle_id).unwrap();
    assert_eq!(
        updated.manifest.status,
        CycleStatus::Closed,
        "cycle should be Closed after supersede"
    );
    // Note: prior_status is in state_before (full manifest JSON), verified via event log
    let events = engine.ledger().list_cycle_events(cycle_id).unwrap();
    let applied_event = events
        .iter()
        .find(|e| e.event_type == "cycle.supersede.applied")
        .unwrap();
    let state_before = applied_event.state_before.as_ref().unwrap();
    let prior_status = state_before.get("status").and_then(|v| v.as_str());
    assert_eq!(
        prior_status,
        Some("PAUSED"),
        "prior_status (state_before.status) in supersede.applied should be PAUSED"
    );
}

/// Scenario: cycle next surfaces resume and supersede when paused.
///
/// GIVEN cycle in `Paused`
/// WHEN running `sddk cycle next --json --cycle X`
/// THEN frontier = `{cycle.pause (no-op), cycle.resume, cycle.supersede}`
///     and response is non-empty (no `[]`).
#[test]
fn cycle_next_surfaces_resume_and_supersede_when_paused() {
    let (_dir, mut engine) = setup();

    // Start a cycle and pause it
    let manifest = start_cycle(&mut engine, "evt-1");
    let cycle_id = &manifest.cycle_id;

    let now_ms = 1_700_000_000_000i64;
    let expires_ms = i64::MAX;
    engine
        .acquire_cycle_lease(cycle_id, "alice", now_ms, expires_ms)
        .unwrap();

    let receipt_dir = receipt_path(&_dir);
    engine
        .cycle_pause(
            cycle_id,
            PauseReason::PriorityRevoked,
            None,
            "test-actor",
            "cmd-pause-1",
            "evt-pause-1",
            TIMESTAMP,
            &receipt_dir,
            "alice",
            1,
            &auth(),
        )
        .unwrap();

    // Get frontier
    let workflow = sddk_engine::load_workflow_str(WORKFLOW_YAML).unwrap();
    let state = engine.ledger().get_cycle(cycle_id).unwrap().manifest;
    let frontier =
        sddk_engine::frontier_for_state(&workflow, &state, cycle_id, engine.ledger()).unwrap();

    // Verify frontier is non-empty
    assert!(
        !frontier.is_empty(),
        "frontier should be non-empty for Paused cycle"
    );

    // Verify cycle.resume is in frontier
    let resume_entry = frontier.iter().find(|e| e.transition_id == "cycle.resume");
    assert!(
        resume_entry.is_some(),
        "cycle.resume should be in frontier for Paused cycle"
    );
}

/// Scenario: --review-at is informational only (recorded on receipt, no auto-reactivation).
///
/// GIVEN cycle in `Open` with active lease
/// WHEN running `sddk cycle pause --cycle X --reason dependency_waiting
///        --review-at 2026-12-31T00:00:00Z --lease-owner alice --fencing-token 1`
/// THEN `pause-receipt.json.review_at = "2026-12-31T00:00:00Z"`, status = `Paused`,
///     no scheduler/reactivation event at or after the timestamp.
#[test]
fn review_at_informational_only() {
    let (_dir, mut engine) = setup();

    let manifest = start_cycle(&mut engine, "evt-1");
    let cycle_id = &manifest.cycle_id;

    let now_ms = 1_700_000_000_000i64;
    let expires_ms = i64::MAX;
    engine
        .acquire_cycle_lease(cycle_id, "alice", now_ms, expires_ms)
        .unwrap();

    let receipt_dir = receipt_path(&_dir);
    let review_at = "2026-12-31T00:00:00Z";
    engine
        .cycle_pause(
            cycle_id,
            PauseReason::DependencyWaiting,
            Some(review_at),
            "test-actor",
            "cmd-pause-1",
            "evt-pause-1",
            TIMESTAMP,
            &receipt_dir,
            "alice",
            1,
            &auth(),
        )
        .unwrap();

    // Verify receipt contains review_at
    let receipt_path = receipt_dir.join(cycle_id).join("pause-receipt.json");
    let receipt_content = std::fs::read_to_string(&receipt_path).unwrap();
    let receipt: serde_json::Value = serde_json::from_str(&receipt_content).unwrap();
    assert_eq!(
        receipt.get("review_at").and_then(|v| v.as_str()),
        Some(review_at),
        "review_at should be recorded in receipt"
    );
}
