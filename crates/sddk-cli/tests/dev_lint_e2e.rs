//! E2E tests for `sddk dev lint deprecated-patterns` (M9 enforcement
//! infrastructure, v1.168.8).
//!
//! These tests only exercise CLI mechanics (help text, format flag acceptance,
//! missing-registry error). They DO NOT invoke the subcommand against the live
//! workspace to evaluate real lint hits, because that would scan the entire
//! crate source tree and produce hits that change with every commit, breaking
//! the deterministic "exit 0 in advisory mode" contract. Real-workspace
//! evaluation is the operator's responsibility (run `sddk dev lint
//! deprecated-patterns --root .` to see current hits).
//!
//! The lint runner's logic is unit-tested in
//! `src/dev/lint/deprecated_patterns.rs::tests` with synthetic workspaces.

use std::process::Command;

/// C4 freeze guard (cycle c4-authority-engine-cutover, WU-C4-4): the M1
/// allowlist must cover every current legacy-authority call site, so
/// `sddk dev doctor` in the real workspace reports
/// `c4.authority_single_admission` as present (R-4-008). A NEW site outside
/// the allowlist is unit-tested with a synthetic fixture in
/// `src/dev/arch_lint.rs::tests::c4_guard_flags_new_for_cli_site_outside_allowlist`.
#[test]
fn dev_doctor_c4_authority_single_admission_green_on_current_workspace() {
    let root = env!("CARGO_MANIFEST_DIR");
    // Two levels up: workspace root of sddk-framework.
    let workspace_root = std::path::Path::new(root)
        .ancestors()
        .nth(2)
        .expect("workspace root")
        .to_path_buf();

    let output = Command::new(env!("CARGO_BIN_EXE_sddk"))
        .args(["dev", "doctor", "--format", "json"])
        .current_dir(&workspace_root)
        .output()
        .expect("sddk binary not found");

    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(
        stdout.contains("\"c4.authority_single_admission\""),
        "doctor must emit the c4 marker\nstdout: {stdout}"
    );
    // present: true — the M1 allowlist covers all current sites.
    let marker_idx = stdout
        .find("\"c4.authority_single_admission\"")
        .expect("marker index");
    let tail = &stdout[marker_idx..];
    let present_idx = tail.find("\"present\"").expect("present field");
    let value = &tail[present_idx..present_idx + 40];
    assert!(
        value.contains("true"),
        "c4.authority_single_admission must be present with the M1 allowlist\ntail: {value}"
    );
}

/// Scenario: Help text is available
///
/// GIVEN any directory
/// WHEN `sddk dev lint deprecated-patterns --help` is invoked
/// THEN exit 0 with help text describing the subcommand.
#[test]
fn dev_lint_deprecated_patterns_help_exits_zero() {
    let tmpdir = tempfile::tempdir().unwrap();

    let output = Command::new(env!("CARGO_BIN_EXE_sddk"))
        .args(["dev", "lint", "deprecated-patterns", "--help"])
        .current_dir(tmpdir.path())
        .output()
        .expect("sddk binary not found");

    let stderr = String::from_utf8_lossy(&output.stderr);
    let stdout = String::from_utf8_lossy(&output.stdout);

    assert_eq!(
        output.status.code(),
        Some(0),
        "--help should exit 0\nstderr: {stderr}"
    );
    assert!(
        stderr.contains("deprecated-patterns")
            || stderr.contains("deprecated_patterns")
            || stdout.contains("deprecated-patterns")
            || stdout.contains("deprecated_patterns"),
        "help text should mention the registry name\nstdout: {stdout}\nstderr: {stderr}"
    );
}

/// Scenario: Missing registry surfaces a clean error
///
/// GIVEN a directory with no `docs/architecture/lints/deprecated_patterns.toml`
/// WHEN the subcommand is invoked with `--root <that-dir>`
/// THEN exit 1 (or 2) with an error message about the missing registry.
#[test]
fn dev_lint_deprecated_patterns_missing_registry_exits_nonzero() {
    let tmpdir = tempfile::tempdir().unwrap();

    let output = Command::new(env!("CARGO_BIN_EXE_sddk"))
        .args([
            "dev",
            "lint",
            "deprecated-patterns",
            "--root",
            tmpdir.path().to_str().unwrap(),
        ])
        .output()
        .expect("sddk binary not found");

    let stderr = String::from_utf8_lossy(&output.stderr);

    assert!(
        output.status.code().unwrap_or(0) != 0,
        "missing registry should exit non-zero\nstderr: {stderr}"
    );
    assert!(
        stderr.contains("registry")
            || stderr.contains("deprecated_patterns.toml")
            || stderr.contains("not found")
            || stderr.contains("No such file"),
        "stderr should mention the missing registry\nstderr: {stderr}"
    );
}

/// Scenario: `--format json` flag is accepted (parse path)
///
/// GIVEN an empty workspace (no registry yet)
/// WHEN `--format json` is added to the invocation
/// THEN the command parses the flag and exits non-zero with the registry
///     error (so we know the flag was accepted, not rejected as unknown).
#[test]
fn dev_lint_deprecated_patterns_json_format_flag_accepted() {
    let tmpdir = tempfile::tempdir().unwrap();

    let output = Command::new(env!("CARGO_BIN_EXE_sddk"))
        .args([
            "dev",
            "lint",
            "deprecated-patterns",
            "--format",
            "json",
            "--root",
            tmpdir.path().to_str().unwrap(),
        ])
        .output()
        .expect("sddk binary not found");

    let stderr = String::from_utf8_lossy(&output.stderr);

    assert!(
        output.status.code().unwrap_or(0) != 0,
        "missing registry with --format json should exit non-zero (registry error, not flag error)\nstderr: {stderr}"
    );
    // The error should be about the registry, not about an unknown flag.
    assert!(
        !stderr.contains("unexpected argument") && !stderr.contains("invalid value"),
        "the --format json flag should be accepted (parse ok)\nstderr: {stderr}"
    );
}
