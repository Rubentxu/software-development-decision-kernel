//! Contract tests for linting the gate classification registry.
//!
//! Tests that `validate_classifications_registry` correctly validates
//! `gates/classifications.toml`:
//! - waiver_expiry_days ≤ 30 per [[REQ-Process-Gate-Recoverable-Default]]
//! - invalid gate kind rejected
//! - invalid recovery action rejected
//! - missing required fields rejected

use sddk_cli::{Diagnostic, Severity, validate_classifications_registry};
use std::fs;
use std::path::{Path, PathBuf};
use tempfile::TempDir;

/// Holds a TempDir to keep it alive for the duration of the test.
struct ClassificationsTestEnv {
    _dir: TempDir,
    path: PathBuf,
}

impl ClassificationsTestEnv {
    fn new(content: &str) -> Self {
        let dir = TempDir::new().unwrap();
        let path = dir.path().join("classifications.toml");
        fs::write(&path, content).unwrap();
        Self { _dir: dir, path }
    }
}

/// Assert that running `validate_classifications_registry` produces zero diagnostics
/// (all valid).
fn assert_clean(env: &ClassificationsTestEnv) {
    let diagnostics = validate_classifications_registry(&env.path);
    assert!(
        diagnostics.is_empty(),
        "expected no diagnostics, got: {:?}",
        diagnostics
    );
}

/// Assert that running `validate_classifications_registry` produces exactly one
/// error diagnostic with the given code and containing the given substring.
fn assert_one_error(env: &ClassificationsTestEnv, code: &str, contains: &str) {
    let diagnostics = validate_classifications_registry(&env.path);
    let errors: Vec<_> = diagnostics
        .iter()
        .filter(|d| d.severity == Severity::Error)
        .collect();
    assert_eq!(
        errors.len(),
        1,
        "expected 1 error, got {}: {:?}",
        errors.len(),
        diagnostics
    );
    assert!(
        errors[0].code.contains(code),
        "expected code containing {}, got {}",
        code,
        errors[0].code
    );
    assert!(
        errors[0].message.contains(contains),
        "expected message containing '{}', got '{}'",
        contains,
        errors[0].message
    );
}

// ── waiver_expiry_days ≤ 30 ───────────────────────────────────────────────────

#[test]
fn waiver_expiry_days_30_is_valid() {
    let env = ClassificationsTestEnv::new(
        r#"
[gate-test]
class = "process"
recoverable = true
waiver_authority = "lead"
waiver_expiry_days = 30
"#,
    );
    assert_clean(&env);
}

#[test]
fn waiver_expiry_days_31_is_rejected() {
    let env = ClassificationsTestEnv::new(
        r#"
[gate-test]
class = "process"
recoverable = true
waiver_authority = "lead"
waiver_expiry_days = 31
"#,
    );
    assert_one_error(&env, "033", "waiver_expiry_days");
}

#[test]
fn waiver_expiry_days_0_is_valid() {
    let env = ClassificationsTestEnv::new(
        r#"
[gate-test]
class = "security"
recoverable = false
waiver_authority = "security"
waiver_expiry_days = 0
"#,
    );
    assert_clean(&env);
}

#[test]
fn waiver_expiry_days_missing_when_authority_present_is_valid() {
    let env = ClassificationsTestEnv::new(
        r#"
[gate-test]
class = "process"
recoverable = true
waiver_authority = "lead"
"#,
    );
    assert_clean(&env);
}

// ── GateKind validation ────────────────────────────────────────────────────────

#[test]
fn invalid_gate_kind_is_rejected() {
    let env = ClassificationsTestEnv::new(
        r#"
[gate-test]
class = "invalid_kind"
recoverable = true
"#,
    );
    assert_one_error(&env, "033", "gate kind");
}

// ── RecoveryAction validation ──────────────────────────────────────────────────

#[test]
fn invalid_recovery_action_is_rejected() {
    let env = ClassificationsTestEnv::new(
        r#"
[gate-test]
class = "process"
recoverable = true
recovery_action = "invalid_action"
"#,
    );
    assert_one_error(&env, "033", "recovery action");
}

// ── Required fields ────────────────────────────────────────────────────────────

#[test]
fn missing_class_field_is_rejected() {
    let env = ClassificationsTestEnv::new(
        r#"
[gate-test]
recoverable = true
"#,
    );
    let diagnostics = validate_classifications_registry(&env.path);
    assert!(
        !diagnostics.is_empty(),
        "expected diagnostics for missing class field"
    );
}

// ── File not found ────────────────────────────────────────────────────────────

#[test]
fn nonexistent_file_returns_empty_diagnostics() {
    let diagnostics = validate_classifications_registry(Path::new("/nonexistent/path.toml"));
    assert!(diagnostics.is_empty());
}
