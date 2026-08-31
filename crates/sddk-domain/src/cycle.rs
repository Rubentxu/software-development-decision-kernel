//! Cycle types for SDDK workflow cycles.
//!
//! Contains cycle status, phase enums, cycle manifest, and related types.

use serde::{Deserialize, Serialize};
use std::collections::HashMap;

use crate::identity::{CycleId, IdentitySource};

/// Cycle status values.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum CycleStatus {
    /// Cycle is open and active.
    #[default]
    Open,
    /// Cycle is blocked waiting on something.
    Blocked,
    /// Cycle is in remediation phase.
    Remediating,
    /// Cycle is pending release.
    ReleasePending,
    /// Cycle has been released.
    Released,
    /// Cycle is closed.
    Closed,
    /// Cycle was abandoned.
    Abandoned,
    /// Cycle is in recovery state.
    Recovering,
    /// Cycle is waiting on a human UAT verdict (ADR-012, synchronous mode).
    UatWaiting,
    /// Cycle is blocked waiting on a human approval decision.
    ApprovalPending,
}

crate::assert_variant_count_eq!(
    CycleStatus,
    10,
    [
        CycleStatus::Open,
        CycleStatus::Blocked,
        CycleStatus::Remediating,
        CycleStatus::ReleasePending,
        CycleStatus::Released,
        CycleStatus::Closed,
        CycleStatus::Abandoned,
        CycleStatus::Recovering,
        CycleStatus::UatWaiting,
        CycleStatus::ApprovalPending,
    ]
);

/// Workflow phases.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Phase {
    /// Exploration phase.
    #[default]
    Explore,
    /// Specification phase.
    Specify,
    /// Design phase.
    Design,
    /// Planning phase.
    Plan,
    /// Build/implementation phase.
    Build,
    /// Verification phase.
    Verify,
    /// UAT phase (optional, orchestrator-decided — ADR-012).
    Uat,
    /// Release phase.
    Release,
    /// Archive phase.
    Archive,
}

crate::assert_variant_count_eq!(
    Phase,
    9,
    [
        Phase::Explore,
        Phase::Specify,
        Phase::Design,
        Phase::Plan,
        Phase::Build,
        Phase::Verify,
        Phase::Uat,
        Phase::Release,
        Phase::Archive,
    ]
);

/// Cycle path types (A-min, A-lite, A-full, B-direct).
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum CyclePath {
    /// Minimal path A.
    AMin,
    /// Lite path A.
    ALite,
    /// Full path A.
    #[default]
    AFull,
    /// Direct path B.
    BDirect,
}

/// Release information for a cycle.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub struct Release {
    /// Tag applied.
    pub tag: Option<String>,
    /// Release notes.
    pub notes: Option<String>,
    /// Timestamp of release.
    pub timestamp: Option<String>,
}

/// A cycle manifest tracking cycle state.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub struct CycleManifest {
    /// Schema version.
    pub schema_version: i32,
    /// Project identifier.
    pub project_id: String,
    /// Workspace identifier.
    pub workspace_id: String,
    /// Cycle identifier.
    pub cycle_id: String,
    /// Display name.
    pub display_name: String,
    /// Current status.
    pub status: CycleStatus,
    /// Current phase.
    pub phase: Phase,
    /// Cycle path.
    pub path: CyclePath,
    /// Git branch name.
    pub branch: String,
    /// Base commit SHA.
    pub base: String,
    /// Head commit SHA.
    pub head: Option<String>,
    /// Artifacts produced by this cycle.
    pub artifacts: HashMap<String, ArtifactRef>,
    /// Release information.
    pub release: Option<Release>,
    /// Delivery kind (REQ-DKA-001, ADR-0076).
    #[serde(default)]
    pub delivery_kind: Option<super::delivery_kind::DeliveryKind>,
    /// Remediation round number.
    #[serde(default)]
    pub remediation_round: u32,
    /// Remote URL for the project.
    #[serde(default)]
    pub remote_url: Option<String>,
    /// Scope for identity computation.
    #[serde(default)]
    pub scope: Option<String>,
}

impl CycleManifest {
    /// Creates a new cycle manifest with defaults.
    /// Note: timestamp is NOT generated here — callers must provide explicit timestamps.
    pub fn new(
        project_id: String,
        workspace_id: String,
        cycle_id: CycleId,
        display_name: String,
        branch: String,
        base: String,
    ) -> Self {
        Self {
            schema_version: 1,
            project_id,
            workspace_id,
            cycle_id: cycle_id.to_string(),
            display_name,
            status: CycleStatus::Open,
            phase: Phase::Explore,
            path: CyclePath::AFull,
            branch,
            base,
            head: None,
            artifacts: HashMap::new(),
            release: None,
            delivery_kind: None,
            remediation_round: 0,
            remote_url: None,
            scope: None,
        }
    }
}

/// Reference to an artifact.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub struct ArtifactRef {
    /// Artifact kind.
    pub kind: String,
    /// Path to the artifact.
    pub path: String,
    /// SHA-256 hash of content (if applicable).
    #[serde(default)]
    pub sha256: Option<String>,
    /// Producer of this artifact.
    #[serde(default)]
    pub producer: Option<String>,
    /// Timestamp when created.
    #[serde(default)]
    pub created_at: Option<String>,
}

impl ArtifactRef {
    /// Creates a new artifact reference.
    pub fn new(kind: impl Into<String>, path: impl Into<String>) -> Self {
        Self {
            kind: kind.into(),
            path: path.into(),
            sha256: None,
            producer: None,
            created_at: None,
        }
    }
}

/// Storage locations bound by an adoption receipt.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub struct AdoptionStoragePaths {
    /// Project-shared knowledge vault directory.
    pub vault: String,
    /// Project-shared content-addressed artifact directory.
    pub artifacts: String,
    /// Project-shared cycle artifact directory (`cycle-artifacts/{cycle_id}/`).
    pub cycle_artifacts: String,
    /// Project-shared generated docs directory (inventory, workflow docs).
    pub generated: String,
    /// Project-shared SQLite database.
    pub ledger: String,
    /// SDDK cache directory.
    pub cache: String,
    /// Workspace-specific adoption receipt.
    pub receipt: String,
}

/// Adoption receipt for one project workspace.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub struct AdoptionReceipt {
    /// Receipt schema version.
    pub schema_version: i32,
    /// SDDK product version that produced the receipt.
    pub sddk_version: String,
    /// Runtime implementation version that produced the receipt.
    pub runtime_version: String,
    /// Stable logical project identifier.
    pub project_id: String,
    /// Stable checkout or worktree identifier.
    pub workspace_id: String,
    /// Human-readable project name.
    pub display_name: String,
    /// Canonical checkout or worktree path.
    pub canonical_workspace_path: String,
    /// Identity derivation source.
    pub identity_source: IdentitySource,
    /// Canonical remote URL used for remote identity.
    pub remote_url: Option<String>,
    /// Required monorepo scope.
    pub scope: String,
    /// Persisted UUID used for fallback identity.
    pub fallback_seed: Option<String>,
    /// SHA-256 hash of the deterministic adoption configuration.
    pub configuration_hash: String,
    /// Resolved storage paths.
    pub paths: AdoptionStoragePaths,
    /// Caller-supplied adoption timestamp.
    pub timestamp: String,
    /// Caller-supplied adopting actor.
    pub actor: String,
}

/// Agent result from a phase execution.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub struct AgentResult {
    /// Schema version.
    pub schema_version: i32,
    /// Agent identifier.
    pub agent: String,
    /// Cycle identifier.
    pub cycle_id: String,
    /// Phase this result is for.
    pub phase: Phase,
    /// Verdict of execution.
    pub verdict: AgentVerdict,
    /// Human-readable summary.
    pub summary: String,
    /// Artifacts produced.
    #[serde(default)]
    pub artifacts: Vec<ArtifactRef>,
    /// Proposed relations.
    #[serde(default)]
    pub proposed_relations: Vec<ProposedRelation>,
    /// Evidence for the result.
    #[serde(default)]
    pub evidence: Vec<String>,
    /// Risks identified.
    #[serde(default)]
    pub risks: Vec<String>,
    /// Capabilities requested.
    #[serde(default)]
    pub requested_capabilities: Vec<CapabilityRequest>,
}

/// Agent verdict values.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum AgentVerdict {
    /// Agent completed successfully.
    Completed,
    /// Agent was blocked.
    Blocked,
    /// Needs input to continue.
    NeedsInput,
    /// Agent failed.
    Failed,
}

/// A proposed relation between entities.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub struct ProposedRelation {
    /// Source entity.
    pub source: String,
    /// Relation type.
    pub relation_type: String,
    /// Target entity.
    pub target: String,
}

/// Capability request from an agent.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub struct CapabilityRequest {
    /// Capability identifier (e.g., "git.create_branch").
    pub capability: String,
    /// Arguments for the capability.
    pub arguments: serde_json::Value,
    /// Reason for requesting.
    pub reason: String,
    /// Expected state digest.
    #[serde(default)]
    pub expected_state_digest: Option<String>,
    /// Idempotency key.
    #[serde(default)]
    pub idempotency_key: Option<String>,
}

impl CapabilityRequest {
    /// Creates a new capability request.
    pub fn new(
        capability: impl Into<String>,
        arguments: serde_json::Value,
        reason: impl Into<String>,
    ) -> Self {
        Self {
            capability: capability.into(),
            arguments,
            reason: reason.into(),
            expected_state_digest: None,
            idempotency_key: None,
        }
    }
}

/// Risk severity levels.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum RiskLevel {
    /// Low risk.
    Low,
    /// Medium risk.
    Medium,
    /// High risk.
    High,
    /// Critical risk.
    Critical,
}

crate::assert_variant_count_eq!(
    RiskLevel,
    4,
    [
        RiskLevel::Low,
        RiskLevel::Medium,
        RiskLevel::High,
        RiskLevel::Critical,
    ]
);

/// Risk categories.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum RiskCategory {
    /// Technical risk.
    Technical,
    /// Schedule risk.
    Schedule,
    /// Resource risk.
    Resource,
    /// External risk.
    External,
}

/// A identified risk.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub struct Risk {
    /// Risk identifier.
    pub id: String,
    /// Risk category.
    pub category: RiskCategory,
    /// Severity level.
    pub level: RiskLevel,
    /// Description.
    pub description: String,
    /// Mitigation strategy.
    #[serde(default)]
    pub mitigation: Option<String>,
}

/// Consequence types for capability execution.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ConsequenceType {
    /// Read-only operation.
    ReadOnly,
    /// Creates new entity.
    Creates,
    /// Modifies existing entity.
    Modifies,
    /// Deletes entity.
    Deletes,
    /// Irreversible operation.
    Irreversible,
}

/// Consequence of a capability.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub struct Consequence {
    /// Consequence type.
    pub consequence_type: ConsequenceType,
    /// Affected scope.
    pub scope: String,
    /// Description of effect.
    pub description: String,
    /// Reversibility.
    #[serde(default)]
    pub reversible: bool,
}

/// Phase result for phase completion.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub struct PhaseResult {
    /// Schema version.
    pub schema_version: i32,
    /// Cycle identifier.
    pub cycle_id: String,
    /// Phase completed.
    pub phase: Phase,
    /// Whether the phase succeeded.
    pub success: bool,
    /// Summary of results.
    pub summary: String,
    /// Artifacts produced.
    #[serde(default)]
    pub artifacts: Vec<ArtifactRef>,
    /// Errors encountered.
    #[serde(default)]
    pub errors: Vec<String>,
    /// Timestamp — caller must provide this explicitly.
    pub timestamp: String,
}

impl PhaseResult {
    /// Creates a new phase result.
    /// Note: `timestamp` must be provided explicitly by the caller
    /// to avoid domain code reading the system clock.
    pub fn new(
        cycle_id: String,
        phase: Phase,
        success: bool,
        summary: impl Into<String>,
        timestamp: String,
    ) -> Self {
        Self {
            schema_version: 1,
            cycle_id,
            phase,
            success,
            summary: summary.into(),
            artifacts: Vec::new(),
            errors: Vec::new(),
            timestamp,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_cycle_status_default() {
        let status = CycleStatus::default();
        assert_eq!(status, CycleStatus::Open);
    }

    #[test]
    fn test_cycle_status_approval_pending_roundtrip() {
        let status = CycleStatus::ApprovalPending;
        let json = serde_json::to_string(&status).unwrap();
        assert_eq!(json, "\"APPROVAL_PENDING\"");
        let roundtrip: CycleStatus = serde_json::from_str(&json).unwrap();
        assert_eq!(roundtrip, CycleStatus::ApprovalPending);
    }

    #[test]
    fn test_phase_default() {
        let phase = Phase::default();
        assert_eq!(phase, Phase::Explore);
    }

    #[test]
    fn test_cycle_path_default() {
        let path = CyclePath::default();
        assert_eq!(path, CyclePath::AFull);
    }

    #[test]
    fn test_artifact_ref_new() {
        let artifact = ArtifactRef::new("specification", "/path/to/spec.md");
        assert_eq!(artifact.kind, "specification");
        assert_eq!(artifact.path, "/path/to/spec.md");
        assert!(artifact.sha256.is_none());
    }

    #[test]
    fn test_capability_request_new() {
        let args = serde_json::json!({"branch": "feature-x"});
        let request = CapabilityRequest::new("git.create_branch", args, "Need new branch");
        assert_eq!(request.capability, "git.create_branch");
        assert_eq!(request.reason, "Need new branch");
    }

    #[test]
    fn test_risk_serialization() {
        let risk = Risk {
            id: "R-01".into(),
            category: RiskCategory::Technical,
            level: RiskLevel::High,
            description: "Complex dependency".into(),
            mitigation: Some("Use interface".into()),
        };
        let json = serde_json::to_string(&risk).unwrap();
        assert!(json.contains("R-01"));
        assert!(json.contains("high"));
    }

    #[test]
    fn test_consequence_serialization() {
        let consequence = Consequence {
            consequence_type: ConsequenceType::Creates,
            scope: "git.ref".into(),
            description: "Creates a new branch".into(),
            reversible: true,
        };
        let json = serde_json::to_string(&consequence).unwrap();
        assert!(json.contains("creates"));
    }

    #[test]
    fn test_adoption_receipt_explicit_timestamp() {
        let receipt = AdoptionReceipt {
            schema_version: 2,
            sddk_version: "3.6".into(),
            runtime_version: "0.1.0".into(),
            project_id: "proj-1".into(),
            workspace_id: "ws-1".into(),
            display_name: "Repo".into(),
            canonical_workspace_path: "/work/repo".into(),
            identity_source: IdentitySource::Remote,
            remote_url: Some("https://github.com/owner/repo".into()),
            scope: ".".into(),
            fallback_seed: None,
            configuration_hash: format!("sha256:{}", "a".repeat(64)),
            paths: AdoptionStoragePaths {
                vault: "/data/sddk/projects/proj-1/vault".into(),
                artifacts: "/data/sddk/projects/proj-1/artifacts".into(),
                cycle_artifacts: "/data/sddk/projects/proj-1/cycle-artifacts".into(),
                generated: "/data/sddk/projects/proj-1/generated".into(),
                ledger: "/state/sddk/projects/proj-1/ledger.sqlite".into(),
                cache: "/cache/sddk".into(),
                receipt: "/data/sddk/projects/proj-1/workspaces/ws-1/adoption.json".into(),
            },
            timestamp: "2026-08-03T12:00:00Z".into(),
            actor: "agent".into(),
        };
        assert_eq!(receipt.timestamp, "2026-08-03T12:00:00Z");
        let json = serde_json::to_value(receipt).unwrap();
        assert_eq!(json["identity_source"], "remote");
        assert!(json.get("configuration_hash").is_some());
    }

    #[test]
    fn test_phase_result_explicit_timestamp() {
        let result = PhaseResult::new(
            "cycle-1".into(),
            Phase::Explore,
            true,
            "Phase complete",
            "2026-08-03T12:00:00Z".into(),
        );
        assert_eq!(result.timestamp, "2026-08-03T12:00:00Z");
    }
}
