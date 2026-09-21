//! AIW-S8 X07 — Second-binary (read-only) integration.
//!
//! Demonstrates that a second consumer process (modeled as a second
//! `Storage` handle opened read-only over the same database file) can
//! consume the operational data written by a first writer process
//! (full `Storage`) without disturbing it:
//!
//! 1. Writer writes a project + workspace + cycle + canonical event;
//!    a reader opened `open_read_only` sees exactly that data.
//! 2. A reader cannot write through its handle (SQLite READ_ONLY flag).
//! 3. Reader schema version matches the writer's.
//! 4. Reader handles an empty ledger gracefully (no panic, empty vec).

use sddk_domain::{
    CycleId, CycleManifest, Ledger, LedgerEventInput, ProjectRecord, WorkspaceRecord,
};
use sddk_storage::{CycleRecord, Storage};
use serde_json::json;
use tempfile::TempDir;

const CREATED_AT: &str = "2026-09-01T12:00:00Z";

fn project_record(id: &str) -> ProjectRecord {
    ProjectRecord {
        project_id: id.into(),
        display_name: "X07 Writer Project".into(),
        remote_url: None,
        scope: "owner".into(),
        created_at: CREATED_AT.into(),
    }
}

fn workspace_record(project_id: &str) -> WorkspaceRecord {
    WorkspaceRecord {
        workspace_id: format!("{project_id}/ws"),
        project_id: project_id.into(),
        canonical_path: "/work/x07".into(),
        created_at: CREATED_AT.into(),
    }
}

fn cycle_manifest(project_id: &str) -> CycleManifest {
    CycleManifest::new(
        project_id.into(),
        format!("{project_id}/ws"),
        CycleId::new(format!("{project_id}/x07")).unwrap(),
        "X07 second binary".into(),
        "sddk/x07".into(),
        "def456".into(),
    )
}

fn event(project_id: &str, event_id: &str, cycle_id: Option<&str>) -> LedgerEventInput {
    LedgerEventInput {
        event_id: event_id.into(),
        project_id: project_id.into(),
        cycle_id: cycle_id.map(str::to_owned),
        frame_id: "frame-x07".into(),
        command_id: "command-x07".into(),
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

/// Writer "process": opens full storage and writes a project, workspace,
/// cycle and one canonical event. Returns the ledger path for the reader.
fn run_writer(dir: &TempDir, project_id: &str) -> std::path::PathBuf {
    let ledger = dir.path().join("ledger.sqlite");
    let mut writer = Storage::open(&ledger).expect("writer open");
    writer.insert_project(&project_record(project_id)).unwrap();
    writer
        .insert_workspace(&workspace_record(project_id))
        .unwrap();
    let manifest = cycle_manifest(project_id);
    writer
        .insert_cycle_with_event(
            &CycleRecord {
                manifest: manifest.clone(),
                created_at: CREATED_AT.into(),
                updated_at: CREATED_AT.into(),
            },
            &event(project_id, "evt-x07-1", Some(manifest.cycle_id.as_str())),
        )
        .unwrap();
    ledger
}

#[test]
fn writer_and_reader_share_one_storage_path() {
    let dir = TempDir::new().expect("tempdir");
    let project_id = "x07-share";
    let ledger = run_writer(&dir, project_id);

    // Second consumer process: same file, read-only.
    let reader = Storage::open_read_only(&ledger).expect("reader open");

    let project = reader
        .get_project(project_id)
        .expect("reader reads project");
    assert_eq!(project.project_id, project_id);
    assert_eq!(project.display_name, "X07 Writer Project");

    let events = reader.list_events().expect("reader lists events");
    assert_eq!(events.len(), 1);
    assert_eq!(events[0].event_id, "evt-x07-1");
    assert_eq!(events[0].project_id, project_id);

    let cycle = reader
        .get_cycle(&format!("{project_id}/x07"))
        .expect("reader reads cycle");
    assert_eq!(
        cycle.manifest.cycle_id.as_str(),
        format!("{project_id}/x07")
    );
}

#[test]
fn reader_cannot_write() {
    let dir = TempDir::new().expect("tempdir");
    let project_id = "x07-readonly";
    let ledger = run_writer(&dir, project_id);

    let reader = Storage::open_read_only(&ledger).expect("reader open");
    // Attempt to write through the reader handle: the SQLite READ_ONLY
    // connection must reject it.
    let attempt = reader.insert_project(&project_record("x07-injected"));
    assert!(
        attempt.is_err(),
        "read-only storage must reject insert_project"
    );
}

#[test]
fn reader_schema_matches_writer() {
    let dir = TempDir::new().expect("tempdir");
    let project_id = "x07-schema";
    let ledger = run_writer(&dir, project_id);

    let reader = Storage::open_read_only(&ledger).expect("reader open");
    // 20 = LATEST_SCHEMA_VERSION (post MIGRATION_20 canonical redirect).
    assert_eq!(reader.schema_version().expect("reader schema version"), 20);
}

#[test]
fn reader_handles_empty_ledger_gracefully() {
    let dir = TempDir::new().expect("tempdir");
    // Writer creates the database (schema applied) but writes no events.
    let ledger = dir.path().join("ledger.sqlite");
    let writer = Storage::open(&ledger).expect("writer open");
    let writer_events = writer.list_events().expect("writer empty ledger");
    assert!(writer_events.is_empty());
    drop(writer);

    let reader = Storage::open_read_only(&ledger).expect("reader open");
    let events = reader.list_events().expect("reader empty ledger");
    assert!(events.is_empty(), "empty ledger must list as empty vec");
}
