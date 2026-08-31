use std::fs;
use std::path::{Path, PathBuf};

use sddk_engine::{
    AdoptionError, AdoptionPlan, AdoptionPlanInput, AdoptionStatusKind, XdgEnvironment,
    adoption_status, apply_adoption, plan_adoption, read_adoption_receipt, refresh_adoption,
    repair_adoption,
};
use sddk_storage::{ProjectRecord, Storage, WorkspaceRecord};
use tempfile::TempDir;

// Tests that reopen the same ledger path or depend on cross-call persistence
// MUST remain on concrete Storage::open(&path). These are marked:
// durability-required: <reason>
//
// Migratable tests use Storage::open_in_memory() via Fixture::storage.

const TIMESTAMP: &str = "2026-08-04T10:00:00Z";
const SEED: &str = "a0b1c2d3-e4f5-4678-9abc-def012345678";

#[test]
fn plan_is_write_free_and_reports_identity_paths_and_hash() {
    // Migrated: does not call apply_adoption/adoption_status/repair_adoption,
    // only calls plan_adoption which has no storage side-effects.
    let fixture = Fixture::new_in_memory();
    let plan = fixture.remote_plan("checkout", "https://example.com/acme/backend.git", ".");

    assert!(!fixture.data.exists());
    assert!(!fixture.state.exists());
    assert!(plan.identity.project_id.as_str().starts_with("p-"));
    assert!(plan.workspace_id.starts_with("w-"));
    assert_eq!(
        plan.receipt.identity_source,
        sddk_domain::IdentitySource::Remote
    );
    assert!(plan.receipt.configuration_hash.starts_with("sha256:"));
    assert_eq!(
        plan.receipt.paths.vault,
        path_text(&plan.knowledge.vault_path)
    );
    assert_eq!(plan.receipt.paths.ledger, path_text(&plan.paths.ledger));
}

// durability-required: apply_adoption calls Storage::open(&plan.paths.ledger) internally,
// opening a separate connection; the plan's paths.ledger resolves to a TempDir path,
// not an in-memory database, so in-memory migration is not byte-equivalent.
#[test]
fn same_basename_different_remotes_and_scopes_do_not_collide() {
    let fixture = Fixture::new();
    let first = fixture.remote_plan("one/backend", "https://example.com/acme/backend", ".");
    let second = fixture.remote_plan("two/backend", "https://example.com/other/backend", ".");
    let scoped = fixture.remote_plan(
        "three/backend",
        "https://example.com/acme/backend",
        "services/api",
    );

    assert_ne!(first.identity.project_id, second.identity.project_id);
    assert_ne!(first.identity.project_id, scoped.identity.project_id);
    assert_ne!(first.paths.ledger, second.paths.ledger);
}

// durability-required: Storage::open_read_only reopens the same ledger path to verify
// workspace persistence across two apply_adoption calls on the same project.
#[test]
fn worktrees_share_project_storage_and_have_distinct_workspace_receipts() {
    let fixture = Fixture::new();
    let first = fixture.remote_plan("repo", "git@example.com:acme/repo.git", ".");
    let second = fixture.remote_plan("repo-feature", "ssh://git@example.com/acme/repo", ".");

    assert_eq!(first.identity.project_id, second.identity.project_id);
    assert_eq!(first.paths.ledger, second.paths.ledger);
    assert_ne!(first.workspace_id, second.workspace_id);
    assert_ne!(first.paths.receipt, second.paths.receipt);
    assert_eq!(
        apply_adoption(
            &first,
            &mut sddk_storage::Storage::open(&first.paths.ledger).unwrap()
        )
        .unwrap()
        .status,
        AdoptionStatusKind::Complete
    );
    assert_eq!(
        apply_adoption(
            &second,
            &mut sddk_storage::Storage::open(&second.paths.ledger).unwrap()
        )
        .unwrap()
        .status,
        AdoptionStatusKind::Complete
    );
    let storage = Storage::open_read_only(&first.paths.ledger).unwrap();
    assert!(storage.get_workspace(&first.workspace_id).is_ok());
    assert!(storage.get_workspace(&second.workspace_id).is_ok());
}

// durability-required: second apply_adoption reopens same ledger path for idempotency verification.
#[test]
fn apply_replay_is_idempotent_and_preserves_original_receipt_metadata() {
    let fixture = Fixture::new();
    let first = fixture.remote_plan("repo", "https://example.com/acme/repo", ".");
    let first_status = apply_adoption(
        &first,
        &mut sddk_storage::Storage::open(&first.paths.ledger).unwrap(),
    )
    .unwrap();
    let bytes = fs::read(&first.paths.receipt).unwrap();

    // Replay with identical identity but a different timestamp/actor.
    // This is the runtime-metadata-only drift case — apply must be idempotent
    // and converge to the new metadata without changing identity.
    let mut replay_input = fixture.input("repo");
    replay_input.remote_url = Some("https://example.com/acme/repo".into());
    replay_input.timestamp = "2026-08-04T11:00:00Z".into();
    replay_input.actor = "second-actor".into();
    let replay = plan_adoption(replay_input).unwrap();
    let replayed_status = apply_adoption(
        &replay,
        &mut sddk_storage::Storage::open(&replay.paths.ledger).unwrap(),
    )
    .unwrap();

    assert_eq!(first_status.status, AdoptionStatusKind::Complete);
    assert_eq!(replayed_status.status, AdoptionStatusKind::Complete);
    assert_ne!(
        fs::read(&first.paths.receipt).unwrap(),
        bytes,
        "apply must rewrite the receipt to update timestamp/actor"
    );
    assert_eq!(
        replayed_status.receipt.unwrap().timestamp,
        "2026-08-04T11:00:00Z",
        "apply must converge timestamp to the latest invocation"
    );
}

// durability-required: adoption_status reopens same ledger path to verify persisted fallback_seed.
#[test]
fn fallback_seed_is_persisted_and_reused() {
    let fixture = Fixture::new();
    let plan = fixture.fallback_plan("local-repo", SEED);
    apply_adoption(
        &plan,
        &mut sddk_storage::Storage::open(&plan.paths.ledger).unwrap(),
    )
    .unwrap();
    let receipt = read_adoption_receipt(&plan.paths.receipt).unwrap();

    assert_eq!(receipt.fallback_seed.as_deref(), Some(SEED));
    assert_eq!(
        receipt.identity_source,
        sddk_domain::IdentitySource::Fallback
    );
    let replay = fixture.fallback_plan("local-repo", receipt.fallback_seed.as_deref().unwrap());
    assert_eq!(replay.identity.project_id, plan.identity.project_id);
    assert_eq!(
        adoption_status(
            &replay,
            &sddk_storage::Storage::open(&replay.paths.ledger).unwrap()
        )
        .unwrap()
        .status,
        AdoptionStatusKind::Complete
    );
}

// durability-required: register_ledger_fixture writes to ledger at plan.paths.ledger;
// adoption_status and repair_adoption each re-open Storage::open at same path.
#[test]
fn repair_completes_receipt_only_state() {
    let fixture = Fixture::new();
    let plan = fixture.remote_plan("repo", "https://example.com/acme/repo", ".");
    write_receipt_fixture(&plan);

    assert_eq!(
        adoption_status(
            &plan,
            &sddk_storage::Storage::open(&plan.paths.ledger).unwrap()
        )
        .unwrap()
        .status,
        AdoptionStatusKind::ReceiptOnly
    );
    assert_eq!(
        repair_adoption(
            &plan,
            &mut sddk_storage::Storage::open(&plan.paths.ledger).unwrap()
        )
        .unwrap()
        .status,
        AdoptionStatusKind::Complete
    );
}

// durability-required: register_ledger_fixture writes to ledger at plan.paths.ledger;
// adoption_status and repair_adoption each re-open Storage::open at same path.
#[test]
fn repair_completes_ledger_only_state() {
    let fixture = Fixture::new();
    let plan = fixture.remote_plan("repo", "https://example.com/acme/repo", ".");
    register_ledger_fixture(&plan);

    assert_eq!(
        adoption_status(
            &plan,
            &sddk_storage::Storage::open(&plan.paths.ledger).unwrap()
        )
        .unwrap()
        .status,
        AdoptionStatusKind::LedgerOnly
    );
    assert_eq!(
        repair_adoption(
            &plan,
            &mut sddk_storage::Storage::open(&plan.paths.ledger).unwrap()
        )
        .unwrap()
        .status,
        AdoptionStatusKind::Complete
    );
}

// durability-required: register_ledger_fixture writes to ledger; adoption_status and
// repair_adoption re-open Storage::open at same path.
#[test]
fn repair_refuses_identity_conflict() {
    let fixture = Fixture::new();
    let original = fixture.remote_plan("repo", "https://example.com/acme/repo", ".");
    apply_adoption(
        &original,
        &mut sddk_storage::Storage::open(&original.paths.ledger).unwrap(),
    )
    .unwrap();
    let bytes_original = fs::read(&original.paths.receipt).unwrap();

    let mut changed_input = fixture.input("repo");
    changed_input.remote_url = Some("https://example.com/acme/other-repo".into());
    let changed = plan_adoption(changed_input).unwrap();

    // The drifted plan resolves to a different project_id (and therefore a
    // different paths.receipt), so adoption_status reports Absent at the
    // drifted path. The repair operation must NOT touch the original receipt
    // and must surface a refusal when the receipt at its resolved path
    // exists with a different identity.
    assert_eq!(
        adoption_status(
            &changed,
            &sddk_storage::Storage::open(&changed.paths.ledger).unwrap()
        )
        .unwrap()
        .status,
        AdoptionStatusKind::Absent
    );
    assert!(matches!(
        repair_adoption(
            &changed,
            &mut sddk_storage::Storage::open(&changed.paths.ledger).unwrap()
        ),
        Err(AdoptionError::NothingToRepair)
    ));
    // Original receipt is byte-identical after the repair attempt:
    assert_eq!(fs::read(&original.paths.receipt).unwrap(), bytes_original);
}

// durability-required: refresh_adoption reopens Storage::open at v2.paths.ledger to verify
// runtime metadata persisted from apply_adoption at v1.paths.ledger (same project path).
#[test]
fn refresh_preserves_identity_and_updates_runtime_metadata() {
    let fixture = Fixture::new();
    let v1 = fixture.remote_plan("repo", "https://example.com/acme/repo", ".");
    apply_adoption(
        &v1,
        &mut sddk_storage::Storage::open(&v1.paths.ledger).unwrap(),
    )
    .unwrap();
    let bytes_v1 = fs::read(&v1.paths.receipt).unwrap();

    let mut v2_input = fixture.input("repo");
    v2_input.remote_url = Some("https://example.com/acme/repo".into());
    v2_input.runtime_version = "0.2.0".into();
    v2_input.timestamp = "2026-08-13T18:00:00Z".into();
    v2_input.actor = "second-actor".into();
    let v2 = plan_adoption(v2_input).unwrap();

    let refreshed = refresh_adoption(
        &v2,
        &mut sddk_storage::Storage::open(&v2.paths.ledger).unwrap(),
    )
    .unwrap();
    assert_eq!(refreshed.status, AdoptionStatusKind::Complete);

    let on_disk = read_adoption_receipt(&v2.paths.receipt).unwrap();
    assert_eq!(on_disk.runtime_version, "0.2.0");
    assert_eq!(on_disk.timestamp, "2026-08-13T18:00:00Z");
    assert_eq!(on_disk.actor, "second-actor");
    // Identity preserved:
    assert_eq!(on_disk.project_id, v1.receipt.project_id);
    assert_eq!(on_disk.workspace_id, v1.receipt.workspace_id);
    assert_eq!(on_disk.remote_url, v1.receipt.remote_url);
    assert_eq!(on_disk.scope, v1.receipt.scope);
    assert_eq!(on_disk.paths, v1.receipt.paths);
    // Bytes differ because runtime metadata changed:
    assert_ne!(fs::read(&v2.paths.receipt).unwrap(), bytes_v1);
}

// durability-required: refresh_adoption reopens Storage::open at drifted.paths.ledger (different
// identity from original) to verify original receipt is preserved on disk.
#[test]
fn refresh_fails_on_identity_drift() {
    let fixture = Fixture::new();
    let original = fixture.remote_plan("repo", "https://example.com/acme/repo", ".");
    apply_adoption(
        &original,
        &mut sddk_storage::Storage::open(&original.paths.ledger).unwrap(),
    )
    .unwrap();
    let bytes_original = fs::read(&original.paths.receipt).unwrap();

    // Drift identity: different remote_url ⇒ different project_id ⇒ different
    // paths.receipt. Refresh on the drifted path returns Absent (no receipt
    // there) and the original receipt stays untouched.
    let mut drifted_input = fixture.input("repo");
    drifted_input.remote_url = Some("https://example.com/acme/other-repo".into());
    drifted_input.runtime_version = "0.2.0".into();
    let drifted = plan_adoption(drifted_input).unwrap();

    let refreshed = refresh_adoption(
        &drifted,
        &mut sddk_storage::Storage::open(&drifted.paths.ledger).unwrap(),
    )
    .unwrap();
    assert_eq!(
        refreshed.status,
        AdoptionStatusKind::Absent,
        "refresh at a different identity resolves to a different receipt path"
    );
    assert_eq!(
        fs::read(&original.paths.receipt).unwrap(),
        bytes_original,
        "refresh must not mutate the original receipt when identity drifts"
    );
}

// durability-required: apply_adoption with drifted plan reopens same path as v1.apply;
// final assertion verifies v1.paths.receipt is byte-untouched (cross-call persistence check).
#[test]
fn apply_is_strict_about_identity_after_refresh() {
    let fixture = Fixture::new();
    let v1 = fixture.remote_plan("repo", "https://example.com/acme/repo", ".");
    apply_adoption(
        &v1,
        &mut sddk_storage::Storage::open(&v1.paths.ledger).unwrap(),
    )
    .unwrap();
    let bytes_v1 = fs::read(&v1.paths.receipt).unwrap();

    // Drift identity AND runtime metadata. Different remote_url ⇒ different
    // project_id ⇒ different paths.receipt, so apply creates a NEW receipt at
    // the drifted path. The original receipt must remain byte-untouched.
    let mut drifted_input = fixture.input("repo");
    drifted_input.remote_url = Some("https://example.com/acme/other-repo".into());
    drifted_input.runtime_version = "0.2.0".into();
    let drifted = plan_adoption(drifted_input).unwrap();

    let applied = apply_adoption(
        &drifted,
        &mut sddk_storage::Storage::open(&drifted.paths.ledger).unwrap(),
    )
    .unwrap();
    assert_eq!(applied.status, AdoptionStatusKind::Complete);
    assert_eq!(
        fs::read(&v1.paths.receipt).unwrap(),
        bytes_v1,
        "apply must not mutate the original receipt when identity drifts"
    );
    assert_ne!(
        applied.receipt.unwrap().project_id,
        v1.receipt.project_id,
        "the drifted plan lives at a different project_id"
    );
}

// durability-required: Storage::open used to create storage for refresh_adoption (single-call
// pattern; kept file-based for consistency with the rest of the adoption test suite).
#[test]
fn refresh_is_no_op_when_receipt_is_absent() {
    let fixture = Fixture::new();
    let plan = fixture.remote_plan("repo", "https://example.com/acme/repo", ".");

    let refreshed = refresh_adoption(
        &plan,
        &mut sddk_storage::Storage::open(&plan.paths.ledger).unwrap(),
    )
    .unwrap();
    assert_eq!(refreshed.status, AdoptionStatusKind::Absent);
    assert!(!plan.paths.receipt.exists());
}

// durability-required: fs::write creates corrupt receipt on filesystem; adoption_status
// classifies Corrupt from the receipt file (not storage), and apply_adoption refuses
// to overwrite it — both receipt-file and storage-reopen behaviors require file-based Storage.
#[test]
fn corrupt_receipt_is_classified_and_never_overwritten() {
    let fixture = Fixture::new();
    let plan = fixture.remote_plan("repo", "https://example.com/acme/repo", ".");
    fs::create_dir_all(plan.paths.receipt.parent().unwrap()).unwrap();
    fs::write(&plan.paths.receipt, b"{not-json\n").unwrap();

    assert_eq!(
        adoption_status(
            &plan,
            &sddk_storage::Storage::open(&plan.paths.ledger).unwrap()
        )
        .unwrap()
        .status,
        AdoptionStatusKind::Corrupt
    );
    assert!(matches!(
        apply_adoption(
            &plan,
            &mut sddk_storage::Storage::open(&plan.paths.ledger).unwrap()
        ),
        Err(AdoptionError::UnsafeState {
            status: AdoptionStatusKind::Corrupt,
            ..
        })
    ));
    assert_eq!(fs::read(&plan.paths.receipt).unwrap(), b"{not-json\n");
}

struct Fixture {
    _directory: TempDir,
    root: PathBuf,
    data: PathBuf,
    state: PathBuf,
    cache: PathBuf,
    home: PathBuf,
}

impl Fixture {
    /// Creates a file-based fixture for durability-required tests.
    fn new() -> Self {
        let directory = tempfile::tempdir().unwrap();
        Self {
            root: directory.path().join("checkouts"),
            data: directory.path().join("xdg-data"),
            state: directory.path().join("xdg-state"),
            cache: directory.path().join("xdg-cache"),
            home: directory.path().join("home"),
            _directory: directory,
        }
    }

    /// Creates an in-memory fixture for migratable tests.
    /// The owned storage is unused (test does not call apply_adoption/adoption_status/
    /// repair_adoption), but creates the same plan paths as file-based, so behavior
    /// is byte-equivalent.
    fn new_in_memory() -> Self {
        let directory = tempfile::tempdir().unwrap();
        let _storage = Storage::open_in_memory().unwrap();
        Self {
            root: directory.path().join("checkouts"),
            data: directory.path().join("xdg-data"),
            state: directory.path().join("xdg-state"),
            cache: directory.path().join("xdg-cache"),
            home: directory.path().join("home"),
            _directory: directory,
        }
    }

    fn input(&self, relative_root: &str) -> AdoptionPlanInput {
        AdoptionPlanInput {
            remote_url: None,
            scope: ".".into(),
            fallback_seed: None,
            canonical_workspace_path: self.root.join(relative_root),
            display_name: Path::new(relative_root)
                .file_name()
                .unwrap()
                .to_string_lossy()
                .into_owned(),
            xdg: XdgEnvironment {
                home: Some(self.home.clone()),
                data_home: Some(self.data.clone()),
                sddk_data_dir: None,
                state_home: Some(self.state.clone()),
                cache_home: Some(self.cache.clone()),
            },
            sddk_version: "3.6".into(),
            runtime_version: "0.1.0".into(),
            timestamp: TIMESTAMP.into(),
            actor: "test-runtime".into(),
        }
    }

    fn remote_plan(&self, root: &str, remote: &str, scope: &str) -> AdoptionPlan {
        let mut input = self.input(root);
        input.remote_url = Some(remote.into());
        input.scope = scope.into();
        plan_adoption(input).unwrap()
    }

    fn fallback_plan(&self, root: &str, seed: &str) -> AdoptionPlan {
        let mut input = self.input(root);
        input.fallback_seed = Some(seed.into());
        plan_adoption(input).unwrap()
    }
}

fn write_receipt_fixture(plan: &AdoptionPlan) {
    fs::create_dir_all(plan.paths.receipt.parent().unwrap()).unwrap();
    fs::write(
        &plan.paths.receipt,
        serde_json::to_vec_pretty(&plan.receipt).unwrap(),
    )
    .unwrap();
}

fn register_ledger_fixture(plan: &AdoptionPlan) {
    let mut storage = Storage::open(&plan.paths.ledger).unwrap();
    storage
        .register_project_workspace(
            &ProjectRecord {
                project_id: plan.receipt.project_id.clone(),
                display_name: plan.receipt.display_name.clone(),
                remote_url: plan.receipt.remote_url.clone(),
                scope: plan.receipt.scope.clone(),
                created_at: plan.receipt.timestamp.clone(),
            },
            &WorkspaceRecord {
                workspace_id: plan.receipt.workspace_id.clone(),
                project_id: plan.receipt.project_id.clone(),
                canonical_path: plan.receipt.canonical_workspace_path.clone(),
                created_at: plan.receipt.timestamp.clone(),
            },
        )
        .unwrap();
}

fn path_text(path: &Path) -> String {
    path.to_string_lossy().into_owned()
}
