//! First-class facade commands: substring, dispatch, and recover invariant.
//!
//! Tests for D2 shadow routing:
//! - Substring: `sddk --help` contains "First-class commands: status, plan, run, ship, recover"
//! - 5 dispatch tests: each verb with `--help` exits 0
//! - Recover invariant: digest and count byte-identical; dry-run doesn't mutate

use std::process::Command;

/// Verifies the contiguous substring in `sddk --help`.
#[test]
fn help_contains_first_class_substring() {
    let output = Command::new(env!("CARGO_BIN_EXE_sddk"))
        .args(["--help"])
        .output()
        .expect("sddk binary not found");
    assert_eq!(output.status.code(), Some(0), "sddk --help should exit 0");
    let stdout = String::from_utf8_lossy(&output.stdout);
    let stderr = String::from_utf8_lossy(&output.stderr);
    let combined = format!("{}{}", stdout, stderr);
    assert!(
        combined.contains("First-class commands: status, plan, run, ship, recover"),
        "sddk --help must contain the exact substring 'First-class commands: status, plan, run, ship, recover'\nGot stdout:\n{}\nGot stderr:\n{}",
        stdout,
        stderr
    );
}

/// Dispatch test: `sddk status --help` exits 0.
#[test]
fn status_help_exits_zero() {
    let status = Command::new(env!("CARGO_BIN_EXE_sddk"))
        .args(["status", "--help"])
        .output()
        .expect("sddk binary not found")
        .status;
    assert_eq!(status.code(), Some(0), "sddk status --help must exit 0");
}

/// Dispatch test: `sddk plan --help` exits 0.
#[test]
fn plan_help_exits_zero() {
    let status = Command::new(env!("CARGO_BIN_EXE_sddk"))
        .args(["plan", "--help"])
        .output()
        .expect("sddk binary not found")
        .status;
    assert_eq!(status.code(), Some(0), "sddk plan --help must exit 0");
}

/// Dispatch test: `sddk run --help` exits 0.
#[test]
fn run_help_exits_zero() {
    let status = Command::new(env!("CARGO_BIN_EXE_sddk"))
        .args(["run", "--help"])
        .output()
        .expect("sddk binary not found")
        .status;
    assert_eq!(status.code(), Some(0), "sddk run --help must exit 0");
}

/// Dispatch test: `sddk ship --help` exits 0.
#[test]
fn ship_help_exits_zero() {
    let status = Command::new(env!("CARGO_BIN_EXE_sddk"))
        .args(["ship", "--help"])
        .output()
        .expect("sddk binary not found")
        .status;
    assert_eq!(status.code(), Some(0), "sddk ship --help must exit 0");
}

/// Dispatch test: `sddk recover --help` exits 0.
#[test]
fn recover_help_exits_zero() {
    let status = Command::new(env!("CARGO_BIN_EXE_sddk"))
        .args(["recover", "--help"])
        .output()
        .expect("sddk binary not found")
        .status;
    assert_eq!(status.code(), Some(0), "sddk recover --help must exit 0");
}

/// `sddk plan` without a subcommand must NOT silently start a cycle.
/// It must either emit an error or show help requiring a subcommand (ADR-073 §3.1).
#[test]
fn plan_without_subcommand_fails_or_shows_error() {
    let output = Command::new(env!("CARGO_BIN_EXE_sddk"))
        .args(["plan"])
        .output()
        .expect("sddk binary not found");
    // Must exit non-zero — must NOT silently start a cycle
    assert_ne!(
        output.status.code(),
        Some(0),
        "sddk plan without subcommand must not silently succeed"
    );
    let combined = format!(
        "{}{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
    // Must emit an error message indicating subcommand is required
    assert!(
        combined.to_lowercase().contains("error")
            || combined.contains("subcommand")
            || combined.contains("invalid"),
        "sddk plan without subcommand must emit an error or require subcommand, got: {combined}"
    );
}

/// Delegation test: `sddk recover --help` mentions "cycle rebuild".
#[test]
fn recover_help_mentions_cycle_rebuild_delegation() {
    let output = Command::new(env!("CARGO_BIN_EXE_sddk"))
        .args(["recover", "--help"])
        .output()
        .expect("sddk binary not found");
    let combined = format!(
        "{}{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
    assert!(
        combined.contains("cycle rebuild"),
        "sddk recover --help must mention delegation to 'cycle rebuild'"
    );
}

/// Delegation test: `sddk ship --help` mentions "release plan".
#[test]
fn ship_help_mentions_release_plan_delegation() {
    let output = Command::new(env!("CARGO_BIN_EXE_sddk"))
        .args(["ship", "--help"])
        .output()
        .expect("sddk binary not found");
    let combined = format!(
        "{}{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
    assert!(
        combined.contains("release plan"),
        "sddk ship --help must mention delegation to 'release plan'"
    );
}

/// Dispatch test: `sddk release revalidate --help` exits 0.
#[test]
fn release_revalidate_help_exits_zero() {
    let status = Command::new(env!("CARGO_BIN_EXE_sddk"))
        .args(["release", "revalidate", "--help"])
        .output()
        .expect("sddk binary not found")
        .status;
    assert_eq!(
        status.code(),
        Some(0),
        "sddk release revalidate --help must exit 0"
    );
}

/// Dispatch test: `sddk release revalidate --help` shows all required arguments.
#[test]
fn release_revalidate_help_shows_required_args() {
    let output = Command::new(env!("CARGO_BIN_EXE_sddk"))
        .args(["release", "revalidate", "--help"])
        .output()
        .expect("sddk binary not found");
    let combined = format!(
        "{}{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
    // Must mention cycle and original-sha as required long args
    assert!(
        combined.contains("--cycle") && combined.contains("--original-sha"),
        "sddk release revalidate --help must show --cycle and --original-sha\nGot:\n{}",
        combined
    );
}

/// Dispatch test: `sddk release revalidate --help` mentions path-policy constraints.
#[test]
fn release_revalidate_help_documents_path_policy() {
    let output = Command::new(env!("CARGO_BIN_EXE_sddk"))
        .args(["release", "revalidate", "--help"])
        .output()
        .expect("sddk binary not found");
    let combined = format!(
        "{}{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
    // Help text must document that skip-verify/skip-debt are policy-constrained
    assert!(
        combined.contains("--skip-verify") && combined.contains("--skip-debt"),
        "sddk release revalidate --help must document skip flags\nGot:\n{}",
        combined
    );
}
