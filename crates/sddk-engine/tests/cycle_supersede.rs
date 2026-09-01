//! Contract tests for `Engine::cycle_supersede`.
//!
//! Per [[REQ-Cycle-Supersede-Contract]] and [[REQ-Cycle-Lease-Fence]]:
//! - lease fence required (STORAGE_LEASE_REQUIRED)
//! - successor XOR reason required (STORAGE_SUPERSEDE_REQUIRES_EXACTLY_ONE)
//! - self-supersede forbidden (STORAGE_SUPERSEDE_SELF_FORBIDDEN)
//! - appends 2 events: cycle.supersede.requested + cycle.supersede.applied
//! - writes atomic supersede-receipt.json

use std::collections::{BTreeSet, HashMap};
use std::path::Path;

use sddk_domain::{
    CycleManifest, CyclePath, CycleStatus, Phase, StorageError as DomainStorageError,
};
use sddk_engine::{CycleStartInput, Engine, EventContext, SupersedeReason};
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
        display_name: "Supersede work".into(),
        status: CycleStatus::Open,
        phase: Phase::Explore,
        path,
        branch: "feat/supersede".into(),
        base: "abc123".into(),
        head: None,
        artifacts: HashMap::new(),
        release: None,
        delivery_kind: None,
        remediation_round: 0,
        remote_url: Some("https://example.com/owner/project".into()),
        scope: Some("owner".into()),
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
        occurred_at: TIMESTAMP.into(),
    }
}

// ── REQ-Cycle-Lease-Fence ────────────────────────────────────────────────────

#[test]
fn supersede_requires_lease_fence() {
    let (_dir, mut engine) = setup();
    let manifest = start_cycle(&mut engine, "event-1");

    // No lease acquired — must fail with StorageError::LeaseConflict
    let result = engine.cycle_supersede(
        &manifest.cycle_id,
        None,
        Some(SupersedeReason::ScopeInvalid),
        &[],
        "test-actor",
        "command-supersede",
        "event-supersede-1",
        TIMESTAMP,
        Path::new("/tmp"),
        "test-actor",
        1,
    );
    assert!(result.is_err());
    let err = result.unwrap_err();
    assert!(
        matches!(
            err,
            sddk_engine::EngineError::Storage(DomainStorageError::LeaseConflict { .. })
        ),
        "expected LeaseConflict error, got: {:?}",
        err
    );
}

// ── REQ-Cycle-Supersede-Contract: XOR ───────────────────────────────────────

#[test]
fn supersede_requires_exactly_one_of_successor_or_reason() {
    let (_dir, mut engine) = setup();
    let manifest = start_cycle(&mut engine, "event-1");

    // Acquire lease
    engine
        .acquire_cycle_lease(&manifest.cycle_id, "test-actor", 0, i64::MAX)
        .unwrap();

    // Both successor AND reason provided — must fail
    let result = engine.cycle_supersede(
        &manifest.cycle_id,
        Some("cycle-successor".into()),
        Some(SupersedeReason::ScopeInvalid),
        &[],
        "test-actor",
        "command-supersede",
        "event-supersede-1",
        TIMESTAMP,
        Path::new("/tmp"),
        "test-actor",
        1,
    );
    assert!(result.is_err());
    let err = result.unwrap_err();
    assert!(
        matches!(err, sddk_engine::EngineError::SupersedeRequiresExactlyOne),
        "expected SupersedeRequiresExactlyOne error, got: {:?}",
        err
    );

    // Neither successor nor reason — must also fail
    let result = engine.cycle_supersede(
        &manifest.cycle_id,
        None,
        None,
        &[],
        "test-actor",
        "command-supersede",
        "event-supersede-1",
        TIMESTAMP,
        Path::new("/tmp"),
        "test-actor",
        1,
    );
    assert!(result.is_err());
    let err = result.unwrap_err();
    assert!(
        matches!(err, sddk_engine::EngineError::SupersedeRequiresExactlyOne),
        "expected SupersedeRequiresExactlyOne error, got: {:?}",
        err
    );
}

// ── Anti-self-supersede ───────────────────────────────────────────────────────

#[test]
fn supersede_self_is_forbidden() {
    let (_dir, mut engine) = setup();
    let manifest = start_cycle(&mut engine, "event-1");

    // Acquire lease
    engine
        .acquire_cycle_lease(&manifest.cycle_id, "test-actor", 0, i64::MAX)
        .unwrap();

    // Self-supersede with reason — must fail
    let result = engine.cycle_supersede(
        &manifest.cycle_id,
        Some(manifest.cycle_id.clone()),
        None,
        &[],
        "test-actor",
        "command-supersede",
        "event-supersede-1",
        TIMESTAMP,
        Path::new("/tmp"),
        "test-actor",
        1,
    );
    assert!(result.is_err());
    let err = result.unwrap_err();
    assert!(
        matches!(err, sddk_engine::EngineError::SupersedeSelfForbidden),
        "expected SupersedeSelfForbidden error, got: {:?}",
        err
    );
}

// ── Two events appended ───────────────────────────────────────────────────────

#[test]
fn supersede_appends_requested_and_applied_events() {
    let (_dir, mut engine) = setup();
    let manifest = start_cycle(&mut engine, "event-1");

    // Acquire lease
    engine
        .acquire_cycle_lease(&manifest.cycle_id, "test-actor", 0, i64::MAX)
        .unwrap();

    let result = engine.cycle_supersede(
        &manifest.cycle_id,
        None,
        Some(SupersedeReason::ScopeInvalid),
        &[],
        "test-actor",
        "command-supersede",
        "event-supersede-1",
        TIMESTAMP,
        Path::new("/tmp"),
        "test-actor",
        1,
    );
    assert!(
        result.is_ok(),
        "supersede should succeed: {:?}",
        result.err()
    );

    // Check 2 new events appended
    let events = engine.ledger().list_cycle_events("cycle-1").unwrap();
    let event_types: Vec<_> = events.iter().map(|e| e.event_type.as_str()).collect();
    assert!(
        event_types.contains(&"cycle.supersede.requested"),
        "must contain cycle.supersede.requested event"
    );
    assert!(
        event_types.contains(&"cycle.supersede.applied"),
        "must contain cycle.supersede.applied event"
    );

    // Receipt written at expected path
    let receipt_path = Path::new("/tmp").join("cycle-1/supersede-receipt.json");
    assert!(
        receipt_path.exists(),
        "supersede-receipt.json must exist at {:?}",
        receipt_path
    );
}

// ── SupersedeReason variants ───────────────────────────────────────────────────

#[test]
fn supersede_reason_scope_invalid() {
    let (_dir, mut engine) = setup();
    let manifest = start_cycle(&mut engine, "event-1");
    engine
        .acquire_cycle_lease(&manifest.cycle_id, "test-actor", 0, i64::MAX)
        .unwrap();

    let result = engine.cycle_supersede(
        &manifest.cycle_id,
        None,
        Some(SupersedeReason::ScopeInvalid),
        &[],
        "test-actor",
        "command-supersede",
        "event-supersede-1",
        TIMESTAMP,
        Path::new("/tmp"),
        "test-actor",
        1,
    );
    assert!(
        result.is_ok(),
        "supersede with ScopeInvalid reason should succeed"
    );
}

#[test]
fn supersede_reason_goal_replaced() {
    let (_dir, mut engine) = setup();
    let manifest = start_cycle(&mut engine, "event-1");
    engine
        .acquire_cycle_lease(&manifest.cycle_id, "test-actor", 0, i64::MAX)
        .unwrap();

    let result = engine.cycle_supersede(
        &manifest.cycle_id,
        None,
        Some(SupersedeReason::GoalReplaced),
        &[],
        "test-actor",
        "command-supersede",
        "event-supersede-1",
        TIMESTAMP,
        Path::new("/tmp"),
        "test-actor",
        1,
    );
    assert!(
        result.is_ok(),
        "supersede with GoalReplaced reason should succeed"
    );
}

#[test]
fn supersede_reason_external_obsolete() {
    let (_dir, mut engine) = setup();
    let manifest = start_cycle(&mut engine, "event-1");
    engine
        .acquire_cycle_lease(&manifest.cycle_id, "test-actor", 0, i64::MAX)
        .unwrap();

    let result = engine.cycle_supersede(
        &manifest.cycle_id,
        None,
        Some(SupersedeReason::ExternalObsolete),
        &[],
        "test-actor",
        "command-supersede",
        "event-supersede-1",
        TIMESTAMP,
        Path::new("/tmp"),
        "test-actor",
        1,
    );
    assert!(
        result.is_ok(),
        "supersede with ExternalObsolete reason should succeed"
    );
}
