//! Default-deny agent permission policy by phase and capability.

use std::collections::HashMap;
use std::path::Path;

use serde::{Deserialize, Serialize};
use thiserror::Error;

/// Errors emitted while loading a permission registry.
#[derive(Debug, Error)]
pub enum PermissionsError {
    /// The registry YAML is not a valid `agents:` mapping.
    #[error("invalid permissions registry: {0}")]
    Invalid(String),
    /// The registry file could not be read.
    #[error("failed to read permissions registry {path:?}: {source}")]
    Io {
        /// Requested registry path.
        path: std::path::PathBuf,
        /// Underlying I/O error.
        source: std::io::Error,
    },
}

/// Declared permissions of one agent.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub struct AgentPermissions {
    /// Phases the agent may execute in.
    #[serde(default)]
    pub phases: Vec<String>,
    /// Capabilities the agent may request.
    #[serde(default)]
    pub capabilities: Vec<String>,
}

/// Immutable default-deny permission registry.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct PermissionPolicy {
    agents: HashMap<String, AgentPermissions>,
}

impl PermissionPolicy {
    /// Parses a registry document of the form `agents: { name: {phases, capabilities} }`.
    pub fn from_yaml(yaml: &str) -> Result<Self, PermissionsError> {
        #[derive(Deserialize)]
        struct Registry {
            #[serde(default)]
            agents: HashMap<String, AgentPermissions>,
        }
        let registry: Registry = serde_saphyr::from_str(yaml)
            .map_err(|error| PermissionsError::Invalid(format!("cannot parse YAML: {error}")))?;
        Ok(Self {
            agents: registry.agents,
        })
    }

    /// Loads a registry from a YAML file.
    pub fn from_file(path: impl AsRef<Path>) -> Result<Self, PermissionsError> {
        let path = path.as_ref();
        let yaml = std::fs::read_to_string(path).map_err(|source| PermissionsError::Io {
            path: path.to_path_buf(),
            source,
        })?;
        Self::from_yaml(&yaml)
    }

    /// Returns the declared agent names.
    pub fn agents(&self) -> impl Iterator<Item = &str> {
        self.agents.keys().map(String::as_str)
    }

    /// Evaluates one agent/phase/capability request under default-deny.
    pub fn authorize(&self, agent: &str, phase: &str, capability: &str) -> PermissionDecision {
        let Some(permissions) = self.agents.get(agent) else {
            return PermissionDecision {
                allowed: false,
                reason: format!("agent {agent} is not declared in the permission registry"),
            };
        };
        if !permissions.phases.iter().any(|allowed| allowed == phase) {
            return PermissionDecision {
                allowed: false,
                reason: format!("agent {agent} is not allowed in phase {phase}"),
            };
        }
        if !permissions
            .capabilities
            .iter()
            .any(|allowed| allowed == capability)
        {
            return PermissionDecision {
                allowed: false,
                reason: format!("agent {agent} is not allowed capability {capability}"),
            };
        }
        PermissionDecision {
            allowed: true,
            reason: format!("agent {agent} is allowed {capability} in phase {phase}"),
        }
    }
}

/// Outcome of one permission evaluation.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub struct PermissionDecision {
    /// Whether the request is permitted.
    pub allowed: bool,
    /// Stable human-readable reason.
    pub reason: String,
}

#[cfg(test)]
mod tests {
    use super::PermissionPolicy;

    const REGISTRY: &str = r#"
agents:
  orchestrator:
    phases: [explore, specify, design, plan, build, verify, review, release, archive]
    capabilities: [git.inspect, git.create_branch, git.commit, git.tag, git.push, pr.create, release.create]
  sddk-apply:
    phases: [build, verify]
    capabilities: [git.inspect, git.commit]
"#;

    #[test]
    fn allows_declared_agent_phase_and_capability() {
        let policy = PermissionPolicy::from_yaml(REGISTRY).unwrap();
        let decision = policy.authorize("orchestrator", "build", "git.commit");
        assert!(decision.allowed);
        assert!(decision.reason.contains("is allowed"));
    }

    #[test]
    fn denies_undeclared_agents_by_default() {
        let policy = PermissionPolicy::from_yaml(REGISTRY).unwrap();
        let decision = policy.authorize("mystery-agent", "build", "git.commit");
        assert!(!decision.allowed);
        assert!(decision.reason.contains("not declared"));
    }

    #[test]
    fn denies_unknown_phase_and_capability() {
        let policy = PermissionPolicy::from_yaml(REGISTRY).unwrap();
        let phase = policy.authorize("sddk-apply", "release", "git.commit");
        assert!(!phase.allowed);
        assert!(phase.reason.contains("phase release"));

        let capability = policy.authorize("sddk-apply", "build", "pr.merge");
        assert!(!capability.allowed);
        assert!(capability.reason.contains("pr.merge"));
    }

    #[test]
    fn rejects_invalid_registry() {
        assert!(PermissionPolicy::from_yaml("not: [valid").is_err());
    }
}
