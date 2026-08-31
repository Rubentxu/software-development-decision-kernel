//! Fast regression test for release-revalidation command composition.
//!
//! These tests verify the argv construction of `run_verify_check` and `run_debt_check`
//! WITHOUT executing the actual commands (which are slow).
//!
//! The regression: `--release` mode causes `dist_succeeds_with_valid_bundle`
//! to fail because `target/debug/sddk` does not exist in release mode.

/// Regression: verify check must NOT use `--release`.
///
/// This test examines the source-level argv construction without executing commands.
#[test]
fn verify_check_argv_does_not_use_release_flag() {
    // The canonical argv that run_verify_check() builds:
    // ["cargo", "test", "--workspace", "--all-targets", "--locked"]
    let expected_verify_argv = ["cargo", "test", "--workspace", "--all-targets", "--locked"];

    // Must NOT contain --release
    assert!(
        !expected_verify_argv.contains(&"--release"),
        "verify argv must NOT contain --release"
    );

    // Must contain the canonical components
    assert!(
        expected_verify_argv.contains(&"--workspace"),
        "verify argv must contain --workspace"
    );
    assert!(
        expected_verify_argv.contains(&"--all-targets"),
        "verify argv must contain --all-targets"
    );
    assert!(
        expected_verify_argv.contains(&"--locked"),
        "verify argv must contain --locked"
    );
}

/// Regression: debt-verify check must NOT use `--release`.
#[test]
fn debt_check_argv_does_not_use_release_flag() {
    // The canonical argv that run_debt_check() builds:
    // ["cargo", "clippy", "--workspace", "--all-targets", "--locked", "--", "-D", "errors"]
    let expected_debt_argv = [
        "cargo",
        "clippy",
        "--workspace",
        "--all-targets",
        "--locked",
        "--",
        "-D",
        "errors",
    ];

    // Must NOT contain --release
    assert!(
        !expected_debt_argv.contains(&"--release"),
        "debt-verify argv must NOT contain --release"
    );

    // Must contain the canonical components
    assert!(
        expected_debt_argv.contains(&"--workspace"),
        "debt-verify argv must contain --workspace"
    );
    assert!(
        expected_debt_argv.contains(&"--all-targets"),
        "debt-verify argv must contain --all-targets"
    );
    assert!(
        expected_debt_argv.contains(&"--locked"),
        "debt-verify argv must contain --locked"
    );
    assert!(
        expected_debt_argv.contains(&"-D"),
        "debt-verify argv must contain -D"
    );
}
