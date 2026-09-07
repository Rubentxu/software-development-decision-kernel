//! Tests for Choice guard evaluation (REQ-WFR3-CHOICE-001).
//!
//! Verifies the guard evaluation logic in Choice::evaluate_guard static method.
//! The full runtime integration tests are blocked by a tick loop issue.

use std::collections::BTreeMap;
use sddk_engine::operator::{Choice, ChoiceGuardError};
use serde_json::Value;

// ── Helper to call evaluate_guard ──────────────────────────────────────────────

fn eval_guard(guard: &str, outputs: &BTreeMap<String, Value>) -> Result<bool, ChoiceGuardError> {
    Choice::evaluate_guard(guard, outputs)
}

// ── LiteralBool tests ─────────────────────────────────────────────────────────

#[test]
fn guard_true_always_passes() {
    let outputs = BTreeMap::new();
    assert_eq!(eval_guard("true", &outputs).unwrap(), true);
}

#[test]
fn guard_true_case_insensitive() {
    let outputs = BTreeMap::new();
    assert_eq!(eval_guard("TRUE", &outputs).unwrap(), true);
    assert_eq!(eval_guard("True", &outputs).unwrap(), true);
}

#[test]
fn guard_false_never_passes() {
    let outputs = BTreeMap::new();
    assert_eq!(eval_guard("false", &outputs).unwrap(), false);
}

#[test]
fn guard_false_case_insensitive() {
    let outputs = BTreeMap::new();
    assert_eq!(eval_guard("FALSE", &outputs).unwrap(), false);
    assert_eq!(eval_guard("False", &outputs).unwrap(), false);
}

// ── IdentifierEq tests ─────────────────────────────────────────────────────────

#[test]
fn guard_identifier_eq_matches() {
    let mut outputs = BTreeMap::new();
    outputs.insert("status".into(), Value::String("ready".into()));

    assert_eq!(eval_guard("status==ready", &outputs).unwrap(), true);
}

#[test]
fn guard_identifier_eq_with_equals_sign() {
    let mut outputs = BTreeMap::new();
    outputs.insert("status".into(), Value::String("ready".into()));

    assert_eq!(eval_guard("status=ready", &outputs).unwrap(), true);
}

#[test]
fn guard_identifier_eq_no_match() {
    let mut outputs = BTreeMap::new();
    outputs.insert("status".into(), Value::String("pending".into()));

    assert_eq!(eval_guard("status==ready", &outputs).unwrap(), false);
}

#[test]
fn guard_identifier_eq_missing_key() {
    let outputs = BTreeMap::new(); // empty outputs

    assert_eq!(eval_guard("status==ready", &outputs).unwrap(), false);
}

#[test]
fn guard_identifier_eq_with_spaces() {
    let mut outputs = BTreeMap::new();
    outputs.insert("status".into(), Value::String("ready".into()));

    assert_eq!(eval_guard("status == ready", &outputs).unwrap(), true);
}

// ── Unsupported guard tests ───────────────────────────────────────────────────

#[test]
fn guard_unsupported_format_treated_as_pass() {
    // Unknown guards are treated as "pass" for backward compatibility
    let outputs = BTreeMap::new();
    let result = eval_guard("complex.expr(x,y)", &outputs);
    assert!(matches!(result, Ok(true)));
}

#[test]
fn guard_always_true_is_literal_bool() {
    // "always-true" is not the special "true" literal, so it's treated as unknown → pass
    let outputs = BTreeMap::new();
    let result = eval_guard("always-true", &outputs);
    assert!(matches!(result, Ok(true)));
}
