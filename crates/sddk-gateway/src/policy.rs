//! Default-deny capability policy derived from the canonical workflow.

use std::collections::HashMap;

use sddk_domain::WorkflowManifest;
use serde::Serialize;

/// Declared risk of a capability.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum Risk {
    /// Low impact.
    Low,
    /// Medium impact.
    Medium,
    /// High impact.
    High,
    /// Critical impact.
    Critical,
}

/// Declared consequence class of a capability.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum Consequence {
    /// Creates a new effect or artifact.
    Creates,
    /// Modifies shared or external state.
    Modifies,
    /// Destructive or hard to reverse.
    Irreversible,
}

/// Parsed definition of one allowed capability.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub struct CapabilityDefinition {
    /// Capability identifier.
    pub capability: String,
    /// Declared risk level.
    pub risk: Risk,
    /// Declared consequence class.
    pub consequence: Consequence,
}

impl CapabilityDefinition {
    /// Whether approval is required before execution.
    pub fn requires_approval(&self) -> bool {
        matches!(self.risk, Risk::High | Risk::Critical)
            || self.consequence == Consequence::Irreversible
            || self.consequence == Consequence::Modifies
    }
}

/// Immutable default-deny policy over named capabilities.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct CapabilityPolicy {
    capabilities: HashMap<String, CapabilityDefinition>,
}

impl CapabilityPolicy {
    /// Builds a policy from the `forge.capabilities` declarations.
    ///
    /// Capabilities absent from the workflow are denied by default.
    pub fn from_workflow(workflow: &WorkflowManifest) -> Self {
        let mut capabilities = HashMap::new();
        if let Some(definitions) = workflow
            .forge
            .as_ref()
            .and_then(|forge| forge.capabilities.as_ref())
        {
            for (name, definition) in definitions {
                capabilities.insert(
                    name.clone(),
                    CapabilityDefinition {
                        capability: name.clone(),
                        risk: parse_risk(definition.risk.as_deref()),
                        consequence: parse_consequence(definition.consequence.as_deref()),
                    },
                );
            }
        }
        Self { capabilities }
    }

    /// Returns the environment allowlist for a capability via the unified
    /// resolver ([`capability_env_allowlist`]).
    pub fn env_allowlist(&self, capability: &str) -> std::collections::BTreeMap<String, String> {
        capability_env_allowlist(capability)
    }

    /// Evaluates a capability request under the policy.
    pub fn authorize(&self, capability: &str, approve: bool) -> PolicyDecision {
        match self.capabilities.get(capability) {
            None => PolicyDecision {
                capability: capability.to_owned(),
                allowed: false,
                requires_approval: false,
                definition: None,
            },
            Some(definition) => {
                let requires_approval = definition.requires_approval();
                PolicyDecision {
                    capability: capability.to_owned(),
                    allowed: !requires_approval || approve,
                    requires_approval,
                    definition: Some(definition.clone()),
                }
            }
        }
    }
}

/// Outcome of one policy evaluation.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub struct PolicyDecision {
    /// Evaluated capability identifier.
    pub capability: String,
    /// Whether the capability may execute under the supplied approval state.
    pub allowed: bool,
    /// Whether the capability requires explicit human approval.
    pub requires_approval: bool,
    /// Parsed workflow definition, when the capability is declared.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub definition: Option<CapabilityDefinition>,
}

/// Returns the capability-specific environment allowlist (v2 unified
/// resolver).
///
/// Known prefixes:
/// - `git.*` → [`crate::git::git_capability_env`]
/// - `uat.*` → [`crate::playwright::browser_env`] (PATH/HOME/NODE_PATH/TMPDIR)
/// - anything else → empty (default-deny)
pub fn capability_env_allowlist(capability: &str) -> std::collections::BTreeMap<String, String> {
    if capability.starts_with("git.") {
        crate::git::git_capability_env()
    } else if capability.starts_with("uat.") {
        crate::playwright::browser_env()
    } else {
        std::collections::BTreeMap::new()
    }
}

fn parse_risk(value: Option<&str>) -> Risk {
    match value.map(str::to_ascii_lowercase).as_deref() {
        Some("medium") => Risk::Medium,
        Some("high") => Risk::High,
        Some("critical") => Risk::Critical,
        _ => Risk::Low,
    }
}

fn parse_consequence(value: Option<&str>) -> Consequence {
    match value.map(str::to_ascii_lowercase).as_deref() {
        Some("modifies") => Consequence::Modifies,
        Some("irreversible") => Consequence::Irreversible,
        _ => Consequence::Creates,
    }
}

#[cfg(test)]
mod tests {
    use sddk_domain::{CapabilityDef, ForgeDef, WorkflowManifest};

    use super::{CapabilityPolicy, Consequence, Risk, capability_env_allowlist};

    const WORKFLOW_YAML: &str = include_str!("../../../workflow/workflow.yaml");

    fn workflow_with(capabilities: &[(&str, Option<&str>, Option<&str>)]) -> WorkflowManifest {
        let mut workflow = sddk_engine::load_workflow_str(WORKFLOW_YAML).unwrap();
        workflow.forge = Some(ForgeDef {
            provider: "auto".into(),
            capabilities: Some(
                capabilities
                    .iter()
                    .map(|(name, risk, consequence)| {
                        (
                            (*name).to_owned(),
                            CapabilityDef {
                                risk: risk.map(|value| value.to_owned()),
                                consequence: consequence.map(|value| value.to_owned()),
                            },
                        )
                    })
                    .collect(),
            ),
        });
        workflow
    }

    #[test]
    fn unknown_capability_is_denied_by_default() {
        let policy = CapabilityPolicy::from_workflow(&workflow_with(&[]));
        let decision = policy.authorize("git.push", true);
        assert!(!decision.allowed);
        assert!(decision.definition.is_none());
    }

    #[test]
    fn declared_low_risk_capability_is_allowed_without_approval() {
        let policy = CapabilityPolicy::from_workflow(&workflow_with(&[(
            "git.push",
            Some("low"),
            Some("creates"),
        )]));
        let decision = policy.authorize("git.push", false);
        assert!(decision.allowed);
        assert!(!decision.requires_approval);
        assert_eq!(decision.definition.as_ref().unwrap().risk, Risk::Low);
        assert_eq!(
            decision.definition.as_ref().unwrap().consequence,
            Consequence::Creates
        );
    }

    #[test]
    fn irreversible_or_modifying_capabilities_require_approval() {
        let policy = CapabilityPolicy::from_workflow(&workflow_with(&[
            ("git.delete_branch", Some("medium"), Some("irreversible")),
            ("git.merge", Some("medium"), Some("modifies")),
        ]));

        let denied = policy.authorize("git.delete_branch", false);
        assert!(denied.requires_approval);
        assert!(!denied.allowed);

        let approved = policy.authorize("git.delete_branch", true);
        assert!(approved.allowed);

        let merge_denied = policy.authorize("git.merge", false);
        assert!(merge_denied.requires_approval);
        assert!(!merge_denied.allowed);
    }

    #[test]
    fn high_risk_capabilities_require_approval_even_when_creating() {
        let policy = CapabilityPolicy::from_workflow(&workflow_with(&[(
            "release.publish",
            Some("high"),
            Some("creates"),
        )]));
        let decision = policy.authorize("release.publish", false);
        assert!(decision.requires_approval);
        assert!(!decision.allowed);
        assert!(policy.authorize("release.publish", true).allowed);
    }

    #[test]
    fn env_allowlist_dispatches_git_prefix() {
        let got = capability_env_allowlist("git.push");
        assert_eq!(got, crate::git::git_capability_env());
    }

    #[test]
    fn env_allowlist_dispatches_uat_prefix() {
        let got = capability_env_allowlist("uat.playwright");
        // PATH is always present in normal test environments
        if std::env::var_os("PATH").is_some() {
            assert!(
                got.contains_key("PATH"),
                "uat.playwright allowlist must contain PATH"
            );
        }
    }

    #[test]
    fn env_allowlist_unknown_capability_is_empty() {
        let got = capability_env_allowlist("foo.bar");
        assert!(got.is_empty(), "unknown prefix must yield empty allowlist");
    }

    #[test]
    fn policy_env_allowlist_delegates() {
        let policy = CapabilityPolicy::from_workflow(&workflow_with(&[]));
        assert_eq!(
            policy.env_allowlist("git.tag"),
            capability_env_allowlist("git.tag")
        );
    }
}
