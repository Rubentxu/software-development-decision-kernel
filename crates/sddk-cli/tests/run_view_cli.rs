//! Contract tests for the `sddk run-view` CLI handler (integration-level).
//!
//! Spec: `REQ-CurrentRunView-Boundary.md` + `REQ-CurrentRunView-Shape.md`.
//!
//! **These expectations changed with the fail-closed fix (session-42), and the
//! change was not cosmetic — it inverted the contract.** The previous version
//! of this file asserted exit 0 and a well-formed `RunStateView` for run ids
//! that exist nowhere in any store. That was INC-DEBT-039: the scaffold
//! guessed `origin` from the `run_id` prefix and passed `frontier`,
//! `blockers` and `pending_decisions` as constant `vec![]`, so a fabricated
//! view reached policy evaluation and `available_actions`.
//!
//! With no real `RunStateViewInputs` implementation, asserting exit 0 here
//! would pin the fabrication in place forever: there is nothing to produce
//! a view, so "succeeds" can only mean "invents". The tests below assert the
//! fail-closed contract instead, and they are mutation-tested — relaxing
//! `load_run_state_view` back to a fabricated view turns all four RED.

use std::process::Command;

/// Exit code the CLI uses for a typed, fail-closed refusal.
///
/// Distinct from 2 (`POLICY_NOT_FOUND`): availability of the *source* is
/// decided before the *policy* is resolved, so an unknown policy name on a
/// run with no source reports the missing source, not a missing policy.
const EXIT_SOURCE_UNAVAILABLE: i32 = 4;
const MARKER_SOURCE_UNAVAILABLE: &str = "RUN_STATE_SOURCE_UNAVAILABLE";

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
    )
}

#[test]
fn no_source_fails_closed_and_emits_nothing_on_stdout() {
    // Scenario: a run id with no durable state is refused, not invented.
    //
    // The stdout assertion is the load-bearing one: a JSON view on stdout is
    // exactly what a consumer parses and trusts, so "fail closed" has to mean
    // no parseable payload at all, not a view with empty fields.
    let (status, stdout, stderr) = run(&["run-view", "R-decl-cli-001", "--format", "json"]);
    assert_eq!(
        status, EXIT_SOURCE_UNAVAILABLE,
        "must fail closed when no RunStateViewInputs source exists"
    );
    assert!(
        stdout.is_empty(),
        "fail-closed must not emit a parseable view on stdout, got: {stdout}"
    );
    assert!(
        stderr.contains(MARKER_SOURCE_UNAVAILABLE),
        "stderr must carry the typed marker, got: {stderr}"
    );
    assert!(
        serde_json::from_str::<serde_json::Value>(stderr.trim()).is_ok(),
        "stderr must be a typed JSON error, got: {stderr}"
    );
}

#[test]
fn source_availability_is_decided_before_policy() {
    // Scenario: an unknown policy on a source-less run reports the source.
    //
    // Order matters. If policy resolution ran first, this would report
    // POLICY_NOT_FOUND and point the operator at a name typo instead of at
    // the real defect: there is no view source at all.
    let (status, stdout, stderr) = run(&[
        "run-view",
        "R-cli-policy",
        "--policy",
        "definitely-not-a-policy",
    ]);
    assert_eq!(
        status, EXIT_SOURCE_UNAVAILABLE,
        "missing source must outrank an unresolvable policy"
    );
    assert!(stdout.is_empty());
    assert!(
        !stderr.contains("POLICY_NOT_FOUND"),
        "must not blame the policy when the real gap is the missing source: {stderr}"
    );
    assert!(stderr.contains(MARKER_SOURCE_UNAVAILABLE));
}

#[test]
fn text_output_never_reports_a_run_state_section_it_did_not_read() {
    // Scenario: the human-readable form must not claim a section it did not read.
    let (status, stdout, _stderr) = run(&["run-view", "R-cli-text"]);
    assert_eq!(status, EXIT_SOURCE_UNAVAILABLE);
    assert!(
        !stdout.contains("# Run state"),
        "must not print a run-state section derived from no source, got: {stdout}"
    );
    assert!(
        !stdout.contains("# Action surface"),
        "must not print an action surface derived from no source, got: {stdout}"
    );
}

#[test]
fn fabricated_view_shape_is_not_reachable_from_the_cli() {
    // Scenario: the exact shape the scaffold used to fabricate stays unreachable.
    //
    // Pins the field that made the defect dangerous rather than merely wrong:
    // a constant empty `frontier` cannot distinguish "nothing ready" from
    // "never consulted", which is what REQ-CurrentRunView-Shape.md:43 forbids.
    let (_status, stdout, _stderr) = run(&["run-view", "R-cli-default-eq", "--format", "json"]);
    assert!(
        stdout.is_empty(),
        "no CLI path may emit a RunStateView while no real source exists, got: {stdout}"
    );
    assert!(
        !stdout.contains("\"frontier\""),
        "a constant empty frontier must not be observable, got: {stdout}"
    );
}
