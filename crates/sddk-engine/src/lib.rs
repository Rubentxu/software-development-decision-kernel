//! Deterministic workflow planning, application, and replay for SDDK.
//!
//! The engine owns workflow interpretation and delegates atomic persistence to
//! `sddk-storage`. Callers supply every identifier and timestamp.

#![forbid(unsafe_code)]
#![deny(clippy::all)]
// ADR-0064 §D-5: internal types that don't need docs
#![allow(missing_docs)]
#![allow(clippy::missing_docs_in_private_items)]

mod adoption;
pub mod cycle_replan;
pub mod cycle_supersede;
pub mod event_bus;
pub mod execution_controller;
pub mod fingerprint;
pub mod gate_evaluator;
pub mod gate_signing;
pub mod inc_generator;
pub mod operator;
pub mod pack_registry;
mod paths;
pub mod receipt_writers;
pub mod retry;
pub mod rules;
pub mod task_executor;
pub mod tasks;
pub mod up_to_date;
pub mod version;
pub mod workflow_runtime;

pub use adoption::*;
pub use cycle_replan::*;
pub use cycle_supersede::*;
pub use event_bus::*;
pub use execution_controller::*;
pub use fingerprint::*;
pub use gate_evaluator::*;
pub use gate_signing::*;
pub use inc_generator::*;
pub use operator::{
    Choice, Map, NodeOutcome, Operator, OperatorContext, OperatorError, Parallel, Sequence, Task,
};
pub use pack_registry::*;
pub use paths::*;
pub use receipt_writers::write_atomic;
pub use retry::{Clock, MockClock, RetryPolicy, RngCore, WallClock};
pub use task_executor::RealTaskExecutor;
pub use tasks::sha256::Sha256Data;
pub use tasks::sha256::Sha256Task;
pub use tasks::{FileWriteTask, HttpFetchTask, SleepTask};
pub use up_to_date::{NotUpToDate, UpToDateVerdict, up_to_date};
pub use version::version;
pub use workflow_runtime::{RunStore, RuntimeError, TickOutcome, WorkflowRuntime};

use std::collections::{BTreeMap, BTreeSet, HashSet};
use std::path::{Path, PathBuf};

use sddk_domain::{
    ArtifactRef, CycleLease, CycleManifest, CyclePath, CycleRecord, CycleStatus, GateOutcomeStatus,
    GateReceipt, GateReceiptNextSeqInput, Ledger, LedgerEvent, LedgerEventInput, Phase,
    Requirement, StateRef, StorageError, Transition, WORKFLOW_SCHEMA_VERSION, WorkflowManifest,
    models::gate_receipt::validate_pass_evidence,
};
use serde::{Deserialize, Serialize};
use serde_json::{Value, json};
use sha2::{Digest, Sha256};
use thiserror::Error;

/// Loads and validates a workflow manifest from a YAML string.
pub fn load_workflow_str(yaml: &str) -> Result<WorkflowManifest, WorkflowLoadError> {
    let manifest = serde_saphyr::from_str(yaml).map_err(WorkflowLoadError::Parse)?;
    validate_workflow(&manifest)?;
    Ok(manifest)
}

/// Loads and validates a workflow manifest from a YAML file.
pub fn load_workflow_path(path: impl AsRef<Path>) -> Result<WorkflowManifest, WorkflowLoadError> {
    let path = path.as_ref();
    let yaml = std::fs::read_to_string(path).map_err(|source| WorkflowLoadError::Io {
        path: path.to_path_buf(),
        source,
    })?;
    load_workflow_str(&yaml)
}

/// Performs semantic validation that is required before workflow execution.
pub fn validate_workflow(manifest: &WorkflowManifest) -> Result<(), WorkflowValidationError> {
    if manifest.schema_version != WORKFLOW_SCHEMA_VERSION {
        return Err(WorkflowValidationError::UnsupportedSchemaVersion {
            actual: manifest.schema_version,
            supported: WORKFLOW_SCHEMA_VERSION,
        });
    }

    ensure_unique(manifest.statuses.iter().copied(), |status| {
        WorkflowValidationError::DuplicateStatus { status }
    })?;
    ensure_unique(manifest.phases.iter().copied(), |phase| {
        WorkflowValidationError::DuplicatePhase { phase }
    })?;

    let mut transition_ids = HashSet::new();
    let mut cycle_starts = Vec::new();
    for transition in &manifest.transitions {
        if transition.id.is_empty() {
            return Err(WorkflowValidationError::EmptyTransitionId);
        }
        if !transition_ids.insert(transition.id.as_str()) {
            return Err(WorkflowValidationError::DuplicateTransitionId {
                transition_id: transition.id.clone(),
            });
        }
        let is_cycle_start =
            transition.id == "cycle.start" || transition.id.starts_with("cycle.start.");
        if is_cycle_start {
            if transition.from.is_some() {
                return Err(WorkflowValidationError::CycleStartHasSource);
            }
            cycle_starts.push(transition);
        } else if transition.from.is_none() {
            return Err(WorkflowValidationError::CreationSourceOnTransition {
                transition_id: transition.id.clone(),
            });
        }
        for path in &transition.paths {
            if !manifest.paths.contains_key(path) {
                return Err(WorkflowValidationError::UnknownTransitionPath {
                    transition_id: transition.id.clone(),
                    path: path.clone(),
                });
            }
        }

        if let Some(from) = &transition.from {
            validate_state_ref(manifest, &transition.id, "from", from)?;
        }
        validate_state_ref(manifest, &transition.id, "to", &transition.to)?;
        if let Some(on_failure) = &transition.on_failure {
            validate_state_ref(manifest, &transition.id, "on_failure", on_failure)?;
        }
        validate_requirements(manifest, transition)?;
    }

    if !cycle_starts
        .iter()
        .any(|transition| transition.id == "cycle.start")
    {
        return Err(WorkflowValidationError::MissingCycleStart);
    }
    if cycle_starts
        .iter()
        .any(|transition| transition.to.phase.is_none())
    {
        return Err(WorkflowValidationError::CycleStartMissingPhase);
    }

    for (path_name, path) in &manifest.paths {
        match path.debt_verification.as_str() {
            "mandatory" | "disabled" => {}
            policy => {
                return Err(WorkflowValidationError::InvalidDebtVerificationPolicy {
                    path: path_name.clone(),
                    policy: policy.to_owned(),
                });
            }
        }
        for phase in &path.phases {
            let parsed =
                parse_phase(phase).ok_or_else(|| WorkflowValidationError::UnknownPathPhase {
                    path: path_name.clone(),
                    phase: phase.clone(),
                })?;
            if !manifest.phases.contains(&parsed) {
                return Err(WorkflowValidationError::UnknownPathPhase {
                    path: path_name.clone(),
                    phase: phase.clone(),
                });
            }
        }
        match cycle_starts
            .iter()
            .filter(|transition| transition_applies_to_path(transition, path_name))
            .count()
        {
            0 => {
                return Err(WorkflowValidationError::MissingPathCycleStart {
                    path: path_name.clone(),
                });
            }
            1 => {}
            _ => {
                return Err(WorkflowValidationError::AmbiguousPathCycleStart {
                    path: path_name.clone(),
                });
            }
        }
    }
    Ok(())
}

fn ensure_unique<T, E>(
    values: impl IntoIterator<Item = T>,
    error: E,
) -> Result<(), WorkflowValidationError>
where
    T: Copy + Eq + std::hash::Hash,
    E: Fn(T) -> WorkflowValidationError,
{
    let mut seen = HashSet::new();
    for value in values {
        if !seen.insert(value) {
            return Err(error(value));
        }
    }
    Ok(())
}

fn validate_state_ref(
    manifest: &WorkflowManifest,
    transition_id: &str,
    field: &'static str,
    state: &StateRef,
) -> Result<(), WorkflowValidationError> {
    if !manifest.statuses.contains(&state.status) {
        return Err(WorkflowValidationError::UnknownTransitionStatus {
            transition_id: transition_id.to_owned(),
            field,
            status: state.status,
        });
    }
    if let Some(phase) = state.phase
        && !manifest.phases.contains(&phase)
    {
        return Err(WorkflowValidationError::UnknownTransitionPhase {
            transition_id: transition_id.to_owned(),
            field,
            phase,
        });
    }
    Ok(())
}

fn validate_requirements(
    manifest: &WorkflowManifest,
    transition: &Transition,
) -> Result<(), WorkflowValidationError> {
    for requirement in &transition.requires {
        let Requirement::Structured { kind, name } = requirement else {
            continue;
        };
        match kind.as_str() {
            "artifact" if !manifest.artifacts.contains_key(name) => {
                return Err(WorkflowValidationError::UnknownArtifactRequirement {
                    transition_id: transition.id.clone(),
                    artifact: name.clone(),
                });
            }
            "gate" if !manifest.gates.contains_key(name) => {
                return Err(WorkflowValidationError::UnknownGateRequirement {
                    transition_id: transition.id.clone(),
                    gate: name.clone(),
                });
            }
            "artifact" | "gate" => {}
            _ => {
                return Err(WorkflowValidationError::UnknownRequirementKind {
                    transition_id: transition.id.clone(),
                    kind: kind.clone(),
                });
            }
        }
    }
    Ok(())
}

fn parse_phase(value: &str) -> Option<Phase> {
    serde_json::from_value(Value::String(value.to_owned())).ok()
}

/// Errors produced while loading a workflow manifest.
#[derive(Debug, Error)]
pub enum WorkflowLoadError {
    /// The manifest file could not be read.
    #[error("failed to read workflow manifest {path:?}: {source}")]
    Io {
        /// Requested manifest path.
        path: PathBuf,
        /// Underlying filesystem failure.
        source: std::io::Error,
    },
    /// YAML could not be deserialized into the workflow domain model.
    #[error("invalid workflow YAML: {0}")]
    Parse(serde_saphyr::Error),
    /// The parsed manifest violates an executable workflow invariant.
    #[error("workflow validation error: {0}")]
    Validation(#[from] WorkflowValidationError),
}

/// Semantic workflow validation errors.
#[derive(Debug, Error, PartialEq, Eq)]
pub enum WorkflowValidationError {
    /// The manifest uses a schema version this runtime does not support.
    #[error("unsupported workflow schema version {actual}; supported version is {supported}")]
    UnsupportedSchemaVersion {
        /// Version found in the manifest.
        actual: i32,
        /// Version supported by this runtime.
        supported: i32,
    },
    /// A status occurs more than once in the declaration.
    #[error("duplicate workflow status: {status:?}")]
    DuplicateStatus {
        /// Repeated status.
        status: CycleStatus,
    },
    /// A phase occurs more than once in the declaration.
    #[error("duplicate workflow phase: {phase:?}")]
    DuplicatePhase {
        /// Repeated phase.
        phase: Phase,
    },
    /// A transition identifier is empty.
    #[error("workflow transition id cannot be empty")]
    EmptyTransitionId,
    /// A transition identifier occurs more than once.
    #[error("duplicate workflow transition id: {transition_id}")]
    DuplicateTransitionId {
        /// Repeated transition identifier.
        transition_id: String,
    },
    /// The required creation transition is absent.
    #[error("workflow does not declare cycle.start")]
    MissingCycleStart,
    /// The creation transition incorrectly declares a source state.
    #[error("cycle.start must declare from: null")]
    CycleStartHasSource,
    /// A non-creation transition incorrectly declares `from: null`.
    #[error("transition {transition_id} declares from: null; only cycle.start may create cycles")]
    CreationSourceOnTransition {
        /// Invalid transition identifier.
        transition_id: String,
    },
    /// The creation target omits the initial phase.
    #[error("cycle.start must declare a target phase")]
    CycleStartMissingPhase,
    /// A transition references a status omitted from the manifest declaration.
    #[error("transition {transition_id} {field} references undeclared status {status:?}")]
    UnknownTransitionStatus {
        /// Transition containing the reference.
        transition_id: String,
        /// State field containing the reference.
        field: &'static str,
        /// Undeclared status.
        status: CycleStatus,
    },
    /// A transition references a phase omitted from the manifest declaration.
    #[error("transition {transition_id} {field} references undeclared phase {phase:?}")]
    UnknownTransitionPhase {
        /// Transition containing the reference.
        transition_id: String,
        /// State field containing the reference.
        field: &'static str,
        /// Undeclared phase.
        phase: Phase,
    },
    /// A structured requirement uses an unsupported kind.
    #[error("transition {transition_id} uses unknown requirement kind {kind:?}")]
    UnknownRequirementKind {
        /// Transition containing the requirement.
        transition_id: String,
        /// Unsupported requirement kind.
        kind: String,
    },
    /// A transition requires an undeclared artifact.
    #[error("transition {transition_id} requires undeclared artifact {artifact:?}")]
    UnknownArtifactRequirement {
        /// Transition containing the requirement.
        transition_id: String,
        /// Missing artifact declaration.
        artifact: String,
    },
    /// A transition requires an undeclared gate.
    #[error("transition {transition_id} requires undeclared gate {gate:?}")]
    UnknownGateRequirement {
        /// Transition containing the requirement.
        transition_id: String,
        /// Missing gate declaration.
        gate: String,
    },
    /// A path names a phase absent from the workflow.
    #[error("path {path} references unknown phase {phase:?}")]
    UnknownPathPhase {
        /// Path containing the phase.
        path: String,
        /// Unknown phase name.
        phase: String,
    },
    /// A path uses an unsupported debt-verification policy.
    #[error("path {path} uses invalid debt verification policy {policy:?}")]
    InvalidDebtVerificationPolicy {
        /// Path containing the policy.
        path: String,
        /// Unsupported policy value.
        policy: String,
    },
    /// A transition is restricted to a path absent from the manifest.
    #[error("transition {transition_id} references unknown workflow path {path:?}")]
    UnknownTransitionPath {
        /// Transition containing the restriction.
        transition_id: String,
        /// Unknown path name.
        path: String,
    },
    /// A workflow path has no applicable cycle creation transition.
    #[error("workflow path {path} has no applicable cycle.start transition")]
    MissingPathCycleStart {
        /// Path without a creation transition.
        path: String,
    },
    /// A workflow path has more than one applicable cycle creation transition.
    #[error("workflow path {path} has multiple applicable cycle.start transitions")]
    AmbiguousPathCycleStart {
        /// Path with ambiguous creation transitions.
        path: String,
    },
}

/// Caller-supplied causal context for one state-changing command.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub struct EventContext {
    /// Stable command invocation identifier.
    pub command_id: String,
    /// Frame grouping events produced by the command.
    pub frame_id: String,
    /// Stable event identifier.
    pub event_id: String,
    /// Actor responsible for the command.
    pub actor: String,
    /// Explicit event timestamp.
    pub occurred_at: String,
}

/// Caller evidence used to evaluate a transition.
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub struct TransitionEvidence {
    /// Named non-artifact preconditions, including cycle-start requirements.
    #[serde(default)]
    pub requirements: BTreeSet<String>,
    /// Artifact results offered to the transition.
    #[serde(default)]
    pub artifacts: BTreeMap<String, ArtifactRef>,
    /// Persisted gate receipts referenced by the transition's required gates.
    #[serde(default)]
    pub gates: BTreeMap<String, GateReceiptRef>,
}

/// Reference to a persisted, authorized gate receipt.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub struct GateReceiptRef {
    /// Identifier of a persisted gate receipt.
    pub receipt_id: String,
}

/// Logical outcome selected while planning a transition.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum TransitionOutcome {
    /// All required gates passed and the normal target applies.
    Succeeded,
    /// At least one gate failed and the declared failure target applies.
    Failed,
}

/// Input for one authorized gate evaluation.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct GateEvaluationInput {
    /// Cycle whose transition requires the gate.
    pub cycle_id: String,
    /// Transition that declares the gate.
    pub transition_id: String,
    /// Gate name being evaluated.
    pub gate: String,
    /// Evaluator identifier registered for this gate.
    pub evaluator: String,
    /// Sanitized evaluation evidence.
    pub evidence: Value,
    /// Evaluation outcome recorded by the evaluator.
    pub outcome: GateOutcomeStatus,
    /// Caller-supplied deterministic timestamp.
    pub evaluated_at: String,
    /// Actor responsible for the evaluation.
    pub actor: String,
    /// Command invocation identifier.
    pub command_id: String,
}

/// Deterministic plan for a declared non-creation transition.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub struct TransitionPlan {
    transition_id: String,
    outcome: TransitionOutcome,
    failed_gates: Vec<String>,
    evidence: TransitionEvidence,
    state_before: CycleManifest,
    state_after: CycleManifest,
}

impl TransitionPlan {
    /// Declared transition identifier.
    pub fn transition_id(&self) -> &str {
        &self.transition_id
    }

    /// Planned success or failure path.
    pub fn outcome(&self) -> TransitionOutcome {
        self.outcome
    }

    /// Gates that selected the failure target, in declaration order.
    pub fn failed_gates(&self) -> &[String] {
        &self.failed_gates
    }

    /// Snapshot before the transition.
    pub fn state_before(&self) -> &CycleManifest {
        &self.state_before
    }

    /// Snapshot after the transition.
    pub fn state_after(&self) -> &CycleManifest {
        &self.state_after
    }
}

/// Explicit input used to plan cycle creation through `cycle.start`.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub struct CycleStartInput {
    /// Caller-constructed cycle manifest containing identity and repository data.
    pub manifest: CycleManifest,
    /// Explicitly satisfied initial workflow requirements.
    pub requirements: BTreeSet<String>,
}

/// Deterministic plan for creating a cycle.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub struct CycleStartPlan {
    input: CycleStartInput,
    state_after: CycleManifest,
}

impl CycleStartPlan {
    /// Initial canonical cycle snapshot.
    pub fn state_after(&self) -> &CycleManifest {
        &self.state_after
    }
}

/// Minimal immutable receipt for an applied ledger event.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub struct EventReceipt {
    /// Stable caller-supplied event identifier.
    pub event_id: String,
    /// Monotonic ledger sequence assigned by storage.
    pub sequence: i64,
    /// Hash of the persisted event and its predecessor link.
    pub event_hash: String,
}

impl From<&LedgerEvent> for EventReceipt {
    fn from(event: &LedgerEvent) -> Self {
        Self {
            event_id: event.event_id.clone(),
            sequence: event.sequence,
            event_hash: event.event_hash.clone(),
        }
    }
}

/// Result of atomically applying a transition.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub struct TransitionResult {
    /// Applied transition identifier.
    pub transition_id: String,
    /// Applied logical outcome.
    pub outcome: TransitionOutcome,
    /// Persisted cycle snapshot.
    pub manifest: CycleManifest,
    /// Causal ledger receipt.
    pub event: EventReceipt,
}

/// Result of atomically creating a cycle.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub struct CycleStartResult {
    /// Persisted initial cycle snapshot.
    pub manifest: CycleManifest,
    /// Causal ledger receipt.
    pub event: EventReceipt,
}

/// Debt verification behavior declared for a workflow path.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum DebtVerificationPolicy {
    /// Debt verification is required for this path.
    Mandatory,
    /// Debt verification is disabled for this path.
    Disabled,
}

/// Verified relationship between the replayed and stored cycle states.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub struct ReplayVerification {
    /// Reconstructed logical cycle snapshot.
    pub manifest: CycleManifest,
    /// Sequence of the state event used for reconstruction.
    pub sequence: i64,
}

/// Outcome of restoring a cycle snapshot from its causal ledger events.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub struct RebuildVerification {
    /// Reconstructed logical cycle snapshot.
    pub manifest: CycleManifest,
    /// Sequence of the state event used for reconstruction.
    pub sequence: i64,
    /// Whether the materialized snapshot was missing and had to be restored.
    pub restored: bool,
}

/// Errors emitted by deterministic engine operations.
#[derive(Debug, Error)]
pub enum EngineError {
    /// The requested transition identifier is not declared.
    #[error("undeclared transition: {transition_id}")]
    UndeclaredTransition {
        /// Unknown transition identifier.
        transition_id: String,
    },
    /// A creation transition was passed to the normal transition API.
    #[error("transition {transition_id} creates a cycle and must use the cycle-start API")]
    CreationTransitionRequiresStartApi {
        /// Creation transition identifier.
        transition_id: String,
    },
    /// The current cycle snapshot does not match the transition source.
    #[error(
        "transition {transition_id} expects {expected_status:?}/{expected_phase:?}, found {actual_status:?}/{actual_phase:?}"
    )]
    SourceStateMismatch {
        /// Requested transition identifier.
        transition_id: String,
        /// Expected source status.
        expected_status: CycleStatus,
        /// Expected source phase, or any phase when absent.
        expected_phase: Option<Phase>,
        /// Actual cycle status.
        actual_status: CycleStatus,
        /// Actual cycle phase.
        actual_phase: Phase,
    },
    /// A required non-artifact precondition was not supplied.
    #[error("transition {transition_id} is missing requirement {requirement:?}")]
    MissingRequirement {
        /// Requested transition identifier.
        transition_id: String,
        /// Missing precondition.
        requirement: String,
    },
    /// A required artifact is absent from both the snapshot and new evidence.
    #[error("transition {transition_id} is missing artifact {artifact:?}")]
    MissingArtifact {
        /// Requested transition identifier.
        transition_id: String,
        /// Missing artifact kind.
        artifact: String,
    },
    /// A required gate outcome was not supplied.
    #[error("transition {transition_id} is missing gate receipt for {gate:?}")]
    MissingGateReceipt {
        /// Requested transition identifier.
        transition_id: String,
        /// Missing gate name.
        gate: String,
    },
    /// A referenced gate receipt does not exist.
    #[error("transition {transition_id} references unknown gate receipt {receipt_id}")]
    UnknownGateReceipt {
        /// Requested transition identifier.
        transition_id: String,
        /// Unknown receipt identifier.
        receipt_id: String,
    },
    /// A gate receipt does not match the requested gate or transition.
    #[error("gate receipt {receipt_id} does not attest gate {gate} for transition {transition_id}")]
    GateReceiptMismatch {
        /// Referenced receipt identifier.
        receipt_id: String,
        /// Expected gate name.
        gate: String,
        /// Expected transition identifier.
        transition_id: String,
    },
    /// A gate receipt attests a different plan state.
    #[error("gate receipt {receipt_id} is stale for the current cycle state")]
    StaleGateReceipt {
        /// Receipt with a mismatched plan hash.
        receipt_id: String,
    },
    /// The evaluator is not registered for the gate.
    #[error("evaluator {evaluator} is not registered for gate {gate}")]
    UnregisteredEvaluator {
        /// Gate being evaluated.
        gate: String,
        /// Unregistered evaluator identifier.
        evaluator: String,
    },
    /// A gate receipt belongs to a different cycle.
    #[error("gate receipt {receipt_id} belongs to cycle {cycle}, expected {expected}")]
    GateReceiptScopeMismatch {
        /// Referenced receipt identifier.
        receipt_id: String,
        /// Receipt cycle.
        cycle: String,
        /// Expected cycle.
        expected: String,
    },
    /// A failed gate has no declared failure target.
    #[error("transition {transition_id} gate {gate:?} failed without an on_failure target")]
    GateFailedWithoutTarget {
        /// Requested transition identifier.
        transition_id: String,
        /// Failed gate name.
        gate: String,
    },
    /// Evidence contains an artifact not declared as an output of this transition.
    #[error("transition {transition_id} does not produce artifact {artifact:?}")]
    UndeclaredProducedArtifact {
        /// Requested transition identifier.
        transition_id: String,
        /// Unexpected artifact name.
        artifact: String,
    },
    /// An artifact map key disagrees with its canonical kind.
    #[error("artifact key {key:?} does not match artifact kind {kind:?}")]
    ArtifactKindMismatch {
        /// Artifact evidence key.
        key: String,
        /// Kind declared by the artifact reference.
        kind: String,
    },
    /// A cycle uses a path absent from the workflow manifest.
    #[error("unknown workflow path: {path}")]
    UnknownPath {
        /// Unknown path name.
        path: String,
    },
    /// The requested transition is not allowed for the cycle's workflow path.
    #[error("transition {transition_id} is not allowed for workflow path {path}")]
    TransitionPathMismatch {
        /// Requested transition identifier.
        transition_id: String,
        /// Current cycle path.
        path: String,
    },
    /// A plan was built from an older cycle snapshot.
    #[error("transition plan is stale for cycle {cycle_id}")]
    StalePlan {
        /// Cycle whose snapshot changed.
        cycle_id: String,
    },
    /// A supplied plan differs from the engine's deterministic recomputation.
    #[error("transition plan failed deterministic revalidation")]
    InvalidPlan,
    /// A workflow state could not be represented as JSON for the ledger.
    #[error("failed to serialize workflow state: {0}")]
    StateSerialization(#[from] serde_json::Error),
    /// The cycle has no workflow state events to replay.
    #[error("cycle {cycle_id} has no replayable state events")]
    MissingReplayState {
        /// Cycle missing state history.
        cycle_id: String,
    },
    /// A workflow event is missing its post-state.
    #[error("cycle state event at sequence {sequence} has no state_after")]
    MissingStateAfter {
        /// Invalid event sequence.
        sequence: i64,
    },
    /// A workflow event stores a non-object post-state.
    #[error("cycle state event at sequence {sequence} has non-object state_after")]
    NonObjectStateAfter {
        /// Invalid event sequence.
        sequence: i64,
    },
    /// A workflow event stores an object that is not a valid cycle manifest.
    #[error("cycle state event at sequence {sequence} has corrupt state_after: {source}")]
    CorruptStateAfter {
        /// Invalid event sequence.
        sequence: i64,
        /// Deserialization failure.
        source: serde_json::Error,
    },
    /// Replayed state and the materialized cycle snapshot differ.
    #[error("replayed cycle state does not match stored snapshot for {cycle_id}")]
    SnapshotMismatch {
        /// Cycle with divergent state.
        cycle_id: String,
    },
    /// Pass evidence validation failed — missing required fields.
    #[error("gate receipt evidence validation failed: {reason}")]
    InvalidPassEvidence {
        /// Human-readable reason naming the specific missing field.
        reason: String,
    },
    /// Goal input could not be read (missing or corrupt cycle/evidence).
    #[error("goal input is unreadable")]
    GoalInputUnreadable,
    /// Cycle supersede requires exactly one of successor or reason.
    #[error("supersede requires exactly one of successor cycle ID or closed-set reason")]
    SupersedeRequiresExactlyOne,
    /// Cycle supersede cannot target itself.
    #[error("cycle cannot supersede itself")]
    SupersedeSelfForbidden,
    /// Cycle supersede evidence_refs list cannot be empty.
    #[error("supersede evidence refs list cannot be empty")]
    SupersedeEvidenceRefsRequired,
    /// Cycle supersede successor cycle does not exist.
    #[error("successor cycle does not exist: {0}")]
    SupersedeSuccessorNotFound(String),
    /// Cycle replan counter limit exceeded.
    #[error("replan limit exceeded: counter > 5")]
    ReplanLimitExceeded,
    /// Cycle replan delta is empty.
    #[error("replan delta is empty")]
    ReplanEmptyDelta,
    /// Persistence rejected the operation.
    #[error("storage error: {0}")]
    Storage(#[from] StorageError),
}

/// Deterministic workflow runtime backed by a ledger implementation.
pub struct Engine<L: Ledger> {
    workflow: WorkflowManifest,
    ledger: L,
    evaluators: BTreeMap<String, BTreeSet<String>>,
}

/// Evaluator identifier registered by default for every declared gate.
pub const DEFAULT_EVALUATOR: &str = "sddk.cli";

impl<L: Ledger> Engine<L> {
    /// Constructs an engine after validating the supplied workflow manifest.
    pub fn new(workflow: WorkflowManifest, ledger: L) -> Result<Self, WorkflowValidationError>
    where
        L: 'static,
    {
        validate_workflow(&workflow)?;
        let mut evaluators = BTreeMap::new();
        for gate in workflow.gates.keys() {
            evaluators.insert(gate.clone(), BTreeSet::from([DEFAULT_EVALUATOR.to_owned()]));
        }
        Ok(Self {
            workflow,
            ledger,
            evaluators,
        })
    }

    /// Registers an evaluator for one gate.
    pub fn register_evaluator(&mut self, gate: &str, evaluator: &str) {
        self.evaluators
            .entry(gate.to_owned())
            .or_default()
            .insert(evaluator.to_owned());
    }

    /// Returns whether an evaluator is registered for a gate.
    pub fn evaluator_registered(&self, gate: &str, evaluator: &str) -> bool {
        self.evaluators
            .get(gate)
            .is_some_and(|evaluators| evaluators.contains(evaluator))
    }

    /// Computes the deterministic plan hash a gate receipt must attest.
    pub fn plan_hash(
        &self,
        cycle_id: &str,
        transition_id: &str,
        state_before: &CycleManifest,
    ) -> String {
        let material = serde_json::json!({
            "cycle_id": cycle_id,
            "transition_id": transition_id,
            "state_before": state_before,
        });
        let digest = Sha256::digest(material.to_string().as_bytes());
        format!("sha256:{digest:x}")
    }

    /// Authorizes and persists one gate evaluation receipt.
    ///
    /// The evaluator must be registered for the gate, the gate must be declared
    /// by the transition, and the receipt is bound to the deterministic plan
    /// hash of the current cycle state.
    pub fn evaluate_gate(
        &mut self,
        input: &GateEvaluationInput,
    ) -> Result<GateReceipt, EngineError> {
        if !self
            .evaluators
            .get(&input.gate)
            .is_some_and(|evaluators| evaluators.contains(&input.evaluator))
        {
            return Err(EngineError::UnregisteredEvaluator {
                gate: input.gate.clone(),
                evaluator: input.evaluator.clone(),
            });
        }
        let transition = self.transition(&input.transition_id)?;
        let declares = transition.requires.iter().any(|requirement| {
            matches!(
                requirement,
                Requirement::Structured { kind, name } if kind == "gate" && name == &input.gate
            )
        });
        if !declares {
            return Err(EngineError::GateReceiptMismatch {
                receipt_id: "evaluation".into(),
                gate: input.gate.clone(),
                transition_id: transition.id.clone(),
            });
        }

        // REQ-IPV (spec-v2): validate pass evidence before persistence.
        // When outcome is Passed, evidence MUST contain argv, exit_code, and output_digest.
        if input.outcome == GateOutcomeStatus::Passed
            && let Err(e) = validate_pass_evidence(&input.evidence)
        {
            return Err(EngineError::InvalidPassEvidence {
                reason: e.to_string(),
            });
        }

        let state_before = self.ledger.get_cycle(&input.cycle_id)?.manifest;
        let plan_hash = self.plan_hash(&input.cycle_id, &input.transition_id, &state_before);
        let frame_id = format!("frame:{}", input.command_id);
        Ok(self
            .ledger
            .insert_gate_receipt_next_seq(&GateReceiptNextSeqInput {
                project_id: state_before.project_id,
                cycle_id: Some(input.cycle_id.clone()),
                gate: input.gate.clone(),
                evaluator: input.evaluator.clone(),
                transition_id: input.transition_id.clone(),
                plan_hash,
                outcome: input.outcome,
                evidence: input.evidence.clone(),
                actor: input.actor.clone(),
                command_id: input.command_id.clone(),
                frame_id,
                evaluated_at: input.evaluated_at.clone(),
            })?)
    }

    /// Returns the validated workflow manifest.
    pub fn workflow(&self) -> &WorkflowManifest {
        &self.workflow
    }

    /// Returns read-only access to the backing ledger.
    pub fn ledger(&self) -> &dyn Ledger {
        &self.ledger
    }

    /// Returns the declared debt-verification behavior for a named path.
    pub fn debt_verification_policy(
        &self,
        path: &str,
    ) -> Result<DebtVerificationPolicy, EngineError> {
        let definition = self
            .workflow
            .paths
            .get(path)
            .ok_or_else(|| EngineError::UnknownPath {
                path: path.to_owned(),
            })?;
        match definition.debt_verification.as_str() {
            "mandatory" => Ok(DebtVerificationPolicy::Mandatory),
            "disabled" => Ok(DebtVerificationPolicy::Disabled),
            _ => unreachable!("workflow validation rejects unknown debt policies"),
        }
    }

    /// Plans creation of a cycle through the declared `cycle.start` transition.
    pub fn plan_cycle_start(&self, input: CycleStartInput) -> Result<CycleStartPlan, EngineError> {
        let path = cycle_path_name(&input.manifest.path);
        self.debt_verification_policy(path)?;
        let transition = self
            .cycle_start_transition(path)
            .expect("workflow validation requires one cycle.start transition per path");
        for requirement in &transition.requires {
            match requirement {
                Requirement::Simple(name) if !input.requirements.contains(name) => {
                    return Err(EngineError::MissingRequirement {
                        transition_id: transition.id.clone(),
                        requirement: name.clone(),
                    });
                }
                Requirement::Structured { .. } => {
                    return Err(EngineError::InvalidPlan);
                }
                Requirement::Simple(_) => {}
            }
        }

        let mut state_after = input.manifest.clone();
        state_after.status = transition.to.status;
        state_after.phase = transition
            .to
            .phase
            .expect("validated cycle.start target has a phase");
        Ok(CycleStartPlan { input, state_after })
    }

    /// Atomically persists a planned cycle snapshot and its creation event.
    pub fn apply_cycle_start(
        &mut self,
        plan: &CycleStartPlan,
        context: &EventContext,
    ) -> Result<CycleStartResult, EngineError> {
        if self.plan_cycle_start(plan.input.clone())? != *plan {
            return Err(EngineError::InvalidPlan);
        }
        let transition_id = self
            .cycle_start_transition(cycle_path_name(&plan.input.manifest.path))
            .expect("workflow validation requires one cycle.start transition per path")
            .id
            .clone();
        let manifest = plan.state_after.clone();
        let state_after = serde_json::to_value(&manifest)?;
        let event_input = event_input(
            &manifest,
            context,
            "cycle.created",
            None,
            Some(state_after),
            json!({
                "transition_id": transition_id,
                "outcome": TransitionOutcome::Succeeded,
            }),
        );
        let cycle = CycleRecord {
            manifest: manifest.clone(),
            created_at: context.occurred_at.clone(),
            updated_at: context.occurred_at.clone(),
        };
        let event = self.ledger.insert_cycle_with_event(&cycle, &event_input)?;
        Ok(CycleStartResult {
            manifest,
            event: EventReceipt::from(&event),
        })
    }

    /// Plans one declared non-creation transition by identifier.
    pub fn plan_transition(
        &self,
        cycle_id: &str,
        transition_id: &str,
        evidence: TransitionEvidence,
    ) -> Result<TransitionPlan, EngineError> {
        let current = self.ledger.get_cycle(cycle_id)?.manifest;
        self.plan_transition_from_state(current, transition_id, evidence)
    }

    /// Atomically applies a plan after revalidating it against current state.
    ///
    /// When the transition moves the cycle into a new `phase` AND the
    /// outcome is `Succeeded`, this method also atomically releases the
    /// cycle lease (deletes the row) and emits a `lease.released` event in
    /// the same transaction. The auto-release keeps the FSM aligned with
    /// "active lock released by Release phase" — operators that need to
    /// retain the lease across phases must `renew` it before the next
    /// transition.
    pub fn apply_transition(
        &mut self,
        plan: &TransitionPlan,
        context: &EventContext,
    ) -> Result<TransitionResult, EngineError> {
        let current = self.ledger.get_cycle(&plan.state_before.cycle_id)?.manifest;
        if current != plan.state_before {
            return Err(EngineError::StalePlan {
                cycle_id: plan.state_before.cycle_id.clone(),
            });
        }
        let recomputed =
            self.plan_transition_from_state(current, &plan.transition_id, plan.evidence.clone())?;
        if recomputed != *plan {
            return Err(EngineError::InvalidPlan);
        }

        let state_before = serde_json::to_value(&plan.state_before)?;
        let state_after = serde_json::to_value(&plan.state_after)?;
        let event_input = event_input(
            &plan.state_after,
            context,
            "cycle.transitioned",
            Some(state_before),
            Some(state_after),
            json!({
                "transition_id": plan.transition_id,
                "outcome": plan.outcome,
                "failed_gates": plan.failed_gates,
            }),
        );
        let should_auto_release = plan.outcome == TransitionOutcome::Succeeded
            && plan.state_before.phase != plan.state_after.phase;
        let event = self.ledger.update_cycle_with_event(
            &plan.state_after,
            &context.occurred_at,
            &event_input,
            should_auto_release,
        )?;
        Ok(TransitionResult {
            transition_id: plan.transition_id.clone(),
            outcome: plan.outcome,
            manifest: plan.state_after.clone(),
            event: EventReceipt::from(&event),
        })
    }

    /// Reconstructs the latest logical cycle snapshot from state events.
    pub fn replay_cycle(&self, cycle_id: &str) -> Result<ReplayVerification, EngineError> {
        let (sequence, manifest, _) = replay_state(cycle_id, &self.ledger)?;
        Ok(ReplayVerification { manifest, sequence })
    }

    /// Restores a missing materialized snapshot from its causal ledger events.
    ///
    /// Returns `restored: true` only when the cycle snapshot row was missing
    /// and was rebuilt. The caller MUST hold an unexpired lease (verified
    /// beforehand with `require_lease_fence`); the engine then emits a
    /// `cycle.snapshot.restored` ledger event in the same transaction so the
    /// audit trail records the act of restoration. A stored snapshot that
    /// disagrees with the replayed state is treated as an integrity alarm,
    /// never overwritten.
    ///
    /// When `dry_run` is `true`, no writes occur: the NotFound case returns an
    /// error instead of persisting, so the ledger digest and event count are unchanged.
    pub fn rebuild_cycle(
        &mut self,
        cycle_id: &str,
        context: &EventContext,
        now_ms: i64,
        dry_run: bool,
    ) -> Result<RebuildVerification, EngineError> {
        let (sequence, manifest, occurred_at) = replay_state(cycle_id, &self.ledger)?;
        match self.ledger.get_cycle(cycle_id) {
            Ok(record) if record.manifest == manifest => Ok(RebuildVerification {
                manifest,
                sequence,
                restored: false,
            }),
            Ok(_) => Err(EngineError::SnapshotMismatch {
                cycle_id: cycle_id.to_owned(),
            }),
            Err(StorageError::NotFound {
                entity: "cycle", ..
            }) => {
                if dry_run {
                    return Err(EngineError::SnapshotMismatch {
                        cycle_id: cycle_id.to_owned(),
                    });
                }
                let cycle = CycleRecord {
                    manifest: manifest.clone(),
                    created_at: occurred_at.clone(),
                    updated_at: occurred_at.clone(),
                };
                let event_input = event_input(
                    &cycle.manifest,
                    context,
                    "cycle.snapshot.restored",
                    None,
                    None,
                    json!({
                        "cycle_id": cycle_id,
                        "restored_at_ms": now_ms,
                    }),
                );
                self.ledger.insert_cycle_with_event(&cycle, &event_input)?;
                Ok(RebuildVerification {
                    manifest,
                    sequence,
                    restored: true,
                })
            }
            Err(error) => Err(error.into()),
        }
    }

    /// Verifies that the current lease still matches the caller's fencing
    /// token and has not expired at `now_ms`. An expired lease is rejected
    /// with the same `LeaseExpired` error variant the storage layer surfaces,
    /// so the CLI's `run_cycle_transition` fails-closed instead of accepting
    /// a fence that should have been re-acquired.
    pub fn require_lease_fence(
        &self,
        cycle_id: &str,
        owner: &str,
        fencing_token: i64,
        now_ms: i64,
    ) -> Result<CycleLease, EngineError> {
        Ok(self
            .ledger
            .verify_cycle_lease(cycle_id, owner, fencing_token, now_ms)?)
    }

    /// Acquires an absent or expired cycle lease.
    pub fn acquire_cycle_lease(
        &mut self,
        cycle_id: &str,
        owner: &str,
        now_ms: i64,
        expires_at_ms: i64,
    ) -> Result<CycleLease, EngineError> {
        Ok(self
            .ledger
            .acquire_cycle_lease(cycle_id, owner, now_ms, expires_at_ms)?)
    }

    /// Replays a cycle and verifies it equals the materialized SQLite snapshot.
    pub fn verify_cycle_snapshot(&self, cycle_id: &str) -> Result<ReplayVerification, EngineError> {
        let replayed = self.replay_cycle(cycle_id)?;
        let stored = self.ledger.get_cycle(cycle_id)?.manifest;
        if replayed.manifest != stored {
            return Err(EngineError::SnapshotMismatch {
                cycle_id: cycle_id.to_owned(),
            });
        }
        Ok(replayed)
    }

    fn transition(&self, transition_id: &str) -> Result<&Transition, EngineError> {
        self.workflow
            .transitions
            .iter()
            .find(|transition| transition.id == transition_id)
            .ok_or_else(|| EngineError::UndeclaredTransition {
                transition_id: transition_id.to_owned(),
            })
    }

    fn cycle_start_transition(&self, path: &str) -> Option<&Transition> {
        self.workflow.transitions.iter().find(|transition| {
            transition.from.is_none() && transition_applies_to_path(transition, path)
        })
    }

    fn plan_transition_from_state(
        &self,
        state_before: CycleManifest,
        transition_id: &str,
        evidence: TransitionEvidence,
    ) -> Result<TransitionPlan, EngineError> {
        let transition = self.transition(transition_id)?;
        let path = cycle_path_name(&state_before.path);
        if !transition_applies_to_path(transition, path) {
            return Err(EngineError::TransitionPathMismatch {
                transition_id: transition.id.clone(),
                path: path.to_owned(),
            });
        }
        let source = transition.from.as_ref().ok_or_else(|| {
            EngineError::CreationTransitionRequiresStartApi {
                transition_id: transition.id.clone(),
            }
        })?;
        if source.status != state_before.status
            || source
                .phase
                .is_some_and(|phase| phase != state_before.phase)
        {
            return Err(EngineError::SourceStateMismatch {
                transition_id: transition.id.clone(),
                expected_status: source.status,
                expected_phase: source.phase,
                actual_status: state_before.status,
                actual_phase: state_before.phase,
            });
        }

        for (name, artifact) in &evidence.artifacts {
            if !transition.produces.contains(name) {
                return Err(EngineError::UndeclaredProducedArtifact {
                    transition_id: transition.id.clone(),
                    artifact: name.clone(),
                });
            }
            if artifact.kind != *name {
                return Err(EngineError::ArtifactKindMismatch {
                    key: name.clone(),
                    kind: artifact.kind.clone(),
                });
            }
        }

        for requirement in &transition.requires {
            match requirement {
                Requirement::Simple(name) if !evidence.requirements.contains(name) => {
                    return Err(EngineError::MissingRequirement {
                        transition_id: transition.id.clone(),
                        requirement: name.clone(),
                    });
                }
                Requirement::Structured { kind, name }
                    if kind == "artifact"
                        && !state_before.artifacts.contains_key(name)
                        && !evidence.artifacts.contains_key(name) =>
                {
                    return Err(EngineError::MissingArtifact {
                        transition_id: transition.id.clone(),
                        artifact: name.clone(),
                    });
                }
                Requirement::Structured { kind, name }
                    if kind == "gate" && !evidence.gates.contains_key(name) =>
                {
                    return Err(EngineError::MissingGateReceipt {
                        transition_id: transition.id.clone(),
                        gate: name.clone(),
                    });
                }
                _ => {}
            }
        }

        let mut failed_gates = Vec::new();
        for requirement in &transition.requires {
            let Requirement::Structured { kind, name } = requirement else {
                continue;
            };
            if kind != "gate" {
                continue;
            }
            let Some(reference) = evidence.gates.get(name) else {
                return Err(EngineError::MissingGateReceipt {
                    transition_id: transition.id.clone(),
                    gate: name.clone(),
                });
            };
            let receipt = self
                .ledger
                .get_gate_receipt(&reference.receipt_id)
                .map_err(|_| EngineError::UnknownGateReceipt {
                    transition_id: transition.id.clone(),
                    receipt_id: reference.receipt_id.clone(),
                })?;
            if receipt.gate != *name || receipt.transition_id != transition.id {
                return Err(EngineError::GateReceiptMismatch {
                    receipt_id: reference.receipt_id.clone(),
                    gate: name.clone(),
                    transition_id: transition.id.clone(),
                });
            }
            if receipt.cycle_id.as_deref() != Some(state_before.cycle_id.as_str()) {
                return Err(EngineError::GateReceiptScopeMismatch {
                    receipt_id: reference.receipt_id.clone(),
                    cycle: receipt.cycle_id.unwrap_or_default(),
                    expected: state_before.cycle_id.clone(),
                });
            }
            let expected_hash = self.plan_hash(
                &state_before.cycle_id,
                transition.id.as_str(),
                &state_before,
            );
            if receipt.plan_hash != expected_hash {
                return Err(EngineError::StaleGateReceipt {
                    receipt_id: reference.receipt_id.clone(),
                });
            }
            if receipt.outcome == GateOutcomeStatus::Failed {
                failed_gates.push(name.clone());
            }
        }
        let (outcome, target) = if failed_gates.is_empty() {
            (TransitionOutcome::Succeeded, &transition.to)
        } else {
            let target = transition.on_failure.as_ref().ok_or_else(|| {
                EngineError::GateFailedWithoutTarget {
                    transition_id: transition.id.clone(),
                    gate: failed_gates[0].clone(),
                }
            })?;
            (TransitionOutcome::Failed, target)
        };

        let mut state_after = state_before.clone();
        state_after.status = target.status;
        state_after.phase = target.phase.unwrap_or(state_before.phase);
        for (name, artifact) in &evidence.artifacts {
            state_after.artifacts.insert(name.clone(), artifact.clone());
        }
        Ok(TransitionPlan {
            transition_id: transition.id.clone(),
            outcome,
            failed_gates,
            evidence,
            state_before,
            state_after,
        })
    }
}

fn event_input(
    manifest: &CycleManifest,
    context: &EventContext,
    event_type: &str,
    state_before: Option<Value>,
    state_after: Option<Value>,
    payload: Value,
) -> LedgerEventInput {
    LedgerEventInput {
        event_id: context.event_id.clone(),
        project_id: manifest.project_id.clone(),
        cycle_id: Some(manifest.cycle_id.clone()),
        frame_id: context.frame_id.clone(),
        command_id: context.command_id.clone(),
        actor: context.actor.clone(),
        event_type: event_type.to_owned(),
        occurred_at: context.occurred_at.clone(),
        state_before,
        state_after,
        payload,
    }
}

fn is_cycle_state_event(event: &LedgerEvent) -> bool {
    matches!(
        event.event_type.as_str(),
        "cycle.created" | "cycle.transitioned"
    )
}

fn replay_state(
    cycle_id: &str,
    ledger: &impl Ledger,
) -> Result<(i64, CycleManifest, String), EngineError> {
    let events = ledger.list_cycle_events(cycle_id)?;
    let mut latest = None;
    for event in events.iter().filter(|event| is_cycle_state_event(event)) {
        let state = event
            .state_after
            .as_ref()
            .ok_or(EngineError::MissingStateAfter {
                sequence: event.sequence,
            })?;
        if !state.is_object() {
            return Err(EngineError::NonObjectStateAfter {
                sequence: event.sequence,
            });
        }
        let manifest = serde_json::from_value(state.clone()).map_err(|source| {
            EngineError::CorruptStateAfter {
                sequence: event.sequence,
                source,
            }
        })?;
        latest = Some((event.sequence, manifest, event.occurred_at.clone()));
    }
    latest.ok_or_else(|| EngineError::MissingReplayState {
        cycle_id: cycle_id.to_owned(),
    })
}

fn cycle_path_name(path: &CyclePath) -> &'static str {
    match path {
        CyclePath::AMin => "A-min",
        CyclePath::ALite => "A-lite",
        CyclePath::AFull => "A-full",
        CyclePath::BDirect => "B-direct",
    }
}

fn transition_applies_to_path(transition: &Transition, path: &str) -> bool {
    transition.paths.is_empty() || transition.paths.iter().any(|candidate| candidate == path)
}

/// One entry in the frontier of legal transitions from a given cycle state.
///
/// Produced by [`frontier_for_state`] and consumed by the `cycle next` CLI
/// command to render the human-readable advisory output.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub struct FrontierEntry {
    /// Declared transition identifier.
    pub transition_id: String,
    /// Source state of this transition.
    pub from: StateRef,
    /// Target state of this transition.
    pub to: StateRef,
    /// Whether all gate requirements are satisfied by fresh ledger receipts.
    pub requires_met: bool,
    /// Gate names that are NOT satisfied (empty when `requires_met` is true).
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub unmet_gates: Vec<String>,
    /// Commands / hints that are NOT satisfied (non-gate requirements).
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub unmet_requirements: Vec<String>,
}

/// Derives the frontier of legal transitions from the declared workflow graph
/// and the current cycle state, intersecting with gate receipts from the ledger.
///
/// This is a pure derivation — no I/O beyond reading the ledger. The function
/// reads the current cycle state (status + phase + path) and the workflow
/// manifest to enumerate all transitions whose `from` state matches the current
/// state and whose `paths` attribute allows the current cycle path.
///
/// Gate satisfaction is determined by querying `list_gate_receipts_for` with the
/// cycle_id, transition_id, and the plan_hash computed from the current state.
/// A transition is `requires_met: true` only when ALL gate requirements have
/// at least one fresh receipt in the ledger.
///
/// # D1 Binding
///
/// This function hard-codes ZERO phase sequences. The frontier is derived
/// entirely from the declared transitions in the workflow manifest filtered by
/// the current state. A different workflow manifest with a different topology
/// (e.g. diamond/parallel) produces a different frontier without any code
/// changes.
pub fn frontier_for_state(
    workflow: &WorkflowManifest,
    state: &CycleManifest,
    cycle_id: &str,
    ledger: &dyn Ledger,
) -> Result<Vec<FrontierEntry>, EngineError> {
    let path_name = cycle_path_name(&state.path);
    let mut entries = Vec::new();

    for transition in &workflow.transitions {
        // Skip transitions without a source state (cycle.start variants)
        let from = match &transition.from {
            Some(f) => f,
            None => continue,
        };

        // Filter by current status
        if from.status != state.status {
            continue;
        }

        // Filter by phase (if declared; None means any phase matches)
        if from.phase.is_some_and(|p| p != state.phase) {
            continue;
        }

        // Filter by workflow path
        if !transition_applies_to_path(transition, path_name) {
            continue;
        }

        // For each gate requirement, check if a fresh receipt exists
        let mut unmet_gates = Vec::new();
        let mut unmet_requirements = Vec::new();
        let mut all_gates_met = true;

        for req in &transition.requires {
            match req {
                Requirement::Structured { kind, name }
                    if kind == "gate" =>
                {
                    let plan_hash = plan_hash_for_receipt(cycle_id, &transition.id, state);
                    let receipts = ledger
                        .list_gate_receipts_for(cycle_id, &transition.id, &plan_hash)
                        .map_err(|e| EngineError::Storage(e.into()))?;
                    if receipts.is_empty() {
                        unmet_gates.push(name.clone());
                        all_gates_met = false;
                    }
                }
                Requirement::Simple(name) => {
                    // Simple requirements are checked against state.artifacts
                    if !state.artifacts.contains_key(name) {
                        unmet_requirements.push(name.clone());
                    }
                }
                Requirement::Structured { kind, name } => {
                    if kind == "artifact" {
                        if !state.artifacts.contains_key(name) {
                            unmet_requirements.push(name.clone());
                        }
                    }
                    // Other structured requirements are treated as requirements
                    // (not gates) — they must be satisfied externally
                }
            }
        }

        entries.push(FrontierEntry {
            transition_id: transition.id.clone(),
            from: from.clone(),
            to: transition.to.clone(),
            requires_met: all_gates_met && unmet_requirements.is_empty(),
            unmet_gates,
            unmet_requirements,
        });
    }

    Ok(entries)
}

/// Computes the plan hash for gate receipt queries.
///
/// Matches the deterministic hash used when the receipt was originally
/// created in `Engine::evaluate_gate`.
fn plan_hash_for_receipt(cycle_id: &str, transition_id: &str, state: &CycleManifest) -> String {
    let material = serde_json::json!({
        "cycle_id": cycle_id,
        "transition_id": transition_id,
        "state_before": state,
    });
    let digest = Sha256::digest(material.to_string().as_bytes());
    format!("sha256:{digest:x}")
}

impl sddk_domain::SddkErrorCode for EngineError {
    fn code(&self) -> &'static str {
        match self {
            Self::UndeclaredTransition { .. } => "ENGINE_UNDECLARED_TRANSITION",
            Self::CreationTransitionRequiresStartApi { .. } => {
                "ENGINE_CREATION_TRANSITION_REQUIRES_START_API"
            }
            Self::SourceStateMismatch { .. } => "ENGINE_SOURCE_STATE_MISMATCH",
            Self::MissingRequirement { .. } => "ENGINE_MISSING_REQUIREMENT",
            Self::MissingArtifact { .. } => "ENGINE_MISSING_ARTIFACT",
            Self::MissingGateReceipt { .. } => "ENGINE_MISSING_GATE_RECEIPT",
            Self::UnknownGateReceipt { .. } => "ENGINE_UNKNOWN_GATE_RECEIPT",
            Self::GateReceiptMismatch { .. } => "ENGINE_GATE_RECEIPT_MISMATCH",
            Self::StaleGateReceipt { .. } => "ENGINE_STALE_GATE_RECEIPT",
            Self::UnregisteredEvaluator { .. } => "ENGINE_UNREGISTERED_EVALUATOR",
            Self::GateReceiptScopeMismatch { .. } => "ENGINE_GATE_RECEIPT_SCOPE_MISMATCH",
            Self::GateFailedWithoutTarget { .. } => "ENGINE_GATE_FAILED_WITHOUT_TARGET",
            Self::UndeclaredProducedArtifact { .. } => "ENGINE_UNDECLARED_PRODUCED_ARTIFACT",
            Self::ArtifactKindMismatch { .. } => "ENGINE_ARTIFACT_KIND_MISMATCH",
            Self::UnknownPath { .. } => "ENGINE_UNKNOWN_PATH",
            Self::TransitionPathMismatch { .. } => "ENGINE_TRANSITION_PATH_MISMATCH",
            Self::StalePlan { .. } => "ENGINE_STALE_PLAN",
            Self::InvalidPlan => "ENGINE_INVALID_PLAN",
            Self::StateSerialization(..) => "ENGINE_STATE_SERIALIZATION",
            Self::MissingReplayState { .. } => "ENGINE_MISSING_REPLAY_STATE",
            Self::MissingStateAfter { .. } => "ENGINE_MISSING_STATE_AFTER",
            Self::NonObjectStateAfter { .. } => "ENGINE_NON_OBJECT_STATE_AFTER",
            Self::CorruptStateAfter { .. } => "ENGINE_CORRUPT_STATE_AFTER",
            Self::SnapshotMismatch { .. } => "ENGINE_SNAPSHOT_MISMATCH",
            Self::InvalidPassEvidence { .. } => "ENGINE_INVALID_PASS_EVIDENCE",
            Self::GoalInputUnreadable => "ENGINE_GOAL_INPUT_UNREADABLE",
            Self::SupersedeRequiresExactlyOne => "ENGINE_SUPERSEDE_REQUIRES_EXACTLY_ONE",
            Self::SupersedeSelfForbidden => "ENGINE_SUPERSEDE_SELF_FORBIDDEN",
            Self::SupersedeEvidenceRefsRequired => "ENGINE_SUPERSEDE_EVIDENCE_REFS_REQUIRED",
            Self::SupersedeSuccessorNotFound(..) => "ENGINE_SUPERSEDE_SUCCESSOR_NOT_FOUND",
            Self::ReplanLimitExceeded => "ENGINE_REPLAN_LIMIT_EXCEEDED",
            Self::ReplanEmptyDelta => "ENGINE_REPLAN_EMPTY_DELTA",
            Self::Storage(..) => "ENGINE_STORAGE",
        }
    }

    fn recovery(&self) -> String {
        match self {
            Self::UndeclaredTransition { .. } => {
                "use a transition declared in the workflow manifest".into()
            }
            Self::CreationTransitionRequiresStartApi { .. } => {
                "use the cycle-start API for creation transitions".into()
            }
            Self::SourceStateMismatch { .. } => {
                "check the current cycle state and retry with the matching transition".into()
            }
            Self::MissingRequirement { .. } => {
                "satisfy the declared precondition before retrying".into()
            }
            Self::MissingArtifact { .. } => "provide the required artifact in the evidence".into(),
            Self::MissingGateReceipt { .. } => {
                "evaluate the gate with `cycle evaluate-gate` first".into()
            }
            Self::UnknownGateReceipt { .. } => "reference an existing gate receipt".into(),
            Self::GateReceiptMismatch { .. } => {
                "use a receipt for the same gate and transition".into()
            }
            Self::StaleGateReceipt { .. } => {
                "re-evaluate the gate against the current cycle state".into()
            }
            Self::UnregisteredEvaluator { .. } => "register the evaluator for the gate".into(),
            Self::GateReceiptScopeMismatch { .. } => "use a receipt from the same cycle".into(),
            Self::GateFailedWithoutTarget { .. } => {
                "declare an on_failure target for the gate".into()
            }
            Self::UndeclaredProducedArtifact { .. } => {
                "only offer artifacts the transition produces".into()
            }
            Self::ArtifactKindMismatch { .. } => "match the artifact key to its kind".into(),
            Self::UnknownPath { .. } => "use a workflow path declared in the manifest".into(),
            Self::TransitionPathMismatch { .. } => {
                "use a transition allowed for the cycle path".into()
            }
            Self::StalePlan { .. } => "re-plan against the current cycle snapshot".into(),
            Self::InvalidPlan => "recompute the plan deterministically".into(),
            Self::StateSerialization(..) => "fix the workflow state JSON".into(),
            Self::MissingReplayState { .. } => "create the cycle before replaying".into(),
            Self::MissingStateAfter { .. } => "restore the ledger or rebuild the cycle".into(),
            Self::NonObjectStateAfter { .. } => "repair the corrupted ledger event".into(),
            Self::CorruptStateAfter { .. } => "repair the corrupted ledger event".into(),
            Self::SnapshotMismatch { .. } => "rebuild the snapshot from ledger events".into(),
            Self::InvalidPassEvidence { .. } => {
                "provide argv, exit_code, and output_digest in pass evidence".into()
            }
            Self::GoalInputUnreadable => "verify goal input is readable before retrying".into(),
            Self::SupersedeRequiresExactlyOne => {
                "supply exactly one of --successor or --reason".into()
            }
            Self::SupersedeSelfForbidden => "do not supersede a cycle with itself".into(),
            Self::SupersedeEvidenceRefsRequired => {
                "supply at least one evidence reference with --evidence-ref".into()
            }
            Self::SupersedeSuccessorNotFound(succ) => {
                format!("create successor cycle {} first", succ)
            }
            Self::ReplanLimitExceeded => "replan counter is at its limit of 5".into(),
            Self::ReplanEmptyDelta => "supply a non-empty replan delta".into(),
            Self::Storage(..) => "resolve the underlying storage error first".into(),
        }
    }
}
