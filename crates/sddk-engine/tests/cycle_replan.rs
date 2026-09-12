//! Contract tests for `Engine::cycle_replan`.
//!
//! Per [[REQ-Cycle-Replan-Bounded-Counter]] and [[REQ-Cycle-Replan-Receipt]]:
//! - counter ≤ 5 (ReplanLimitExceeded on the 6th attempt)
//! - delta must be non-empty (ReplanEmptyDelta)
//! - lease fence required (LeaseConflict when no lease is held)
//! - success increments replan_count, emits events, writes replan-receipt.json

use std::collections::{BTreeSet, HashMap};
use std::path::Path;

use sddk_domain::{CycleManifest, CyclePath, CycleStatus, Phase};
use sddk_engine::{CycleStartInput, Engine, EventContext, REPLAN_LIMIT, RestageTo};
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
        .apply_cycle_start(&plan, &context(event_id, "command-a"), &auth())
        .unwrap()
        .manifest
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

fn manifest_for_path(path: CyclePath) -> CycleManifest {
    CycleManifest {
        schema_version: 1,
        project_id: "project-1".into(),
        workspace_id: "workspace-1".into(),
        cycle_id: "cycle-1".into(),
        display_name: "Replan work".into(),
        status: CycleStatus::Open,
        phase: Phase::Explore,
        path,
        branch: "feat/replan".into(),
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

fn delta() -> sddk_engine::ReplanDelta {
    sddk_engine::ReplanDelta {
        changed_files: vec!["src/lib.rs".into()],
        reason: "fix bug".into(),
    }
}

#[allow(clippy::too_many_arguments)]
fn call_replan(
    engine: &mut Engine<Storage>,
    cycle_id: &str,
    event_id: &str,
    receipt_dir: &Path,
) -> Result<sddk_engine::EventReceipt, sddk_engine::EngineError> {
    engine.cycle_replan(
        cycle_id,
        RestageTo::Design,
        &delta(),
        &[],
        "test-actor",
        "command-replan",
        event_id,
        TIMESTAMP,
        receipt_dir,
        "test-actor",
        1,
    )
}

// ── Happy path ───────────────────────────────────────────────────────────────

#[test]
fn replan_success_increments_counter_and_writes_receipt() {
    let (dir, mut engine) = setup();
    let manifest = start_cycle(&mut engine, "event-1");

    engine
        .acquire_cycle_lease(&manifest.cycle_id, "test-actor", 0, i64::MAX)
        .unwrap();

    let receipt = call_replan(
        &mut engine,
        &manifest.cycle_id,
        "event-replan-1",
        dir.path(),
    )
    .expect("first replan must succeed");

    assert!(receipt.sequence > 0);

    // Counter incremented by exactly 1.
    let record = engine.ledger().get_cycle(&manifest.cycle_id).unwrap();
    assert_eq!(record.manifest.replan_count, 1);
    assert_eq!(record.manifest.phase, Phase::Design);

    // Receipt written atomically with the delta.
    let receipt_file = dir
        .path()
        .join(&manifest.cycle_id)
        .join("replan-receipt.json");
    assert!(receipt_file.exists(), "replan-receipt.json must exist");
    let raw = std::fs::read_to_string(&receipt_file).unwrap();
    let parsed: serde_json::Value = serde_json::from_str(&raw).unwrap();
    assert_eq!(parsed["replan_count"], 1);
    assert_eq!(parsed["reason"], "fix bug");
    assert_eq!(parsed["lease_owner"], "test-actor");

    // Both ledger events recorded.
    let events = engine
        .ledger()
        .list_cycle_events(&manifest.cycle_id)
        .unwrap();
    let types: Vec<&str> = events.iter().map(|e| e.event_type.as_str()).collect();
    assert!(
        types.contains(&"cycle.replan.requested"),
        "types: {types:?}"
    );
    assert!(types.contains(&"cycle.replan.applied"), "types: {types:?}");
}

#[test]
fn replan_sixth_attempt_fails_with_limit() {
    let (dir, mut engine) = setup();
    let manifest = start_cycle(&mut engine, "event-1");

    engine
        .acquire_cycle_lease(&manifest.cycle_id, "test-actor", 0, i64::MAX)
        .unwrap();

    assert_eq!(REPLAN_LIMIT, 5);
    for i in 0..REPLAN_LIMIT {
        call_replan(
            &mut engine,
            &manifest.cycle_id,
            &format!("event-replan-{i}"),
            dir.path(),
        )
        .unwrap_or_else(|e| panic!("replan {i} must succeed: {e:?}"));
    }

    // 6th replan fails with ReplanLimitExceeded; manifest unchanged.
    let result = call_replan(
        &mut engine,
        &manifest.cycle_id,
        "event-replan-6",
        dir.path(),
    );
    assert!(
        matches!(result, Err(sddk_engine::EngineError::ReplanLimitExceeded)),
        "expected ReplanLimitExceeded, got: {result:?}"
    );
    let record = engine.ledger().get_cycle(&manifest.cycle_id).unwrap();
    assert_eq!(record.manifest.replan_count, REPLAN_LIMIT);
}

// ── Empty delta ───────────────────────────────────────────────────────────────

#[test]
fn replan_empty_delta_returns_error() {
    let (dir, mut engine) = setup();
    let manifest = start_cycle(&mut engine, "event-1");

    engine
        .acquire_cycle_lease(&manifest.cycle_id, "test-actor", 0, i64::MAX)
        .unwrap();

    let empty = sddk_engine::ReplanDelta {
        changed_files: vec![],
        reason: "".into(),
    };
    let result = engine.cycle_replan(
        &manifest.cycle_id,
        RestageTo::Specify,
        &empty,
        &[],
        "test-actor",
        "command-replan",
        "event-replan-1",
        TIMESTAMP,
        dir.path(),
        "test-actor",
        1,
    );
    assert!(
        matches!(result, Err(sddk_engine::EngineError::ReplanEmptyDelta)),
        "expected ReplanEmptyDelta, got: {result:?}"
    );
    // Manifest untouched.
    let record = engine.ledger().get_cycle(&manifest.cycle_id).unwrap();
    assert_eq!(record.manifest.replan_count, 0);
}

// ── Lease fence ───────────────────────────────────────────────────────────────

#[test]
fn replan_without_lease_returns_lease_conflict() {
    let (dir, mut engine) = setup();
    let manifest = start_cycle(&mut engine, "event-1");

    // No lease acquired — must fail closed with LeaseConflict.
    let result = call_replan(
        &mut engine,
        &manifest.cycle_id,
        "event-replan-1",
        dir.path(),
    );
    match result {
        Err(sddk_engine::EngineError::Storage(sddk_domain::StorageError::LeaseConflict {
            ..
        })) => {}
        other => panic!("expected LeaseConflict, got: {other:?}"),
    }
    let record = engine.ledger().get_cycle(&manifest.cycle_id).unwrap();
    assert_eq!(record.manifest.replan_count, 0);
}

fn auth() -> sddk_engine::authority::AuthorityContext {
    sddk_engine::authority::AuthorityContext::for_test(
        sddk_domain::ActorKind::System,
        "test-system",
    )
}
