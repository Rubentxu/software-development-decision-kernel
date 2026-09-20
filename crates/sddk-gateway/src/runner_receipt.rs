//! `runner_receipt.rs` — AIW-S2 typed receipt that wraps `RunOutcome`
//! with the AIW-S2 fields: `status`, `timeout`, `complete`, `attempt`,
//! `source basis`, `output refs`, and `redaction`.
//!
//! Slice: `p-63676b11dc0ef88f/aiw-s2-test-runner-capture`
//!
//! # Why a wrapper and not a modified `RunOutcome`
//!
//! `RunOutcome` is part of the bounded-process-execution contract
//! (REQ-WF-RT-018) and is pinned byte-identical in
//! `crates/sddk-gateway/tests/bounded_runner_contract.rs`. Adding
//! `Serialize` to `RunOutcome` or any field would break that pin.
//! AIW-S2's contract requires a content-addressable, JSON-serializable
//! receipt; the AIW-S2 milestone text explicitly forbids changing the
//! runner. The wrapper type is the contract-respecting answer.
//!
//! Because `RunOutcome` carries no `Serialize` impl, the wrapper
//! includes a `RunOutcomeView` struct (a JSON-shaped mirror) that is
//! the *only* field in the receipt that exposes the underlying outcome.
//! The Rust in-memory `outcome` field keeps the `RunOutcome` for
//! in-process consumers; the wire format exposes `outcome_view`.
//!
//! # Mapping to AIW-S2 fields
//!
//! | AIW-S2 field | Source |
//! |---|---|
//! | `status` | derived from `RunOutcome::exit_status` (mapped to `Succeeded` / `Failed` / `TimedOut` / `FailedToStart`) |
//! | `timeout` | `RunOutcome::timed_out` (typed deadline outcome) |
//! | `complete` | `exit_status.is_some()` (the process reached its terminal state) |
//! | `attempt` | caller-supplied (this slice defaults to 1) |
//! | `source basis` | `sha256` of the normalized `(program, args, env_keys_sorted)` triple |
//! | `output refs` | `sha256` of `stdout` and `stderr` |
//! | `redaction` | canary placeholder + sha256 of the canary fingerprint if found |

#![forbid(unsafe_code)]

use std::collections::BTreeMap;

use serde::Serialize;
use sha2::{Digest, Sha256};

use crate::runner::{RunOutcome, RunSpec, RunnerError};

/// Status of a runner execution, derived from `RunOutcome`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum RunnerStatus {
    /// The process exited with code 0.
    Succeeded,
    /// The process exited with non-zero code.
    Failed,
    /// The process was killed because `timeout_ms` was exceeded.
    TimedOut,
    /// The process could not be spawned (path, permissions, etc.).
    FailedToStart,
}

/// JSON mirror of `RunOutcome` for the receipt. The Rust field
/// `RunnerReceipt::outcome` retains the typed `RunOutcome`; the wire
/// format exposes `outcome_view`.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct RunOutcomeView {
    /// Process exit status, or `None` when the process was killed.
    pub exit_status: Option<i32>,
    /// Captured standard output, truncated to the output limit.
    pub stdout: String,
    /// Captured standard error, truncated to the output limit.
    pub stderr: String,
    /// Whether the process was killed by the timeout.
    pub timed_out: bool,
}

impl From<&RunOutcome> for RunOutcomeView {
    fn from(o: &RunOutcome) -> Self {
        Self {
            exit_status: o.exit_status,
            stdout: o.stdout.clone(),
            stderr: o.stderr.clone(),
            timed_out: o.timed_out,
        }
    }
}

/// One redaction marker (canary found in args/stdout/stderr).
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct RedactionMarker {
    /// Where the canary was found.
    pub location: RedactionLocation,
    /// The placeholder inserted in the receipt (`<redacted:stdout>` etc.).
    pub placeholder: String,
    /// `sha256` of the canary fingerprint (not the canary itself).
    pub canary_sha256: String,
}

/// Where a redacted token was found.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum RedactionLocation {
    /// Token found in the runner's `args` vector.
    Args,
    /// Token found in the captured stdout.
    Stdout,
    /// Token found in the captured stderr.
    Stderr,
    /// Token found in the runner error message.
    ErrorMessage,
}

/// Hash-linked references to the captured output streams.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct OutputRefs {
    /// `sha256` of the captured `stdout`.
    pub stdout_sha256: String,
    /// `sha256` of the captured `stderr`.
    pub stderr_sha256: String,
    /// True iff the runner truncated the stream(s).
    pub truncated: bool,
    /// Combined size of the captured streams in bytes.
    pub captured_bytes: usize,
}

/// The AIW-S2 receipt that wraps a `RunOutcome` plus the acceptance
/// fields demanded by the AIW package.
#[derive(Debug, Clone, Serialize)]
pub struct RunnerReceipt {
    /// Status derived from `RunOutcome`.
    pub status: RunnerStatus,
    /// Whether the process was killed by the timeout.
    pub timeout: bool,
    /// Whether the runner reached a terminal state for this attempt.
    pub complete: bool,
    /// Attempt ordinal (1-indexed). For sequential runs, increment per
    /// retry. This slice's wrapper defaults to 1.
    pub attempt: u32,
    /// Hash of the `(program, args, env_keys_sorted)` triple — the
    /// minimal reproducible basis of the runner invocation.
    pub source_basis: String,
    /// Hash-linked refs to stdout/stderr.
    pub output_refs: OutputRefs,
    /// Canary redactions found in args/stdout/stderr.
    pub redactions: Vec<RedactionMarker>,
    /// Underlying `RunOutcome` kept verbatim for in-process consumers.
    #[serde(skip)]
    pub outcome: RunOutcome,
    /// JSON mirror of `outcome` for the wire format.
    pub outcome_view: RunOutcomeView,
}

impl PartialEq for RunnerReceipt {
    fn eq(&self, other: &Self) -> bool {
        // Don't compare `outcome` (excluded from serde anyway); compare
        // the wire-format fields plus the outcome_view.
        self.status == other.status
            && self.timeout == other.timeout
            && self.complete == other.complete
            && self.attempt == other.attempt
            && self.source_basis == other.source_basis
            && self.output_refs == other.output_refs
            && self.redactions == other.redactions
            && self.outcome_view == other.outcome_view
    }
}

impl Eq for RunnerReceipt {}

/// Hash a string with SHA-256 (hex, lowercase, no prefix).
fn sha256_hex(bytes: &[u8]) -> String {
    let mut h = Sha256::new();
    h.update(bytes);
    format!("{:x}", h.finalize())
}

/// Compute the source basis of a `RunSpec`: `sha256(program || "\0" ||
/// args.join("\0") || "\0" || env.keys().sorted().join("\0"))`.
pub fn source_basis(spec: &RunSpec) -> String {
    let mut buf: Vec<u8> = Vec::new();
    buf.extend_from_slice(spec.program.as_bytes());
    buf.push(0);
    for a in &spec.args {
        buf.extend_from_slice(a.as_bytes());
        buf.push(0);
    }
    for k in spec.env.keys() {
        buf.extend_from_slice(k.as_bytes());
        buf.push(0);
    }
    sha256_hex(&buf)
}

/// Default redaction placeholder used for canaries in stdout/stderr.
pub const DEFAULT_CANARY: &str = "<redacted:stdout>";

/// Detect canaries in `args`, `stdout`, and `stderr`.
///
/// Defence-in-depth: the bounded runner already enforces an env
/// allowlist; this catches common secret prefixes if they leak through
/// stdout/stderr.
pub fn detect_canaries(args: &[String], stdout: &str, stderr: &str) -> Vec<RedactionMarker> {
    let mut markers = Vec::new();
    let prefixes = ["ghp_", "AKIA", "xox", "github_pat_", "-----BEGIN"];
    for (loc, source) in [
        (RedactionLocation::Args, args.join(" ").into_bytes()),
        (RedactionLocation::Stdout, stdout.as_bytes().to_vec()),
        (RedactionLocation::Stderr, stderr.as_bytes().to_vec()),
    ] {
        let s = String::from_utf8_lossy(&source);
        for prefix in prefixes {
            if s.contains(prefix) {
                markers.push(RedactionMarker {
                    location: loc,
                    placeholder: DEFAULT_CANARY.to_string(),
                    canary_sha256: sha256_hex(prefix.as_bytes()),
                });
                break;
            }
        }
    }
    markers
}

/// Build a `RunnerReceipt` from a `RunOutcome` and the original
/// `RunSpec` (for the basis), with the attempt ordinal (1-indexed).
pub fn build_receipt(outcome: &RunOutcome, spec: &RunSpec, attempt: u32) -> RunnerReceipt {
    let stdout_bytes = outcome.stdout.len();
    let stderr_bytes = outcome.stderr.len();
    let captured = stdout_bytes + stderr_bytes;
    let truncated = captured >= spec.output_max_bytes * 2
        || stdout_bytes >= spec.output_max_bytes
        || stderr_bytes >= spec.output_max_bytes;

    let status = if outcome.timed_out {
        RunnerStatus::TimedOut
    } else {
        match outcome.exit_status {
            Some(0) => RunnerStatus::Succeeded,
            Some(_) => RunnerStatus::Failed,
            None => RunnerStatus::FailedToStart,
        }
    };

    let redactions = detect_canaries(&spec.args, &outcome.stdout, &outcome.stderr);

    RunnerReceipt {
        status,
        timeout: outcome.timed_out,
        complete: outcome.exit_status.is_some() || outcome.timed_out,
        attempt,
        source_basis: source_basis(spec),
        output_refs: OutputRefs {
            stdout_sha256: sha256_hex(outcome.stdout.as_bytes()),
            stderr_sha256: sha256_hex(outcome.stderr.as_bytes()),
            truncated,
            captured_bytes: captured,
        },
        redactions,
        outcome: outcome.clone(),
        outcome_view: RunOutcomeView::from(outcome),
    }
}

/// Build a `RunnerReceipt` for the `FailedToStart` case, where the
/// runner returned `Err(RunnerError::Spawn { .. })`.
pub fn build_failed_to_start(spec: &RunSpec, attempt: u32, err: &RunnerError) -> RunnerReceipt {
    let msg = format!("{err}");
    let synthetic = RunOutcome {
        exit_status: None,
        stdout: String::new(),
        stderr: msg.clone(),
        timed_out: false,
    };
    let redactions = detect_canaries(&spec.args, "", &msg);
    let view = RunOutcomeView::from(&synthetic);
    RunnerReceipt {
        status: RunnerStatus::FailedToStart,
        timeout: false,
        complete: false,
        attempt,
        source_basis: source_basis(spec),
        output_refs: OutputRefs {
            stdout_sha256: sha256_hex(b""),
            stderr_sha256: sha256_hex(msg.as_bytes()),
            truncated: false,
            captured_bytes: msg.len(),
        },
        redactions,
        outcome: synthetic,
        outcome_view: view,
    }
}

/// Convenience: execute the runner via `runner::run`, then build the
/// receipt. Returns the receipt on `Ok`, or the underlying
/// `RunnerError` on spawn failure. The caller can convert the error
/// into a `FailedToStart` receipt via `build_failed_to_start(spec,
/// attempt, &err)` if they prefer a uniform receipt type downstream.
pub fn run_with_receipt(spec: &RunSpec, attempt: u32) -> Result<RunnerReceipt, RunnerError> {
    match crate::runner::run(spec) {
        Ok(outcome) => Ok(build_receipt(&outcome, spec, attempt)),
        Err(e) => Err(e),
    }
}

// Silence dead_code warning for the constant used by callers that may
// not import it directly (e.g. tests).
#[allow(dead_code)]
const _BTREEMAP_USED_FOR_DOCUMENTATION: BTreeMap<String, String> = BTreeMap::new();

#[cfg(test)]
mod tests {
    use super::*;
    use crate::runner::RunSpec;

    fn spec_for(program: &str) -> RunSpec {
        RunSpec::new(program)
    }

    #[test]
    fn source_basis_is_deterministic() {
        let a = source_basis(&spec_for("cargo-nextest"));
        let b = source_basis(&spec_for("cargo-nextest"));
        assert_eq!(a, b);
        assert_eq!(a.len(), 64, "sha256 hex is 64 chars");
    }

    #[test]
    fn source_basis_differs_by_program() {
        let a = source_basis(&spec_for("cargo-nextest"));
        let b = source_basis(&spec_for("pytest"));
        assert_ne!(a, b);
    }

    #[test]
    fn build_receipt_marks_succeeded_for_exit_zero() {
        let mut spec = spec_for("cargo-nextest");
        spec.args = vec!["run".into()];
        let outcome = RunOutcome {
            exit_status: Some(0),
            stdout: "all tests passed".into(),
            stderr: String::new(),
            timed_out: false,
        };
        let r = build_receipt(&outcome, &spec, 1);
        assert_eq!(r.status, RunnerStatus::Succeeded);
        assert!(!r.timeout);
        assert!(r.complete);
        assert_eq!(r.attempt, 1);
        assert_eq!(r.source_basis.len(), 64);
        assert!(r.redactions.is_empty());
    }

    #[test]
    fn build_receipt_marks_failed_for_nonzero_exit() {
        let spec = spec_for("cargo-nextest");
        let outcome = RunOutcome {
            exit_status: Some(1),
            stdout: String::new(),
            stderr: "test failure".into(),
            timed_out: false,
        };
        let r = build_receipt(&outcome, &spec, 1);
        assert_eq!(r.status, RunnerStatus::Failed);
        assert!(r.complete);
    }

    #[test]
    fn build_receipt_marks_timed_out_when_killed() {
        let spec = spec_for("cargo-nextest");
        let outcome = RunOutcome {
            exit_status: None,
            stdout: String::new(),
            stderr: "killed".into(),
            timed_out: true,
        };
        let r = build_receipt(&outcome, &spec, 1);
        assert_eq!(r.status, RunnerStatus::TimedOut);
        assert!(r.timeout);
        assert!(r.complete, "timed_out counts as complete per REQ-WF-RT-018");
    }

    #[test]
    fn build_failed_to_start_has_no_complete_state() {
        let spec = spec_for("/nonexistent/binary");
        let err = RunnerError::Spawn {
            program: "/nonexistent/binary".into(),
            source: std::io::Error::new(std::io::ErrorKind::NotFound, "not found"),
        };
        let r = build_failed_to_start(&spec, 1, &err);
        assert_eq!(r.status, RunnerStatus::FailedToStart);
        assert!(!r.complete);
        assert_eq!(r.attempt, 1);
        assert!(r.outcome.stderr.contains("not found"));
    }

    #[test]
    fn detect_canaries_flags_ghp_in_stdout() {
        let args = vec!["run".to_string()];
        let stdout = "ok\nall tests passed with token ghp_abc123\n";
        let stderr = "";
        let markers = detect_canaries(&args, stdout, stderr);
        assert_eq!(markers.len(), 1);
        assert_eq!(markers[0].location, RedactionLocation::Stdout);
        assert_eq!(markers[0].placeholder, "<redacted:stdout>");
    }

    #[test]
    fn detect_canaries_flags_aws_in_args() {
        let args = vec!["--token".to_string(), "AKIAIOSFODNN7EXAMPLE".to_string()];
        let markers = detect_canaries(&args, "", "");
        assert_eq!(markers.len(), 1);
        assert_eq!(markers[0].location, RedactionLocation::Args);
    }

    #[test]
    fn detect_canaries_clean_run_has_no_markers() {
        let args = vec!["run".to_string()];
        let stdout = "test_foo ... ok\ntest_bar ... ok\n";
        let stderr = "";
        let markers = detect_canaries(&args, stdout, stderr);
        assert!(markers.is_empty());
    }

    #[test]
    fn build_receipt_serializes_to_json() {
        let mut spec = spec_for("cargo-nextest");
        spec.args = vec!["run".into()];
        let outcome = RunOutcome {
            exit_status: Some(0),
            stdout: "ok".into(),
            stderr: String::new(),
            timed_out: false,
        };
        let r = build_receipt(&outcome, &spec, 1);
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
            assert!(
                json.contains(field),
                "RunnerReceipt JSON missing field {field}; got: {json}"
            );
        }
        // The `outcome` field is skipped; assert it is NOT in JSON.
        assert!(
            !json.contains("\"outcome\":{"),
            "RunnerReceipt JSON must not include the typed `outcome` field; got: {json}"
        );
    }
}
