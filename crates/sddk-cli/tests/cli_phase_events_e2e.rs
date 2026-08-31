//! E2E tests for workflow.phase event emission on cycle transitions.
//!
//! Verifies that `sddk cycle transition` emits `workflow.phase.exited` and
//! `workflow.phase.entered` events to the events_v1 table.
//!
//! MS-05: Uses the full gate evaluation workflow (evaluate-gate → transition).

use sddk_domain::EventStore;
use serde_json::Value;

const TIMESTAMP: &str = "2026-08-18T10:00:00Z";

/// E2E test: cycle transition emits workflow.phase.exited + workflow.phase.entered.
#[test]
fn e2e_transition_emits_phase_events() {
    // Set up isolated XDG dirs.
    let tmp = tempfile::tempdir().unwrap();
    let root = tmp.path().join("repo");
    let state = tmp.path().join("state");
    let data = tmp.path().join("data");
    let cache = tmp.path().join("cache");
    std::fs::create_dir_all(&root).unwrap();
    std::fs::create_dir_all(state.join("sddk")).unwrap();

    // Run CLI with isolated XDG.
    let run = |args: &[&str]| {
        std::process::Command::new(env!("CARGO_BIN_EXE_sddk"))
            .args(args)
            .env("HOME", tmp.path())
            .env("XDG_DATA_HOME", &data)
            .env("XDG_STATE_HOME", &state)
            .env("XDG_CACHE_HOME", &cache)
            .output()
            .unwrap()
    };

    // Initialize git repo so adopt works.
    std::process::Command::new("git")
        .args(["init", "--initial-branch=main"])
        .current_dir(&root)
        .output()
        .unwrap();
    std::process::Command::new("git")
        .args(["config", "user.email", "test@test.com"])
        .current_dir(&root)
        .output()
        .unwrap();
    std::process::Command::new("git")
        .args(["config", "user.name", "test"])
        .current_dir(&root)
        .output()
        .unwrap();
    std::process::Command::new("git")
        .args(["add", "."])
        .current_dir(&root)
        .output()
        .unwrap();
    std::process::Command::new("git")
        .args(["commit", "-m", "init"])
        .current_dir(&root)
        .output()
        .unwrap();

    // Adopt project.
    let adopt_out = run(&[
        "adopt",
        "apply",
        "--root",
        root.to_str().unwrap(),
        "--scope",
        ".",
        "--timestamp",
        TIMESTAMP,
        "--actor",
        "test",
        "--format",
        "json",
    ]);
    assert!(
        adopt_out.status.success(),
        "adopt failed: {}",
        String::from_utf8_lossy(&adopt_out.stderr)
    );
    let proj: Value = serde_json::from_slice(&adopt_out.stdout).unwrap();
    let project_id = proj["project_id"].as_str().unwrap();

    // Start cycle.
    let start_out = run(&[
        "cycle",
        "start",
        "--root",
        root.to_str().unwrap(),
        "--scope",
        ".",
        "--name",
        "phase-events-test",
        "--timestamp",
        TIMESTAMP,
        "--actor",
        "test",
        "--format",
        "json",
    ]);
    assert!(
        start_out.status.success(),
        "cycle start failed: {}",
        String::from_utf8_lossy(&start_out.stderr)
    );
    let started: Value = serde_json::from_slice(&start_out.stdout).unwrap();
    let cycle_id = started["cycle_id"].as_str().unwrap();

    // Evaluate gate exploration-sufficient to get a receipt.
    let gate_out = run(&[
        "cycle",
        "evaluate-gate",
        "--root",
        root.to_str().unwrap(),
        "--scope",
        ".",
        "--cycle",
        cycle_id,
        "--transition",
        "phase.explore.complete",
        "--gate",
        "exploration-sufficient",
        "--outcome",
        "passed",
        "--evidence",
        r#"{"argv":["cargo","test","--workspace","--locked"],"exit_code":0,"output_digest":"sha256:5e9d4600f9ae6feccfb09bcea2ac7d94aaf47b00e1f72717b3bbb26e5b64f1ee"}"#,
        "--timestamp",
        TIMESTAMP,
        "--actor",
        "test",
        "--format",
        "json",
    ]);
    assert!(
        gate_out.status.success(),
        "gate evaluate failed: {}",
        String::from_utf8_lossy(&gate_out.stderr)
    );
    let receipt: Value = serde_json::from_slice(&gate_out.stdout).unwrap();
    let receipt_id = receipt["receipt_id"].as_str().unwrap();

    // Transition explore -> specify using the gate receipt.
    let trans_out = run(&[
        "cycle",
        "transition",
        "--root",
        root.to_str().unwrap(),
        "--scope",
        ".",
        "--cycle",
        cycle_id,
        "--transition",
        "phase.explore.complete",
        "--artifact",
        "exploration-report=/dev/null",
        "--gate-receipt",
        receipt_id,
        "--timestamp",
        TIMESTAMP,
        "--actor",
        "test",
        "--format",
        "json",
    ]);
    assert!(
        trans_out.status.success(),
        "transition failed: {}",
        String::from_utf8_lossy(&trans_out.stderr)
    );

    // Open the event store and verify the phase events.
    let ledger_path = state.join("sddk").join("projects").join(project_id);
    let store = sddk_storage::SqliteEventStore::open(&ledger_path).unwrap();

    // Load the stream for this cycle.
    let events = store.load_stream(cycle_id, None, 100).unwrap();

    // Should have 3 events: transition.succeeded + phase.exited + phase.entered.
    assert!(
        !events.is_empty(),
        "events_v1 should have events for cycle {cycle_id}"
    );
    assert_eq!(
        events.len(),
        3,
        "expected 3 events (succeeded + exited + entered), got {}",
        events.len()
    );

    // Events are ordered by sequence: succeeded (1), exited (2), entered (3).
    let succeeded = &events[0];
    let exited = &events[1];
    let entered = &events[2];

    assert_eq!(
        succeeded.event_type, "workflow.transition.succeeded",
        "first event should be workflow.transition.succeeded"
    );
    assert_eq!(
        succeeded.payload.get("outcome").unwrap().as_str().unwrap(),
        "succeeded",
        "outcome should be succeeded"
    );
    assert_eq!(
        succeeded
            .payload
            .get("transition_id")
            .unwrap()
            .as_str()
            .unwrap(),
        "phase.explore.complete",
        "transition_id should match"
    );
    assert!(
        succeeded
            .payload
            .get("failed_gates")
            .unwrap()
            .as_array()
            .unwrap()
            .is_empty(),
        "failed_gates should be empty for succeeded"
    );

    assert_eq!(
        exited.event_type, "workflow.phase.exited",
        "second event should be workflow.phase.exited"
    );
    assert_eq!(
        entered.event_type, "workflow.phase.entered",
        "third event should be workflow.phase.entered"
    );
    assert_eq!(
        entered.payload.get("phase").unwrap().as_str().unwrap(),
        "specify",
        "entered phase should be 'specify'"
    );
    assert_eq!(
        exited.payload.get("phase").unwrap().as_str().unwrap(),
        "explore",
        "exited phase should be 'explore'"
    );

    // Verify content_hash is computed.
    assert!(
        succeeded.content_hash.starts_with("sha256:"),
        "succeeded.content_hash should start with sha256:"
    );
    assert!(
        exited.content_hash.starts_with("sha256:"),
        "exited.content_hash should start with sha256:"
    );
    assert!(
        entered.content_hash.starts_with("sha256:"),
        "entered.content_hash should start with sha256:"
    );
}
