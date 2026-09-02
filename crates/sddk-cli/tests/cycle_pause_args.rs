//! Clap-level parse tests for `sddk cycle pause` and `sddk cycle resume` commands.
//!
//! Tests that invalid values for `--reason` are rejected at parse time rather than
//! at runtime.

use std::process::Command;

/// Verify that `sddk cycle pause --reason not_a_real_reason` fails at clap parse time.
/// Invalid enum values should be rejected before the command handler is invoked.
#[test]
fn invalid_reason_rejected_at_parse_time() {
    let output = Command::new(env!("CARGO_BIN_EXE_sddk"))
        .args([
            "cycle",
            "pause",
            "--reason",
            "not_a_real_reason",
            "--lease-owner",
            "alice",
            "--fencing-token",
            "1",
        ])
        .output()
        .expect("sddk binary not found");

    // Clap should reject the invalid enum value with a parse error (non-zero exit)
    assert!(
        output.status.code() != Some(0),
        "sddk cycle pause with invalid --reason should fail at parse time"
    );

    let stderr = String::from_utf8_lossy(&output.stderr);
    let stdout = String::from_utf8_lossy(&output.stdout);
    let combined = format!("{}{}", stdout, stderr);

    // Verify the error mentions the invalid value or enum parsing
    assert!(
        combined.to_lowercase().contains("invalid")
            || combined.to_lowercase().contains("not_a_real_reason")
            || combined.to_lowercase().contains("pause reason")
            || combined.to_lowercase().contains("value"),
        "Error output should mention invalid value or parse failure.\nGot:\n{}",
        combined
    );
}

/// Verify that `sddk cycle pause --reason` without a value fails at clap parse time.
#[test]
fn missing_reason_value_rejected_at_parse_time() {
    let output = Command::new(env!("CARGO_BIN_EXE_sddk"))
        .args([
            "cycle",
            "pause",
            "--reason",
            "--lease-owner",
            "alice",
            "--fencing-token",
            "1",
        ])
        .output()
        .expect("sddk binary not found");

    // Clap should reject missing value for --reason
    assert!(
        output.status.code() != Some(0),
        "sddk cycle pause with missing --reason value should fail at parse time"
    );
}

/// Verify that valid pause reasons are accepted at parse time (smoke test).
/// Actual behavior is tested in engine-level tests.
#[test]
fn valid_pause_reasons_accepted_at_parse_time() {
    // Note: clap ValueEnum uses kebab-case for display (priority-revoked, not priority_revoked)
    for reason in &["priority-revoked", "context-switch", "dependency-waiting"] {
        let output = Command::new(env!("CARGO_BIN_EXE_sddk"))
            .args([
                "cycle",
                "pause",
                "--reason",
                reason,
                "--lease-owner",
                "alice",
                "--fencing-token",
                "1",
            ])
            .output()
            .expect("sddk binary not found");

        // These should NOT fail at parse time (they may fail later if no cycle is available,
        // but parse should succeed)
        let stderr = String::from_utf8_lossy(&output.stderr);
        assert!(
            !stderr.contains("invalid") || output.status.code() == Some(0),
            "Valid reason '{}' should not be rejected at parse time.\nStderr:\n{}",
            reason,
            stderr
        );
    }
}
