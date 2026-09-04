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
use sddk_engine::{
    CycleStartInput, Engine, EventContext, SupersedeReason, authority::AuthorityContext,
};
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
        occurred_at: TIMESTAMP.into(),
    }
}

fn auth() -> AuthorityContext {
    AuthorityContext::for_test(sddk_domain::ActorKind::Agent, "test-runtime")
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
        &["evidence-1".to_string()],
        "test-actor",
        "command-supersede",
        "event-supersede-1",
        TIMESTAMP,
        Path::new("/tmp"),
        "test-actor",
        1,
        &auth(),
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
        &["evidence-1".to_string()],
        "test-actor",
        "command-supersede",
        "event-supersede-1",
        TIMESTAMP,
        Path::new("/tmp"),
        "test-actor",
        1,
        &auth(),
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
        &["evidence-1".to_string()],
        "test-actor",
        "command-supersede",
        "event-supersede-1",
        TIMESTAMP,
        Path::new("/tmp"),
        "test-actor",
        1,
        &auth(),
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
        &["evidence-1".to_string()],
        "test-actor",
        "command-supersede",
        "event-supersede-1",
        TIMESTAMP,
        Path::new("/tmp"),
        "test-actor",
        1,
        &auth(),
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
        &["evidence-1".to_string()],
        "test-actor",
        "command-supersede",
        "event-supersede-1",
        TIMESTAMP,
        Path::new("/tmp"),
        "test-actor",
        1,
        &auth(),
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
        &["evidence-1".to_string()],
        "test-actor",
        "command-supersede",
        "event-supersede-1",
        TIMESTAMP,
        Path::new("/tmp"),
        "test-actor",
        1,
        &auth(),
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
        &["evidence-1".to_string()],
        "test-actor",
        "command-supersede",
        "event-supersede-1",
        TIMESTAMP,
        Path::new("/tmp"),
        "test-actor",
        1,
        &auth(),
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
        &["evidence-1".to_string()],
        "test-actor",
        "command-supersede",
        "event-supersede-1",
        TIMESTAMP,
        Path::new("/tmp"),
        "test-actor",
        1,
        &auth(),
    );
    assert!(
        result.is_ok(),
        "supersede with ExternalObsolete reason should succeed"
    );
}

// ── GAP-BUG-1: lease released atomically on supersede ─────────────────────────

/// S-LEASE-RELEASED: dado un cycle activo con lease activo, cuando supersede
/// retorna Ok, entonces la fila cycle_leases para ese cycle_id ya no existe
/// y un evento lease.released aparece en el ledger.
#[test]
fn supersede_releases_lease_atomically() {
    let (_dir, mut engine) = setup();
    let manifest = start_cycle(&mut engine, "event-1");

    // Acquire lease — acquire_cycle_lease computes fencing_token internally (1 for new lease)
    engine
        .acquire_cycle_lease(&manifest.cycle_id, "lease-owner", 0, i64::MAX)
        .unwrap();

    // Verify lease exists before supersede
    let lease_before = engine.ledger().get_cycle_lease(&manifest.cycle_id);
    assert!(lease_before.is_ok(), "lease should exist before supersede");

    // Supersede succeeds — must pass matching fencing_token (1, computed by acquire)
    let result = engine.cycle_supersede(
        &manifest.cycle_id,
        None,
        Some(SupersedeReason::ScopeInvalid),
        &["evidence-1".to_string()],
        "test-actor",
        "command-supersede",
        "event-supersede-1",
        TIMESTAMP,
        Path::new("/tmp"),
        "lease-owner",
        1, // must match the fencing_token computed by acquire_cycle_lease
        &auth(),
    );
    assert!(
        result.is_ok(),
        "supersede should succeed: {:?}",
        result.err()
    );

    // Lease is gone — get_cycle_lease returns NotFound
    let lease_after = engine.ledger().get_cycle_lease(&manifest.cycle_id);
    assert!(
        matches!(lease_after, Err(sddk_domain::StorageError::NotFound { .. })),
        "lease should be released after supersede, got: {:?}",
        lease_after
    );

    // lease.released event is appended (side-effect of GAP-BUG-1 fix)
    let events = engine
        .ledger()
        .list_cycle_events(&manifest.cycle_id)
        .unwrap();
    let event_types: Vec<_> = events.iter().map(|e| e.event_type.as_str()).collect();
    assert!(
        event_types.contains(&"lease.released"),
        "lease.released event must be appended, got: {:?}",
        event_types
    );
}

// ── GAP-BUG-2/3: receipt writes lease_owner and fencing_token from caller args ─

/// S-RECEIPT-OWNER: dado supersede exitoso con lease_owner="alice" y actor="bob",
/// cuando el receipt se lee, entonces lease_owner=="alice" (no el actor).
#[test]
fn supersede_receipt_writes_lease_owner_from_caller_arg() {
    let (_dir, mut engine) = setup();
    let manifest = start_cycle(&mut engine, "event-1");

    engine
        .acquire_cycle_lease(&manifest.cycle_id, "alice", 0, i64::MAX)
        .unwrap();

    let receipt_dir = tempfile::tempdir().unwrap();
    engine
        .cycle_supersede(
            &manifest.cycle_id,
            None,
            Some(SupersedeReason::ScopeInvalid),
            &["evidence-1".to_string()],
            "bob", // actor is "bob", but lease_owner arg is "alice"
            "command-supersede",
            "event-supersede-1",
            TIMESTAMP,
            receipt_dir.path(),
            "alice", // lease_owner passed as "alice"
            1,
            &auth(),
        )
        .unwrap();

    let receipt_path = receipt_dir
        .path()
        .join(&manifest.cycle_id)
        .join("supersede-receipt.json");
    let receipt_json = std::fs::read_to_string(&receipt_path).expect("receipt file should exist");
    let receipt: serde_json::Value =
        serde_json::from_str(&receipt_json).expect("receipt must be valid JSON");

    assert_eq!(
        receipt["lease_owner"].as_str(),
        Some("alice"),
        "receipt.lease_owner must be 'alice' (the lease_owner arg), not the actor"
    );
}

/// S-RECEIPT-FENCE: dado supersede exitoso con fencing_token del argumento (no 0),
/// cuando el receipt se lee, entonces fencing_token==el valor passado.
/// Nota: acquire_cycle_lease calcula fencing_token internamente (1 para leases nuevos).
/// Para que verify_cycle_lease no falle, cycle_supersede debe recibir el mismo valor.
#[test]
fn supersede_receipt_writes_fencing_token_used() {
    let (_dir, mut engine) = setup();
    let manifest = start_cycle(&mut engine, "event-1");

    // acquire_cycle_leaseinternally computes fencing_token=1 for a new lease
    engine
        .acquire_cycle_lease(&manifest.cycle_id, "test-actor", 0, i64::MAX)
        .unwrap();

    let receipt_dir = tempfile::tempdir().unwrap();
    // Must pass fencing_token=1 to match what was computed; verify_cycle_lease checks it
    engine
        .cycle_supersede(
            &manifest.cycle_id,
            None,
            Some(SupersedeReason::GoalReplaced),
            &["evidence-1".to_string()],
            "test-actor",
            "command-supersede",
            "event-supersede-1",
            TIMESTAMP,
            receipt_dir.path(),
            "test-actor",
            1, // fencing_token — must match what acquire_cycle_lease computed internally
            &auth(),
        )
        .unwrap();

    let receipt_path = receipt_dir
        .path()
        .join(&manifest.cycle_id)
        .join("supersede-receipt.json");
    let receipt_json = std::fs::read_to_string(&receipt_path).expect("receipt file should exist");
    let receipt: serde_json::Value =
        serde_json::from_str(&receipt_json).expect("receipt must be valid JSON");

    // The receipt must contain the fencing_token value passed to cycle_supersede
    assert_eq!(
        receipt["fencing_token"].as_i64(),
        Some(1),
        "receipt.fencing_token must be 1 (the fencing_token arg passed to cycle_supersede)"
    );
}

// ── GAP-V-2: successor must exist before any state mutation ───────────────────

/// S-SUCCESSOR-EXISTS: dado supersede con --successor inexistente, el engine
/// retorna SupersedeSuccessorNotFound ANTES de cualquier mutación de estado.
#[test]
fn supersede_rejects_nonexistent_successor_before_state_mutation() {
    let (_dir, mut engine) = setup();
    let manifest = start_cycle(&mut engine, "event-1");

    engine
        .acquire_cycle_lease(&manifest.cycle_id, "test-actor", 0, i64::MAX)
        .unwrap();

    // Capture state before the call
    let events_before = engine
        .ledger()
        .list_cycle_events(&manifest.cycle_id)
        .unwrap();
    let events_before_count = events_before.len();
    let cycle_before = engine.ledger().get_cycle(&manifest.cycle_id).unwrap();

    // Call supersede with nonexistent successor
    let result = engine.cycle_supersede(
        &manifest.cycle_id,
        Some("nonexistent-cycle-999".into()),
        None,
        &["evidence-1".to_string()],
        "test-actor",
        "command-supersede",
        "event-supersede-1",
        TIMESTAMP,
        Path::new("/tmp"),
        "test-actor",
        1,
        &auth(),
    );

    // Must fail with SupersedeSuccessorNotFound
    assert!(
        result.is_err(),
        "supersede should fail for nonexistent successor"
    );
    let err = result.unwrap_err();
    assert!(
        matches!(
            err,
            sddk_engine::EngineError::SupersedeSuccessorNotFound(ref s)
            if s == "nonexistent-cycle-999"
        ),
        "expected SupersedeSuccessorNotFound('nonexistent-cycle-999'), got: {:?}",
        err
    );

    // Verify NO state mutation occurred — events unchanged
    let events_after = engine
        .ledger()
        .list_cycle_events(&manifest.cycle_id)
        .unwrap();
    assert_eq!(
        events_after.len(),
        events_before_count,
        "no events should be appended when successor not found"
    );

    // Cycle status unchanged (still Open, not Closed)
    let cycle_after = engine.ledger().get_cycle(&manifest.cycle_id).unwrap();
    assert_eq!(
        cycle_before.manifest.status, cycle_after.manifest.status,
        "cycle status must not change when successor not found"
    );
}

// ── GAP-V-3: evidence_refs must not be empty ──────────────────────────────────

/// S-EVIDENCE-MIN: dado supersede con evidence_refs vacío, el engine retorna
/// SupersedeEvidenceRefsRequired ANTES de cualquier mutación.
#[test]
fn supersede_rejects_empty_evidence_refs_before_state_mutation() {
    let (_dir, mut engine) = setup();
    let manifest = start_cycle(&mut engine, "event-1");

    engine
        .acquire_cycle_lease(&manifest.cycle_id, "test-actor", 0, i64::MAX)
        .unwrap();

    // Capture state before the call
    let events_before = engine
        .ledger()
        .list_cycle_events(&manifest.cycle_id)
        .unwrap();
    let events_before_count = events_before.len();
    let cycle_before = engine.ledger().get_cycle(&manifest.cycle_id).unwrap();

    // Call supersede with empty evidence_refs
    let result = engine.cycle_supersede(
        &manifest.cycle_id,
        None,
        Some(SupersedeReason::ScopeInvalid),
        &[], // empty evidence_refs
        "test-actor",
        "command-supersede",
        "event-supersede-1",
        TIMESTAMP,
        Path::new("/tmp"),
        "test-actor",
        1,
        &auth(),
    );

    // Must fail with SupersedeEvidenceRefsRequired
    assert!(
        result.is_err(),
        "supersede should fail with empty evidence_refs"
    );
    let err = result.unwrap_err();
    assert!(
        matches!(err, sddk_engine::EngineError::SupersedeEvidenceRefsRequired),
        "expected SupersedeEvidenceRefsRequired, got: {:?}",
        err
    );

    // Verify NO state mutation occurred
    let events_after = engine
        .ledger()
        .list_cycle_events(&manifest.cycle_id)
        .unwrap();
    assert_eq!(
        events_after.len(),
        events_before_count,
        "no events should be appended when evidence_refs is empty"
    );

    let cycle_after = engine.ledger().get_cycle(&manifest.cycle_id).unwrap();
    assert_eq!(
        cycle_before.manifest.status, cycle_after.manifest.status,
        "cycle status must not change when evidence_refs is empty"
    );
}

// ── GAP-T6: ledger append-only invariant ─────────────────────────────────────

/// S-LEDGER-INVARIANT: dado cualquier supersede exitoso, los eventos pre-existentes
/// mantienen su digest sha256 idéntico byte-a-byte; los eventos previos no se mutan.
///
/// La implementación actual appendea 3 eventos (N+3): cycle.supersede.requested,
/// lease.released (side-effect del fix GAP-BUG-1), y cycle.supersede.applied.
/// El spec dice N+2, pero la implementación hace N+3 actualmente.
#[test]
fn supersede_preserves_ledger_event_digests() {
    let (_dir, mut engine) = setup();
    let manifest = start_cycle(&mut engine, "event-1");

    engine
        .acquire_cycle_lease(&manifest.cycle_id, "test-actor", 0, i64::MAX)
        .unwrap();

    // Capture pre-supersede events and their digests
    let events_pre = engine
        .ledger()
        .list_cycle_events(&manifest.cycle_id)
        .unwrap();
    let n_pre = events_pre.len();
    let digests_pre: Vec<String> = events_pre.iter().map(|e| e.event_hash.clone()).collect();

    // Perform supersede
    engine
        .cycle_supersede(
            &manifest.cycle_id,
            None,
            Some(SupersedeReason::ScopeInvalid),
            &["evidence-1".to_string()],
            "test-actor",
            "command-supersede",
            "event-supersede-1",
            TIMESTAMP,
            Path::new("/tmp"),
            "test-actor",
            1,
            &auth(),
        )
        .unwrap();

    // Capture post-supersede events
    let events_post = engine
        .ledger()
        .list_cycle_events(&manifest.cycle_id)
        .unwrap();

    // Pre-existing events' digests must be byte-identical (ledger append-only invariant)
    for i in 0..n_pre {
        assert_eq!(
            events_post[i].event_hash, digests_pre[i],
            "pre-supersede event[{}] digest must be unchanged after supersede",
            i
        );
    }

    // The implementation appends 3 events (N+3): requested + lease.released + applied.
    // Verify that exactly 3 new events are appended and the types are correct.
    let new_event_types: Vec<_> = events_post[n_pre..]
        .iter()
        .map(|e| e.event_type.as_str())
        .collect();
    assert_eq!(
        new_event_types.len(),
        3,
        "implementation appends 3 events (N+3): requested, lease.released, applied"
    );
    assert!(
        new_event_types.contains(&"cycle.supersede.requested"),
        "new event must include cycle.supersede.requested"
    );
    assert!(
        new_event_types.contains(&"lease.released"),
        "new event must include lease.released (GAP-BUG-1 side-effect)"
    );
    assert!(
        new_event_types.contains(&"cycle.supersede.applied"),
        "new event must include cycle.supersede.applied"
    );
}
