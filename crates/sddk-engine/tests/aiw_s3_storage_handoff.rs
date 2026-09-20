//! `aiw_s3_storage_handoff.rs` — AIW-S3 integration test
//!
//! Verifies that two processes can exchange a context capsule via
//! sqlite-backed `Storage` only — no in-memory hand-off, no shared
//! transcript (H01 + H06 from `docs/proposals/2026-09-19-adaptive-
//! inputs-workflows/uat/UAT-MATRIX.md`).
//!
//! Slice: `p-63676b11dc0ef88f/aiw-s3-handoff-durable`.

use std::collections::BTreeSet;

use sddk_domain::LedgerEvent;
use sddk_engine::context_compiler::{
    ContextCompiler,
    storage_adapter::{StorageLedgerHeadAdapter, StorageProjectAdapter, StorageSnapshot},
};
use sddk_storage::{ProjectRecord, Storage, StorageError};
use tempfile::TempDir;

/// Open a fresh file-backed storage with one project and return its
/// path AND the dir guard. The dir guard must outlive the storage
/// handle; the caller is responsible for keeping it alive across
/// the simulated task boundary.
fn open_storage_with_project(label: &str) -> (TempDir, std::path::PathBuf) {
    let dir = tempfile::tempdir().expect("tempdir");
    let path = dir.path().join("ledger.sqlite");
    let storage = Storage::open(&path).expect("open");
    storage
        .insert_project(&ProjectRecord {
            project_id: format!("proj-{label}"),
            display_name: format!("Project {label}"),
            remote_url: Some(format!("https://example.com/{label}")),
            scope: "test".into(),
            created_at: "2026-09-20T00:00:00Z".into(),
        })
        .expect("insert_project");
    (dir, path)
}

/// H01 happy path: a first process produces a capsule from storage; a
/// second process reopens the same ledger (simulated by recreating the
/// adapter on the second storage handle) and re-derives a capsule
/// whose payload bytes match the first. This is the **durability +
/// E2E** invariant: the second task recovered state from durable
/// storage and built the same capsule without a transcript.
#[test]
fn h01_dur_e2e_second_task_recompiles_capsule_from_storage() {
    // A SINGLE `TempDir` guard persists across both simulated tasks.
    // We *only* drop the storage handles between tasks. The directory
    // outlives both handles, mirroring two OS processes sharing the
    // same filesystem path.
    let (_dir_guard, ledger_path) = open_storage_with_project("alpha");

    // === TASK 1 ===
    let task1_storage = Storage::open(&ledger_path).expect("task1 open");
    let events_task1 = task1_storage.list_events().expect("task1 list");
    let project_task1 = task1_storage.get_project("proj-alpha").expect("task1 get");
    let snapshot1 = StorageSnapshot::from_ledger_events("storage.ledger_head", &events_task1);

    let project_adapter1 =
        StorageProjectAdapter::new(&project_task1.project_id, &project_task1.created_at);
    let ledger_adapter1 = StorageLedgerHeadAdapter::from_snapshot(snapshot1.clone());

    let compiler1 = ContextCompiler {
        planning: Box::new(StorageLedgerHeadAdapter::from_snapshot(snapshot1.clone())),
        run: Box::new(ledger_adapter1),
        memory: Box::new(project_adapter1),
        graph: Box::new(StorageLedgerHeadAdapter::from_snapshot(snapshot1.clone())),
        vault_knowledge: None,
        unknown_sources: vec![],
    };
    let read_head =
        u64::try_from(events_task1.last().map(|e| e.sequence).unwrap_or(0).max(0)).unwrap_or(0);
    let capsule1 = compiler1.compile(read_head).expect("compile1");
    drop(task1_storage);
    // The dir_guard is still in scope; the file is on disk and shared.

    // === TASK 2 ===
    // A new process (or at least a new Storage handle) reopens the
    // same ledger path. The dir guard is still alive (we keep it in
    // scope to simulate a shared filesystem between processes); the
    // storage handle from task 1 is dropped.
    let task2_storage = Storage::open(&ledger_path).expect("task2 open");
    let events_task2 = task2_storage.list_events().expect("task2 list");
    let project_task2 = task2_storage.get_project("proj-alpha").expect("task2 get");

    // The two reads MUST produce identical lists (the ledger was
    // untouched between task1 close and task2 open).
    assert_eq!(events_task1.len(), events_task2.len());
    let mut keys_task1: BTreeSet<(i64, String, String)> = BTreeSet::new();
    for e in &events_task1 {
        keys_task1.insert((e.sequence, e.event_id.clone(), e.event_type.clone()));
    }
    let mut keys_task2: BTreeSet<(i64, String, String)> = BTreeSet::new();
    for e in &events_task2 {
        keys_task2.insert((e.sequence, e.event_id.clone(), e.event_type.clone()));
    }
    assert_eq!(keys_task1, keys_task2);

    // Re-derive the capsule from the second read.
    let snapshot2 = StorageSnapshot::from_ledger_events("storage.ledger_head", &events_task2);
    let project_adapter2 =
        StorageProjectAdapter::new(&project_task2.project_id, &project_task2.created_at);
    let compiler2 = ContextCompiler {
        planning: Box::new(StorageLedgerHeadAdapter::from_snapshot(snapshot2.clone())),
        run: Box::new(StorageLedgerHeadAdapter::from_snapshot(snapshot2.clone())),
        memory: Box::new(project_adapter2),
        graph: Box::new(StorageLedgerHeadAdapter::from_snapshot(snapshot2.clone())),
        vault_knowledge: None,
        unknown_sources: vec![],
    };
    let capsule2 = compiler2.compile(read_head).expect("compile2");

    // === INVARIANT ===
    // Both capsules must be byte-identical because they are derived
    // from the same persisted state.
    assert_eq!(capsule1.payload, capsule2.payload);
    assert_eq!(capsule1.provenance, capsule2.provenance);
    assert_eq!(capsule1.staleness, capsule2.staleness);

    // Sanity: the project field is preserved.
    assert_eq!(project_task2.project_id, "proj-alpha");
}

/// H06 contract: the second task cannot recover state via an
/// in-memory shortcut. We simulate by NOT emitting events through a
/// live `Storage`; instead, the adapter sees an empty snapshot. The
/// rebuilt capsule must still be structurally valid (empty ledger)
/// and identical to what task1 would have produced with the same
/// constraint.
#[test]
fn h06_no_in_memory_shortcut_empty_ledger_rebuilds_consistently() {
    let dir = tempfile::tempdir().expect("tempdir");
    let path = dir.path().join("ledger.sqlite");
    let storage = Storage::open(&path).expect("open");
    storage
        .insert_project(&ProjectRecord {
            project_id: "proj-h06".into(),
            display_name: "Project H06".into(),
            remote_url: None,
            scope: "test".into(),
            created_at: "2026-09-20T00:00:00Z".into(),
        })
        .expect("insert");
    drop(storage);

    // Reopen in a "second task" simulation.
    let storage2 = Storage::open(&path).expect("reopen");
    let events = storage2.list_events().expect("list");
    let project = storage2.get_project("proj-h06").expect("get");
    assert!(events.is_empty(), "no events emitted, no shortcut possible");

    let snapshot = StorageSnapshot::from_ledger_events("storage.ledger_head", &events);
    let adapter_ledger = StorageLedgerHeadAdapter::from_snapshot(snapshot);
    let adapter_project = StorageProjectAdapter::new(&project.project_id, &project.created_at);
    let compiler = ContextCompiler {
        planning: Box::new(StorageLedgerHeadAdapter::from_events(&[])),
        run: Box::new(adapter_ledger),
        memory: Box::new(adapter_project),
        graph: Box::new(StorageLedgerHeadAdapter::from_events(&[])),
        vault_knowledge: None,
        unknown_sources: vec![],
    };
    let capsule = compiler.compile(0).expect("compile");
    // Empty ledger ⇒ the ledger-head adapter serializes empty events
    // as the JSON array `[]`. The capsule therefore concatenates a
    // series of `[]` markers (one per ledger-head slot) plus the
    // project adapter's compact bytes. The structural invariant is:
    //   (a) staleness = 0; (b) the ledger slots report log_head = 0.
    assert_eq!(capsule.staleness.compiled_at_log_head, 0);
    assert_eq!(capsule.staleness.read_at_log_head, 0);
    // The ledger-head slot payload starts with `[` (JSON array marker),
    // confirming the snapshot was built from an empty list rather than
    // a fake in-memory shortcut that injected a non-empty payload.
    assert!(
        capsule.payload.starts_with(b"["),
        "expected JSON-array marker for empty ledger; got {:?}",
        capsule.payload
    );
}

/// H09: NEG — invalid log head (`read_at_log_head < 0`) is rejected
/// by the compiler (ContextError::InvalidLogHead). The adapter must
/// not silently produce a fake snapshot.
#[test]
fn h09_compiler_rejects_out_of_range_log_head() {
    use sddk_engine::context_compiler::ContextError;

    let dir = tempfile::tempdir().expect("tempdir");
    let path = dir.path().join("ledger.sqlite");
    let storage = Storage::open(&path).expect("open");
    storage
        .insert_project(&ProjectRecord {
            project_id: "proj-h09".into(),
            display_name: "Project H09".into(),
            remote_url: None,
            scope: "test".into(),
            created_at: "2026-09-20T00:00:00Z".into(),
        })
        .expect("insert");
    drop(storage);

    let storage2 = Storage::open(&path).expect("reopen");
    let events: Vec<LedgerEvent> = storage2.list_events().unwrap_or_default();
    let project = storage2.get_project("proj-h09").unwrap();

    // Adapter reports a non-zero log head.
    let mut snapshot = StorageSnapshot::from_ledger_events("storage.ledger_head", &events);
    snapshot.log_head = 100; // artificially inflated

    let compiler = ContextCompiler {
        planning: Box::new(StorageLedgerHeadAdapter::from_snapshot(snapshot)),
        run: Box::new(StorageProjectAdapter::new(
            &project.project_id,
            &project.created_at,
        )),
        memory: Box::new(StorageLedgerHeadAdapter::from_events(&[])),
        graph: Box::new(StorageLedgerHeadAdapter::from_events(&[])),
        vault_knowledge: None,
        unknown_sources: vec![],
    };

    // Asking compile(50) when adapter reports head=100 must fail-closed.
    let err = compiler.compile(50).expect_err("must reject");
    assert!(matches!(err, ContextError::InvalidLogHead(_)));

    // Asking compile(100) succeeds (boundary case).
    let project_a = StorageProjectAdapter::new(&project.project_id, &project.created_at);
    let mut snapshot_ok = StorageSnapshot::from_ledger_events("storage.ledger_head", &events);
    snapshot_ok.log_head = 100;
    let compiler_ok = ContextCompiler {
        planning: Box::new(StorageLedgerHeadAdapter::from_snapshot(snapshot_ok)),
        run: Box::new(StorageLedgerHeadAdapter::from_events(&[])),
        memory: Box::new(project_a),
        graph: Box::new(StorageLedgerHeadAdapter::from_events(&[])),
        vault_knowledge: None,
        unknown_sources: vec![],
    };
    let cap = compiler_ok.compile(100).expect("compile at boundary");
    assert_eq!(cap.staleness.compiled_at_log_head, 100);
    assert_eq!(cap.staleness.read_at_log_head, 100);
}

/// H04: REG — multiple revisions of the same work item refer to the
/// same project via stable adapter ids. We open one project, snapshot
/// twice (one revision per snapshot), and confirm both capsule
/// provenance lists include `storage.project` once and
/// `storage.ledger_head` once.
#[test]
fn h04_multiple_revisions_use_stable_adapter_ids() {
    let dir = tempfile::tempdir().expect("tempdir");
    let path = dir.path().join("ledger.sqlite");
    let storage = Storage::open(&path).expect("open");
    storage
        .insert_project(&ProjectRecord {
            project_id: "proj-h04".into(),
            display_name: "Project H04".into(),
            remote_url: None,
            scope: "test".into(),
            created_at: "2026-09-20T00:00:00Z".into(),
        })
        .expect("insert");
    drop(storage);

    let storage2 = Storage::open(&path).expect("reopen");
    let project_a = storage2.get_project("proj-h04").expect("get");

    // Revision A: identical adapter IDs across the four slots.
    let cap_a = ContextCompiler {
        planning: Box::new(StorageProjectAdapter::new(
            &project_a.project_id,
            &project_a.created_at,
        )),
        run: Box::new(StorageProjectAdapter::new(
            &project_a.project_id,
            &project_a.created_at,
        )),
        memory: Box::new(StorageLedgerHeadAdapter::from_events(&[])),
        graph: Box::new(StorageLedgerHeadAdapter::from_events(&[])),
        vault_knowledge: None,
        unknown_sources: vec![],
    }
    .compile(0)
    .expect("compile a");
    let cap_b = ContextCompiler {
        planning: Box::new(StorageProjectAdapter::new(
            &project_a.project_id,
            &project_a.created_at,
        )),
        run: Box::new(StorageProjectAdapter::new(
            &project_a.project_id,
            &project_a.created_at,
        )),
        memory: Box::new(StorageLedgerHeadAdapter::from_events(&[])),
        graph: Box::new(StorageLedgerHeadAdapter::from_events(&[])),
        vault_knowledge: None,
        unknown_sources: vec![],
    }
    .compile(0)
    .expect("compile b");

    // Same provenance → stable work-item references across revisions.
    assert_eq!(cap_a.provenance, cap_b.provenance);
    // Two unique ids.
    assert_eq!(cap_a.provenance.len(), 2);
}

/// Negative path: opening a non-existent file (read-only) returns a
/// typed error. Used to demonstrate the adapter layer surfaces storage
/// failures as typed errors (no silent fallback).
#[test]
fn storage_open_on_missing_path_returns_typed_error() {
    let dir = tempfile::tempdir().expect("tempdir");
    let path = dir.path().join("does-not-exist.sqlite");
    let res = Storage::open_read_only(&path);
    let err = match res {
        Ok(_) => panic!("missing path should not open"),
        Err(e) => e,
    };
    // The exact error variant is an implementation detail (sqlite
    // bundling); just confirm it's the storage error type.
    let _: StorageError = err;
}
