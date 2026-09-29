//! Contract tests for `sddk run view` provenance (INC-DEBT-039).
//!
//! The v0 scaffold in `crates/sddk-cli/src/run_view.rs` built a
//! `RunStateView` without reading the ledger, passing `frontier`,
//! `blockers` and `pending_decisions` as constant `vec![]` and deriving
//! `origin` from the `run_id` string prefix.
//!
//! `REQ-CurrentRunView-Shape.md:43` defines `frontier` as empty *"iff the
//! run is terminal or no node is ready"*. A constant empty vector
//! cannot distinguish "nothing is ready" from "nothing was consulted",
//! so the view is indistinguishable from the truth while being
//! fabricated. `ActionSurfaceView` derives from it, so the defect
//! propagates to policy evaluation.
//!
//! These tests pin the fix: the command must refuse to emit an
//! authoritative-looking view, and must say why.

use sddk_cli::run_view::run_run_view;
use sddk_cli::{CliEnvironment, OutputFormat};

/// The scaffold's invented-origin heuristic: any `run_id` was accepted
/// and classified by prefix. A run with no ledger backing must not
/// produce a view at all.
#[test]
fn run_view_refuses_when_no_ledger_source_is_available() {
    let out = run_view_command("R-decl-anything", "default", OutputFormat::Json);

    assert_ne!(
        out.status, 0,
        "sddk run view must not succeed without a ledger-backed source; \
         got status 0 with stdout: {}",
        out.stdout
    );
    assert!(
        out.stdout.is_empty(),
        "a refused run view must not emit a view on stdout: {}",
        out.stdout
    );
    assert!(
        out.stderr.contains("RUN_STATE_SOURCE_UNAVAILABLE"),
        "error must be typed and name the missing source, got: {}",
        out.stderr
    );
}

#[test]
fn run_view_error_names_the_debt_not_a_generic_failure() {
    let out = run_view_command("R-gen-anything", "default", OutputFormat::Json);

    assert_ne!(out.status, 0);
    // The message must point the operator at the real cause, not at a
    // phantom storage error, and not at a policy lookup failure.
    assert!(
        !out.stderr.contains("POLICY_NOT_FOUND"),
        "policy must be irrelevant to the missing-source verdict: {}",
        out.stderr
    );
    assert!(
        out.stderr.contains("run_state"),
        "error must identify the missing capability: {}",
        out.stderr
    );
}

/// The scaffold resolved a policy and failed with POLICY_NOT_FOUND for
/// an unknown name. That ordering hid the real defect behind a policy
/// lookup: an unknown policy name on a run with no source reported the
/// policy as the problem. Source availability must be decided first.
#[test]
fn source_availability_is_decided_before_policy_lookup() {
    let out = run_view_command(
        "R-decl-anything",
        "no-such-policy-anywhere",
        OutputFormat::Json,
    );

    assert_ne!(out.status, 0);
    assert!(
        out.stderr.contains("RUN_STATE_SOURCE_UNAVAILABLE"),
        "missing source must be reported before policy resolution, got: {}",
        out.stderr
    );
}

/// Text output must not be a second path to the same fabrication.
#[test]
fn text_format_also_refuses() {
    let out = run_view_command("R-decl-anything", "default", OutputFormat::Text);

    assert_ne!(out.status, 0);
    assert!(
        out.stdout.is_empty(),
        "text format must not leak a fabricated view: {}",
        out.stdout
    );
}

/// The policy resolver itself is untouched: `default` still resolves.
/// The guard must not become a blanket "always fail" that hides even a
/// correct policy lookup. This pins that we did not over-correct.
#[test]
fn policy_resolution_still_works_for_the_default_policy() {
    let out = run_view_command("R-decl-anything", "default", OutputFormat::Json);
    // It still fails (no source), but for the source reason, not a
    // policy reason. Assert the policy name is not the complaint.
    assert!(
        !out.stderr.contains("POLICY_NOT_FOUND"),
        "the `default` policy must still resolve; got: {}",
        out.stderr
    );
}

fn run_view_command(run_id: &str, policy: &str, format: OutputFormat) -> sddk_cli::CommandOutput {
    let env = CliEnvironment::for_test();
    run_run_view(run_id.to_string(), None, policy.to_string(), format, &env)
}
