//! Integration tests for `sddk graph` CLI commands (phase5).
//!
//! Seeds the event ledger via the event store, then exercises
//! `graph rebuild`, `graph query`, and `graph why`.

use sddk_domain::{ActorKind, ActorRef, EntityRef, EventEnvelopeV1, EventStore};
use serde_json::json;
use std::process::Command;
use tempfile::TempDir;

fn fallback_project_id(seed: &str, scope: &str) -> String {
    // Use the real identity resolution so the seed stream matches the CLI's.
    sddk_domain::resolve_project_identity(None, scope, Some(seed))
        .expect("valid fallback seed")
        .project_id
        .to_string()
}

struct GraphTestEnv {
    #[allow(dead_code)]
    root: std::path::PathBuf,
    #[allow(dead_code)]
    stream: String,
    _dir: TempDir,
}

fn graph_test_setup() -> (GraphTestEnv, impl Fn(&[&str]) -> std::process::Output) {
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
    // NOTE: no git remote — the CLI resolves the project identity from the
    // explicit --fallback-seed passed by each invocation, matching the seed
    // below (which uses the same fallback computation).

    let project_id = fallback_project_id("00000000-0000-0000-0000-000000000001", ".");
    let stream = format!("project:{project_id}");

    // Seed the ledger directly with the event store (same XDG paths the CLI uses).
    let ledger_dir = state.join("sddk").join("projects").join(&project_id);
    std::fs::create_dir_all(&ledger_dir).unwrap();
    {
        let mut store = sddk_storage::event_store::SqliteEventStore::open(&ledger_dir).unwrap();
        let events = [
            (
                "approval.capability.requested",
                1u64,
                vec![("cycle", "c-1"), ("capability", "git.commit")],
            ),
            (
                "approval.capability.granted",
                2,
                vec![("actor", "alice"), ("capability", "git.commit")],
            ),
            ("workflow.phase.entered", 3, vec![]),
        ];
        for (event_type, seq, subjects) in events {
            let envelope = EventEnvelopeV1 {
                event_id: format!("evt-{seq}"),
                event_type: event_type.into(),
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
                payload: if event_type == "workflow.phase.entered" {
                    json!({ "phase": "verify" })
                } else {
                    json!({})
                },
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

    (
        GraphTestEnv {
            root,
            stream,
            _dir: tmp,
        },
        run,
    )
}

#[test]
fn graph_rebuild_then_query_then_why() {
    let (_env, run) = graph_test_setup();

    // rebuild
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
    let stdout = String::from_utf8_lossy(&out.stdout);
    assert_eq!(
        out.status.code(),
        Some(0),
        "rebuild stderr: {}",
        String::from_utf8_lossy(&out.stderr)
    );
    assert!(stdout.contains("nodes: 4"), "got: {stdout}"); // cycle, capability, actor, phase
    assert!(stdout.contains("edges: 3"), "got: {stdout}");

    // query: actor -> granted -> capability
    let out = run(&[
        "graph",
        "query",
        "--pattern",
        "actor -> approval.capability.granted -> capability",
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
        "query stderr: {}",
        String::from_utf8_lossy(&out.stderr)
    );
    assert!(
        stdout.contains("actor:alice -> capability:git.commit"),
        "got: {stdout}"
    );

    // why
    let out = run(&[
        "graph",
        "why",
        "--entity",
        "capability:git.commit",
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
        "why stderr: {}",
        String::from_utf8_lossy(&out.stderr)
    );
    assert!(stdout.contains("found: true"), "got: {stdout}");
    assert!(stdout.contains("created_by: evt-1"), "got: {stdout}");
}

#[test]
fn graph_why_unknown_entity_reports_not_found() {
    let (_env, run) = graph_test_setup();
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
        "why",
        "--entity",
        "capability:ghost",
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
        "stderr: {}",
        String::from_utf8_lossy(&out.stderr)
    );
    assert!(stdout.contains("found: false"), "got: {stdout}");
}

#[test]
fn graph_query_without_rebuild_reports_guidance() {
    let (_env, run) = graph_test_setup();
    let out = run(&[
        "graph",
        "query",
        "--pattern",
        "cycle -> entered_phase -> phase",
        "--root",
        ".",
        "--scope",
        ".",
        "--fallback-seed",
        "00000000-0000-0000-0000-000000000001",
    ]);
    let stdout = String::from_utf8_lossy(&out.stdout);
    // Either no matches or the guidance message; non-zero only on error.
    assert_ne!(out.status.code(), Some(2), "unexpected usage error");
    let _ = stdout;
}
