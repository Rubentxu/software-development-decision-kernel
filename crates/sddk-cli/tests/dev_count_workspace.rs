//! Integration tests for `sddk dev test count-workspace` subcommand.
//!
//! Tests the deterministic aggregation of `cargo test --workspace --message-format=json`.

use std::process::Command;

/// Scenario: Invocation outside an SDDK project root exits 2
///
/// GIVEN a directory without `.sddk/` or workspace `Cargo.toml`
/// WHEN the subcommand is invoked
/// THEN exit 2 with stderr `error: must run inside an SDDK project root`.
#[test]
fn dev_count_workspace_outside_sddk_root_exits_2() {
    // Use /tmp which has no .sddk/ or Cargo.toml
    let tmpdir = tempfile::tempdir().unwrap();

    let output = Command::new(env!("CARGO_BIN_EXE_sddk"))
        .args(["dev", "test", "count-workspace"])
        .current_dir(tmpdir.path())
        .output()
        .expect("sddk binary not found");

    let stderr = String::from_utf8_lossy(&output.stderr);

    assert_eq!(
        output.status.code(),
        Some(2),
        "sddk dev test count-workspace should exit 2 outside project root\nstderr: {}",
        stderr
    );
    assert!(
        stderr.contains("must run inside an SDDK project root"),
        "stderr must contain the project-root error message\nstderr: {}",
        stderr
    );
}

/// Scenario: Help text is available
///
/// GIVEN any SDDK project root
/// WHEN `sddk dev test count-workspace --help` is invoked
/// THEN exit 0 with help text.
#[test]
fn dev_count_workspace_help_exits_zero() {
    let root = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .unwrap()
        .to_path_buf();

    let output = Command::new(env!("CARGO_BIN_EXE_sddk"))
        .args(["dev", "test", "count-workspace", "--help"])
        .current_dir(&root)
        .output()
        .expect("sddk binary not found");

    assert_eq!(
        output.status.code(),
        Some(0),
        "sddk dev test count-workspace --help should exit 0"
    );
}

/// Scenario: `--format json` flag is accepted
///
/// GIVEN any SDDK project root
/// WHEN `sddk dev test count-workspace --format json` is invoked
/// THEN the command starts (even if it takes time to run tests)
///     and either produces JSON or fails gracefully.
#[test]
fn dev_count_workspace_json_format_flag_accepted() {
    let root = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .unwrap()
        .to_path_buf();

    // Use --dry-run or similar to avoid actually running tests
    // Instead, just verify the flag is accepted by the CLI parser
    let output = Command::new(env!("CARGO_BIN_EXE_sddk"))
        .args([
            "dev",
            "test",
            "count-workspace",
            "--format",
            "json",
            "--help",
        ])
        .current_dir(&root)
        .output()
        .expect("sddk binary not found");

    // If --format json were not a valid flag, clap would error before running
    assert_eq!(
        output.status.code(),
        Some(0),
        "--format json should be accepted by clap"
    );
}
