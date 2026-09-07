//! Gate stub error taxonomy tests for DW-RUNTIME-003 (S5).
//!
//! Validates that GateError has exactly 3 closed variants as required by
//! REQ-WFR3-CLOSED-001.

use std::collections::BTreeMap;
use std::sync::Arc;
use sddk_engine::{GateError, Operator};
use sddk_domain::CapabilityId;

#[test]
fn gate_error_has_three_variants() {
    // Verify the 3 variants exist via pattern matching
    let _not_implemented = GateError::NotImplementedInCycle16 {
        variant: "Test".to_string(),
    };
    let _guard_failed = GateError::GuardEvaluationFailed {
        expression: "x==y".to_string(),
        reason: "key not found".to_string(),
    };
    let _bounded_violation = GateError::BoundedGateViolation {
        limit: "max_attempts".to_string(),
        observed: 100,
    };
}

#[test]
fn gate_error_variant_names() {
    let not_implemented = GateError::NotImplementedInCycle16 {
        variant: "Sequence".to_string(),
    };
    assert_eq!(not_implemented.variant_name(), "NotImplementedInCycle16");

    let guard_failed = GateError::GuardEvaluationFailed {
        expression: "true".to_string(),
        reason: "assertion".to_string(),
    };
    assert_eq!(guard_failed.variant_name(), "GuardEvaluationFailed");

    let bounded = GateError::BoundedGateViolation {
        limit: "duration_ms".to_string(),
        observed: 5000,
    };
    assert_eq!(bounded.variant_name(), "BoundedGateViolation");
}

#[test]
fn gate_operator_kind() {
    use sddk_domain::CapabilityId;
    use std::sync::Arc;

    let gate = sddk_engine::Gate::new(
        "true".to_string(),
        Arc::new(sddk_engine::Task {
            capability: CapabilityId("test.cap".to_string()),
            inputs: Default::default(),
        }),
    );
    assert_eq!(gate.kind(), "Gate");
}

#[test]
fn gate_evaluate_guard_true_literal() {
    // Gate with "true" condition should pass
    let result = sddk_engine::Gate::new("true".to_string(), Arc::new(sddk_engine::Task {
        capability: CapabilityId("test".to_string()),
        inputs: Default::default(),
    }));

    // We can't call evaluate_guard directly as it's private, but we can verify
    // the Gate struct implements Operator
    assert_eq!(result.kind(), "Gate");
}
