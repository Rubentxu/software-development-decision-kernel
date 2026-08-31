//! Integration tests for `sddk fork` CLI commands (phase7).
//!
//! Seeds CEP events, creates forks at specific events, runs replay, diffs
//! against the parent, and verifies fail-closed promotion.

use sddk_domain::{ActorKind, ActorRef, EntityRef, EventEnvelopeV1, EventStore};
use serde_json::json;
use std::process::Command;
use tempfile::TempDir;

fn fallback_project_id(seed: &str, scope: &str) -> String {
    sddk_domain::resolve_project_identity(None, scope, Some(seed))
        .expect("valid fallback seed")
        .project_id
        .to_string()
}

struct ForkTestEnv {
    _dir: TempDir,
}

fn fork_test_setup() -> (ForkTestEnv, impl Fn(&[&str]) -> std::process::Output) {
    let tmp = tempfile::tempdir().unwrap();
    let root = tmp.path().join("repo");
    let state = tmp.path().join("state");
    let data = tmp.path().join("data");
    let cache = tmp.path().join("cache");
    let home = tmp.path().join("home");
    for dir in [&root, &state, &data, &cache, &home] {
        std::fs::create_dir_all(dir).unwrap();
    }
    let _ = Command::new("git")
        .args(["init", "-q"])
        .current_dir(&root)
        .output();

    let project_id = fallback_project_id("00000000-0000-0000-0000-000000000001", ".");
    let stream = format!("project:{project_id}");

    let ledger_dir = state.join("sddk").join("projects").join(&project_id);
    std::fs::create_dir_all(&ledger_dir).unwrap();
    {
        let mut store = sddk_storage::event_store::SqliteEventStore::open(&ledger_dir).unwrap();
        #[allow(clippy::type_complexity)]
        let events: Vec<(String, u64, Vec<(&str, &str)>, serde_json::Value)> = vec![
            (
                "approval.capability.requested".into(),
                1,
                vec![("cycle", "c-1"), ("capability", "git.commit")],
                json!({}),
            ),
            (
                "approval.capability.granted".into(),
                2,
                vec![("actor", "alice"), ("capability", "git.commit")],
                json!({}),
            ),
            (
                "workflow.phase.entered".into(),
                3,
                vec![],
                json!({ "phase": "verify" }),
            ),
            ("workflow.phase.exited".into(), 4, vec![], json!({})),
        ];
        for (event_type, seq, subjects, payload) in events {
            let envelope = EventEnvelopeV1 {
                event_id: format!("evt-{seq}"),
                event_type,
                schema_version: 1,
                stream_id: stream.clone(),
                sequence: seq,
                project_id: project_id.clone(),
                occurred_at: format!("2026-08-18T10:00:{seq:02}Z"),
                recorded_at: format!("2026-08-18T10:00:{seq:02}Z"),
                actor: ActorRef {
                    kind: ActorKind::System,
                    id: "sddk-test".into(),
                    definition_hash: None,
                    policy_hash: None,
                    model: None,
                },
                subjects: subjects
                    .into_iter()
                    .map(|(kind, id)| EntityRef {
                        kind: kind.into(),
                        id: id.into(),
                        version: None,
                        content_hash: None,
                    })
                    .collect(),
                payload,
                evidence_refs: vec![],
                content_hash: String::new(),
                metadata: None,
                causation_id: None,
                correlation_id: None,
                cycle_id: Some("c-1".into()),
                frame_id: None,
                fork_id: None,
            };
            let hash = envelope.compute_content_hash();
            let mut final_envelope = envelope;
            final_envelope.content_hash = hash;
            store.append(&final_envelope).unwrap();
        }
    }

    let home_c = home.clone();
    let data_c = data.clone();
    let state_c = state.clone();
    let cache_c = cache.clone();
    let root_c = root.clone();

    // Append the identity args to every invocation.
    let run = move |args: &[&str]| {
        let mut full: Vec<&str> = args.to_vec();
        full.extend_from_slice(&[
            "--root",
            ".",
            "--scope",
            ".",
            "--fallback-seed",
            "00000000-0000-0000-0000-000000000001",
        ]);
        Command::new(env!("CARGO_BIN_EXE_sddk"))
            .args(&full)
            .env("HOME", &home_c)
            .env("XDG_DATA_HOME", &data_c)
            .env("XDG_STATE_HOME", &state_c)
            .env("XDG_CACHE_HOME", &cache_c)
            .current_dir(&root_c)
            .output()
            .unwrap()
    };

    (ForkTestEnv { _dir: tmp }, run)
}

#[allow(dead_code)]
const SEED_ARGS: &[&str] = &[
    "--root",
    ".",
    "--scope",
    ".",
    "--fallback-seed",
    "00000000-0000-0000-0000-000000000001",
];

#[test]
fn fork_create_run_diff_roundtrip() {
    let (_env, run) = fork_test_setup();

    // Create fork at evt-2.
    let out = run(&[
        "fork",
        "create",
        "--fork-id",
        "f-1",
        "--at",
        "evt-2",
        "--label",
        "exp",
    ]);
    let stdout = String::from_utf8_lossy(&out.stdout);
    assert_eq!(
        out.status.code(),
        Some(0),
        "create: {}",
        String::from_utf8_lossy(&out.stderr)
    );
    assert!(stdout.contains("at_sequence: 2"), "got: {stdout}");
    assert!(
        stdout.contains("shared_prefix_hash: sha256:"),
        "got: {stdout}"
    );

    // Run (reconstruct prefix 1..=2).
    let out = run(&["fork", "run", "--fork-id", "f-1"]);
    let stdout = String::from_utf8_lossy(&out.stdout);
    assert_eq!(
        out.status.code(),
        Some(0),
        "run: {}",
        String::from_utf8_lossy(&out.stderr)
    );
    assert!(stdout.contains("events_applied: 2"), "got: {stdout}");

    // Diff parent (1..=1) vs fork (1..=2) — evt-2 adds actor + edge.
    let out = run(&["fork", "diff", "--fork-id", "f-1"]);
    let stdout = String::from_utf8_lossy(&out.stdout);
    assert_eq!(
        out.status.code(),
        Some(0),
        "diff: {}",
        String::from_utf8_lossy(&out.stderr)
    );
    assert!(
        stdout.contains("nodes_added") || stdout.contains("edges_changed"),
        "expected diff content, got: {stdout}"
    );
    assert!(stdout.contains("actor:alice"), "got: {stdout}");
}

#[test]
fn fork_promote_passes_when_parent_unchanged() {
    let (_env, run) = fork_test_setup();
    let out = run(&["fork", "create", "--fork-id", "f-1", "--at", "evt-4"]);
    assert_eq!(
        out.status.code(),
        Some(0),
        "create: {}",
        String::from_utf8_lossy(&out.stderr)
    );

    // Parent head unchanged → promote OK.
    let out = run(&["fork", "promote", "--fork-id", "f-1"]);
    let stdout = String::from_utf8_lossy(&out.stdout);
    assert_eq!(
        out.status.code(),
        Some(0),
        "promote should pass, stderr: {}",
        String::from_utf8_lossy(&out.stderr)
    );
    assert!(stdout.contains("promoted: true"), "got: {stdout}");
}

#[test]
fn fork_promote_fails_closed_when_parent_changed() {
    let (_env, run) = fork_test_setup();
    let out = run(&["fork", "create", "--fork-id", "f-1", "--at", "evt-2"]);
    assert_eq!(
        out.status.code(),
        Some(0),
        "create: {}",
        String::from_utf8_lossy(&out.stderr)
    );

    // Append a new event to the parent stream after the fork point.
    let out = run(&["fork", "create", "--fork-id", "f-2", "--at", "evt-4"]);
    assert_eq!(
        out.status.code(),
        Some(0),
        "second create: {}",
        String::from_utf8_lossy(&out.stderr)
    );

    // f-1 was created at evt-2; parent now has evt-3/evt-4 → head changed.
    let out = run(&["fork", "promote", "--fork-id", "f-1"]);
    assert_ne!(
        out.status.code(),
        Some(0),
        "promote must fail closed, stdout: {}",
        String::from_utf8_lossy(&out.stdout)
    );
    let stderr = String::from_utf8_lossy(&out.stderr);
    assert!(stderr.contains("parent changed"), "got: {stderr}");
}

#[test]
fn fork_create_rejects_duplicate() {
    let (_env, run) = fork_test_setup();
    let out = run(&["fork", "create", "--fork-id", "f-1", "--at", "evt-1"]);
    assert_eq!(out.status.code(), Some(0));
    let out = run(&["fork", "create", "--fork-id", "f-1", "--at", "evt-2"]);
    assert_ne!(out.status.code(), Some(0), "duplicate must be rejected");
}
