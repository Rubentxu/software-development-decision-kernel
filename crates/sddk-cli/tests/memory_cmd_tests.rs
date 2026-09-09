//! Integration tests for `sddk memory` CLI (M4 / SPEC-004 §CLI).
//!
//! Each test invokes the real `sddk` binary and exercises one of the
//! eight SPEC-004 subcommands against the deterministic in-memory
//! fixture shipped by the CLI. No project seeding is required: the
//! `Memory` subcommand family builds its own `InMemoryMemoryStore`.

#![allow(clippy::needless_borrow)]

use std::process::Command;

fn run_sddk(args: &[&str]) -> std::process::Output {
    Command::new(env!("CARGO_BIN_EXE_sddk"))
        .args(args)
        .env("HOME", std::env::temp_dir())
        .env("XDG_DATA_HOME", std::env::temp_dir())
        .env("XDG_STATE_HOME", std::env::temp_dir())
        .env("XDG_CACHE_HOME", std::env::temp_dir())
        .output()
        .expect("spawn sddk")
}

#[test]
fn memory_help_lists_eight_subcommands() {
    let out = run_sddk(&["memory", "--help"]);
    let stdout = String::from_utf8_lossy(&out.stdout);
    let stderr = String::from_utf8_lossy(&out.stderr);
    // clap prints --help to stdout; some builds route via stderr.
    let combined = format!("{stdout}{stderr}");
    assert!(
        out.status.code() == Some(0) || combined.contains("Usage: sddk memory"),
        "expected help exit, code={:?}, stdout={stdout}, stderr={stderr}",
        out.status.code()
    );
    for sub in [
        "status",
        "log",
        "tree",
        "show",
        "diff",
        "merge-base",
        "reflog",
        "audit",
    ] {
        assert!(combined.contains(sub), "missing subcommand in help: {sub}");
    }
}

#[test]
fn memory_status_reports_three_commits() {
    let out = run_sddk(&["memory", "status", "--format", "json"]);
    assert_eq!(out.status.code(), Some(0));
    let v: serde_json::Value = serde_json::from_slice(&out.stdout).expect("json");
    assert_eq!(v["commits"], serde_json::json!(3));
    assert_eq!(v["trees"], serde_json::json!(3));
    assert_eq!(v["blobs"], serde_json::json!(3));
    assert!(v["head"].is_string());
    assert!(v["reflog_head"].is_object());
}

#[test]
fn memory_log_returns_three_oids() {
    let out = run_sddk(&["memory", "log"]);
    assert_eq!(out.status.code(), Some(0));
    let stdout = String::from_utf8_lossy(&out.stdout);
    let lines: Vec<&str> = stdout.lines().filter(|s| !s.is_empty()).collect();
    assert_eq!(lines.len(), 3, "expected 3 oids, got: {lines:?}");
    for line in &lines {
        assert_eq!(line.len(), 64, "oid should be 64 hex chars: {line}");
    }
}

#[test]
fn memory_show_head_round_trips() {
    let out = run_sddk(&["memory", "show", "--oid", "HEAD"]);
    assert_eq!(out.status.code(), Some(0));
    let v: serde_json::Value = serde_json::from_slice(&out.stdout).expect("json");
    assert!(v["commit_id"].is_string());
    assert!(v["tree_id"].is_string());
    assert!(v["message"].is_string());
    assert!(v["refs_pointing_here"].is_array());
}

#[test]
fn memory_tree_head_round_trips() {
    let out = run_sddk(&["memory", "tree", "--oid", "HEAD"]);
    assert_eq!(out.status.code(), Some(0));
    let v: serde_json::Value = serde_json::from_slice(&out.stdout).expect("json");
    assert!(v["at_commit"].is_string());
    assert!(v["entries"].is_object());
}

#[test]
fn memory_diff_two_oids_runs() {
    // We need a known pair: log gives us three; pick the first and
    // the last from a separate `log --max 1024` invocation.
    let log = run_sddk(&["memory", "log"]);
    let lines: Vec<String> = String::from_utf8_lossy(&log.stdout)
        .lines()
        .filter(|s| !s.is_empty())
        .map(String::from)
        .collect();
    let a = lines.first().expect("a").clone();
    let b = lines.last().expect("b").clone();
    let out = run_sddk(&["memory", "diff", "--range", &format!("{a}..{b}")]);
    assert_eq!(out.status.code(), Some(0));
    let v: serde_json::Value = serde_json::from_slice(&out.stdout).expect("json");
    assert!(v["a"].is_string());
    assert!(v["b"].is_string());
    assert!(v["added_blobs"].is_array());
    assert!(v["removed_blobs"].is_array());
}

#[test]
fn memory_diff_rejects_triple_dot() {
    let out = run_sddk(&["memory", "diff", "--range", "a...b"]);
    assert_ne!(out.status.code(), Some(0));
    let stderr = String::from_utf8_lossy(&out.stderr);
    assert!(stderr.contains("triple-dot"), "stderr: {stderr}");
}

#[test]
fn memory_merge_base_returns_a_known_oid() {
    let log = run_sddk(&["memory", "log"]);
    let lines: Vec<String> = String::from_utf8_lossy(&log.stdout)
        .lines()
        .filter(|s| !s.is_empty())
        .map(String::from)
        .collect();
    let a = lines[0].clone();
    let b = lines[2].clone();
    let out = run_sddk(&["memory", "merge-base", "--a", &a, "--b", &b]);
    assert_eq!(out.status.code(), Some(0));
    let stdout = String::from_utf8_lossy(&out.stdout);
    // Text rendering for `merge-base` falls back to a JSON string
    // (`"<oid>"`); accept either the bare 64-char hex or the quoted form.
    let oid_trim = stdout.trim().trim_matches('"');
    assert_eq!(
        oid_trim.len(),
        64,
        "merge-base must return 64 hex chars, got: {stdout}"
    );
    assert!(
        lines.contains(&oid_trim.to_string()),
        "merge-base oid must appear in log"
    );
}

#[test]
fn memory_reflog_head_has_one_entry() {
    let out = run_sddk(&["memory", "reflog", "--scope", "head"]);
    assert_eq!(out.status.code(), Some(0));
    let v: serde_json::Value = serde_json::from_slice(&out.stdout).expect("json");
    let arr = v.as_array().expect("array");
    assert_eq!(arr.len(), 1, "exactly one reflog entry from fixture");
    assert_eq!(arr[0]["ref_path"], "HEAD");
    assert!(arr[0]["new_target"].is_string());
}

#[test]
fn memory_audit_returns_status_and_reflog_len() {
    let out = run_sddk(&["memory", "audit", "--format", "json"]);
    assert_eq!(out.status.code(), Some(0));
    let v: serde_json::Value = serde_json::from_slice(&out.stdout).expect("json");
    assert!(v["status"].is_object());
    assert_eq!(v["status"]["commits"], serde_json::json!(3));
    assert_eq!(v["reflog_len"], serde_json::json!(1));
}

#[test]
fn memory_show_unknown_oid_fails_closed() {
    let out = run_sddk(&[
        "memory",
        "show",
        "--oid",
        "0000000000000000000000000000000000000000000000000000000000000000",
    ]);
    assert_ne!(out.status.code(), Some(0), "unknown oid must fail closed");
    let stderr = String::from_utf8_lossy(&out.stderr);
    assert!(
        stderr.contains("unknown") || stderr.contains("not found"),
        "stderr: {stderr}"
    );
}
