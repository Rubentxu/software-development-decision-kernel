//! Integration tests for `sddk approval` CLI commands.
//!
//! Tests the full approval flow: approval.capability.requested → approval list → grant → list (empty)
//!
//! Follows the same pattern as `cli_phase_events_e2e.rs`: XDG env vars are set to
//! temp directories so the CLI uses the same storage path as the test.

use sddk_domain::EventStore;
use serde_json::json;
use sha2::Digest;
use sha2::Sha256;
use std::process::Command;
use tempfile::TempDir;

/// Computes the stable project ID for a fallback seed + scope.
/// Matches `sddk_domain::identity::stable_fallback_project_id`.
fn fallback_project_id(seed: &str, scope: &str) -> String {
    let hex = {
        let mut hasher = Sha256::new();
        // framed_hash format: domain_len || domain || part0_len || part0 || part1_len || part1
        let domain = "sddk.project.fallback.v1";
        hasher.update((domain.len() as u64).to_be_bytes());
        hasher.update(domain.as_bytes());
        hasher.update((seed.len() as u64).to_be_bytes());
        hasher.update(seed.as_bytes());
        hasher.update((scope.len() as u64).to_be_bytes());
        hasher.update(scope.as_bytes());
        format!("{:x}", hasher.finalize())
    };
    format!("p-{}", &hex[..16])
}

/// Helper to create a minimal `EventEnvelopeV1` with a computed content_hash.
fn make_event(
    project_id: &str,
    stream_id: &str,
    event_type: &str,
    sequence: u64,
    payload: serde_json::Value,
) -> sddk_domain::EventEnvelopeV1 {
    use sddk_domain::{ActorKind, ActorRef};
    let mut env = sddk_domain::EventEnvelopeV1 {
        event_id: format!("e-{stream_id}-{sequence}"),
        event_type: event_type.into(),
        schema_version: 1,
        stream_id: stream_id.into(),
        sequence,
        project_id: project_id.into(),
        occurred_at: "2026-08-17T10:00:00Z".into(),
        recorded_at: "2026-08-17T10:00:01Z".into(),
        actor: ActorRef {
            kind: ActorKind::System,
            id: "sddk-cli".into(),
            definition_hash: None,
            policy_hash: None,
            model: None,
        },
        subjects: vec![],
        payload,
        evidence_refs: vec![],
        content_hash: String::new(),
        metadata: None,
        causation_id: None,
        correlation_id: None,
        cycle_id: None,
        frame_id: None,
        fork_id: None,
    };
    env.content_hash = env.compute_content_hash();
    env
}

/// Sets up a temp environment with proper XDG directories and returns:
/// (ApprovalTestEnv { root, state, data, cache, home, project_id, _dir }, run closure)
fn approval_test_setup() -> (ApprovalTestEnv, impl Fn(&[&str]) -> std::process::Output) {
    let tmp = tempfile::tempdir().unwrap();
    let root = tmp.path().join("repo");
    let state = tmp.path().join("state");
    let data = tmp.path().join("data");
    let cache = tmp.path().join("cache");
    let home = tmp.path().join("home");
    std::fs::create_dir_all(&root).unwrap();
    std::fs::create_dir_all(&state).unwrap();
    std::fs::create_dir_all(&data).unwrap();
    std::fs::create_dir_all(&cache).unwrap();
    std::fs::create_dir_all(&home).unwrap();

    let project_id = fallback_project_id("00000000-0000-0000-0000-000000000001", ".");

    // Create the ledger directory so the CLI can open the event store.
    let ledger_dir = state.join("sddk").join("projects").join(&project_id);
    std::fs::create_dir_all(&ledger_dir).unwrap();

    // Clone paths for the closure.
    let home_c = home.clone();
    let data_c = data.clone();
    let state_c = state.clone();
    let cache_c = cache.clone();

    let run = move |args: &[&str]| {
        Command::new(env!("CARGO_BIN_EXE_sddk"))
            .args(args)
            .env("HOME", &home_c)
            .env("XDG_DATA_HOME", &data_c)
            .env("XDG_STATE_HOME", &state_c)
            .env("XDG_CACHE_HOME", &cache_c)
            .output()
            .unwrap()
    };

    let env = ApprovalTestEnv {
        root,
        state,
        project_id,
        _dir: tmp,
    };
    (env, run)
}

struct ApprovalTestEnv {
    root: std::path::PathBuf,
    state: std::path::PathBuf,
    project_id: String,
    _dir: TempDir,
}

/// Opens the event store at the ledger path for the approval test environment.
fn open_test_ledger(env: &ApprovalTestEnv) -> sddk_storage::SqliteEventStore {
    let ledger_dir = env
        .state
        .join("sddk")
        .join("projects")
        .join(&env.project_id);
    sddk_storage::SqliteEventStore::open(&ledger_dir).unwrap()
}

/// Appends an approval.requested event to the test ledger.
fn append_approval_requested(env: &ApprovalTestEnv) {
    let mut store = open_test_ledger(env);
    let event = make_event(
        &env.project_id,
        "c-1",
        "approval.capability.requested",
        1,
        json!({
            "cycle_id": "c-1",
            "capability": "git.delete_branch",
            "request_hash": "sha256:abc1234",
            "expires_at": "2026-08-18T18:00:00Z"
        }),
    );
    store.append(&event).unwrap();
}

/// Appends approval.requested + approval.granted events to the test ledger.
fn append_approval_granted(env: &ApprovalTestEnv) {
    let mut store = open_test_ledger(env);
    store
        .append(&make_event(
            &env.project_id,
            "c-1",
            "approval.capability.requested",
            1,
            json!({
                "cycle_id": "c-1",
                "capability": "git.delete_branch",
                "request_hash": "sha256:abc1234",
                "expires_at": "2026-08-18T18:00:00Z"
            }),
        ))
        .unwrap();
    store
        .append(&make_event(
            &env.project_id,
            "c-1",
            "approval.capability.granted",
            2,
            json!({
                "cycle_id": "c-1",
                "capability": "git.delete_branch",
                "request_hash": "sha256:abc1234",
                "actor": "alice",
                "reason": "ok, reversible via reflog"
            }),
        ))
        .unwrap();
}

// ── Tests ────────────────────────────────────────────────────────────────────────

#[test]
fn cli_approval_list_shows_pending() {
    let (env, run) = approval_test_setup();
    append_approval_requested(&env);

    let out = run(&[
        "approval",
        "list",
        "--root",
        env.root.to_str().unwrap(),
        "--scope",
        ".",
        "--cycle",
        "c-1",
        "--fallback-seed",
        "00000000-0000-0000-0000-000000000001",
    ]);

    assert_eq!(
        out.status.code(),
        Some(0),
        "stderr: {}",
        String::from_utf8_lossy(&out.stderr)
    );
    let stdout = String::from_utf8_lossy(&out.stdout);
    assert!(
        stdout.contains("git.delete_branch"),
        "expected git.delete_branch in output, got: {stdout}"
    );
    assert!(
        stdout.contains("sha256:abc1234"),
        "expected request_hash in output, got: {stdout}"
    );
}

#[test]
fn cli_approval_list_empty_when_no_pending() {
    let (env, run) = approval_test_setup();
    append_approval_granted(&env);

    let out = run(&[
        "approval",
        "list",
        "--root",
        env.root.to_str().unwrap(),
        "--scope",
        ".",
        "--cycle",
        "c-1",
        "--fallback-seed",
        "00000000-0000-0000-0000-000000000001",
    ]);

    assert_eq!(
        out.status.code(),
        Some(0),
        "stderr: {}",
        String::from_utf8_lossy(&out.stderr)
    );
    let stdout = String::from_utf8_lossy(&out.stdout);
    assert!(
        stdout.contains("no pending approvals"),
        "expected 'no pending approvals' in output, got: {stdout}"
    );
}

#[test]
fn cli_approval_grant_resolves_pending() {
    let (env, run) = approval_test_setup();
    append_approval_requested(&env);

    let out = run(&[
        "approval",
        "grant",
        "--root",
        env.root.to_str().unwrap(),
        "--scope",
        ".",
        "--cycle",
        "c-1",
        "--capability",
        "git.delete_branch",
        "--reason",
        "ok approved",
        "--fallback-seed",
        "00000000-0000-0000-0000-000000000001",
    ]);

    assert_eq!(
        out.status.code(),
        Some(0),
        "stderr: {}",
        String::from_utf8_lossy(&out.stderr)
    );
    let stdout = String::from_utf8_lossy(&out.stdout);
    assert!(
        stdout.contains("approval-cap-git-delete_branch-sha256:abc1234-granted"),
        "expected event_id in output, got: {stdout}"
    );
    assert!(
        stdout.contains("decision: granted"),
        "expected 'decision: granted' in output, got: {stdout}"
    );
}

#[test]
fn cli_approval_deny_requires_reason() {
    let (env, run) = approval_test_setup();
    append_approval_requested(&env);

    let out = run(&[
        "approval",
        "deny",
        "--root",
        env.root.to_str().unwrap(),
        "--scope",
        ".",
        "--cycle",
        "c-1",
        "--capability",
        "git.delete_branch",
        "--fallback-seed",
        "00000000-0000-0000-0000-000000000001",
    ]);

    // Should fail because --reason is required.
    assert_ne!(out.status.code(), Some(0), "expected non-zero exit");
}

#[test]
fn cli_approval_grant_idempotent_already_resolved() {
    let (env, run) = approval_test_setup();
    append_approval_requested(&env); // Only add the requested event; grant will add granted

    // First grant should succeed.
    let out = run(&[
        "approval",
        "grant",
        "--root",
        env.root.to_str().unwrap(),
        "--scope",
        ".",
        "--cycle",
        "c-1",
        "--capability",
        "git.delete_branch",
        "--reason",
        "already granted",
        "--fallback-seed",
        "00000000-0000-0000-0000-000000000001",
    ]);
    assert_eq!(
        out.status.code(),
        Some(0),
        "first grant should succeed: {}",
        String::from_utf8_lossy(&out.stderr)
    );

    // Second grant should fail with "already resolved".
    let out2 = run(&[
        "approval",
        "grant",
        "--root",
        env.root.to_str().unwrap(),
        "--scope",
        ".",
        "--cycle",
        "c-1",
        "--capability",
        "git.delete_branch",
        "--reason",
        "trying again",
        "--fallback-seed",
        "00000000-0000-0000-0000-000000000001",
    ]);
    assert_ne!(out2.status.code(), Some(0), "second grant should fail");
    let stderr2 = String::from_utf8_lossy(&out2.stderr);
    assert!(
        stderr2.contains("already resolved")
            || stderr2.contains("approval already resolved")
            || stderr2.contains("no pending approval"),
        "expected 'already resolved' or 'no pending approval' error, got: {stderr2}"
    );
}
