//! Gate error taxonomy for DW-RUNTIME-003 (REQ-WFR3-GATE-001, REQ-WFR3-CLOSED-001).
//!
//! GateError is a closed enum with exactly 3 variants:
//! - `NotImplementedInCycle16`: operator variant not yet implemented
//! - `GuardEvaluationFailed`: guard expression evaluation failed
//! - `BoundedGateViolation`: gate violated budget bounds

use thiserror::Error;

/// Errors from Gate operator evaluation.
#[derive(Debug, Clone, Error)]
pub enum GateError {
    /// Operator variant is not implemented in this cycle.
    #[error("operator {variant} not implemented in cycle 16")]
    NotImplementedInCycle16 {
        /// The operator variant name.
        variant: String,
    },

    /// Guard evaluation failed for the given expression.
    #[error("guard evaluation failed for {expression}: {reason}")]
    GuardEvaluationFailed {
        /// The guard expression that failed.
        expression: String,
        /// The reason for the failure.
        reason: String,
    },

    /// Gate violated a budget bound.
    #[error("bounded gate violation: {limit} observed {observed}")]
    BoundedGateViolation {
        /// The limit that was violated.
        limit: String,
        /// The observed value.
        observed: u64,
    },
}

impl GateError {
    /// Returns the variant name for each case.
    pub fn variant_name(&self) -> &'static str {
        match self {
            GateError::NotImplementedInCycle16 { .. } => "NotImplementedInCycle16",
            GateError::GuardEvaluationFailed { .. } => "GuardEvaluationFailed",
            GateError::BoundedGateViolation { .. } => "BoundedGateViolation",
        }
    }
}
