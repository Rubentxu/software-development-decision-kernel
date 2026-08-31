//! Regression tests for the BLOCKER/HIGH findings reported on the
//! sddk-release flow. Each test maps to a specific finding:
//!
//! * `reconcile_does_not_close_started_when_remote_effect_is_absent` — finding 3
//!   (pre-effect crash must not be turned into a Failed receipt).
//! * `reconcile_closes_started_when_remote_effect_is_present` — finding 3
//!   (post-effect crash must be reconciled to Succeeded).
//! * `apply_local_release_replays_after_interrupted_push` — finding 3
//!   (after a Started receipt, a retry must apply the missing effect).
//!
//! The test file lives in the same integration suite as `release_flow.rs` so
//! it has access to the same `local_git_repo` and `gateway` fixtures.

use sddk_gateway::{
    CapabilityGateway, CapabilityPlanInput, CapabilityPolicy, GitExecutor, LocalReleaseInput,
    LocalReleasePreconditions, MockForge, apply_local_release,
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
fn reconcile_does_not_close_started_when_remote_effect_is_absent() {
    let (_directory, git) = local_git_repo();
    let (_ledger, mut gateway) = gateway();
    let sha = git.head_sha().unwrap();
    let input = local_release_input("v1.0.0");

    // Begin a push receipt but DO NOT push to the remote. The remote branch
    // is still absent, so this represents a pre-effect crash.
    let push = gateway
        .begin_effect(&CapabilityPlanInput {
            project_id: input.project_id.clone(),
            cycle_id: input.cycle_id.clone(),
            capability: "git.push".into(),
            reason: "interrupted before push".into(),
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
    assert_eq!(push.status, CapabilityStatus::Started);

    let forge = MockForge::new();
    // reconcile_pending also reconciles local receipts through the gateway;
    // the local path used by the release flow is the same.
    let reconciled = sddk_gateway::reconcile_pending(&mut gateway, &forge, &git).unwrap();

    // The pre-effect receipt MUST stay Started so the retry can apply the
    // effect. A regression that marks it Failed would be a BLOCKER.
    let still_started = !reconciled
        .iter()
        .any(|receipt| receipt.receipt_id == push.receipt_id);
    assert!(
        still_started,
        "pre-effect Started receipt must NOT be finalized as Failed by reconcile_pending"
    );

    // And the storage still shows the receipt as Started.
    let stored = gateway
        .receipts("project-1")
        .unwrap()
        .into_iter()
        .find(|receipt| receipt.receipt_id == push.receipt_id)
        .expect("started receipt is present in storage");
    assert_eq!(
        stored.status,
        CapabilityStatus::Started,
        "pre-effect receipt must remain Started for a safe retry"
    );
}

#[test]
fn reconcile_closes_started_when_remote_effect_is_present() {
    let (_directory, git) = local_git_repo();
    let (_ledger, mut gateway) = gateway();
    let sha = git.head_sha().unwrap();
    let input = local_release_input("v1.0.0");

    // Begin a push receipt and actually push. This is the post-effect crash
    // scenario: the local effect is on the remote, so reconcile must close
    // the Started receipt as Succeeded.
    let push = gateway
        .begin_effect(&CapabilityPlanInput {
            project_id: input.project_id.clone(),
            cycle_id: input.cycle_id.clone(),
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

    let forge = MockForge::new();
    let reconciled = sddk_gateway::reconcile_pending(&mut gateway, &forge, &git).unwrap();
    let finalized = reconciled
        .iter()
        .find(|receipt| receipt.receipt_id == push.receipt_id)
        .expect("reconcile finalizes the post-effect receipt");
    assert_eq!(
        finalized.status,
        CapabilityStatus::Succeeded,
        "post-effect receipt with a present remote SHA must be Succeeded"
    );
}

#[test]
fn apply_local_release_replays_after_interrupted_push() {
    let (_directory, git) = local_git_repo();
    let (_ledger, mut gateway) = gateway();
    let sha = git.head_sha().unwrap();
    let input = local_release_input("v1.0.0");

    // Begin a push receipt without pushing. After reconcile (which must keep
    // it Started), apply_local_release must apply the missing effect and
    // converge. We use the same `reason` and `args` that `apply_local_release`
    // uses internally so the idempotency key matches.
    let push = gateway
        .begin_effect(&CapabilityPlanInput {
            project_id: input.project_id.clone(),
            cycle_id: input.cycle_id.clone(),
            capability: "git.push".into(),
            reason: "push direct trunk branch".into(),
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
    let forge = MockForge::new();
    let _ = sddk_gateway::reconcile_pending(&mut gateway, &forge, &git).unwrap();

    // Re-running the local apply with the same idempotency key MUST apply the
    // missing effect (the Started receipt is reused) and converge.
    let outcome = apply_local_release(&mut gateway, &input, &git).unwrap();
    assert!(
        outcome.converged,
        "apply_local_release must converge after a pre-effect crash (converged={})",
        outcome.converged
    );
    let stored = gateway
        .receipts("project-1")
        .unwrap()
        .into_iter()
        .find(|receipt| receipt.receipt_id == push.receipt_id)
        .expect("started receipt is present in storage");
    assert_eq!(stored.status, CapabilityStatus::Succeeded);
}
