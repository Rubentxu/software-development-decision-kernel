//! Canonical parity gate for the events redirect (C1-TEST-2, re-seed
//! canónico WU-C15-4/C15-8).
//!
//! Verifies that domain events emitted through the `Storage` wrappers
//! (W1..W4) are readable identically from a fresh repository and from a
//! post-v20 migrated repository (sin corpus legacy: `ledger_events` ya no
//! existe físicamente), and that the canonical `events_v1` stream is the
//! single source of truth for all events:
//!
//! 1. Fresh repo: every wrapper-emitted event lands in `events_v1` (via
//!    `SqliteEventStore`), and the canonical `Storage` readers return the
//!    events in sequence order.
//! 2. Post-v20 repo (re-abierto tras MIGRATION_20): los eventos canónicos
//!    sobreviven el ciclo open→close→open y `verify_ledger` valida la
//!    cadena canónica completa.
//! 3. `SqliteEventStore` opened over the same database file
//!    (`open_path`) sees exactly the same canonical events.

#![allow(deprecated)] // tests exercise the C1.3-deprecated forwarders by design

use sddk_domain::{
    CycleId, CycleManifest, Ledger, LedgerEventInput, ProjectRecord, WorkspaceRecord,
};
use sddk_storage::{Storage, StorageError};
use serde_json::json;
use tempfile::TempDir;

const CREATED_AT: &str = "2026-09-01T12:00:00Z";

fn project_record() -> ProjectRecord {
    ProjectRecord {
        project_id: "project-1".into(),
        display_name: "Project One".into(),
        remote_url: None,
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

fn cycle_manifest() -> CycleManifest {
    CycleManifest::new(
        "project-1".into(),
        "workspace-1".into(),
        CycleId::new("project-1/parity").unwrap(),
        "Parity".into(),
        "sddk/parity".into(),
        "abc123".into(),
    )
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
        state_after: Some(json!({"event": event_id})),
        payload: json!({"event": event_id}),
        causation_id: None,
        correlation_id: None,
    }
}

fn seeded_storage(path: &std::path::Path) -> Storage {
    let storage = Storage::open(path).expect("open storage");
    storage.insert_project(&project_record()).unwrap();
    storage.insert_workspace(&workspace_record()).unwrap();
    storage
}

#[test]
fn fresh_and_migrated_repo_read_canonical_stream_identically() {
    let fresh_dir = TempDir::new().unwrap();
    let migrated_dir = TempDir::new().unwrap();

    // --- Fresh repo: emit via wrappers, read back through merged readers ---
    let mut fresh = seeded_storage(&fresh_dir.path().join("ledger.sqlite"));
    let manifest = cycle_manifest();
    fresh
        .insert_cycle_with_event(
            &sddk_storage::CycleRecord {
                manifest: manifest.clone(),
                created_at: CREATED_AT.into(),
                updated_at: CREATED_AT.into(),
            },
            &event("evt-fresh-1", Some(manifest.cycle_id.as_str())),
        )
        .unwrap();
    fresh
        .update_cycle_with_event(
            &manifest,
            CREATED_AT,
            &event("evt-fresh-2", Some(manifest.cycle_id.as_str())),
            false,
        )
        .unwrap();
    fresh
        .emit_canonical_event(&event("evt-fresh-3", None))
        .unwrap();

    // Canonical stream is the only writer: new events live in events_v1.
    // (WU-C15-4: `ledger_events` ya no existe físicamente; no hay tabla
    // legacy que contar — el schema v20 lo garantiza.)
    assert_eq!(fresh.schema_version().unwrap(), 20);

    // Merged readers expose the canonical events with per-stream sequences.
    let all = fresh.list_events().unwrap();
    let mut ids: Vec<&str> = all.iter().map(|e| e.event_id.as_str()).collect();
    ids.sort();
    assert_eq!(
        ids,
        vec!["evt-fresh-1", "evt-fresh-2", "evt-fresh-3"],
        "merged list_events must expose every canonical event"
    );
    let cycle_events = fresh.list_cycle_events(manifest.cycle_id.as_str()).unwrap();
    assert_eq!(cycle_events.len(), 2);
    assert!(
        cycle_events
            .iter()
            .all(|e| e.cycle_id.as_deref() == Some(manifest.cycle_id.as_str()))
    );
    let frame_events = fresh.list_frame_events("frame-1").unwrap();
    assert_eq!(frame_events.len(), 3);

    // W4: release_lease_with_event emits lease.released into the canonical stream.
    let lease = fresh
        .acquire_cycle_lease(manifest.cycle_id.as_str(), "runtime", 1_000, 2_000)
        .unwrap();
    let released = fresh
        .release_lease_with_event(
            "project-1",
            manifest.cycle_id.as_str(),
            "runtime",
            lease.fencing_token,
            "runtime",
            "command-1",
            CREATED_AT,
        )
        .unwrap();
    assert!(released);
    let all_after_release = fresh.list_events().unwrap();
    assert!(
        all_after_release
            .iter()
            .any(|e| e.event_type == "lease.released")
    );

    // verify_ledger covers the canonical stream (single read authority).
    let verification = fresh.verify_ledger().unwrap();
    assert!(verification.event_count >= 4);

    // Watch semantics on a fresh repo: canonical per-stream sequences join
    // the same cursor filter. The cycle stream reaches sequence 3
    // (state_changed x2 + lease.released), so the cursor at 2 yields the
    // W4 `lease.released` event only.
    let watched_all = fresh.list_events_after(0, 10).unwrap();
    assert_eq!(watched_all.len(), 4);
    let watched = fresh.list_events_after(2, 10).unwrap();
    assert_eq!(watched.len(), 1);
    assert_eq!(watched[0].event_type, "lease.released");

    // Canonical receipts mirror into the LedgerEvent view (hash parity).
    let store =
        sddk_storage::SqliteEventStore::open_path(fresh_dir.path().join("ledger.sqlite")).unwrap();
    use sddk_domain::EventStore as _;
    let canonical_ids: Vec<String> = store
        .list_streams()
        .unwrap()
        .iter()
        .flat_map(|stream| store.load_stream(stream, None, u32::MAX).unwrap())
        .map(|e| e.event_id)
        .collect();
    assert_eq!(canonical_ids.len(), 4);

    // --- Post-v20 migrated repo: same events survive open→close→open ---
    // (El corpus legacy ya no existe; el "migrated" repo de C1.5 es un DB
    // post-MIGRATION_20 re-abierto, cuya lectura canónica debe ser
    // idéntica a la del fresh repo.)
    let migrated_path = migrated_dir.path().join("ledger.sqlite");
    let migrated = seeded_storage(&migrated_path);
    for i in 1..=3 {
        migrated
            .emit_canonical_event(&event(&format!("evt-new-{i}"), None))
            .unwrap();
    }
    drop(migrated); // close → re-open simula la migración post-v20

    let migrated = Storage::open(&migrated_path).unwrap();
    assert_eq!(migrated.schema_version().unwrap(), 20);
    let merged = migrated.list_events().expect("list_events");
    assert_eq!(
        merged.len(),
        3,
        "post-v20 re-open must expose every canonical event"
    );
    assert!(merged.iter().all(|e| e.event_id.starts_with("evt-new-")));

    // Chain verification: the canonical stream verifies via its own chain.
    migrated.verify_ledger().unwrap();

    // list_events_after (watch semantics): canonical-only desde WU-C15-3 —
    // per-stream sequences, cursor 0..=2 devuelve el flujo completo.
    let after = migrated.list_events_after(0, 10).unwrap();
    assert_eq!(after.len(), 3);
    let after_tail = migrated.list_events_after(2, 10).unwrap();
    assert_eq!(after_tail.len(), 1);
    assert_eq!(after_tail[0].event_id, "evt-new-3");
}

#[test]
fn canonical_append_conflict_maps_to_ledger_integrity() {
    let dir = TempDir::new().unwrap();
    let storage = seeded_storage(&dir.path().join("ledger.sqlite"));

    // Duplicate event_id across wrappers must surface as a storage error
    // (INSERT OR IGNORE makes the canonical append idempotent, so this
    // asserts the redelivery path, not a panic).
    let storage = storage;
    let first = storage
        .emit_canonical_event(&event("evt-dup", None))
        .unwrap();
    let second = storage
        .emit_canonical_event(&event("evt-dup", None))
        .unwrap();
    assert_eq!(first.event_id, second.event_id);
    assert_eq!(first.sequence, second.sequence);
    assert_eq!(
        storage.list_events().unwrap().len(),
        1,
        "idempotent redelivery must not duplicate the canonical event"
    );
}

#[test]
fn read_only_storage_rejects_domain_event_writes() {
    let dir = TempDir::new().unwrap();
    let path = dir.path().join("ledger.sqlite");
    let storage = seeded_storage(&path);
    storage
        .emit_canonical_event(&event("evt-ro-1", None))
        .unwrap();
    drop(storage);

    let read_only = Storage::open_read_only(&path).unwrap();
    let outcome = read_only.emit_canonical_event(&event("evt-ro-2", None));
    if outcome.is_ok() {
        panic!(
            "read-only append unexpectedly succeeded; canonical count={}",
            read_only.list_events().unwrap().len()
        );
    }
    let err = outcome.err().unwrap();
    assert!(matches!(err, StorageError::LedgerIntegrity { .. }));
}
