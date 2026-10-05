//! A cycle row cannot exist before the project and workspace its foreign key
//! names do.
//!
//! MEDIDO (session-84 bis 7, `sddk 2.11.3`, binario publicado `b37bcf5d`):
//! `sddk cycle start` against a fresh state home failed 3/3 with
//!
//! ```text
//! error[ENGINE_STORAGE]: storage error: database error: FOREIGN KEY constraint failed
//!   recovery: resolve the underlying storage error first
//! ```
//!
//! The FK has been declared since the initial import — `cycles` carries
//! `FOREIGN KEY (project_id, workspace_id) REFERENCES
//! workspaces(project_id, workspace_id)`, and `workspaces` references
//! `projects` — so this was never a regression. It was always the case that
//! adoption is what registers the pair, and `sddk cycle start --help`
//! declared no prerequisite. The defect is that the situation was reported as
//! a *storage* fault: the recovery line sent an operator to debug SQLite when
//! the fix is one command (`sddk adopt apply`).
//!
//! These tests pin three things, and the third is what stops the file from
//! being a tautology:
//!
//! 1. nothing registered → the error names the missing **project**;
//! 2. project registered at a different path → the error names the missing
//!    **workspace**, which is a different situation with the same first half
//!    of the story and the same fix;
//! 3. the pair registered → the same call succeeds.
//!
//! Without (3) a reader could "fix" the check by rejecting every insert and
//! every test above would still pass.

use sddk_domain::SddkErrorCode;
use sddk_domain::cycle::CycleManifest;
use sddk_domain::identity::CycleId;
use sddk_domain::{ProjectRecord, WorkspaceRecord};
use sddk_domain::Ledger;
use sddk_storage::{Storage, StorageError};
use tempfile::TempDir;

/// A fresh ledger with migrations applied and nothing registered — the exact
/// state a first-ever `sddk cycle start` meets.
fn cold_ledger() -> (TempDir, std::path::PathBuf) {
    let tmp = TempDir::new().expect("tempdir");
    let db = tmp.path().join("ledger.sqlite");
    let _ = Storage::open(&db).expect("fresh ledger opens and migrates");
    (tmp, db)
}

fn project() -> ProjectRecord {
    ProjectRecord {
        project_id: "p-test".to_string(),
        display_name: "test".to_string(),
        remote_url: Some("https://example.test/x".to_string()),
        scope: ".".to_string(),
        created_at: "2026-01-01".to_string(),
    }
}

fn workspace(id: &str) -> WorkspaceRecord {
    WorkspaceRecord {
        workspace_id: id.to_string(),
        project_id: "p-test".to_string(),
        canonical_path: format!("/tmp/{id}"),
        created_at: "2026-01-01".to_string(),
    }
}

fn cycle(cycle_id: &str, workspace_id: &str) -> sddk_domain::CycleRecord {
    let id = CycleId::new(cycle_id).expect("valid cycle id");
    let manifest = CycleManifest::new(
        "p-test".to_string(),
        workspace_id.to_string(),
        id,
        "a cycle".to_string(),
        "main".to_string(),
        "0".repeat(40),
    );
    sddk_domain::CycleRecord {
        manifest,
        created_at: "2026-01-01".to_string(),
        updated_at: "2026-01-01".to_string(),
    }
}

/// Nothing has ever been registered. The error must say the *project* is
/// absent, and — the point of the whole change — it must not be a generic
/// database fault.
#[test]
fn an_unadopted_project_is_named_as_such_and_not_as_a_database_fault() {
    let (_tmp, db) = cold_ledger();
    let storage = Storage::open(&db).expect("ledger reopens");

    let err = storage
        .insert_cycle(&cycle("p-test/c-1", "w-here"))
        .expect_err("a cycle cannot be stored before its project exists");

    assert!(
        !matches!(err, StorageError::Database(..)),
        "the FK is the guarantee, but reporting it as a raw SQLite fault is the \
         defect this file pins: an operator reading 'FOREIGN KEY constraint \
         failed' debugs the database, when the cause is that adoption never \
         ran; got: {err}"
    );
    match err {
        StorageError::ProjectNotAdopted {
            cycle_id,
            project_id,
            missing,
            ..
        } => {
            assert_eq!(missing, "project", "nothing at all was registered");
            assert_eq!(project_id, "p-test");
            assert_eq!(cycle_id, "p-test/c-1");
        }
        other => panic!("expected the typed adoption error, got: {other}"),
    }
}

/// A second clone of an already-adopted project. `workspace_id` is derived from
/// the canonical path, so a checkout anywhere else is a *different* workspace
/// under the *same* project. Reporting this as "the project is not adopted"
/// would be false — the project is, and the operator would go looking for
/// something that is already there.
#[test]
fn an_unadopted_workspace_of_an_adopted_project_is_named_separately() {
    let (_tmp, db) = cold_ledger();
    let mut storage = Storage::open(&db).expect("ledger reopens");
    storage
        .register_project_workspace(&project(), &workspace("w-elsewhere"))
        .expect("adoption registers the pair at one path");

    let err = storage
        .insert_cycle(&cycle("p-test/c-2", "w-here"))
        .expect_err("the workspace this cycle names is still absent");

    match err {
        StorageError::ProjectNotAdopted { missing, parent_id, .. } => {
            assert_eq!(
                missing, "workspace",
                "the project IS registered; only this path's workspace is not"
            );
            assert_eq!(parent_id, "w-here");
        }
        other => panic!("expected the typed adoption error, got: {other}"),
    }
}

/// The operator-facing half. `recovery()` is what the CLI prints under
/// `recovery:`; a correct error code with a useless recovery line would leave
/// the operator exactly where this defect found them.
#[test]
fn the_recovery_names_the_command_that_fixes_it() {
    let (_tmp, db) = cold_ledger();
    let storage = Storage::open(&db).expect("ledger reopens");

    let err = storage
        .insert_cycle(&cycle("p-test/c-3", "w-here"))
        .expect_err("still unadopted");

    assert_eq!(err.code(), "STORAGE_PROJECT_NOT_ADOPTED");
    let recovery = err.recovery();
    assert!(
        recovery.contains("sddk adopt apply"),
        "the recovery line is the whole point: it must name the one command \
         that makes the insert legal, not tell the reader to resolve an \
         underlying storage error; got: {recovery}"
    );
}

/// The anti-tautology control. Without this, a check that rejected *every*
/// insert would satisfy all three tests above.
#[test]
fn the_same_call_succeeds_once_the_pair_is_registered() {
    let (_tmp, db) = cold_ledger();
    let mut storage = Storage::open(&db).expect("ledger reopens");
    storage
        .register_project_workspace(&project(), &workspace("w-here"))
        .expect("adoption registers the pair");

    let record = cycle("p-test/c-4", "w-here");
    storage
        .insert_cycle(&record)
        .expect("a registered pair makes the cycle storable");

    let read_back = storage
        .get_cycle("p-test/c-4")
        .expect("and the row is really there");
    assert_eq!(read_back.manifest.workspace_id, "w-here");
}

// ── The path the operator actually takes ──────────────────────────────────
//
// MEDIDO (session-84 bis 7): `sddk cycle start` does NOT reach
// `insert_cycle`. It goes `Engine::apply_cycle_start` ->
// `Ledger::insert_cycle_with_event` -> `insert_cycle_on`. The four tests
// above pin the *other* public method.
//
// That gap was found by falsifying, not by reading: the first falsification
// attempt removed the check from `insert_cycle_with_event` and the suite
// still reported 4/4 green. A mutation that does not apply to the code under
// test is not a passing guard, it is a silent one — and it reported PASS.
//
// So the defect's own path gets its own pin, or the suite would be green for
// a defect that is still there.

/// The event that accompanies a cycle creation. Built through the same
/// [`LedgerEventInput`] the engine builds, so the scope check is satisfied by
/// construction rather than by a hand-tuned fixture.
fn creation_event(cycle_id: &str, project_id: &str) -> sddk_domain::LedgerEventInput {
    sddk_domain::LedgerEventInput {
        event_id: "evt-1".to_string(),
        project_id: project_id.to_string(),
        cycle_id: Some(cycle_id.to_string()),
        frame_id: "frame-1".to_string(),
        command_id: "cycle.start-1".to_string(),
        actor: "system".to_string(),
        actor_ref: None,
        event_type: "cycle.created".to_string(),
        occurred_at: "2026-01-01T00:00:00Z".to_string(),
        state_before: None,
        state_after: None,
        payload: serde_json::json!({"transition_id": "cycle.start"}),
        causation_id: None,
        correlation_id: None,
    }
}

/// The defect, on the path `sddk cycle start` takes. The `Ledger` port
/// surfaces the DOMAIN error type, so that is what the assertions name.
#[test]
fn the_cycle_start_path_reports_adoption_instead_of_a_foreign_key() {
    let (_tmp, db) = cold_ledger();
    let mut storage = Storage::open(&db).expect("ledger reopens");

    let err = storage
        .insert_cycle_with_event(
            &cycle("p-test/c-5", "w-here"),
            &creation_event("p-test/c-5", "p-test"),
        )
        .expect_err("the pair is absent, so the cycle cannot be stored");

    assert!(
        !matches!(err, sddk_domain::StorageError::Database(..)),
        "on the path `sddk cycle start` takes, the old behaviour was a raw \
         'FOREIGN KEY constraint failed'; got: {err}"
    );
    assert_eq!(
        err.code(),
        "STORAGE_PROJECT_NOT_ADOPTED",
        "the code the CLI prints as error[STORAGE_PROJECT_NOT_ADOPTED] is the \
         one an operator can act on; got {}",
        err.code()
    );
}

/// The same control on the same path: registered pair → the cycle AND its
/// causal event commit. Without it, the test above could be satisfied by a
/// check that refuses every write.
#[test]
fn the_cycle_start_path_succeeds_once_adopted() {
    let (_tmp, db) = cold_ledger();
    let mut storage = Storage::open(&db).expect("ledger reopens");
    storage
        .register_project_workspace(&project(), &workspace("w-here"))
        .expect("adoption registers the pair");

    let event = storage
        .insert_cycle_with_event(
            &cycle("p-test/c-6", "w-here"),
            &creation_event("p-test/c-6", "p-test"),
        )
        .expect("an adopted project can hold its first cycle");

    assert_eq!(event.event_type, "cycle.created");
    storage
        .get_cycle("p-test/c-6")
        .expect("the cycle row is durable");
}

