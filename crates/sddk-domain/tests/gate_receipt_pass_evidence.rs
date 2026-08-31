//! Contract tests for gate receipt pass evidence validation.
//!
//! Per REQ-IPV (spec-v2 cycle-44), a phase gate MUST NOT be satisfied by an
//! agent-reported PASS alone. For a Passed outcome, the evidence MUST contain
//! ALL THREE of:
//! - `argv`: the command executed (array of strings)
//! - `exit_code`: the process exit code (integer)
//! - `output_digest`: a SHA-256 or similar digest of the output (string)
//!
//! Scenarios covered:
//! - PASS with all three fields → accepted
//! - PASS with only argv missing → rejected with MissingArgv
//! - PASS with only exit_code missing → rejected with MissingExitCode
//! - PASS with only output_digest missing → rejected with MissingOutputDigest
//! - Evidence is not an object → rejected with NotAnObject
//! - Happy path with all three fields present

use sddk_domain::models::gate_receipt::{PassEvidenceError, validate_pass_evidence};
use serde_json::json;

// ── REQ-IPV spec-v2: all three fields required for Passed ────────────────────────

#[test]
fn passed_with_all_three_fields_is_accepted() {
    // Happy path: all three required fields present
    let evidence = json!({
        "argv": ["cargo", "test", "--workspace"],
        "exit_code": 0,
        "output_digest": "sha256:abc123",
    });
    assert!(validate_pass_evidence(&evidence).is_ok());
}

#[test]
fn passed_with_only_argv_missing_is_rejected() {
    // REQ-IPV scenario: missing argv alone is sufficient to reject
    let evidence = json!({
        "exit_code": 0,
        "output_digest": "sha256:abc123",
    });
    let err = validate_pass_evidence(&evidence).unwrap_err();
    assert!(matches!(err, PassEvidenceError::MissingArgv));
}

#[test]
fn passed_with_only_exit_code_missing_is_rejected() {
    // REQ-IPV scenario: missing exit_code alone is sufficient to reject
    let evidence = json!({
        "argv": ["cargo", "test", "--workspace"],
        "output_digest": "sha256:abc123",
    });
    let err = validate_pass_evidence(&evidence).unwrap_err();
    assert!(matches!(err, PassEvidenceError::MissingExitCode));
}

#[test]
fn passed_with_only_output_digest_missing_is_rejected() {
    // REQ-IPV scenario: missing output_digest alone is sufficient to reject
    let evidence = json!({
        "argv": ["cargo", "test", "--workspace"],
        "exit_code": 0,
    });
    let err = validate_pass_evidence(&evidence).unwrap_err();
    assert!(matches!(err, PassEvidenceError::MissingOutputDigest));
}

#[test]
fn non_object_evidence_is_rejected() {
    // Evidence must be a JSON object
    let evidence = serde_json::Value::String("not an object".into());
    let err = validate_pass_evidence(&evidence).unwrap_err();
    assert!(matches!(err, PassEvidenceError::NotAnObject));
}

// ── Backward compatibility: agent_reported no longer bypasses validation ─────────

#[test]
fn passed_agent_reported_without_all_three_fields_is_rejected() {
    // Per spec-v2, agent_reported flag no longer provides bypass.
    // All three fields are required regardless of who made the claim.
    let evidence = json!({
        "agent_reported": true,
        "result": "passed",
        "argv": ["cargo", "test", "--workspace"],
        // exit_code and output_digest missing
    });
    let err = validate_pass_evidence(&evidence).unwrap_err();
    assert!(matches!(
        err,
        PassEvidenceError::MissingExitCode | PassEvidenceError::MissingOutputDigest
    ));
}

#[test]
fn passed_agent_reported_with_all_three_fields_is_accepted() {
    // agent_reported flag is irrelevant when all three fields are present
    let evidence = json!({
        "agent_reported": true,
        "argv": ["cargo", "test", "--workspace"],
        "exit_code": 0,
        "output_digest": "sha256:abc123",
    });
    assert!(validate_pass_evidence(&evidence).is_ok());
}
