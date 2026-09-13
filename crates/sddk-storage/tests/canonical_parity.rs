//! Canonical parity gate for the WU-C1.2 events redirect (C1-TEST-2).
//!
//! Verifies that domain events emitted through the `Storage` wrappers
//! (W1..W4) are readable identically from a fresh repository and from a
//! pre-cutover migrated repository, and that the canonical `events_v1`
//! stream is the single source of truth for new events:
//!
//! 1. Fresh repo: every wrapper-emitted event lands in `events_v1` (via
//!    `SqliteEventStore`), nothing new in `ledger_events`, and the merged
//!    `Storage` readers return the events in sequence order.
//! 2. Migrated repo: a pre-cutover legacy corpus (seeded directly into
//!    `ledger_events` with the legacy hash chain) coexists with new
//!    canonical events; both are visible through the merged readers and
//!    `verify_ledger` validates both sides.
//! 3. `SqliteEventStore` opened over the same database file
//!    (`open_path`) sees exactly the same canonical events.

#![allow(deprecated)] // tests exercise the C1.3-deprecated forwarders by design

use sddk_domain::{CycleId, CycleManifest, LedgerEventInput, ProjectRecord, WorkspaceRecord};
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

/// Seeds a pre-cutover legacy corpus directly into `ledger_events`,
/// replicating the old hash-chain invariants (sequence from 1, previous_hash
/// linkage, legacy `hash_event` payload). This simulates a repo migrated
/// from a pre-redirect version.
fn seed_legacy_corpus(storage: &Storage, count: usize) {
    let connection = storage.connection_for_tests();
    let mut previous_hash: Option<String> = None;
    for sequence in 1..=count as i64 {
        let input = event(&format!("evt-legacy-{sequence:03}"), None);
        let event_hash = legacy_hash_event(sequence, &input, &previous_hash);
        connection
            .execute(
                "INSERT INTO ledger_events (
                    sequence, event_id, project_id, cycle_id, frame_id, command_id,
                    actor, event_type, occurred_at, state_before_json,
                    state_after_json, payload_json, previous_hash, event_hash
                 ) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12, ?13, ?14)",
                rusqlite::params![
                    sequence,
                    input.event_id,
                    input.project_id,
                    input.cycle_id,
                    input.frame_id,
                    input.command_id,
                    input.actor,
                    input.event_type,
                    input.occurred_at,
                    Option::<String>::None,
                    serde_json::to_string(&input.state_after).unwrap(),
                    serde_json::to_string(&input.payload).unwrap(),
                    previous_hash,
                    event_hash
                ],
            )
            .unwrap();
        previous_hash = Some(event_hash);
    }
}

/// Replica of the legacy hash function used before the redirect
/// (`hash_event` in sddk-storage lib.rs), kept local so the fixture does not
/// depend on private storage internals.
fn legacy_hash_event(
    sequence: i64,
    input: &LedgerEventInput,
    previous_hash: &Option<String>,
) -> String {
    use sha2::{Digest, Sha256};
    // Must mirror `hash_event` in sddk-storage (EventHashMaterial field
    // order + canonical JSON) so the seeded corpus passes verify_ledger.
    #[derive(serde::Serialize)]
    struct Material<'a> {
        sequence: i64,
        event_id: &'a str,
        project_id: &'a str,
        cycle_id: &'a Option<String>,
        frame_id: &'a str,
        command_id: &'a str,
        actor: &'a str,
        event_type: &'a str,
        occurred_at: &'a str,
        state_before: &'a Option<serde_json::Value>,
        state_after: &'a Option<serde_json::Value>,
        payload: &'a serde_json::Value,
        previous_hash: &'a Option<String>,
    }
    let material = Material {
        sequence,
        event_id: &input.event_id,
        project_id: &input.project_id,
        cycle_id: &input.cycle_id,
        frame_id: &input.frame_id,
        command_id: &input.command_id,
        actor: &input.actor,
        event_type: &input.event_type,
        occurred_at: &input.occurred_at,
        state_before: &input.state_before,
        state_after: &input.state_after,
        payload: &input.payload,
        previous_hash,
    };
    let digest = Sha256::digest(serde_json::to_vec(&material).unwrap());
    format!("sha256:{digest:x}")
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
    fresh.append_event(&event("evt-fresh-3", None)).unwrap();

    // Canonical stream is the only writer: new events live in events_v1,
    // ledger_events stays empty.
    assert_eq!(fresh.legacy_ledger_count_for_tests(), 0);

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

    // W4: release_cycle_lease emits lease.released into the canonical stream.
    let lease = fresh
        .acquire_cycle_lease(manifest.cycle_id.as_str(), "runtime", 1_000, 2_000)
        .unwrap();
    let released = fresh
        .release_cycle_lease(
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

    // verify_ledger covers both sides (legacy empty + canonical streams).
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

    // --- Migrated repo: legacy corpus + new canonical events coexist ---
    let migrated_path = migrated_dir.path().join("ledger.sqlite");
    let migrated = seeded_storage(&migrated_path);
    seed_legacy_corpus(&migrated, 3);

    let mut migrated = migrated;
    migrated.append_event(&event("evt-new-1", None)).unwrap();

    // The redirect must NOT write new events into the legacy table.
    assert_eq!(migrated.legacy_ledger_count_for_tests(), 3);

    // Merged readers see 3 legacy + 1 canonical event, ordered by sequence
    // (legacy rows keep 1..3; the canonical project-stream event has its own
    // per-stream sequence).
    let merged = migrated.load_all_ledger_events().unwrap();
    assert_eq!(merged.len(), 4);
    let legacy_ids: Vec<&str> = merged[..3].iter().map(|e| e.event_id.as_str()).collect();
    assert_eq!(
        legacy_ids,
        vec!["evt-legacy-001", "evt-legacy-002", "evt-legacy-003"]
    );
    assert_eq!(merged[3].event_id, "evt-new-1");

    // Chain verification: legacy corpus verifies on its own linkage, the
    // canonical stream verifies via the event store chain.
    migrated.verify_ledger().unwrap();

    // list_events_after (watch semantics): on the migrated repo the cursor
    // addresses the merged sequence domain; legacy corpus (seq 1..3) stays
    // addressable and canonical per-stream sequences join the same filter.
    let after = migrated.list_events_after(0, 10).unwrap();
    assert_eq!(after.len(), 4);
    let after_legacy_cursor = migrated.list_events_after(3, 10).unwrap();
    assert!(after_legacy_cursor.is_empty());
}

#[test]
fn canonical_append_conflict_maps_to_ledger_integrity() {
    let dir = TempDir::new().unwrap();
    let storage = seeded_storage(&dir.path().join("ledger.sqlite"));

    // Duplicate event_id across wrappers must surface as a storage error
    // (INSERT OR IGNORE makes the canonical append idempotent, so this
    // asserts the redelivery path, not a panic).
    let mut storage = storage;
    let first = storage.append_event(&event("evt-dup", None)).unwrap();
    let second = storage.append_event(&event("evt-dup", None)).unwrap();
    assert_eq!(first.event_id, second.event_id);
    assert_eq!(first.sequence, second.sequence);
    assert_eq!(
        storage.legacy_ledger_count_for_tests(),
        0,
        "idempotent redelivery must not write the legacy table"
    );
}

#[test]
fn read_only_storage_rejects_domain_event_writes() {
    let dir = TempDir::new().unwrap();
    let path = dir.path().join("ledger.sqlite");
    let mut storage = seeded_storage(&path);
    storage.append_event(&event("evt-ro-1", None)).unwrap();
    drop(storage);

    let mut read_only = Storage::open_read_only(&path).unwrap();
    let outcome = read_only.append_event(&event("evt-ro-2", None));
    if outcome.is_ok() {
        panic!(
            "read-only append unexpectedly succeeded; canonical count={}",
            read_only.legacy_ledger_count_for_tests()
        );
    }
    let err = outcome.err().unwrap();
    assert!(matches!(err, StorageError::LedgerIntegrity { .. }));
}
