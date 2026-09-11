//! E2E tests for `sddk dev cockpit diff-watch` (M9+ interactive drift
//! sessions).
//!
//! The watch command polls two ActiveGraphInput JSON files and emits a
//! drift row (NDJSON or text) whenever the content-only projection
//! digest changes. These tests cover the four behavioural contracts:
//!
//!   * identical inputs → no drift row emitted (idle timeout exits
//!     with `emitted: 0`)
//!   * different inputs → baseline drift row emitted on tick 1
//!   * `--from-tail` skips the baseline; only NEW drift appears
//!   * modifying input A between ticks emits a new drift row
//!
//! The tests run as plain subprocess tests against the CLI binary; no
//! project adoption or RuntimeContext is needed because
//! `diff-watch` resolves inputs directly from `--input-a` / `--input-b`.

use std::process::Command;
use std::time::Duration;

/// Writes a minimal ActiveGraphInput JSON fixture to a file in `dir`.
fn write_input(dir: &std::path::Path, name: &str, nodes: &[&str]) -> std::path::PathBuf {
    let path = dir.join(name);
    let workflow_nodes: Vec<String> = nodes.iter().map(|s| s.to_string()).collect();
    let json = serde_json::json!({
        "workflow_nodes": workflow_nodes,
        "workflow_edges": [],
    });
    std::fs::write(&path, serde_json::to_vec_pretty(&json).unwrap()).unwrap();
    path
}

/// Helper to invoke `sddk dev cockpit diff-watch` from a tempdir with
/// the given extra args. The caller passes paths in `extra` as
/// `--input-a <a>` etc.
fn invoke_watch(extra: &[&str], workdir: &std::path::Path) -> std::process::Output {
    let mut args = vec!["dev", "cockpit", "diff-watch"];
    args.extend_from_slice(extra);
    Command::new(env!("CARGO_BIN_EXE_sddk"))
        .args(&args)
        .current_dir(workdir)
        .output()
        .expect("sddk binary not found")
}

// ── Tests ────────────────────────────────────────────────────────────────────────

/// Identical inputs → no drift ever appears → idle timeout exits with
/// `emitted: 0`. First tick baseline drift IS emitted (since last
/// digests are None → drift_changed=true), but since inputs are
/// identical, the DriftRow has zero counters.
#[test]
fn diff_watch_identical_inputs_exits_on_idle() {
    let tmp = tempfile::tempdir().unwrap();
    let dir = tmp.path();
    let a = write_input(dir, "a.json", &["sha-1", "sha-2"]);
    let b = write_input(dir, "b.json", &["sha-1", "sha-2"]);

    let out = invoke_watch(
        &[
            "--input-a",
            a.to_str().unwrap(),
            "--input-b",
            b.to_str().unwrap(),
            "--idle-timeout-ms",
            "100",
            "--interval-ms",
            "20",
            "--format",
            "json",
        ],
        dir,
    );

    assert_eq!(out.status.code(), Some(0));
    let stdout = String::from_utf8_lossy(&out.stdout);
    // Baseline tick: zero counters but still emits one row.
    assert!(
        stdout.contains(r#""nodes_added":0"#),
        "expected zero-counter baseline row, got: {stdout}"
    );
    assert!(
        stdout.contains(r#""emitted":1"#),
        "expected exactly 1 emitted (baseline), got: {stdout}"
    );
    assert!(
        stdout.contains(r#""__watch_complete":true"#),
        "expected watch completion marker, got: {stdout}"
    );
}

/// Different inputs on tick 1 → baseline drift row emitted with
/// non-zero counters (one node added/removed depending on order).
#[test]
fn diff_watch_distinct_inputs_emits_baseline() {
    let tmp = tempfile::tempdir().unwrap();
    let dir = tmp.path();
    let a = write_input(dir, "a.json", &["sha-1", "sha-2"]);
    let b = write_input(dir, "b.json", &["sha-3", "sha-4"]);

    let out = invoke_watch(
        &[
            "--input-a",
            a.to_str().unwrap(),
            "--input-b",
            b.to_str().unwrap(),
            "--idle-timeout-ms",
            "100",
            "--interval-ms",
            "20",
            "--format",
            "json",
        ],
        dir,
    );

    assert_eq!(out.status.code(), Some(0));
    let stdout = String::from_utf8_lossy(&out.stdout);
    // 2 nodes in A, 2 nodes in B, none shared → 2 added + 2 removed.
    assert!(
        stdout.contains(r#""nodes_added":2"#),
        "expected nodes_added=2, got: {stdout}"
    );
    assert!(
        stdout.contains(r#""nodes_removed":2"#),
        "expected nodes_removed=2, got: {stdout}"
    );
    assert!(
        stdout.contains(r#""emitted":1"#),
        "expected 1 emitted baseline, got: {stdout}"
    );
}

/// `--from-tail` skips the baseline row → if no further drift appears,
/// the watch exits with `emitted: 0`.
#[test]
fn diff_watch_from_tail_skips_baseline() {
    let tmp = tempfile::tempdir().unwrap();
    let dir = tmp.path();
    let a = write_input(dir, "a.json", &["sha-1", "sha-2"]);
    let b = write_input(dir, "b.json", &["sha-3", "sha-4"]);

    let out = invoke_watch(
        &[
            "--input-a",
            a.to_str().unwrap(),
            "--input-b",
            b.to_str().unwrap(),
            "--from-tail",
            "--idle-timeout-ms",
            "100",
            "--interval-ms",
            "20",
            "--format",
            "json",
        ],
        dir,
    );

    assert_eq!(out.status.code(), Some(0));
    let stdout = String::from_utf8_lossy(&out.stdout);
    assert!(
        stdout.contains(r#""emitted":0"#),
        "expected emitted=0 under --from-tail, got: {stdout}"
    );
    assert!(
        stdout.contains(r#""__watch_complete":true"#),
        "expected watch completion marker, got: {stdout}"
    );
}

/// Modifying input A between ticks (after baseline) emits a NEW drift
/// row even under `--from-tail`. We use `--max-events 1` so the
/// command exits as soon as the new drift row is emitted.
///
/// Note on semantics: `diff(a, b)` reports nodes "in a but not in b"
/// as `removed` and nodes "in b but not in a" as `added`. So when we
/// add `sha-NEW` to A (and B stays at `sha-1`), the new drift row
/// reports `nodes_removed: 1` (sha-NEW is in A but not in B).
#[test]
fn diff_watch_detects_change_to_input_after_start() {
    let tmp = tempfile::tempdir().unwrap();
    let dir = tmp.path();
    let a = write_input(dir, "a.json", &["sha-1"]);
    let b = write_input(dir, "b.json", &["sha-1"]);

    // Spawn the watch with --from-tail + --max-events 1 in a thread so
    // we can modify the input file before the baseline would have
    // been emitted. The watch's first action is to set
    // last_digest_a/b from tick 1, then on tick 2 if the digest
    // changes, emit.
    let a_clone = a.clone();
    let child = Command::new(env!("CARGO_BIN_EXE_sddk"))
        .args([
            "dev",
            "cockpit",
            "diff-watch",
            "--input-a",
            a_clone.to_str().unwrap(),
            "--input-b",
            b.to_str().unwrap(),
            "--from-tail",
            "--interval-ms",
            "50",
            "--max-events",
            "1",
            "--max-ticks",
            "10",
            "--format",
            "json",
        ])
        .current_dir(dir)
        .stdout(std::process::Stdio::piped())
        .stderr(std::process::Stdio::piped())
        .spawn()
        .expect("spawn watch");

    // Wait long enough for tick 1 to register the baseline digests,
    // then modify input A so tick 2 detects a change.
    std::thread::sleep(Duration::from_millis(120));
    write_input(dir, "a.json", &["sha-1", "sha-NEW"]);

    let out = child.wait_with_output().expect("watch output");
    assert_eq!(
        out.status.code(),
        Some(0),
        "watch should exit cleanly: stderr={}",
        String::from_utf8_lossy(&out.stderr)
    );
    let stdout = String::from_utf8_lossy(&out.stdout);
    // After modifying A, `sha-NEW` is in A but not in B → reported as
    // "removed" by `diff(a, b)`.
    assert!(
        stdout.contains(r#""nodes_removed":1"#),
        "expected 1 node removed after A gained sha-NEW, got: {stdout}"
    );
    assert!(
        stdout.contains(r#""emitted":1"#),
        "expected emitted=1, got: {stdout}"
    );
}
