//! Architecture-rules registry: domain types + YAML loader.

pub mod registry;
pub mod types;

pub use registry::{RegistryError, RuleRegistry};
pub use types::{
    ArchitectureRule, BaselineRef, EvaluatorKind, RuleEvaluation, RuleSeverity, RuleStatus,
    RuleTarget, Waiver,
};
/// Current schema version for `architecture-rules.yaml` files.
///
/// Bumped when the schema structure changes. The YAML parser in `RuleRegistry::from_yaml_str`
/// rejects any file with a different `schema_version` value.
pub const ARCHITECTURE_RULES_SCHEMA_VERSION: &str = "1.2.0";
