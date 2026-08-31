//! Integration tests for `sddk explore` (phase8).

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

struct ExploreTestEnv {
    _dir: TempDir,
    tmp: std::path::PathBuf,
}

fn explore_test_setup() -> (ExploreTestEnv, impl Fn(&[&str]) -> std::process::Output) {
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
    let tmp_out = tmp.path().join("out");
    std::fs::create_dir_all(&tmp_out).unwrap();

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

    (
        ExploreTestEnv {
            _dir: tmp,
            tmp: tmp_out,
        },
        run,
    )
}

#[test]
fn explore_graph_renders_html_with_entity() {
    let (env, run) = explore_test_setup();
    let out_html = env.tmp.join("graph.html");
    let out = run(&[
        "explore",
        "render",
        "--view",
        "graph",
        "--entity",
        "cycle:c-1",
        "--out",
        out_html.to_str().unwrap(),
    ]);
    assert_eq!(
        out.status.code(),
        Some(0),
        "stderr: {}",
        String::from_utf8_lossy(&out.stderr)
    );
    let html = std::fs::read_to_string(&out_html).unwrap();
    assert!(
        html.contains("<html"),
        "expected html, got len {}",
        html.len()
    );
    assert!(html.contains("cycle:c-1"), "entity must appear");
    assert!(
        html.contains("capability:git.commit"),
        "reachable node must appear"
    );
}

#[test]
fn explore_timeline_renders_events() {
    let (env, run) = explore_test_setup();
    let out_html = env.tmp.join("timeline.html");
    let out = run(&[
        "explore",
        "render",
        "--view",
        "timeline",
        "--out",
        out_html.to_str().unwrap(),
    ]);
    assert_eq!(
        out.status.code(),
        Some(0),
        "stderr: {}",
        String::from_utf8_lossy(&out.stderr)
    );
    let html = std::fs::read_to_string(&out_html).unwrap();
    assert!(html.contains("evt-1"), "event 1 must appear");
    assert!(html.contains("evt-3"), "event 3 must appear");
}

#[test]
fn explore_verification_renders() {
    let (env, run) = explore_test_setup();
    let out_html = env.tmp.join("verif.html");
    let out = run(&[
        "explore",
        "render",
        "--view",
        "verification",
        "--out",
        out_html.to_str().unwrap(),
    ]);
    assert_eq!(
        out.status.code(),
        Some(0),
        "stderr: {}",
        String::from_utf8_lossy(&out.stderr)
    );
    let html = std::fs::read_to_string(&out_html).unwrap();
    assert!(html.contains("SDDK Explorer"), "title must render");
}

#[test]
fn explore_unknown_view_errors() {
    let (_env, run) = explore_test_setup();
    let out = run(&["explore", "render", "--view", "nope"]);
    assert_ne!(out.status.code(), Some(0), "unknown view must error");
    let stderr = String::from_utf8_lossy(&out.stderr);
    assert!(stderr.contains("unknown view"), "got: {stderr}");
}
