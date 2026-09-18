//! First-class facade help substring (A5-4b retirement).
//!
//! The prior shell test incorrectly grepped `skills/_shared/cli-usage-contract.md`
//! instead of the actual binary help output. This test exercises
//! `env!("CARGO_BIN_EXE_sddk")` and asserts the legacy facade substring is
//! GONE (A5-4b) and the honest about-line points operators to
//! `sddk agent-help`.

use std::process::Command;

/// A5-4b: `sddk --help` no longer claims legacy M6.1 facades as
/// "first-class commands". The about-line now directs operators to
/// `sddk agent-help` for the operator-facing surface.
#[test]
fn help_drops_first_class_substring() {
    let output = Command::new(env!("CARGO_BIN_EXE_sddk"))
        .args(["--help"])
        .output()
        .expect("sddk binary not found");
    assert_eq!(output.status.code(), Some(0), "sddk --help should exit 0");
    let stdout = String::from_utf8_lossy(&output.stdout);
    let stderr = String::from_utf8_lossy(&output.stderr);
    let combined = format!("{}{}", stdout, stderr);
    assert!(
        !combined.contains("First-class commands: status"),
        "A5-4b retired the lying first-class substring. Got combined:\n{combined}",
    );
    assert!(
        combined.contains("`sddk agent-help`"),
        "A5-4b about-line must reference `sddk agent-help`. Got combined:\n{combined}",
    );
}
