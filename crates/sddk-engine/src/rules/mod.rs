//! Baseline consumer + stub evaluator for the architecture-rule registry.

pub mod baseline;
pub mod evaluators;

pub use baseline::{
    Baseline, BaselineConsumer, BaselineError, CrossCrateImport, CrossCrateImportKind,
};
pub use evaluators::evaluate_all;

/// Version of the rule evaluator binary.
pub const EVALUATOR_VERSION: &str = evaluators::EVALUATOR_VERSION;
