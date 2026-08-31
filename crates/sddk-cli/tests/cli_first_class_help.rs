//! First-class facade help substring — migrated from `tests/test_top_level_help_substring.sh`.
//!
//! The prior shell test incorrectly grepped `skills/_shared/cli-usage-contract.md`
//! instead of the actual binary help output. This test exercises `env!("CARGO_BIN_EXE_sddk")`
//! and asserts the substring against the real rendered help.

use std::process::Command;

/// Verifies `sddk --help` contains the required first-class command substring.
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
