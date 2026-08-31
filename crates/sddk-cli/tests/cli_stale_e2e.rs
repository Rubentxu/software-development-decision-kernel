//! Integration tests for `sddk stale` and `sddk graph why-stale` (phase6).
//!
//! Seeds CEP events with `verifies`/`modified` subjects into `events_v1`,
//! rebuilds the graph, then exercises stale list, impact, why-stale, and gate.

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

struct StaleTestEnv {
    _dir: TempDir,
}

fn stale_test_setup() -> (StaleTestEnv, impl Fn(&[&str]) -> std::process::Output) {
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

    // Seed CEP events: verifies edge + later modification.
    let ledger_dir = state.join("sddk").join("projects").join(&project_id);
    std::fs::create_dir_all(&ledger_dir).unwrap();
    {
        let mut store = sddk_storage::event_store::SqliteEventStore::open(&ledger_dir).unwrap();
        #[allow(clippy::type_complexity)]
        let events: Vec<(String, u64, Vec<(&str, &str)>, serde_json::Value)> = vec![
            (
                "uat.acceptance.verified".into(),
                1,
                vec![("test", "T1"), ("requirement", "R1")],
                json!({}),
            ),
            (
                "requirement.modified".into(),
                2,
                vec![("requirement", "R1")],
                json!({}),
            ),
            (
                "task.depends_on".into(),
                3,
                vec![("task", "A"), ("task", "B")],
                json!({}),
            ),
            ("task.blocked".into(), 4, vec![("task", "B")], json!({})),
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

    let run = move |args: &[&str]| {
        Command::new(env!("CARGO_BIN_EXE_sddk"))
            .args(args)
            .env("HOME", &home_c)
            .env("XDG_DATA_HOME", &data_c)
            .env("XDG_STATE_HOME", &state_c)
            .env("XDG_CACHE_HOME", &cache_c)
            .current_dir(&root_c)
            .output()
            .unwrap()
    };

    (StaleTestEnv { _dir: tmp }, run)
}

#[allow(dead_code)]
const ROOT_ARGS: &[&str] = &[
    "--root",
    ".",
    "--scope",
    ".",
    "--fallback-seed",
    "00000000-0000-0000-0000-000000000001",
];

#[test]
fn stale_list_shows_possibly_stale_with_path() {
    let (_env, run) = stale_test_setup();
    let out = run(&[
        "graph",
        "rebuild",
        "--root",
        ".",
        "--scope",
        ".",
        "--fallback-seed",
        "00000000-0000-0000-0000-000000000001",
    ]);
    assert_eq!(
        out.status.code(),
        Some(0),
        "rebuild: {}",
        String::from_utf8_lossy(&out.stderr)
    );

    let out = run(&[
        "stale",
        "list",
        "--root",
        ".",
        "--scope",
        ".",
        "--fallback-seed",
        "00000000-0000-0000-0000-000000000001",
    ]);
    let stdout = String::from_utf8_lossy(&out.stdout);
    assert_eq!(
        out.status.code(),
        Some(0),
        "stale: {}",
        String::from_utf8_lossy(&out.stderr)
    );
    // requirement:R1 is verified then modified → possibly_stale with evt-2.
    assert!(stdout.contains("requirement:R1"), "got: {stdout}");
    assert!(stdout.contains("possibly_stale"), "got: {stdout}");
    assert!(stdout.contains("evt-2"), "got: {stdout}");
}

#[test]
fn stale_impact_lists_reachable_nodes() {
    let (_env, run) = stale_test_setup();
    let out = run(&[
        "graph",
        "rebuild",
        "--root",
        ".",
        "--scope",
        ".",
        "--fallback-seed",
        "00000000-0000-0000-0000-000000000001",
    ]);
    assert_eq!(out.status.code(), Some(0));

    // task:A --depends_on--> task:B --blocked--> task:B
    let out = run(&[
        "stale",
        "impact",
        "--entity",
        "task:A",
        "--root",
        ".",
        "--scope",
        ".",
        "--fallback-seed",
        "00000000-0000-0000-0000-000000000001",
    ]);
    let stdout = String::from_utf8_lossy(&out.stdout);
    assert_eq!(
        out.status.code(),
        Some(0),
        "impact: {}",
        String::from_utf8_lossy(&out.stderr)
    );
    assert!(stdout.contains("task:B"), "got: {stdout}");
}

#[test]
fn graph_why_stale_shows_causal_path() {
    let (_env, run) = stale_test_setup();
    let out = run(&[
        "graph",
        "rebuild",
        "--root",
        ".",
        "--scope",
        ".",
        "--fallback-seed",
        "00000000-0000-0000-0000-000000000001",
    ]);
    assert_eq!(out.status.code(), Some(0));

    let out = run(&[
        "graph",
        "why-stale",
        "--entity",
        "requirement:R1",
        "--root",
        ".",
        "--scope",
        ".",
        "--fallback-seed",
        "00000000-0000-0000-0000-000000000001",
    ]);
    let stdout = String::from_utf8_lossy(&out.stdout);
    assert_eq!(
        out.status.code(),
        Some(0),
        "why-stale: {}",
        String::from_utf8_lossy(&out.stderr)
    );
    assert!(stdout.contains("state: possibly_stale"), "got: {stdout}");
    assert!(stdout.contains("evt-2"), "got: {stdout}");
}

#[test]
fn stale_gate_fails_on_stale_critical() {
    let (_env, run) = stale_test_setup();
    let out = run(&[
        "graph",
        "rebuild",
        "--root",
        ".",
        "--scope",
        ".",
        "--fallback-seed",
        "00000000-0000-0000-0000-000000000001",
    ]);
    assert_eq!(out.status.code(), Some(0));

    // requirement:R1 is possibly_stale (not hard stale) — advisory by default.
    let out = run(&[
        "stale",
        "gate",
        "--critical",
        "requirement:R1",
        "--root",
        ".",
        "--scope",
        ".",
        "--fallback-seed",
        "00000000-0000-0000-0000-000000000001",
    ]);
    let stdout = String::from_utf8_lossy(&out.stdout);
    // PossiblyStale is advisory (warn) → gate passes by default.
    assert_eq!(
        out.status.code(),
        Some(0),
        "gate should pass advisory, got: {stdout}"
    );

    // With advisory=fail, possibly_stale critical → gate fails.
    let out = run(&[
        "stale",
        "gate",
        "--critical",
        "requirement:R1",
        "--advisory",
        "fail",
        "--root",
        ".",
        "--scope",
        ".",
        "--fallback-seed",
        "00000000-0000-0000-0000-000000000001",
    ]);
    assert_ne!(
        out.status.code(),
        Some(0),
        "gate should fail with advisory=fail"
    );
}

#[test]
fn uat_stale_help_intact() {
    let (_env, run) = stale_test_setup();
    // `uat stale --help` must still be a recognized subcommand (surface intact).
    let out = run(&["uat", "stale", "--help"]);
    let combined = format!(
        "{}{}",
        String::from_utf8_lossy(&out.stdout),
        String::from_utf8_lossy(&out.stderr)
    );
    assert_eq!(
        out.status.code(),
        Some(0),
        "uat stale must remain, stderr: {}",
        String::from_utf8_lossy(&out.stderr)
    );
    assert!(
        combined.to_lowercase().contains("staleness") || combined.contains("stale"),
        "expected staleness help, got: {combined}"
    );
}
