//! RED tests for the `sddk run-view` CLI handler (integration-level).
//!
//! Spec: ~/.sddk-knowledge/sddk-framework/specs/engine/REQ-CurrentRunView-Boundary.md

use std::process::Command;

/// Locate the compiled `sddk` binary.
fn sddk_bin() -> std::path::PathBuf {
    // The CI/release build lives under cargo-targets; fallback to debug.
    let release = std::path::PathBuf::from("/home/rubentxu/cargo-targets/debug/sddk");
    if release.exists() {
        release
    } else {
        std::path::PathBuf::from("./target/debug/sddk")
    }
}

fn run(args: &[&str]) -> (i32, String, String) {
    let out = Command::new(sddk_bin())
        .args(args)
        .output()
        .expect("sddk binary must run");
    (
        out.status.code().unwrap_or(-1),
        String::from_utf8_lossy(&out.stdout).to_string(),
        String::from_utf8_lossy(&out.stderr).to_string(),
    }
}

#[test]
fn json_output_round_trips_through_serde() {
    // Scenario: JSON output round-trips through serde
    let (status, stdout, _stderr) = run(&["run-view", "R-decl-cli-001", "--format", "json"]);
    assert_eq!(status, 0, "exit code must be 0");
    let parsed: serde_json::Value =
        serde_json::from_str(&stdout).expect("stdout must be valid JSON");
    assert!(parsed.get("run_state").is_some());
    assert!(parsed.get("action_surface").is_some());
    assert_eq!(parsed["run_state"]["run_id"], "R-decl-cli-001");
    assert_eq!(parsed["action_surface"]["run_id"], "R-decl-cli-001");
}

#[test]
fn unknown_policy_yields_policy_not_found() {
    // Scenario: Unknown policy yields POLICY_NOT_FOUND
    let (status, _stdout, stderr) = run(&[
        "run-view",
        "R-cli-policy",
        "--policy",
        "definitely-not-a-policy",
    ]);
    assert_eq!(status, 2, "exit code must be 2 (POLICY_NOT_FOUND)");
    assert!(
        stderr.contains("POLICY_NOT_FOUND"),
        "stderr must contain POLICY_NOT_FOUND marker, got: {stderr}"
    );
}

#[test]
fn default_policy_matches_explicit_default() {
    // Scenario: --policy default is the same as no flag
    let (s1, out1, _) = run(&["run-view", "R-cli-default-eq", "--format", "json"]);
    let (s2, out2, _) = run(&[
        "run-view",
        "R-cli-default-eq",
        "--policy",
        "default",
        "--format",
        "json",
    ]);
    assert_eq!(s1, 0);
    assert_eq!(s2, 0);
    let p1: serde_json::Value = serde_json::from_str(&out1).unwrap();
    let p2: serde_json::Value = serde_json::from_str(&out2).unwrap();
    assert_eq!(
        p1["action_surface"]["available_actions"], p2["action_surface"]["available_actions"],
        "available_actions must be identical"
    );
    assert_eq!(
        p1["action_surface"]["policy_digest"], p2["action_surface"]["policy_digest"],
        "policy_digest must be identical"
    );
}

#[test]
fn text_output_contains_two_section_headers() {
    // Scenario: Default invocation emits human-readable text on TTY
    let (status, stdout, _stderr) = run(&["run-view", "R-cli-text"]);
    assert_eq!(status, 0);
    assert!(stdout.contains("# Run state"));
    assert!(stdout.contains("# Action surface"));
}
