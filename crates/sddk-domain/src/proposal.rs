//! Proposal domain model for the governed capability flow.
//!
//! A `Proposal` represents an intent to exercise a capability, including
//! scope, constraints, idempotency, and version hashes for traceability.

use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use thiserror::Error;

/// Errors that can occur during proposal operations.
#[derive(Debug, Error)]
pub enum ProposalError {
    /// The proposal has expired.
    #[error("proposal has expired")]
    Expired,
    /// The proposal declares no capabilities.
    #[error("proposal must declare at least one capability")]
    NoCapabilitiesDeclared,
    /// The agent version hash is empty.
    #[error("agent version hash cannot be empty")]
    EmptyAgentVersionHash,
    /// The behavior version hash is empty.
    #[error("behavior version hash cannot be empty")]
    EmptyBehaviorVersionHash,
    /// The expiry timestamp is invalid.
    #[error("invalid expiry timestamp format")]
    InvalidExpiryFormat,
    /// Serialization error.
    #[error("serialization error: {0}")]
    Serialization(#[from] serde_json::Error),
}

/// A stable idempotency key binding a proposal to a specific capability request.
///
/// The key must be unique per (project, capability) pair and is used to
/// detect duplicate requests and ensure exactly-once semantics.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub struct IdempotencyKey {
    /// Owning project identifier.
    pub project_id: String,
    /// Related cycle, if any.
    pub cycle_id: Option<String>,
    /// Declared capability identifier.
    pub capability: String,
    /// Deterministic hash of the request arguments.
    pub request_hash: String,
}

impl IdempotencyKey {
    /// Returns the composite idempotency key string.
    pub fn as_str(&self) -> String {
        format!(
            "{}/{}/{}/{}",
            self.project_id,
            self.cycle_id.as_deref().unwrap_or("none"),
            self.capability,
            &self.request_hash[..16]
        )
    }
}

/// Lifecycle state of a proposal.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ProposalStatus {
    /// The proposal is active and awaiting authorization.
    Pending,
    /// The proposal was authorized and its capability executed.
    Authorized,
    /// The proposal was denied by policy.
    Denied,
    /// The proposal expired without execution.
    Expired,
}

/// Declared intent and scope of a capability execution.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub struct Proposal {
    /// Stable proposal identifier.
    pub proposal_id: String,
    /// Owning project identifier.
    pub project_id: String,
    /// Related cycle, if any.
    pub cycle_id: Option<String>,
    /// Human-readable justification for the capability.
    pub reason: String,
    /// Declared capability to exercise.
    pub capability: String,
    /// Executable program to invoke.
    pub program: String,
    /// Positional arguments for the program.
    pub args: Vec<String>,
    /// Environment variables (subset allowed by policy).
    pub env: std::collections::BTreeMap<String, String>,
    /// Runner timeout in milliseconds.
    pub timeout_ms: u64,
    /// Maximum output bytes per stream.
    pub output_max_bytes: usize,
    /// Caller-supplied creation timestamp.
    pub created_at: String,
    /// Expiry timestamp (RFC 3339), after which the proposal is invalid.
    /// This field also serves as the approval timeout: if an approval request
    /// is pending when `now > expires_at`, the gateway returns
    /// `GatewayError::ApprovalExpired`. Reusing this field avoids a new column
    /// while keeping approval and proposal expiry in sync (ADR-NNN).
    pub expires_at: String,
    /// SHA-256 hash of the agent binary authorized to execute this proposal.
    pub agent_version_hash: String,
    /// SHA-256 hash of the workflow/behavior that authorized this proposal.
    pub behavior_version_hash: String,
    /// Current proposal status.
    pub status: ProposalStatus,
}

impl Proposal {
    /// Validates the proposal constraints.
    ///
    /// Returns `Ok(())` if valid, or an error describing the violation.
    pub fn validate(&self) -> Result<(), ProposalError> {
        if self.capability.is_empty() {
            return Err(ProposalError::NoCapabilitiesDeclared);
        }
        if self.agent_version_hash.is_empty() {
            return Err(ProposalError::EmptyAgentVersionHash);
        }
        if self.behavior_version_hash.is_empty() {
            return Err(ProposalError::EmptyBehaviorVersionHash);
        }
        Ok(())
    }

    /// Checks if the proposal has expired based on RFC 3339 timestamp comparison.
    ///
    /// Returns `true` if the expiry timestamp is in the past.
    pub fn is_expired(&self) -> bool {
        // Simple lexicographic comparison works for RFC 3339 timestamps
        // Format: "2026-08-18T10:00:00Z" or "2026-08-18T10:00:00+00:00"
        let now = time::OffsetDateTime::now_utc();
        let now_str = now
            .format(&time::format_description::well_known::Rfc3339)
            .unwrap_or_else(|_| String::new());

        // Handle both Z suffix and +00:00 offset formats
        let expires_normalized = self.expires_at.trim_end_matches('Z');
        let now_normalized = now_str.trim_end_matches('Z');

        expires_normalized < now_normalized
    }

    /// Computes a deterministic SHA-256 hash of the proposal's structural content.
    ///
    /// The hash covers: intent (reason), scope (project_id, cycle_id),
    /// proposed_capability (capability, program, args, env, timeout_ms,
    /// output_max_bytes), and idempotency_key (derived from args and reason).
    ///
    /// Excludes: proposal_id (assigned after hash), created_at, expires_at,
    /// version hashes (agent_version_hash, behavior_version_hash), and status.
    pub fn hash_structural(&self) -> String {
        // Build a canonical serialization excluding non-structural fields
        let canonical = serde_json::json!({
            "project_id": self.project_id,
            "cycle_id": self.cycle_id,
            "reason": self.reason,
            "capability": self.capability,
            "program": self.program,
            "args": self.args,
            "env": self.env,
            "timeout_ms": self.timeout_ms,
            "output_max_bytes": self.output_max_bytes,
        });
        let bytes = serde_json::to_vec(&canonical)
            .expect("proposal structural content is always serializable");
        let mut hasher = Sha256::new();
        hasher.update(&bytes);
        format!("{:032x}", hasher.finalize())
    }
}

/// Policy evaluation decision for a proposal.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ProposalPolicyDecision {
    /// The proposal is allowed to proceed.
    Allow,
    /// The proposal is denied.
    Deny,
    /// The proposal requires explicit human approval.
    ApprovalRequired,
}

/// Default-deny policy evaluator for proposals.
///
/// A proposal is allowed only if:
/// - It has non-empty version hashes
/// - It has not expired
/// - The declared capability is authorized by the workflow policy
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct ProposalPolicy {
    /// Authorized capabilities from the workflow definition.
    capabilities: std::collections::HashMap<String, super::workflow::CapabilityDef>,
}

impl ProposalPolicy {
    /// Constructs a policy from workflow capability definitions.
    pub fn from_workflow(workflow: &super::WorkflowManifest) -> Self {
        let mut capabilities = std::collections::HashMap::new();
        if let Some(definitions) = workflow
            .forge
            .as_ref()
            .and_then(|forge| forge.capabilities.as_ref())
        {
            for (name, def) in definitions {
                capabilities.insert(name.clone(), def.clone());
            }
        }
        Self { capabilities }
    }

    /// Evaluates a proposal under this policy.
    ///
    /// Returns `ProposalPolicyDecision::Allow` if:
    /// - The capability is declared in the workflow
    /// - Both version hashes are non-empty
    /// - The proposal has not expired
    ///
    /// Returns `ProposalPolicyDecision::ApprovalRequired` for high-risk
    /// capabilities that require explicit human approval.
    ///
    /// Returns `ProposalPolicyDecision::Deny` for undeclared capabilities,
    /// expired proposals, or empty version hashes.
    pub fn authorize(&self, proposal: &Proposal, approve: bool) -> ProposalPolicyDecision {
        // Check expiry
        if proposal.is_expired() {
            return ProposalPolicyDecision::Deny;
        }

        // Check version hashes are non-empty
        if proposal.agent_version_hash.is_empty() || proposal.behavior_version_hash.is_empty() {
            return ProposalPolicyDecision::Deny;
        }

        // Check capability is declared
        let Some(cap_def) = self.capabilities.get(&proposal.capability) else {
            return ProposalPolicyDecision::Deny;
        };

        // Check if approval is required
        let requires_approval = cap_def.requires_approval();
        if requires_approval && !approve {
            return ProposalPolicyDecision::ApprovalRequired;
        }

        ProposalPolicyDecision::Allow
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn make_proposal(
        capability: &str,
        agent_hash: &str,
        behavior_hash: &str,
        expired: bool,
    ) -> Proposal {
        let now = time::OffsetDateTime::now_utc();
        let expires_at = if expired {
            (now - time::Duration::hours(1))
                .format(&time::format_description::well_known::Rfc3339)
                .unwrap_or_else(|_| "2020-01-01T00:00:00Z".into())
        } else {
            (now + time::Duration::hours(1))
                .format(&time::format_description::well_known::Rfc3339)
                .unwrap_or_else(|_| "2099-01-01T00:00:00Z".into())
        };

        Proposal {
            proposal_id: "prop-001".into(),
            project_id: "project-1".into(),
            cycle_id: None,
            reason: "test".into(),
            capability: capability.into(),
            program: "echo".into(),
            args: vec!["hello".into()],
            env: Default::default(),
            timeout_ms: 5000,
            output_max_bytes: 1024,
            created_at: now
                .format(&time::format_description::well_known::Rfc3339)
                .unwrap_or_else(|_| "2026-01-01T00:00:00Z".into()),
            expires_at,
            agent_version_hash: agent_hash.into(),
            behavior_version_hash: behavior_hash.into(),
            status: ProposalStatus::Pending,
        }
    }

    #[test]
    fn valid_proposal_passes_validation() {
        let proposal = make_proposal("echo.test", "abc123", "def456", false);
        assert!(proposal.validate().is_ok());
    }

    #[test]
    fn empty_capability_fails_validation() {
        let mut proposal = make_proposal("echo.test", "abc123", "def456", false);
        proposal.capability = "".into();
        assert!(matches!(
            proposal.validate(),
            Err(ProposalError::NoCapabilitiesDeclared)
        ));
    }

    #[test]
    fn empty_agent_hash_fails_validation() {
        let mut proposal = make_proposal("echo.test", "abc123", "def456", false);
        proposal.agent_version_hash = "".into();
        assert!(matches!(
            proposal.validate(),
            Err(ProposalError::EmptyAgentVersionHash)
        ));
    }

    #[test]
    fn empty_behavior_hash_fails_validation() {
        let mut proposal = make_proposal("echo.test", "abc123", "def456", false);
        proposal.behavior_version_hash = "".into();
        assert!(matches!(
            proposal.validate(),
            Err(ProposalError::EmptyBehaviorVersionHash)
        ));
    }

    #[test]
    fn expired_proposal_is_denied() {
        let policy = ProposalPolicy::default();
        let proposal = make_proposal("echo.test", "abc123", "def456", true);
        assert!(matches!(
            policy.authorize(&proposal, false),
            ProposalPolicyDecision::Deny
        ));
    }

    #[test]
    fn undeclared_capability_is_denied() {
        let policy = ProposalPolicy::default();
        let proposal = make_proposal("undeclared.cap", "abc123", "def456", false);
        assert!(matches!(
            policy.authorize(&proposal, false),
            ProposalPolicyDecision::Deny
        ));
    }

    #[test]
    fn hash_structural_identical_proposals_produce_same_hash() {
        let now = time::OffsetDateTime::now_utc();
        let expires_at = (now + time::Duration::hours(1))
            .format(&time::format_description::well_known::Rfc3339)
            .unwrap_or_else(|_| "2099-01-01T00:00:00Z".into());
        let created_at = now
            .format(&time::format_description::well_known::Rfc3339)
            .unwrap_or_else(|_| "2026-01-01T00:00:00Z".into());

        let make_base = || Proposal {
            proposal_id: "prop-001".into(),
            project_id: "project-1".into(),
            cycle_id: Some("cycle-1".into()),
            reason: "test reason".into(),
            capability: "echo.test".into(),
            program: "echo".into(),
            args: vec!["hello".into()],
            env: std::collections::BTreeMap::new(),
            timeout_ms: 5000,
            output_max_bytes: 1024,
            created_at: created_at.clone(),
            expires_at: expires_at.clone(),
            agent_version_hash: "abc123".into(),
            behavior_version_hash: "def456".into(),
            status: ProposalStatus::Authorized,
        };

        let hash_a = make_base().hash_structural();
        let hash_b = make_base().hash_structural();
        assert_eq!(
            hash_a, hash_b,
            "identical structural content must produce same hash"
        );
        assert_eq!(hash_a.len(), 64, "SHA-256 hex is 64 characters");
    }

    #[test]
    fn hash_structural_different_proposals_produce_different_hash() {
        let now = time::OffsetDateTime::now_utc();
        let expires_at = (now + time::Duration::hours(1))
            .format(&time::format_description::well_known::Rfc3339)
            .unwrap_or_else(|_| "2099-01-01T00:00:00Z".into());
        let created_at = now
            .format(&time::format_description::well_known::Rfc3339)
            .unwrap_or_else(|_| "2026-01-01T00:00:00Z".into());

        let proposal_a = Proposal {
            proposal_id: "prop-001".into(),
            project_id: "project-1".into(),
            cycle_id: None,
            reason: "test reason".into(),
            capability: "echo.test".into(),
            program: "echo".into(),
            args: vec!["hello".into()],
            env: std::collections::BTreeMap::new(),
            timeout_ms: 5000,
            output_max_bytes: 1024,
            created_at: created_at.clone(),
            expires_at: expires_at.clone(),
            agent_version_hash: "abc123".into(),
            behavior_version_hash: "def456".into(),
            status: ProposalStatus::Pending,
        };

        let mut proposal_b = proposal_a.clone();
        proposal_b.reason = "different reason".into();

        let hash_a = proposal_a.hash_structural();
        let hash_b = proposal_b.hash_structural();
        assert_ne!(
            hash_a, hash_b,
            "different structural content must produce different hash"
        );
    }
}
