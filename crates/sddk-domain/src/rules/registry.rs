//! Rule registry: parses `architecture-rules.yaml`.

use super::ARCHITECTURE_RULES_SCHEMA_VERSION;
use super::types::{ArchitectureRule, Waiver};
use serde::Deserialize;
use std::collections::BTreeMap;
use thiserror::Error;

#[derive(Debug, Deserialize)]
struct RulesFile {
    #[serde(default)]
    schema_version: Option<String>,
    rules: Vec<ArchitectureRule>,
    #[serde(default)]
    waivers: Vec<Waiver>,
}

/// Errors that can occur when loading and validating `architecture-rules.yaml`.
#[derive(Debug, Error)]
pub enum RegistryError {
    /// The YAML document is malformed or cannot be parsed as a rule file.
    #[error("failed to parse architecture-rules YAML: {0}")]
    Parse(#[from] serde_saphyr::Error),
    /// The file declares a `schema_version` that this library does not recognise.
    /// The `supported` field carries the current version this library implements.
    #[error("unsupported architecture-rules schema_version {actual}; supported is {supported}")]
    UnsupportedSchemaVersion {
        /// The version declared in the YAML file.
        actual: String,
        /// The version this library currently supports.
        supported: &'static str,
    },
    /// A rule entry is missing the required `id` field.
    #[error("architecture rule entry missing id field")]
    MissingRuleId,
    /// Two rule entries carry the same `id`; identifiers must be unique within a file.
    #[error("duplicate architecture rule id: {0}")]
    DuplicateRuleId(String),
}

/// In-memory registry of architecture rules and waivers parsed from a YAML file.
///
/// Constructed via `from_yaml_str`. Exposes read-only access to the rule list
/// and waiver lookup indexed by `rule_id`.
#[derive(Debug, Clone)]
pub struct RuleRegistry {
    rules: Vec<ArchitectureRule>,
    waivers: BTreeMap<String, Waiver>,
}

impl RuleRegistry {
    /// Parse and validate an `architecture-rules.yaml` string into a `RuleRegistry`.
    ///
    /// Validates that the `schema_version` matches `ARCHITECTURE_RULES_SCHEMA_VERSION`,
    /// that every rule has a non-empty `id`, and that no `id` is duplicated.
    pub fn from_yaml_str(yaml: &str) -> Result<Self, RegistryError> {
        let file: RulesFile = serde_saphyr::from_str(yaml)?;
        if let Some(v) = file.schema_version
            && v != ARCHITECTURE_RULES_SCHEMA_VERSION
        {
            return Err(RegistryError::UnsupportedSchemaVersion {
                actual: v,
                supported: ARCHITECTURE_RULES_SCHEMA_VERSION,
            });
        }
        let mut rules = Vec::with_capacity(file.rules.len());
        let mut seen = std::collections::HashSet::new();
        for rule in file.rules {
            if rule.id.is_empty() {
                return Err(RegistryError::MissingRuleId);
            }
            if !seen.insert(rule.id.clone()) {
                return Err(RegistryError::DuplicateRuleId(rule.id));
            }
            rules.push(rule);
        }
        let waivers = file
            .waivers
            .into_iter()
            .map(|w| (w.rule_id.clone(), w))
            .collect();
        Ok(Self { rules, waivers })
    }
    /// Look up the waiver for a given rule ID, if one exists in the registry.
    ///
    /// Note that an existing waiver may still be expired at a particular baseline
    /// — callers must compare `baseline.head_anchor <= waiver.granted_until_sha`
    /// to determine whether the waiver is active.
    pub fn waiver_for(&self, rule_id: &str) -> Option<&Waiver> {
        self.waivers.get(rule_id)
    }
    /// Iterate over all registered architecture rules in document order.
    pub fn iter(&self) -> impl Iterator<Item = &ArchitectureRule> {
        self.rules.iter()
    }
}
