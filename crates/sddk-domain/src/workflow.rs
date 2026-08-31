//! Workflow definition types and validation.
//!
//! Contains the workflow manifest, policies, transitions, and
//! transition validation logic.

use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use thiserror::Error;

use crate::cycle::{CycleStatus, Phase};

/// Errors that can occur during workflow operations.
#[derive(Debug, Error)]
/// Errors for legacy workflow manifest validation.
///
/// Audit (cycle 3, kernel-cycle-3-carries-over): trimmed 4 unused variants
/// (`InvalidTransition`, `InvalidStateRef`, `ManifestNotFound`, `PolicyViolation`).
/// All remaining variants are emitted by `WorkflowManifest::validate`. Adding a
/// variant requires updating the manifest validator and the cycle-3 audit
/// results at `docs/audit/error-variants.md`.
pub enum WorkflowError {
    /// A transition requires an artifact that is not available.
    #[error("missing required artifact: {0}")]
    MissingArtifact(String),
    /// A transition requires a gate that is not satisfied.
    #[error("missing required gate: {0}")]
    MissingGate(String),
    /// The requested transition identifier is not declared.
    #[error("transition not found by id: {0}")]
    TransitionNotFound(String),
}

// Compile-time guard: 3 variants (post-cycle-3 trim).
crate::assert_variant_count_eq!(
    WorkflowError,
    3,
    [
        WorkflowError::MissingArtifact(_),
        WorkflowError::MissingGate(_),
        WorkflowError::TransitionNotFound(_),
    ]
);

/// Schema version for workflow manifests.
pub const WORKFLOW_SCHEMA_VERSION: i32 = 1;

/// A workflow manifest definition — mirrors the YAML canonical structure exactly.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub struct WorkflowManifest {
    /// Schema version for this manifest.
    pub schema_version: i32,
    /// Workflow metadata.
    pub workflow: WorkflowDef,
    /// Available statuses in this workflow.
    pub statuses: Vec<CycleStatus>,
    /// Available phases in this workflow.
    pub phases: Vec<Phase>,
    /// Cycle paths with their configurations.
    #[serde(default)]
    pub paths: HashMap<String, PathDef>,
    /// Workflow policies.
    #[serde(default)]
    pub policies: Policies,
    /// State transitions.
    pub transitions: Vec<Transition>,
    /// Artifact definitions by kind.
    #[serde(default)]
    pub artifacts: HashMap<String, ArtifactDef>,
    /// Gate definitions by name.
    #[serde(default)]
    pub gates: HashMap<String, GateDef>,
    /// Forge provider configuration.
    #[serde(default)]
    pub forge: Option<ForgeDef>,
    /// Storage configuration.
    #[serde(default)]
    pub storage: Option<StorageDef>,
    /// Project identity scheme.
    #[serde(default)]
    pub project_identity: Option<ProjectIdentityDef>,
}

/// Top-level workflow definition.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub struct WorkflowDef {
    /// Workflow identifier.
    pub id: String,
    /// Semantic version.
    pub version: String,
    /// Human-readable description.
    pub description: String,
}

/// Workflow info for API responses.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub struct WorkflowInfo {
    /// Workflow ID.
    pub id: String,
    /// Version.
    pub version: String,
    /// Description.
    pub description: String,
}

impl From<&WorkflowManifest> for WorkflowInfo {
    fn from(manifest: &WorkflowManifest) -> Self {
        Self {
            id: manifest.workflow.id.clone(),
            version: manifest.workflow.version.clone(),
            description: manifest.workflow.description.clone(),
        }
    }
}

/// A cycle path definition.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub struct PathDef {
    /// Human-readable description.
    pub description: String,
    /// Debt verification setting.
    pub debt_verification: String,
    /// Phases in this path.
    pub phases: Vec<String>,
}

/// Workflow policies — maps to YAML structure.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Default)]
#[serde(rename_all = "snake_case")]
pub struct Policies {
    /// Active cycles per project limit.
    #[serde(default)]
    pub active_cycles_per_project: Option<i32>,
    /// Debt verification policy settings.
    #[serde(default)]
    pub debt_verification: Option<HashMap<String, String>>,
    /// Require clean worktree on start.
    #[serde(default)]
    pub require_clean_worktree_on_start: Option<bool>,
    /// Shell arbitrary setting.
    #[serde(default)]
    pub shell_arbitrary: Option<String>,
}

/// A workflow state reference — phase is optional for block/unblock states.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub struct StateRef {
    /// Target status.
    pub status: CycleStatus,
    /// Target phase (optional for some states).
    #[serde(default)]
    pub phase: Option<Phase>,
}

/// A requirement for a transition — accepts string, artifact map, or gate map.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case", untagged)]
pub enum Requirement {
    /// Simple string requirement (e.g., "project.adopted").
    Simple(String),
    /// Structured requirement with kind and name.
    Structured {
        /// Requirement kind.
        kind: String,
        /// The required item name.
        name: String,
    },
}

impl Requirement {
    /// Returns the name of this requirement.
    pub fn name(&self) -> &str {
        match self {
            Requirement::Simple(s) => s,
            Requirement::Structured { name, .. } => name,
        }
    }

    /// Returns the kind of this requirement if structured.
    pub fn kind(&self) -> Option<&str> {
        match self {
            Requirement::Simple(_) => None,
            Requirement::Structured { kind, .. } => Some(kind),
        }
    }
}

/// A state transition definition.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub struct Transition {
    /// Transition identifier.
    pub id: String,
    /// Source state (null for initial transitions).
    #[serde(default)]
    pub from: Option<StateRef>,
    /// Target state.
    pub to: StateRef,
    /// Requirements for this transition.
    pub requires: Vec<Requirement>,
    /// Workflow paths allowed to use this transition; empty applies to all paths.
    #[serde(default)]
    pub paths: Vec<String>,
    /// Artifacts produced by this transition.
    #[serde(default)]
    pub produces: Vec<String>,
    /// Implementation binding for capability execution.
    #[serde(default)]
    pub implementation_binding: Option<String>,
    /// Failure state (optional).
    #[serde(default)]
    pub on_failure: Option<StateRef>,
}

/// An artifact definition.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub struct ArtifactDef {
    /// Producer phase or agent.
    pub producer: String,
    /// Consumer phases or agents.
    pub consumers: Vec<String>,
    /// Whether this artifact is required or optional.
    #[serde(default)]
    pub required: bool,
    /// Whether this artifact is intentionally retained without downstream consumers.
    #[serde(default)]
    pub terminal: bool,
    /// Description of this artifact.
    #[serde(default)]
    pub description: Option<String>,
}

/// A gate definition.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub struct GateDef {
    /// Gate type definition.
    #[serde(default)]
    pub gate_type: Option<GateTypeDef>,
    /// Description.
    #[serde(default)]
    pub description: Option<String>,
}

/// Gate type definitions — matches YAML structure.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case", untagged)]
pub enum GateTypeDef {
    /// Binary pass/fail gate.
    Binary {},
    /// Percentage threshold gate.
    Percentage {
        /// Minimum required coverage percentage.
        threshold: u8,
    },
    /// Count-based gate.
    Count {
        /// Minimum required number of approvals.
        min: u32,
    },
    /// Simple string form ("binary" etc).
    String(String),
}

impl Default for GateTypeDef {
    fn default() -> Self {
        GateTypeDef::Binary {}
    }
}

/// Forge provider definition.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub struct ForgeDef {
    /// Provider name.
    pub provider: String,
    /// Capability definitions.
    #[serde(default)]
    pub capabilities: Option<HashMap<String, CapabilityDef>>,
}

/// A capability definition.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub struct CapabilityDef {
    /// Risk level.
    #[serde(default)]
    pub risk: Option<String>,
    /// Consequence type.
    #[serde(default)]
    pub consequence: Option<String>,
}

impl CapabilityDef {
    /// Whether this capability requires explicit human approval based on
    /// risk level and consequence type.
    pub fn requires_approval(&self) -> bool {
        let risk_high_or_critical = self
            .risk
            .as_ref()
            .is_some_and(|r| r.eq_ignore_ascii_case("high") || r.eq_ignore_ascii_case("critical"));
        let consequence_irreversible_or_modifies = self.consequence.as_ref().is_some_and(|c| {
            c.eq_ignore_ascii_case("irreversible") || c.eq_ignore_ascii_case("modifies")
        });

        risk_high_or_critical || consequence_irreversible_or_modifies
    }
}

/// Storage configuration.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub struct StorageDef {
    /// Use XDG paths.
    pub xdg: bool,
    /// Path configuration.
    #[serde(default)]
    pub paths: Option<StoragePaths>,
}

/// Storage paths configuration.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub struct StoragePaths {
    /// Data path.
    #[serde(default)]
    pub data: Option<String>,
    /// State path.
    #[serde(default)]
    pub state: Option<String>,
    /// Cache path.
    #[serde(default)]
    pub cache: Option<String>,
}

/// Project identity scheme.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub struct ProjectIdentityDef {
    /// Identity scheme.
    pub scheme: String,
    /// Scope requirement.
    pub scope: String,
    /// Fallback setting.
    #[serde(default)]
    pub fallback: Option<String>,
}

/// Checks if a transition is valid given available artifacts and gates.
/// Uses deterministic lookup by transition id.
pub fn valid_transition(
    manifest: &WorkflowManifest,
    transition_id: &str,
    available_artifacts: &[String],
    available_gates: &[String],
) -> Result<(), WorkflowError> {
    // Find transition by ID
    let transition = manifest
        .transitions
        .iter()
        .find(|t| t.id == transition_id)
        .ok_or_else(|| WorkflowError::TransitionNotFound(transition_id.to_string()))?;

    // Check requirements
    for req in &transition.requires {
        match req.kind().or(Some("artifact")) {
            Some("gate") => {
                if !available_gates.contains(&req.name().to_string()) {
                    return Err(WorkflowError::MissingGate(req.name().to_string()));
                }
            }
            _ => {
                // Treat as artifact requirement
                if !available_artifacts.contains(&req.name().to_string()) {
                    return Err(WorkflowError::MissingArtifact(req.name().to_string()));
                }
            }
        }
    }

    Ok(())
}

/// Validates a transition by looking up its ID and checking requirements.
pub fn validate_transition_by_id(
    manifest: &WorkflowManifest,
    transition_id: &str,
    available_artifacts: &[String],
    available_gates: &[String],
) -> TransitionValidation {
    let mut missing_artifacts = Vec::new();
    let mut missing_gates = Vec::new();

    if let Some(transition) = manifest.transitions.iter().find(|t| t.id == transition_id) {
        for req in &transition.requires {
            match req.kind().or(Some("artifact")) {
                Some("gate") => {
                    if !available_gates.contains(&req.name().to_string()) {
                        missing_gates.push(req.name().to_string());
                    }
                }
                _ => {
                    if !available_artifacts.contains(&req.name().to_string()) {
                        missing_artifacts.push(req.name().to_string());
                    }
                }
            }
        }
    }

    TransitionValidation {
        is_valid: missing_artifacts.is_empty() && missing_gates.is_empty(),
        missing_artifacts,
        missing_gates,
    }
}

/// Result of transition validation with detailed error information.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub struct TransitionValidation {
    /// Whether the transition is valid.
    pub is_valid: bool,
    /// Artifacts that are missing.
    #[serde(default)]
    pub missing_artifacts: Vec<String>,
    /// Gates that are missing.
    #[serde(default)]
    pub missing_gates: Vec<String>,
}

#[cfg(test)]
mod tests {
    use super::*;

    fn create_test_manifest() -> WorkflowManifest {
        let statuses = vec![
            CycleStatus::Open,
            CycleStatus::Blocked,
            CycleStatus::Remediating,
        ];
        let phases = vec![Phase::Explore, Phase::Specify, Phase::Design];
        let transitions = vec![
            Transition {
                id: "phase.explore.complete".into(),
                from: Some(StateRef {
                    status: CycleStatus::Open,
                    phase: Some(Phase::Explore),
                }),
                to: StateRef {
                    status: CycleStatus::Open,
                    phase: Some(Phase::Specify),
                },
                requires: vec![
                    Requirement::Structured {
                        kind: "artifact".into(),
                        name: "exploration-report".into(),
                    },
                    Requirement::Structured {
                        kind: "gate".into(),
                        name: "exploration-sufficient".into(),
                    },
                ],
                paths: vec![],
                produces: vec![],
                implementation_binding: None,
                on_failure: None,
            },
            Transition {
                id: "phase.specify.complete".into(),
                from: Some(StateRef {
                    status: CycleStatus::Open,
                    phase: Some(Phase::Specify),
                }),
                to: StateRef {
                    status: CycleStatus::Open,
                    phase: Some(Phase::Design),
                },
                requires: vec![Requirement::Structured {
                    kind: "artifact".into(),
                    name: "specification".into(),
                }],
                paths: vec![],
                produces: vec![],
                implementation_binding: None,
                on_failure: None,
            },
        ];
        let artifacts = HashMap::new();
        let gates = HashMap::new();

        WorkflowManifest {
            schema_version: WORKFLOW_SCHEMA_VERSION,
            workflow: WorkflowDef {
                id: "test-workflow".into(),
                version: "0.1.0".into(),
                description: "Test workflow".into(),
            },
            statuses,
            phases,
            paths: HashMap::new(),
            policies: Policies::default(),
            transitions,
            artifacts,
            gates,
            forge: None,
            storage: None,
            project_identity: None,
        }
    }

    #[test]
    fn test_valid_transition_by_id_success() {
        let manifest = create_test_manifest();

        let result = valid_transition(
            &manifest,
            "phase.explore.complete",
            &["exploration-report".into()],
            &["exploration-sufficient".into()],
        );
        assert!(result.is_ok());
    }

    #[test]
    fn test_valid_transition_by_id_missing_artifact() {
        let manifest = create_test_manifest();

        let result = valid_transition(
            &manifest,
            "phase.explore.complete",
            &[], // No artifacts
            &["exploration-sufficient".into()],
        );
        assert!(matches!(result, Err(WorkflowError::MissingArtifact(_))));
    }

    #[test]
    fn test_valid_transition_by_id_missing_gate() {
        let manifest = create_test_manifest();

        let result = valid_transition(
            &manifest,
            "phase.explore.complete",
            &["exploration-report".into()],
            &[], // No gates
        );
        assert!(matches!(result, Err(WorkflowError::MissingGate(_))));
    }

    #[test]
    fn test_valid_transition_not_found() {
        let manifest = create_test_manifest();

        let result = valid_transition(&manifest, "nonexistent.transition", &[], &[]);
        assert!(matches!(result, Err(WorkflowError::TransitionNotFound(_))));
    }

    #[test]
    fn test_validate_transition_by_id() {
        let manifest = create_test_manifest();

        let validation = validate_transition_by_id(
            &manifest,
            "phase.explore.complete",
            &["exploration-report".into()],
            &["exploration-sufficient".into()],
        );

        assert!(validation.is_valid);
        assert!(validation.missing_artifacts.is_empty());
        assert!(validation.missing_gates.is_empty());
    }

    #[test]
    fn test_workflow_info_from_manifest() {
        let manifest = create_test_manifest();
        let info = WorkflowInfo::from(&manifest);
        assert_eq!(info.id, "test-workflow");
        assert_eq!(info.version, "0.1.0");
    }

    #[test]
    fn test_requirement_kind_detection() {
        let req_artifact = Requirement::Structured {
            kind: "artifact".into(),
            name: "spec".into(),
        };
        assert_eq!(req_artifact.kind(), Some("artifact"));
        assert_eq!(req_artifact.name(), "spec");

        let req_gate = Requirement::Structured {
            kind: "gate".into(),
            name: "tests-pass".into(),
        };
        assert_eq!(req_gate.kind(), Some("gate"));

        let req_simple = Requirement::Simple("project.adopted".into());
        assert_eq!(req_simple.kind(), None);
        assert_eq!(req_simple.name(), "project.adopted");
    }
}
