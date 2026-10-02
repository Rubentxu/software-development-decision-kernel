//! Release flow integration tests: plan, idempotent apply, reconciliation.

use sddk_engine::version_source::VersionAuthority;
use sddk_gateway::{
    CapabilityGateway, CapabilityPlanInput, CapabilityPolicy, Forge, GitExecutor,
    LocalReleaseInput, LocalReleasePreconditions, MockForge, ReleasePlanInput, apply_local_release,
    apply_release, plan_release, reconcile_pending,
};
use sddk_storage::{CapabilityStatus, ProjectRecord, Storage};

const WORKFLOW_YAML: &str = include_str!("../../../workflow/workflow.yaml");

fn gateway() -> (tempfile::TempDir, CapabilityGateway) {
    let directory = tempfile::tempdir().unwrap();
    let path = directory.path().join("ledger.sqlite");
    let storage = Storage::open(&path).unwrap();
    storage
        .insert_project(&ProjectRecord {
            project_id: "project-1".into(),
            display_name: "project".into(),
            remote_url: Some("https://example.com/owner/project".into()),
            scope: "owner".into(),
            created_at: "2026-08-04T10:00:00Z".into(),
        })
        .unwrap();
    let workflow = sddk_engine::load_workflow_str(WORKFLOW_YAML).unwrap();
    let policy = CapabilityPolicy::from_workflow(&workflow);
    let gateway = CapabilityGateway::new(policy, workflow, Storage::open(&path).unwrap());
    (directory, gateway)
}

fn release_input(tag: &str) -> ReleasePlanInput {
    ReleasePlanInput {
        project_id: "project-1".into(),
        cycle_id: None,
        branch: "feat/release".into(),
        base_branch: "main".into(),
        pr_title: "Release".into(),
        pr_body: "body".into(),
        tag: tag.into(),
        release_title: "v1".into(),
        release_notes: "notes".into(),
        approve: true,
        timestamp: "2026-08-04T10:00:00Z".into(),
        actor: "release-test".into(),
    }
}

fn local_release_input(tag: &str) -> LocalReleaseInput {
    LocalReleaseInput {
        project_id: "project-1".into(),
        cycle_id: None,
        branch: "main".into(),
        tag: tag.into(),
        tag_message: "release test".into(),
        approve: true,
        timestamp: "2026-08-04T10:00:00Z".into(),
        actor: "release-test".into(),
        preconditions: LocalReleasePreconditions {
            verification_passed: true,
            uat_passed: true,
            version_lockstep_passed: true,
            manifest_exact_set_verified: true,
            bundle_roundtrip_verified: true,
            release_receipt_verified: true,
        },
    }
}

/// La autoridad que estos tests declaran. Antes se pasaba `false`, y el valor
/// era arbitrario porque nadie leia el campo: se llevaba a `ReleaseOutcome` y
/// de ahi a ninguna parte. Ahora es un hecho comprobable, y un test lo fija.
fn checked_version() -> VersionAuthority {
    VersionAuthority::CrossChecked {
        version: "1.0.0".to_string(),
        candidates: Vec::new(),
    }
}

fn local_git_repo() -> (tempfile::TempDir, GitExecutor) {
    let directory = tempfile::tempdir().unwrap();
    let origin = directory.path().join("origin.git");
    let worktree = directory.path().join("worktree");
    for (directory, args) in [
        (directory.path(), vec!["init", "--bare", "origin.git"]),
        (directory.path(), vec!["init", "-b", "main", "worktree"]),
    ] {
        let status = std::process::Command::new("git")
            .args(args)
            .current_dir(directory)
            .status()
            .unwrap();
        assert!(status.success());
    }
    for args in [
        vec!["config", "user.name", "SDDK Test"],
        vec!["config", "user.email", "test@sddk.dev"],
        vec!["commit", "--allow-empty", "-m", "initial"],
        vec!["remote", "add", "origin", origin.to_str().unwrap()],
    ] {
        let status = std::process::Command::new("git")
            .args(args)
            .current_dir(&worktree)
            .status()
            .unwrap();
        assert!(status.success());
    }
    (directory, GitExecutor::new(worktree))
}

#[test]
fn full_release_creates_pr_merges_and_publishes() {
    let (_directory, mut gateway) = gateway();
    let mut forge = MockForge::new();
    forge.seed_checks(0, vec![]);

    let plan = plan_release(release_input("v1.0.0"), &forge).unwrap();
    assert_eq!(plan.steps.len(), 3);

    let outcome = apply_release(&mut gateway, &plan, &mut forge, checked_version()).unwrap();
    assert_eq!(outcome.applied.len(), 3);
    assert!(outcome.skipped.is_empty());
    assert!(outcome.converged);

    assert_eq!(forge.find_open_pr("feat/release", "main").unwrap(), None);
    assert!(forge.release_state("v1.0.0").unwrap().unwrap().published);
    let receipts = gateway.receipts("project-1").unwrap();
    assert_eq!(receipts.len(), 3);
    assert!(
        receipts
            .iter()
            .all(|receipt| receipt.status == CapabilityStatus::Succeeded)
    );
}

#[test]
fn forge_release_does_not_gate_on_provider_checks() {
    let (_directory, _gateway) = gateway();
    let mut forge = MockForge::new();
    forge.seed_open_pr("feat/release", "main", 3);
    forge.seed_checks(
        3,
        vec![sddk_gateway::CheckState {
            name: "external-ci".into(),
            passed: Some(false),
        }],
    );

    let plan = plan_release(release_input("v1.0.0"), &forge).unwrap();

    assert_eq!(
        plan.steps,
        vec![
            sddk_gateway::ReleaseStep::MergePr,
            sddk_gateway::ReleaseStep::CreateRelease
        ]
    );
}

#[test]
fn local_release_pushes_main_and_an_annotated_tag_idempotently() {
    let (_directory, git) = local_git_repo();
    let (_ledger, mut gateway) = gateway();

    let first = apply_local_release(&mut gateway, &local_release_input("v1.0.0"), &git).unwrap();
    assert!(first.converged);
    assert_eq!(first.applied.len(), 2);
    assert_eq!(first.sha, git.remote_branch_sha("main").unwrap().unwrap());
    assert_eq!(
        git.remote_annotated_tag_target("v1.0.0")
            .unwrap()
            .as_deref(),
        Some(first.sha.as_str())
    );

    let second = apply_local_release(&mut gateway, &local_release_input("v1.0.0"), &git).unwrap();
    assert!(second.converged);
    assert!(second.applied.is_empty());
    assert_eq!(second.skipped.len(), 2);
    assert_eq!(gateway.receipts("project-1").unwrap().len(), 2);
}

#[test]
fn local_release_pushes_an_existing_annotated_tag_missing_from_remote() {
    let (_directory, git) = local_git_repo();
    let (_ledger, mut gateway) = gateway();
    let sha = git.head_sha().unwrap();
    git.create_annotated_tag("v1.0.0", &sha, "release test")
        .unwrap();

    let outcome = apply_local_release(&mut gateway, &local_release_input("v1.0.0"), &git).unwrap();

    assert!(outcome.converged);
    assert_eq!(outcome.applied.len(), 2);
    assert_eq!(
        git.remote_annotated_tag_target("v1.0.0")
            .unwrap()
            .as_deref(),
        Some(sha.as_str())
    );
}

#[test]
fn local_release_rejects_dirty_or_non_trunk_checkouts() {
    let (_directory, git) = local_git_repo();
    let (_ledger, mut gateway) = gateway();
    std::fs::write(git.root().join("dirty.txt"), "uncommitted").unwrap();

    let dirty = apply_local_release(&mut gateway, &local_release_input("v1.0.0"), &git)
        .unwrap_err()
        .to_string();
    assert!(dirty.contains("worktree must be clean"));

    std::fs::remove_file(git.root().join("dirty.txt")).unwrap();
    let mut missing_evidence = local_release_input("v1.0.0");
    missing_evidence.preconditions.verification_passed = false;
    let missing_evidence = apply_local_release(&mut gateway, &missing_evidence, &git)
        .unwrap_err()
        .to_string();
    assert!(missing_evidence.contains("local verification evidence"));

    git.create_branch("release-candidate").unwrap();
    let non_trunk = apply_local_release(&mut gateway, &local_release_input("v1.0.0"), &git)
        .unwrap_err()
        .to_string();
    assert!(non_trunk.contains("checkout must be main"));
}

#[test]
fn interrupted_release_converges_without_duplicating_effects() {
    let (_directory, mut gateway) = gateway();
    let mut forge = MockForge::new();
    forge.seed_open_pr("feat/release", "main", 3);
    forge.seed_checks(3, vec![]);
    forge.seed_release("v1.0.0");

    let plan = plan_release(release_input("v1.0.0"), &forge).unwrap();
    assert_eq!(plan.steps, vec![sddk_gateway::ReleaseStep::MergePr]);

    let outcome = apply_release(&mut gateway, &plan, &mut forge, checked_version()).unwrap();
    assert_eq!(outcome.applied.len(), 1);
    assert!(outcome.converged);
    assert_eq!(outcome.skipped.len(), 0);
    assert!(forge.is_merged(3));

    let second = apply_release(&mut gateway, &plan, &mut forge, checked_version()).unwrap();
    assert!(second.applied.is_empty());
    assert_eq!(second.skipped.len(), 1);
    assert!(second.converged);

    let receipts = gateway.receipts("project-1").unwrap();
    assert_eq!(receipts.len(), 1);
}

#[test]
fn release_without_open_pr_creates_and_merges() {
    let (_directory, mut gateway) = gateway();
    let mut forge = MockForge::new();
    forge.seed_release("v1.0.0");

    let plan = plan_release(release_input("v1.0.0"), &forge).unwrap();
    assert_eq!(plan.steps.len(), 2);
    assert!(
        forge
            .find_open_pr("feat/release", "main")
            .unwrap()
            .is_none()
    );

    let outcome = apply_release(&mut gateway, &plan, &mut forge, checked_version()).unwrap();
    assert_eq!(outcome.applied.len(), 2);
    assert!(outcome.skipped.is_empty());
    assert!(outcome.converged);
    assert_eq!(forge.find_open_pr("feat/release", "main").unwrap(), None);
    assert!(forge.release_state("v1.0.0").unwrap().unwrap().published);
}

#[test]
fn reconcile_finalizes_interrupted_receipts_against_provider() {
    let (_directory, mut gateway) = gateway();
    let mut forge = MockForge::new();
    forge.seed_release("v9.9.9");

    let begin = |gateway: &mut CapabilityGateway, tag: &str| {
        gateway
            .begin_effect(&CapabilityPlanInput {
                project_id: "project-1".into(),
                cycle_id: None,
                capability: "release.create".into(),
                reason: "interrupted".into(),
                program: "forge".into(),
                args: vec![tag.into()],
                env: Default::default(),
                timeout_ms: 60_000,
                output_max_bytes: 1_048_576,
                approve: true,
                timestamp: "2026-08-04T10:00:00Z".into(),
                actor: "release-test".into(),
            })
            .unwrap()
            .receipt_id
    };
    let present = begin(&mut gateway, "v9.9.9");
    let absent = begin(&mut gateway, "v0.0.1");

    let (_git_directory, git) = local_git_repo();
    let reconciled = reconcile_pending(&mut gateway, &forge, &git).unwrap();
    assert_eq!(reconciled.len(), 2);
    let by_id = |id: &str| {
        reconciled
            .iter()
            .find(|receipt| receipt.receipt_id == id)
            .unwrap()
    };
    assert_eq!(by_id(&present).status, CapabilityStatus::Succeeded);
    assert_eq!(by_id(&absent).status, CapabilityStatus::Failed);

    let again = reconcile_pending(&mut gateway, &forge, &git).unwrap();
    assert!(again.is_empty());
}

#[test]
fn reconcile_finalizes_started_local_receipts_after_remote_effects() {
    let (_directory, git) = local_git_repo();
    let (_ledger, mut gateway) = gateway();
    let sha = git.head_sha().unwrap();
    let input = local_release_input("v1.0.0");

    let push = gateway
        .begin_effect(&CapabilityPlanInput {
            project_id: input.project_id.clone(),
            cycle_id: None,
            capability: "git.push".into(),
            reason: "interrupted after remote push".into(),
            program: "git".into(),
            args: vec!["main".into(), sha.clone()],
            env: Default::default(),
            timeout_ms: 60_000,
            output_max_bytes: 1_048_576,
            approve: true,
            timestamp: input.timestamp.clone(),
            actor: input.actor.clone(),
        })
        .unwrap();
    git.push_and_verify_branch("main").unwrap();

    let tag = gateway
        .begin_effect(&CapabilityPlanInput {
            project_id: input.project_id,
            cycle_id: None,
            capability: "git.tag".into(),
            reason: "interrupted after remote tag".into(),
            program: "git".into(),
            args: vec![input.tag.clone(), sha.clone()],
            env: Default::default(),
            timeout_ms: 60_000,
            output_max_bytes: 1_048_576,
            approve: true,
            timestamp: input.timestamp,
            actor: input.actor,
        })
        .unwrap();
    git.create_annotated_tag(&input.tag, &sha, &input.tag_message)
        .unwrap();
    git.push_and_verify_annotated_tag(&input.tag, &sha).unwrap();

    let forge = MockForge::new();
    let reconciled = reconcile_pending(&mut gateway, &forge, &git).unwrap();
    assert_eq!(reconciled.len(), 2);
    assert_eq!(
        reconciled
            .iter()
            .find(|receipt| receipt.receipt_id == push.receipt_id)
            .unwrap()
            .status,
        CapabilityStatus::Succeeded
    );
    assert_eq!(
        reconciled
            .iter()
            .find(|receipt| receipt.receipt_id == tag.receipt_id)
            .unwrap()
            .status,
        CapabilityStatus::Succeeded
    );
}

/// El resultado de un release dice **de dónde** salió la versión, no que el
/// lockstep pasó. Antes era un `bool` que el llamador fijaba a mano, y sobre
/// un proyecto sin versión declarada informaba una comprobación que no había
/// ocurrido.
#[test]
fn release_outcome_records_where_the_version_came_from() {
    let (_dir, mut gateway) = gateway();
    let mut forge = MockForge::new();
    let plan = plan_release(release_input("v1.0.0"), &forge).unwrap();

    let checked = apply_release(
        &mut gateway,
        &plan,
        &mut forge,
        VersionAuthority::CrossChecked {
            version: "1.0.0".into(),
            candidates: Vec::new(),
        },
    )
    .unwrap();
    assert!(matches!(
        checked.version_authority,
        VersionAuthority::CrossChecked { .. }
    ));

    // Un proyecto Go o Bazel no declara versión en ningún manifiesto: el tag
    // no tenía contra qué contrastarse, y el resultado tiene que decirlo en
    // vez de presentarse como comprobado.
    let tag_only = apply_release(
        &mut gateway,
        &plan,
        &mut forge,
        VersionAuthority::TagIsTheOnlyAuthority {
            ecosystems: vec!["go"],
        },
    )
    .unwrap();
    assert_eq!(
        tag_only.version_authority,
        VersionAuthority::TagIsTheOnlyAuthority {
            ecosystems: vec!["go"]
        }
    );
    assert!(
        !checked.version_authority.was_cross_checked()
            || !matches!(
                tag_only.version_authority,
                VersionAuthority::CrossChecked { .. }
            ),
        "un proyecto sin version declarada no puede registrarse como comprobado"
    );
}

/// El nombre `version_lockstep_passed` ya no aparece en el resultado: lo que
/// queda es un campo con nombre propio y tipo propio. Es la homonimia del
/// lote 2 (§2 del SCOPE) resuelta por construcción y no por una nota.
#[test]
fn the_outcome_has_no_boolean_named_after_the_lockstep() {
    let json = serde_json::to_value(sddk_gateway::ReleaseOutcome {
        applied: Vec::new(),
        skipped: Vec::new(),
        converged: true,
        version_authority: VersionAuthority::TagIsTheOnlyAuthority {
            ecosystems: vec!["go"],
        },
    })
    .unwrap();
    assert!(
        json.get("version_lockstep_passed").is_none(),
        "el resultado no puede afirmar un lockstep que no se comprobó: {json}"
    );
    assert_eq!(
        json["version_authority"]["kind"], "tag_is_the_only_authority",
        "{json}"
    );
    assert_eq!(json["version_authority"]["ecosystems"][0], "go");
}

/// Test **estructural** del doc de `apply_release`. Un doc no se ejecuta, así
/// que ningún test de comportamiento lo vigila: el único que puede es uno que
/// lee el fuente. El doc nombraba `Cargo.toml`, que ADR-0153 dejó de ser
/// cierto, y volver a nombrarlo sería la tercera copia de una suposición que
/// ya no existe.
#[test]
fn the_apply_release_doc_does_not_name_a_manifest() {
    let source = include_str!("../src/release.rs");
    let start = source
        .find("pub fn apply_release(")
        .expect("apply_release no encontrado");
    // El doc comment es lo que precede a la firma.
    // El doc comment es el bloque CONTIGUO de lineas `///` que precede a la
    // firma. Tomar solo la ultima dejaria una linea suelta y el test pasaria
    // sobre un doc que ya miente: el fallo inicial del propio test.
    let before = &source[..start];
    let mut lines: Vec<&str> = Vec::new();
    for line in before.lines().rev() {
        let trimmed = line.trim_start();
        if trimmed.starts_with("///") {
            lines.push(trimmed);
        } else if trimmed.is_empty() || trimmed.starts_with("#[") {
            continue;
        } else {
            break;
        }
    }
    let doc = lines.join("\n");
    assert!(
        !doc.contains("Cargo.toml"),
        "el doc de apply_release vuelve a nombrarlo: {doc}"
    );
    assert!(
        doc.contains("version_authority"),
        "el doc tiene que nombrar el parametro real: {doc}"
    );
}
