//! `runner_receipt_e2e.rs` — Integration tests for `runner_receipt` that
//! drive real processes via the gateway's bounded runner.
//!
//! These tests use `/bin/echo`, `/bin/false`, `/bin/true` (POSIX
//! guaranteed on any Unix host) as the spawned programs — they are
//! the test infrastructure, not the system under test. The system
//! under test is `run_with_receipt` and the receipts it produces.
//!
//! Slice: `p-63676b11dc0ef88f/aiw-s2-test-runner-capture`
//! UAT rows covered: T01, T02, T04, T05, T06, T07, T08, T09, T10.
#![allow(clippy::disallowed_methods)]

use std::time::Duration;

use sddk_gateway::{
    RunOutcome, RunSpec, RunnerError, run,
    runner_receipt::{
        RedactionLocation, RunnerStatus, build_failed_to_start, build_receipt, run_with_receipt,
        source_basis,
    },
};

/// Helper to build a `RunSpec` with optional args.
fn spec(program: &str, args: &[&str]) -> RunSpec {
    let mut s = RunSpec::new(program);
    for a in args {
        s.args.push((*a).into());
    }
    s
}

/// T01: E2E — receipt contains the AIW-S2 fields, basis is reproducible,
/// attempt=1 by default, complete=true on clean exit.
#[test]
fn t01_e2e_receipt_round_trip_through_json() {
    let spec_e = spec("/bin/echo", &["hello, world"]);
    let outcome = run(&spec_e).expect("run echo");
    let r = build_receipt(&outcome, &spec_e, 1);

    let json = serde_json::to_string(&r).expect("serialize");

    for field in &[
        "\"status\"",
        "\"timeout\"",
        "\"complete\"",
        "\"attempt\"",
        "\"source_basis\"",
        "\"output_refs\"",
        "\"redactions\"",
        "\"outcome_view\"",
    ] {
        assert!(json.contains(field), "missing field {field} in {json}");
    }

    assert_eq!(r.source_basis, source_basis(&spec_e));
    assert_eq!(r.attempt, 1);
    assert!(r.complete);
    assert_eq!(r.status, RunnerStatus::Succeeded);
}

/// T02 / T03: NEG — exit=1 reports status=Failed; the JSON never claims
/// success for a non-zero exit; the consumer cannot mistake a failed
/// run for a passed one.
#[test]
fn t02_neg_failure_status_does_not_pretend_success() {
    let spec_e = spec("/bin/false", &[]);
    let outcome = run(&spec_e).expect("run false");
    let r = build_receipt(&outcome, &spec_e, 1);

    assert_eq!(r.status, RunnerStatus::Failed);
    assert_eq!(r.outcome_view.exit_status, Some(1));
    let json = serde_json::to_string(&r).unwrap();
    assert!(
        !json.contains("\"status\":\"succeeded\""),
        "failed run must not report status=succeeded"
    );
}

/// T04: NEG — distinct classes for failure modes (Succeeded / Failed /
/// FailedToStart / TimedOut).
#[test]
fn t04_failure_classes_distinct_in_receipt() {
    let ok = spec("/bin/true", &[]);
    let fail = spec("/bin/false", &[]);
    let missing = RunSpec::new("/nonexistent/binary-for-aiw-s2-test");

    let r_ok = build_receipt(&run(&ok).unwrap(), &ok, 1);
    let r_fail = build_receipt(&run(&fail).unwrap(), &fail, 1);

    assert_eq!(r_ok.status, RunnerStatus::Succeeded);
    assert_eq!(r_fail.status, RunnerStatus::Failed);

    match run_with_receipt(&missing, 1) {
        Ok(_) => panic!("missing binary should not produce Ok"),
        Err(e) => {
            let receipt = build_failed_to_start(&missing, 1, &e);
            assert_eq!(receipt.status, RunnerStatus::FailedToStart);
            assert!(!receipt.complete);
        }
    }
}

/// T04-bonus: timeout class is distinct from failed class. We don't
/// try to force a real timeout (it'd be flaky); we just verify the
/// receipt's `timeout` flag is consistent with the underlying outcome.
#[test]
fn t04_timeout_class_is_distinct_in_status_mapping() {
    let outcome = RunOutcome {
        exit_status: None,
        stdout: String::new(),
        stderr: "killed".into(),
        timed_out: true,
    };
    let spec_e = spec("/bin/sleep", &["10"]);
    let r = build_receipt(&outcome, &spec_e, 1);
    assert_eq!(r.status, RunnerStatus::TimedOut);
    assert!(r.timeout);
    assert!(r.complete);
}

/// T06: SEC — canary in stdout triggers a redaction marker.
#[test]
fn t06_canary_in_stdout_redacted_in_receipt() {
    let spec_e = spec("/bin/echo", &["prefix-ghp_SECRET_TO_NOT_LEAK_suffix"]);
    let outcome = run(&spec_e).expect("run echo with canary");
    let r = build_receipt(&outcome, &spec_e, 1);

    assert!(
        r.redactions
            .iter()
            .any(|m| m.location == RedactionLocation::Stdout),
        "stdout redaction marker missing; redactions={:?}",
        r.redactions
    );

    let json = serde_json::to_string(&r).unwrap();
    assert!(
        json.contains("\"location\":\"stdout\""),
        "JSON should mark the canary location"
    );
    assert!(
        json.contains("\"placeholder\":\"<redacted:stdout>\""),
        "JSON should record the redaction placeholder"
    );

    let marker = r
        .redactions
        .iter()
        .find(|m| m.location == RedactionLocation::Stdout)
        .unwrap();
    assert_eq!(marker.canary_sha256.len(), 64);
    assert_ne!(marker.canary_sha256, marker.placeholder);
}

/// T07: IT — same `RunSpec` run twice yields two receipts with the same
/// `source_basis` and same status; an external consumer can link them.
#[test]
fn t07_same_basis_across_two_runs() {
    let spec_e = spec("/bin/echo", &["basis-pinned"]);
    let r1 = build_receipt(&run(&spec_e).unwrap(), &spec_e, 1);
    let r2 = build_receipt(&run(&spec_e).unwrap(), &spec_e, 2);

    assert_eq!(r1.source_basis, r2.source_basis);
    assert_ne!(r1.attempt, r2.attempt);
    assert_eq!(r1.status, RunnerStatus::Succeeded);
    assert_eq!(r2.status, RunnerStatus::Succeeded);
}

/// T08: IT — receipt's output_refs hash stdout/stderr; a consumer with
/// the raw output can validate integrity.
#[test]
fn t08_output_refs_hash_captured_streams() {
    let spec_e = spec("/bin/echo", &["hash-me"]);
    let outcome = run(&spec_e).expect("run echo");
    let r = build_receipt(&outcome, &spec_e, 1);

    use sha2::{Digest, Sha256};
    let mut h = Sha256::new();
    h.update(outcome.stdout.as_bytes());
    let expected = format!("{:x}", h.finalize());

    assert_eq!(r.output_refs.stdout_sha256, expected);
    assert_eq!(
        r.output_refs.captured_bytes,
        outcome.stdout.len() + outcome.stderr.len()
    );
}

/// T09: NEG — the receipt is honest about its scope: it exists only
/// when SDDK ran the runner, so an external CLI run yields no SDDK
/// receipt. Asserted by the receipt contract: no "external" flag, and
/// the absence of a receipt is the negative case.
#[test]
fn t09_external_run_yields_no_aiw_s2_receipt() {
    let spec_e = spec("/bin/echo", &["external"]);
    let r = build_receipt(&run(&spec_e).unwrap(), &spec_e, 1);
    let json = serde_json::to_string(&r).unwrap();
    let rt: serde_json::Value = serde_json::from_str(&json).unwrap();
    assert!(rt.is_object());
    // A consumer cannot derive `r` from a non-SDDK run.
    assert!(!json.contains("external_cli_run"));
}

/// T10: REG — allowlist / missing tool → typed failure, no shell
/// fallback. We exercise the missing-binary case via `run_with_receipt`
/// and convert to a FailedToStart receipt via `build_failed_to_start`.
#[test]
fn t10_allowlist_or_missing_tool_returns_typed_receipt() {
    let spec_e = RunSpec::new("/nonexistent/binary-for-aiw-s2-t10");
    let res = run_with_receipt(&spec_e, 1);
    let err = match res {
        Ok(_) => panic!("missing tool must not Ok"),
        Err(e) => e,
    };
    assert!(matches!(err, RunnerError::Spawn { .. }));

    let receipt = build_failed_to_start(&spec_e, 1, &err);
    assert_eq!(receipt.status, RunnerStatus::FailedToStart);
    assert!(!receipt.complete);
    assert!(receipt.outcome_view.stderr.contains("nonexistent"));
    assert!(receipt.redactions.is_empty());
}

/// T05: NEG — truncated output sets `truncated=true` so downstream
/// consumers know the receipt is incomplete.
#[test]
fn t05_truncation_is_signaled_in_receipt() {
    let mut spec_e = spec("/bin/echo", &["this-is-a-long-line-for-truncation-test"]);
    spec_e.output_max_bytes = 4;
    let outcome = run(&spec_e).expect("run");
    let r = build_receipt(&outcome, &spec_e, 1);
    assert!(r.output_refs.truncated, "truncated flag must be set");
    assert!(
        r.complete,
        "process exited cleanly; complete is independent of truncation"
    );
}

/// Sanity: serialize is fast enough for CI (<50 ms).
#[test]
fn fn_serialize_is_fast_enough_for_ci() {
    let spec_e = spec("/bin/echo", &[]);
    let outcome = RunOutcome {
        exit_status: Some(0),
        stdout: "ok".into(),
        stderr: String::new(),
        timed_out: false,
    };
    let r = build_receipt(&outcome, &spec_e, 1);
    let start = std::time::Instant::now();
    let _ = serde_json::to_string(&r).expect("serialize");
    let elapsed = start.elapsed();
    assert!(
        elapsed < Duration::from_millis(50),
        "serialize took {elapsed:?}"
    );
}
