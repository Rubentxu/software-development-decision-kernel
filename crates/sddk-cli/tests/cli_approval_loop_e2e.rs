//! End-to-end tests for the M2 approval loop (WU-C4-7 / R-4-002).
//!
//! Exercises the full loop: blocking `RequireApproval` emits
//! `approval.capability.requested` (S1), `sddk approval grant` writes the
//! granted event (S2), and the next `admit_governed` for the same
//! `(surface, action, target, actor)` proceeds without needing the approval
//! gate (S3 — grant pre-injected via `ApprovalProjection`).
//!
//! Uses `cli_approval_e2e.rs`'s setup pattern: XDG env vars redirect to
//! temp directories so the CLI uses the same storage as the test.

use sddk_domain::EventStore;
use serde_json::json;
use sha2::Digest;
use sha2::Sha256;
use std::process::Command;
use tempfile::TempDir;

/// Stable fallback project id (matches `cli_approval_e2e.rs`).
fn fallback_project_id(seed: &str, scope: &str) -> String {
    let hex = {
        let mut hasher = Sha256::new();
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

struct LoopTestEnv {
    root: std::path::PathBuf,
    state: std::path::PathBuf,
    project_id: String,
    _dir: TempDir,
}

fn loop_test_setup() -> (LoopTestEnv, impl Fn(&[&str]) -> std::process::Output) {
    let tmp = tempfile::tempdir().unwrap();
    let root = tmp.path().join("repo");
    let state = tmp.path().join("state");
    let data = tmp.path().join("data");
    let cache = tmp.path().join("cache");
    let home = tmp.path().join("home");
    for d in [&root, &state, &data, &cache, &home] {
        std::fs::create_dir_all(d).unwrap();
    }
    let project_id = fallback_project_id("00000000-0000-0000-0000-0000000000a2", ".");
    let ledger_dir = state.join("sddk").join("projects").join(&project_id);
    std::fs::create_dir_all(&ledger_dir).unwrap();

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
            .env("USER", "alice")
            .env("SDDK_ACTOR", "user:alice")
            .output()
            .unwrap()
    };
    let env = LoopTestEnv {
        root,
        state,
        project_id,
        _dir: tmp,
    };
    (env, run)
}

fn open_ledger(env: &LoopTestEnv) -> sddk_storage::SqliteEventStore {
    let ledger_dir = env
        .state
        .join("sddk")
        .join("projects")
        .join(&env.project_id);
    sddk_storage::SqliteEventStore::open(&ledger_dir).unwrap()
}

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
        occurred_at: "2026-09-14T10:00:00Z".into(),
        recorded_at: "2026-09-14T10:00:01Z".into(),
        actor: ActorRef {
            kind: ActorKind::System,
            id: "test".into(),
            definition_hash: None,
            policy_hash: None,
            model: None,
            role: None,
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

fn count_events(env: &LoopTestEnv, event_type: &str) -> i64 {
    let store = open_ledger(env);
    let events: Vec<sddk_domain::EventEnvelopeV1> = store
        .load_stream("c-m2-loop", None, u32::MAX)
        .unwrap_or_default();
    events.iter().filter(|e| e.event_type == event_type).count() as i64
}

/// End-to-end M2 approval loop: S1 emit request → S2 grant → S3 admit proceeds.
///
/// The loop is exercised through the CLI's `sddk approval grant` (S2)
/// against a pre-seeded `approval.capability.requested` event. The S3 step
/// is verified by direct ledger inspection: the `ApprovalProjection` would
/// see the granted event and inject `Facts::approval_refs` into the next
/// `admit_governed` (unit-tested in `admission.rs::admission_tests`).
#[test]
fn m2_approval_loop_request_grant_list_empty() {
    let (env, run) = loop_test_setup();
    let mut store = open_ledger(&env);

    // S1: pre-seed approval.capability.requested (S1 emission is verified
    // by the unit tests in `admission.rs::admission_tests::emits_approval_requested_for_*`
    // and the deny e2e coverage; here we exercise S2+S3 through the CLI).
    store
        .append(&make_event(
            &env.project_id,
            "c-m2-loop",
            "approval.capability.requested",
            1,
            json!({
                "cycle_id": "c-m2-loop",
                "capability": "surface.knowledge_graph_vault#vault_index",
                "request_hash": "sha256:m2loop-request-hash",
                "expires_at": "2026-09-15T10:00:00Z",
                "actor_id": "agent:user:alice",
                "actor_kind": "agent"
            }),
        ))
        .unwrap();

    // Sanity: list should show one pending approval.
    let list_out = run(&[
        "approval",
        "list",
        "--root",
        env.root.to_str().unwrap(),
        "--scope",
        ".",
        "--cycle",
        "c-m2-loop",
        "--fallback-seed",
        "00000000-0000-0000-0000-0000000000a2",
    ]);
    assert_eq!(
        list_out.status.code(),
        Some(0),
        "stderr: {}",
        String::from_utf8_lossy(&list_out.stderr)
    );
    let stdout = String::from_utf8_lossy(&list_out.stdout);
    assert!(
        stdout.contains("sha256:m2loop-request-hash"),
        "expected pending approval in list, got: {stdout}"
    );

    // S2: grant the approval via the CLI.
    let grant_out = run(&[
        "approval",
        "grant",
        "--root",
        env.root.to_str().unwrap(),
        "--scope",
        ".",
        "--cycle",
        "c-m2-loop",
        "--capability",
        "surface.knowledge_graph_vault#vault_index",
        "--reason",
        "M2 loop test grant",
        "--fallback-seed",
        "00000000-0000-0000-0000-0000000000a2",
    ]);
    assert_eq!(
        grant_out.status.code(),
        Some(0),
        "stderr: {}",
        String::from_utf8_lossy(&grant_out.stderr)
    );

    // After granting, list should be empty.
    let list_out2 = run(&[
        "approval",
        "list",
        "--root",
        env.root.to_str().unwrap(),
        "--scope",
        ".",
        "--cycle",
        "c-m2-loop",
        "--fallback-seed",
        "00000000-0000-0000-0000-0000000000a2",
    ]);
    let stdout2 = String::from_utf8_lossy(&list_out2.stdout);
    assert!(
        stdout2.contains("no pending approvals"),
        "expected empty list after grant, got: {stdout2}"
    );

    // Ledger must contain exactly one requested + one granted event.
    assert_eq!(count_events(&env, "approval.capability.requested"), 1);
    assert_eq!(count_events(&env, "approval.capability.granted"), 1);

    // S3 (contract): ApprovalProjection on the granted state would return
    // the granted evidence ref. This is the input `Facts::approval_refs`
    // for `admit_governed`'s next call — verified at unit level in
    // `admission.rs::admission_tests::granted_approval_refs_*`. Here we
    // pin the ledger invariant: granted event exists and matches the
    // request hash.
    let store = open_ledger(&env);
    let events: Vec<sddk_domain::EventEnvelopeV1> =
        store.load_stream("c-m2-loop", None, u32::MAX).unwrap();
    let granted = events
        .iter()
        .find(|e| e.event_type == "approval.capability.granted")
        .expect("granted event must exist");
    let request_hash = granted
        .payload
        .get("request_hash")
        .and_then(|v| v.as_str())
        .expect("granted event must carry request_hash");
    assert_eq!(request_hash, "sha256:m2loop-request-hash");
}

/// Idempotent denial: denying an already-granted approval must fail closed.
#[test]
fn m2_approval_deny_after_grant_is_rejected() {
    let (env, run) = loop_test_setup();
    let mut store = open_ledger(&env);
    store
        .append(&make_event(
            &env.project_id,
            "c-m2-loop",
            "approval.capability.requested",
            1,
            json!({
                "cycle_id": "c-m2-loop",
                "capability": "git.delete_branch",
                "request_hash": "sha256:deny-test",
                "expires_at": "2026-09-15T10:00:00Z",
                "actor_id": "agent:user:alice",
                "actor_kind": "agent"
            }),
        ))
        .unwrap();

    let grant_out = run(&[
        "approval",
        "grant",
        "--root",
        env.root.to_str().unwrap(),
        "--scope",
        ".",
        "--cycle",
        "c-m2-loop",
        "--capability",
        "git.delete_branch",
        "--reason",
        "first grant",
        "--fallback-seed",
        "00000000-0000-0000-0000-0000000000a2",
    ]);
    assert_eq!(
        grant_out.status.code(),
        Some(0),
        "first grant must succeed: {}",
        String::from_utf8_lossy(&grant_out.stderr)
    );

    // Deny after grant must be rejected (approval already resolved).
    let deny_out = run(&[
        "approval",
        "deny",
        "--root",
        env.root.to_str().unwrap(),
        "--scope",
        ".",
        "--cycle",
        "c-m2-loop",
        "--capability",
        "git.delete_branch",
        "--reason",
        "trying to deny",
        "--fallback-seed",
        "00000000-0000-0000-0000-0000000000a2",
    ]);
    assert_ne!(
        deny_out.status.code(),
        Some(0),
        "deny after grant must fail closed"
    );
    let stderr = String::from_utf8_lossy(&deny_out.stderr);
    assert!(
        stderr.contains("already resolved")
            || stderr.contains("no pending approval")
            || stderr.contains("approval already resolved"),
        "expected already-resolved error, got: {stderr}"
    );
}
