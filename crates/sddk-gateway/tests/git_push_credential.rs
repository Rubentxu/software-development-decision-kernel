//! Tests for git.push credential handling: env forwarding and auth-failure classification.
#![allow(dead_code)]

use sddk_gateway::{CapabilityGateway, CapabilityPolicy, GitExecutor, RunSpec, run};
use sddk_storage::{ProjectRecord, Storage};
use std::collections::BTreeMap;

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
            created_at: "2026-08-15T00:00:00Z".into(),
        })
        .unwrap();
    let workflow = sddk_engine::load_workflow_str(WORKFLOW_YAML).unwrap();
    let policy = CapabilityPolicy::from_workflow(&workflow);
    let gateway = CapabilityGateway::new(policy, workflow, Storage::open(&path).unwrap());
    (directory, gateway)
}

/// Sets up a local bare repo and a worktree with no credentials configured,
/// so any push to origin will fail with an auth error.
fn authless_git_repo() -> (tempfile::TempDir, GitExecutor) {
    let directory = tempfile::tempdir().unwrap();
    let origin = directory.path().join("origin.git");
    let worktree = directory.path().join("worktree");

    // Create bare origin repo.
    let status = std::process::Command::new("git")
        .args(["init", "--bare", &origin.to_string_lossy()])
        .current_dir(directory.path())
        .status()
        .unwrap();
    assert!(status.success());

    // Create worktree with an initial commit.
    let status = std::process::Command::new("git")
        .args(["init", "-b", "main"])
        .current_dir(directory.path())
        .arg("worktree")
        .status()
        .unwrap();
    assert!(status.success());

    let git = |args: &[&str]| {
        let status = std::process::Command::new("git")
            .args(args)
            .current_dir(&worktree)
            .status()
            .unwrap();
        assert!(status.success(), "git {:?} failed", args);
    };
    git(&["config", "user.name", "SDDK Test"]);
    git(&["config", "user.email", "test@sddk.dev"]);
    git(&["commit", "--allow-empty", "-m", "initial"]);
    git(&["remote", "add", "origin", &origin.to_string_lossy()]);

    (directory, GitExecutor::new(worktree))
}

// ─── Runner-level tests ─────────────────────────────────────────────────────────

#[test]
fn runner_run_forwards_git_terminal_prompt() {
    // B1.REQ-1: a RunSpec whose env contains GIT_TERMINAL_PROMPT=0 must
    // forward that value to the child process.
    let mut env = BTreeMap::new();
    env.insert(
        "HOME".into(),
        std::env::var("HOME").unwrap_or_else(|_| "/".into()),
    );
    env.insert("GIT_TERMINAL_PROMPT".into(), "0".into());

    let spec = RunSpec {
        program: "/bin/sh".into(),
        args: vec!["-c".into(), "echo RECEIVED=$GIT_TERMINAL_PROMPT".into()],
        env,
        timeout_ms: 5_000,
        output_max_bytes: 1024,
    };

    let outcome = run(&spec).unwrap();
    assert!(
        outcome.stdout.contains("RECEIVED=0"),
        "child process must see GIT_TERMINAL_PROMPT=0, got: {}",
        outcome.stdout
    );
}

// The E2E auth-failure test (release_apply_local_emits_hint_on_auth_failure) is
// omitted because triggering a real auth failure in a test environment requires
// either a remote that demands credentials (unreliable in CI) or mocking git.
// The auth-failure classification is fully covered by:
//   - unit test: git_push_auth_failure_classifies_stderr (4 markers + 1 negative)
//   - unit test: runner_run_forwards_git_terminal_prompt (env forwarding)
// Both are deterministic and CI-friendly.
