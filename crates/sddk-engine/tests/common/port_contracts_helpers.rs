//! Shared builder helpers for [`port_contracts`](super::port_contracts).
//!
//! These are `pub(crate)` so they are accessible from the integration tests
//! in the parent directory but not exposed outside the test crate.

use sddk_domain::{ActorKind, ActorRef, EntityRef, LedgerEventInput};
use sddk_domain::{
    ControlPlane, CycleManifest, CycleRecord, CycleStatus, Ledger, LedgerFactory, Phase,
};
use sddk_domain::{CyclePath, ProjectRecord, WorkspaceRecord};
use sddk_storage::{SqliteControlPlane, SqliteLedgerFactory, Storage};
use sddk_testkit::InMemoryLedger;
use serde_json::json;
use tempfile::TempDir;

/// Build a [`ProjectRecord`] for contract tests.
pub(crate) fn mk_project() -> ProjectRecord {
    ProjectRecord {
        project_id: "p-contract".into(),
        display_name: "Test".into(),
        remote_url: Some("https://example.com".into()),
        scope: "test".into(),
        created_at: TS.into(),
    }
}

/// Build a [`WorkspaceRecord`] for contract tests.
pub(crate) fn mk_workspace() -> WorkspaceRecord {
    WorkspaceRecord {
        workspace_id: "ws-contract".into(),
        project_id: "p-contract".into(),
        canonical_path: "/test".into(),
        created_at: TS.into(),
    }
}

/// Build a [`CycleRecord`] for contract tests.
pub(crate) fn mk_cycle(id: &str) -> CycleRecord {
    CycleRecord {
        manifest: CycleManifest {
            schema_version: 1,
            project_id: "p-contract".into(),
            workspace_id: "ws-contract".into(),
            cycle_id: id.into(),
            display_name: id.into(),
            status: CycleStatus::Open,
            phase: Phase::Explore,
            path: CyclePath::AFull,
            branch: "main".into(),
            base: "a".into(),
            head: None,
            artifacts: Default::default(),
            release: None,
            delivery_kind: None,
            remediation_round: 0,
            remote_url: Some("https://example.com/test".into()),
            scope: Some("test".into()),
        },
        created_at: TS.into(),
        updated_at: TS.into(),
    }
}

/// Build a [`LedgerEventInput`] for contract tests.
pub(crate) fn mk_event(cycle_id: &str) -> LedgerEventInput {
    LedgerEventInput {
        event_id: format!("evt-{cycle_id}"),
        project_id: "p-contract".into(),
        cycle_id: Some(cycle_id.into()),
        frame_id: "frame-contract".into(),
        command_id: "cmd-contract".into(),
        actor: "test-runtime".into(),
        event_type: "cycle.created".into(),
        occurred_at: TS.into(),
        state_before: None,
        state_after: None,
        payload: json!({}),
    }
}

/// Timestamp constant used in test fixtures.
pub(crate) const TS: &str = "2026-08-22T00:00:00Z";

/// Create and register a SqliteLedger with project+workspace.
/// Returns the ledger (Storage type) that implements Ledger.
pub(crate) fn mk_registered_sqlite_ledger() -> Storage {
    let factory = SqliteLedgerFactory;
    let mut ledger = factory.open_in_memory().unwrap();
    ledger
        .register_project_workspace(&mk_project(), &mk_workspace())
        .unwrap();
    ledger
}

/// Create and register an InMemoryLedger with project+workspace.
pub(crate) fn mk_registered_mem_ledger() -> InMemoryLedger {
    let mut ledger = InMemoryLedger::new();
    ledger
        .register_project_workspace(&mk_project(), &mk_workspace())
        .unwrap();
    ledger
}

/// Create and register both SqliteLedger and InMemoryLedger for byte-equiv tests.
pub(crate) fn mk_both_ledgers() -> (InMemoryLedger, Storage) {
    let mem = mk_registered_mem_ledger();
    let sqlite = mk_registered_sqlite_ledger();
    (mem, sqlite)
}

/// Create a SqliteControlPlane with the test project already upserted.
pub(crate) fn mk_control_plane_with_project() -> (SqliteControlPlane, TempDir) {
    let dir = TempDir::new().unwrap();
    let mut cp = SqliteControlPlane::open(dir.path()).unwrap();
    cp.upsert_project(
        "p-contract",
        "Test Contract",
        "test",
        Some("https://example.com"),
        TS,
    )
    .unwrap();
    (cp, dir)
}
