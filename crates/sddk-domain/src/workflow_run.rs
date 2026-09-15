//! Workflow run types — runtime state machines for workflow executions.

use std::collections::BTreeMap;
use std::collections::BTreeSet;

use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use thiserror::Error;

// Re-export IR types needed by run types
pub use super::workflow_ir::{
    Budgets, CapabilityId, ContentHash, ExpansionPermission, NodeId, OperatorId, RevisionId, RunId,
    SCHEMA_VERSION,
};

/// Schema version constant for run types.
pub const RUN_SCHEMA_VERSION: u32 = 1;

// ── Newtypes ─────────────────────────────────────────────────────────────────

/// Attempt identifier (UUID v7).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct AttemptId(pub String);

/// Correlation identifier for cross-system tracing.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct CorrelationId(pub String);

// ── Route ─────────────────────────────────────────────────────────────────

/// Route resolved at dispatch time.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Route {
    /// LLM provider name.
    pub provider: String,
    /// Model identifier.
    pub model: String,
    /// Host endpoint.
    pub host: String,
}

// ── Usage ─────────────────────────────────────────────────────────────────

/// Usage metrics for an attempt.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Usage {
    /// Input tokens consumed.
    pub tokens_in: u64,
    /// Output tokens produced.
    pub tokens_out: u64,
    /// Cost in microdollars.
    pub cost_micros: u64,
    /// Wall-clock time in milliseconds.
    pub wall_ms: u64,
}

// ── ContextCapsuleRef ────────────────────────────────────────────────────

/// Reference to a context capsule.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum ContextCapsuleRef {
    /// Pointer variant — CID reference.
    Pointer {
        /// Context identifier.
        cid: String,
    },
    /// Inline variant — bounded content.
    Inline {
        /// Content summary.
        summary: String,
        /// SHA-256 content hash (bare 64 hex chars, no prefix).
        sha256: String,
    },
}

/// Errors from [`ContextCapsuleRef::validate`].
#[derive(Debug, Clone, PartialEq, Eq, Error)]
pub enum CapsuleError {
    /// Inline capsule exceeds the maximum size limit.
    #[error("inline capsule exceeds 4096 bytes: got {got}")]
    InlineTooLarge {
        /// Actual byte count.
        got: usize,
    },
    /// SHA-256 digest mismatch between declared and recomputed.
    #[error("inline capsule sha256 mismatch: expected {expected}, got {got}")]
    Sha256Mismatch {
        /// Expected digest (recomputed).
        expected: String,
        /// Actual digest found.
        got: String,
    },
    /// SHA-256 string is malformed (not 64 lowercase hex chars).
    #[error("inline capsule sha256 malformed: {0}")]
    Sha256Malformed(String),
}

impl ContextCapsuleRef {
    /// Maximum inline context capsule size in bytes.
    pub const INLINE_CAPSULE_MAX_BYTES: usize = crate::workflow_ir::INLINE_CAPSULE_MAX_BYTES;

    /// Validates this capsule reference.
    ///
    /// - `Pointer` variants always pass (CID resolution is a runtime concern).
    /// - `Inline` variants are validated for:
    ///   1. SHA-256 format (64 lowercase hex chars)
    ///   2. Size bound (≤ 4096 bytes)
    ///   3. Digest integrity (recomputed sha256 matches declared)
    pub fn validate(&self) -> Result<(), CapsuleError> {
        match self {
            ContextCapsuleRef::Pointer { .. } => Ok(()),
            ContextCapsuleRef::Inline { summary, sha256 } => {
                // 1. Check sha256 format: 64 lowercase hex chars, no prefix
                if sha256.len() != 64
                    || !sha256
                        .chars()
                        .all(|c| c.is_ascii_hexdigit() && !c.is_ascii_uppercase())
                {
                    return Err(CapsuleError::Sha256Malformed(sha256.clone()));
                }

                // 2. Check size bound
                let summary_bytes = summary.len();
                if summary_bytes > Self::INLINE_CAPSULE_MAX_BYTES {
                    return Err(CapsuleError::InlineTooLarge { got: summary_bytes });
                }

                // 3. Verify sha256 integrity
                let computed = format!("{:064x}", Sha256::digest(summary.as_bytes()));
                if computed != *sha256 {
                    return Err(CapsuleError::Sha256Mismatch {
                        expected: computed,
                        got: sha256.clone(),
                    });
                }

                Ok(())
            }
        }
    }
}

// ── IdempotencyKey ────────────────────────────────────────────────────────

/// Idempotency key for an attempt.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct IdempotencyKey {
    /// Project identifier.
    pub project_id: String,
    /// Run identifier.
    pub run_id: RunId,
    /// Node identifier.
    pub node_id: NodeId,
    /// Attempt sequence number.
    pub attempt_seq: u32,
}

impl IdempotencyKey {
    /// Returns the string representation of this key.
    pub fn as_str(&self) -> String {
        format!(
            "{}:{}:{}:{}",
            self.project_id, self.run_id.0, self.node_id.0, self.attempt_seq
        )
    }
}

impl std::fmt::Display for IdempotencyKey {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.as_str())
    }
}

// ── AttemptOutcome ─────────────────────────────────────────────────────────

/// Outcome of an attempt.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum AttemptOutcome {
    /// Attempt succeeded with outputs.
    Succeeded {
        /// Output values from the attempt.
        outputs: BTreeMap<String, serde_json::Value>,
    },
    /// Attempt failed with error.
    Failed {
        /// Error message.
        error: String,
    },
    /// Attempt timed out.
    Timeout,
    /// Attempt was cancelled.
    Cancelled,
    /// Attempt returned Pending — child is in-flight (cycle-20 multi-tick).
    /// The `resume_token` carries the checkpoint for cross-tick resumption.
    Pending {
        /// Resume token from the pending checkpoint.
        resume_token: u64,
        /// Sequence number of this attempt.
        attempt_seq: u32,
    },
}

// ── Attempt ────────────────────────────────────────────────────────────────

/// One physical execution of a node.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Attempt {
    /// Attempt identifier (UUID v7).
    pub attempt_id: AttemptId,
    /// Node this attempt belongs to.
    pub node_id: NodeId,
    /// Resolved route at dispatch.
    pub route: Route,
    /// When the attempt started (RFC 3339 string).
    pub started_at: String,
    /// When the attempt ended (None while in-flight).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub ended_at: Option<String>,
    /// Outcome (None while in-flight).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub outcome: Option<AttemptOutcome>,
    /// Usage metrics.
    pub usage: Usage,
    /// Context capsule reference.
    pub context_capsule: ContextCapsuleRef,
    /// Idempotency key for this attempt.
    pub idempotency_key: IdempotencyKey,
    /// Schema version.
    pub schema_version: u32,
}

impl Attempt {
    /// Returns true if this attempt is in-flight (no outcome set).
    pub fn is_in_flight(&self) -> bool {
        self.outcome.is_none()
    }

    /// Marks this attempt as complete with the given outcome.
    ///
    /// Returns an error if already terminal.
    pub fn complete(
        &mut self,
        outcome: AttemptOutcome,
        ended_at: String,
    ) -> Result<(), AttemptError> {
        if self.outcome.is_some() {
            return Err(AttemptError::AlreadyTerminal);
        }
        self.outcome = Some(outcome);
        self.ended_at = Some(ended_at);
        Ok(())
    }
}

/// Errors for Attempt operations.
#[derive(Debug, Clone, PartialEq, Eq)]
/// Errors for attempt lifecycle transitions.
///
/// Audit (cycle 3, kernel-cycle-3-carries-over): trimmed 3 unused variants
/// (`StillInFlight`, `IdempotencyCollision`, `CapsuleMissing`). All remaining
/// variants are emitted by `Attempt::complete()`. Adding a variant requires
/// updating the lifecycle code and the cycle-3 audit results at
/// `docs/audit/error-variants.md`.
pub enum AttemptError {
    /// Attempt is already terminal and cannot be mutated.
    AlreadyTerminal,
}

// Compile-time guard: 1 variant (post-cycle-3 trim).
crate::assert_variant_count_eq!(AttemptError, 1, [AttemptError::AlreadyTerminal,]);

// ── NodeRunState ─────────────────────────────────────────────────────────

/// State of a node run.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum NodeRunState {
    /// Waiting for dependencies.
    Pending,
    /// Dependencies satisfied, ready to schedule.
    Ready,
    /// Currently executing.
    Running,
    /// Completed successfully.
    Completed,
    /// Failed (may retry).
    Failed,
    /// Skipped (dependency was skipped).
    Skipped,
}

// ── NodeRun ───────────────────────────────────────────────────────────────

/// Per-node state machine with attempt history.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct NodeRun {
    /// Node identifier.
    pub node_id: NodeId,
    /// Current state.
    pub state: NodeRunState,
    /// Upstream dependencies.
    pub dependencies: BTreeSet<NodeId>,
    /// Attempt history (append-only).
    pub attempts: Vec<Attempt>,
    /// Expansion permissions inherited from IR.
    pub expansion_permissions: BTreeSet<ExpansionPermission>,
    /// Schema version.
    pub schema_version: u32,
}

impl NodeRun {
    /// Returns true if this node can transition to Ready.
    pub fn can_ready(&self) -> bool {
        self.state == NodeRunState::Pending
    }

    /// Transitions to Ready (deps satisfied).
    pub fn to_ready(&mut self) -> Result<(), NodeRunError> {
        if self.state != NodeRunState::Pending {
            return Err(NodeRunError::InvalidStateTransition);
        }
        self.state = NodeRunState::Ready;
        Ok(())
    }

    /// Returns true if this node is in a terminal state.
    pub fn is_terminal(&self) -> bool {
        matches!(
            self.state,
            NodeRunState::Completed | NodeRunState::Failed | NodeRunState::Skipped
        )
    }
}

/// Errors for NodeRun operations.
#[derive(Debug, Clone, PartialEq, Eq)]
/// Errors for node-run state transitions.
///
/// Audit (cycle 3, kernel-cycle-3-carries-over): trimmed 4 unused variants
/// (`DepsUnsatisfied`, `MaxRetriesExceeded`, `AlreadyRunning`, `CascadeRequired`).
/// All remaining variants are emitted by `NodeRunState` transition logic.
/// Adding a variant requires updating the state machine and the cycle-3 audit
/// results at `docs/audit/error-variants.md`.
pub enum NodeRunError {
    /// Invalid state transition.
    InvalidStateTransition,
}

// Compile-time guard: 1 variant (post-cycle-3 trim).
crate::assert_variant_count_eq!(NodeRunError, 1, [NodeRunError::InvalidStateTransition,]);

// ── WorkflowRunState ─────────────────────────────────────────────────────

/// State of a workflow run.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum WorkflowRunState {
    /// Waiting to start.
    Pending,
    /// Currently executing.
    Running,
    /// Temporarily paused (may resume).
    Paused,
    /// Completed successfully.
    Completed,
    /// Failed with error.
    Failed,
    /// Cancelled.
    Cancelled,
}

impl WorkflowRunState {
    /// Returns true if this is a terminal state.
    pub fn is_terminal(&self) -> bool {
        matches!(
            self,
            WorkflowRunState::Completed | WorkflowRunState::Failed | WorkflowRunState::Cancelled
        )
    }
}

// ── WorkflowRun ────────────────────────────────────────────────────────────

/// Runtime instance of a compiled workflow IR.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct WorkflowRun {
    /// Unique run identifier (UUID v7, also stream_id for events).
    pub run_id: RunId,
    /// Template this run was instantiated from.
    pub template_ref: super::workflow_ir::TemplateRef,
    /// IR content hash at compilation time.
    pub ir_hash: ContentHash,
    /// Current graph revision.
    pub graph_revision: RevisionId,
    /// Current state.
    pub state: WorkflowRunState,
    /// Input parameters.
    pub inputs: BTreeMap<String, serde_json::Value>,
    /// Output results (populated on terminal state).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub outputs: Option<BTreeMap<String, serde_json::Value>>,
    /// Cross-system correlation identifier.
    pub correlation_id: CorrelationId,
    /// Remaining execution budget.
    pub budget: Budgets,
    /// Schema version.
    pub schema_version: u32,
}

impl WorkflowRun {
    /// Starts a pending run, transitioning it to Running.
    pub fn start(&mut self) -> Result<(), WorkflowRunError> {
        if self.state != WorkflowRunState::Pending {
            return Err(WorkflowRunError::InvalidTransition {
                from: format!("{:?}", self.state),
                to: "Running".into(),
            });
        }
        self.state = WorkflowRunState::Running;
        Ok(())
    }

    /// Pauses a running run.
    pub fn pause(&mut self) -> Result<(), WorkflowRunError> {
        if self.state != WorkflowRunState::Running {
            return Err(WorkflowRunError::InvalidTransition {
                from: format!("{:?}", self.state),
                to: "Paused".into(),
            });
        }
        self.state = WorkflowRunState::Paused;
        Ok(())
    }

    /// Resumes a paused run.
    pub fn resume(&mut self) -> Result<(), WorkflowRunError> {
        if self.state != WorkflowRunState::Paused {
            return Err(WorkflowRunError::InvalidTransition {
                from: format!("{:?}", self.state),
                to: "Running".into(),
            });
        }
        self.state = WorkflowRunState::Running;
        Ok(())
    }

    /// Completes a running run.
    pub fn complete(
        &mut self,
        outputs: BTreeMap<String, serde_json::Value>,
    ) -> Result<(), WorkflowRunError> {
        if self.state.is_terminal() {
            return Err(WorkflowRunError::AlreadyTerminal);
        }
        self.state = WorkflowRunState::Completed;
        self.outputs = Some(outputs);
        Ok(())
    }

    /// Cancels a run.
    pub fn cancel(&mut self) -> Result<(), WorkflowRunError> {
        if self.state.is_terminal() {
            return Err(WorkflowRunError::AlreadyTerminal);
        }
        self.state = WorkflowRunState::Cancelled;
        Ok(())
    }

    /// Marks a run as failed.
    pub fn fail(&mut self, error: String) -> Result<(), WorkflowRunError> {
        if self.state.is_terminal() {
            return Err(WorkflowRunError::AlreadyTerminal);
        }
        self.state = WorkflowRunState::Failed;
        let mut outputs = BTreeMap::new();
        outputs.insert("error".into(), serde_json::Value::String(error));
        self.outputs = Some(outputs);
        Ok(())
    }
}

/// Errors for WorkflowRun operations.
#[derive(Debug, Clone, PartialEq, Eq)]
/// Errors for workflow-run state transitions.
///
/// Audit (cycle 3, kernel-cycle-3-carries-over): trimmed 2 unused variants
/// (`BudgetExhausted`, `IrHashMismatch`). All remaining variants are emitted by
/// `WorkflowRun::transition()`. Adding a variant requires updating the state
/// machine and the cycle-3 audit results at `docs/audit/error-variants.md`.
pub enum WorkflowRunError {
    /// Invalid state transition attempted.
    InvalidTransition {
        /// Source state.
        from: String,
        /// Target state.
        to: String,
    },
    /// Run is already in a terminal state.
    AlreadyTerminal,
}

// Compile-time guard: 2 variants (post-cycle-3 trim).
crate::assert_variant_count_eq!(
    WorkflowRunError,
    2,
    [
        WorkflowRunError::InvalidTransition { .. },
        WorkflowRunError::AlreadyTerminal,
    ]
);

// ── WorkflowRunPersistError ─────────────────────────────────────────────────

/// Errors from workflow-run persistence operations (REQ-WFR-ERR-001).
///
/// This is a closed 6-variant enum. The `assert_variant_count_eq!` macro
/// enforces the variant count at compile time.
#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
pub enum WorkflowRunPersistError {
    /// Schema version mismatch.
    #[error("unsupported schema version: got {got}, want {want}")]
    UnsupportedSchemaVersion {
        /// Actual schema version.
        got: u32,
        /// Expected schema version.
        want: u32,
    },
    /// Invalid state transition attempted.
    #[error("invalid state transition from {from} to {to}")]
    InvalidStateTransition {
        /// Source state.
        from: String,
        /// Target state.
        to: String,
    },
    /// Run is already in a terminal state.
    #[error("run already terminal: {state:?}")]
    RunAlreadyTerminal {
        /// The terminal state.
        state: WorkflowRunState,
    },
    /// Run not found.
    #[error("run not found: {run_id}")]
    RunNotFound {
        /// The missing run identifier.
        run_id: RunId,
    },
    /// Idempotency key conflict.
    #[error("idempotency conflict for attempt: {key}")]
    IdempotencyConflict {
        /// The conflicting idempotency key.
        key: IdempotencyKey,
    },
    /// Graph revision missing for the run.
    #[error("graph revision missing for run {run_id}, expected: {expected}")]
    GraphRevisionMissing {
        /// The run identifier.
        run_id: RunId,
        /// The expected revision identifier.
        expected: RevisionId,
    },
}

// Compile-time guard: exactly 6 variants (REQ-WFR-ERR-001).
crate::assert_variant_count_eq!(
    WorkflowRunPersistError,
    6,
    [
        WorkflowRunPersistError::UnsupportedSchemaVersion { .. },
        WorkflowRunPersistError::InvalidStateTransition { .. },
        WorkflowRunPersistError::RunAlreadyTerminal { .. },
        WorkflowRunPersistError::RunNotFound { .. },
        WorkflowRunPersistError::IdempotencyConflict { .. },
        WorkflowRunPersistError::GraphRevisionMissing { .. },
    ]
);

impl From<WorkflowRunPersistError> for crate::StorageError {
    fn from(err: WorkflowRunPersistError) -> Self {
        match err {
            WorkflowRunPersistError::RunNotFound { run_id } => crate::StorageError::NotFound {
                entity: "workflow_run",
                id: run_id.0,
            },
            WorkflowRunPersistError::GraphRevisionMissing { run_id, .. } => {
                crate::StorageError::NotFound {
                    entity: "graph_revision",
                    id: run_id.0,
                }
            }
            // Database-level errors map to StorageError::Database
            WorkflowRunPersistError::UnsupportedSchemaVersion { .. } => {
                crate::StorageError::Database(err.to_string())
            }
            WorkflowRunPersistError::InvalidStateTransition { .. } => {
                crate::StorageError::Database(err.to_string())
            }
            WorkflowRunPersistError::RunAlreadyTerminal { .. } => {
                crate::StorageError::Database(err.to_string())
            }
            WorkflowRunPersistError::IdempotencyConflict { key } => {
                crate::StorageError::IdempotencyConflict { key }
            }
        }
    }
}

// ─────────────────────────────────────────────────────────────────────────────
// AdvisoryContext + ContextCapsule (A3 closeout: the agent advisory boundary)
// ─────────────────────────────────────────────────────────────────────────────

/// The kind of an advisory item.
///
/// Closed on purpose: an advisory payload must not become a free-text channel,
/// because a free-text channel cannot be audited for "did it act as authority?".
/// `AlignmentTension` is **reserved**: A4 delivers the evaluator; A3 closeout
/// establishes only the delivery boundary.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum AdvisoryKind {
    /// A paradigm-lens observation (AC7). Advisory by construction.
    ParadigmObservation,
    /// A software-alignment tension. Reserved for A4; no producer exists yet.
    AlignmentTension,
    /// A knowledge freshness / status warning.
    KnowledgeStatus,
}

impl AdvisoryKind {
    /// Every variant, in canonical order.
    pub const ALL: [AdvisoryKind; 3] = [
        AdvisoryKind::ParadigmObservation,
        AdvisoryKind::AlignmentTension,
        AdvisoryKind::KnowledgeStatus,
    ];

    /// Stable short tag used in canonical hashing.
    pub fn canonical_tag(self) -> &'static str {
        match self {
            AdvisoryKind::ParadigmObservation => "paradigm_observation",
            AdvisoryKind::AlignmentTension => "alignment_tension",
            AdvisoryKind::KnowledgeStatus => "knowledge_status",
        }
    }
}

/// How strongly an advisory statement is held.
///
/// Mirrors the provenance classes the rest of the system distinguishes, so a
/// reader never has to guess whether a hint was observed or inferred.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum AdvisoryProvenance {
    /// Derived from evidence the system collected.
    Observed,
    /// Stated by a declaration.
    Declared,
    /// Derived by a documented rule from other knowledge.
    Inferred,
}

impl AdvisoryProvenance {
    /// Stable short tag used in canonical hashing.
    pub fn canonical_tag(self) -> &'static str {
        match self {
            AdvisoryProvenance::Observed => "observed",
            AdvisoryProvenance::Declared => "declared",
            AdvisoryProvenance::Inferred => "inferred",
        }
    }
}

/// One advisory statement.
///
/// `note` is **informational only and never authority**, exactly as the finding
/// `message` and the claim `note` are elsewhere in the system.
#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct AdvisoryItem {
    /// Closed kind tag.
    pub kind: AdvisoryKind,
    /// The subject the advisory concerns (a unit id, contract id, ...).
    pub subject: String,
    /// Informational text. NEVER authority.
    pub note: String,
    /// Provenance class.
    pub provenance: AdvisoryProvenance,
}

/// A typed advisory payload delivered **alongside** compiled instructions.
///
/// # What this is not
///
/// Advisory content **cannot** become an `InstructionSource`, **cannot** grant a
/// capability, and **cannot** reach the authority engine except as an ordinary
/// explicit fact a policy specifically asks for. That is enforced by the type
/// system rather than by convention: there is no conversion from
/// [`AdvisoryContext`] to any of those concepts, and `AdvisoryContext` is not an
/// input to them.
///
/// The A3 exit criterion this exists to satisfy:
///
/// ```text
/// same EffectiveInstructions + different advisory_context
///   ⇒ same effective_instruction_set_hash, different context_capsule_hash
/// ```
#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct AdvisoryContext {
    /// Sorted, deduplicated advisory statements.
    pub items: Vec<AdvisoryItem>,
}

impl AdvisoryContext {
    /// Domain prefix for canonical hashing.
    pub const DOMAIN: &'static str = "sddk.advisory_context.v1|";

    /// An empty advisory payload.
    pub fn none() -> Self {
        Self { items: Vec::new() }
    }

    /// True iff there is nothing to advise.
    pub fn is_empty(&self) -> bool {
        self.items.is_empty()
    }

    /// Insert an item, keeping the payload canonically ordered and deduplicated.
    ///
    /// Ordering is by `(kind, subject, note, provenance)`, so two payloads that
    /// differ only in insertion order hash identically.
    pub fn push(&mut self, item: AdvisoryItem) {
        self.items.push(item);
        self.normalize();
    }

    /// Sort + deduplicate in canonical order.
    pub fn normalize(&mut self) {
        self.items.sort_by(|a, b| {
            (a.kind, &a.subject, &a.note, a.provenance).cmp(&(
                b.kind,
                &b.subject,
                &b.note,
                b.provenance,
            ))
        });
        self.items.dedup();
    }

    /// Canonical bytes: stable across insertion order, so hashing is stable too.
    pub fn canonical_bytes(&self) -> Vec<u8> {
        let mut sorted = self.items.clone();
        sorted.sort_by(|a, b| {
            (a.kind, &a.subject, &a.note, a.provenance).cmp(&(
                b.kind,
                &b.subject,
                &b.note,
                b.provenance,
            ))
        });
        let mut out = String::from(Self::DOMAIN);
        for i in &sorted {
            out.push_str(i.kind.canonical_tag());
            out.push('|');
            out.push_str(&i.subject);
            out.push('|');
            out.push_str(&i.note);
            out.push('|');
            out.push_str(i.provenance.canonical_tag());
            out.push('\n');
        }
        out.into_bytes()
    }
}

/// The content a [`ContextCapsuleRef`] points at.
///
/// # Two identities, deliberately separate
///
/// | field | identity of |
/// |---|---|
/// | `effective_instruction_set_hash` | the **compiled instructions only** |
/// | `context_capsule_hash` | instructions **plus** the advisory payload |
///
/// Changing the advisory payload changes `context_capsule_hash` and **must not**
/// change `effective_instruction_set_hash`. That separation is the whole point:
/// advisory content is delivered to an agent, but it is not part of what the
/// instructions *are*, so it can never be mistaken for a normative directive.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ContextCapsule {
    /// Stable ref token for the compiled instruction set, e.g. `effective@v1`.
    pub effective_instructions_ref: String,
    /// Hash of the compiled instruction set alone.
    pub effective_instruction_set_hash: String,
    /// The advisory payload.
    pub advisory_context: AdvisoryContext,
    /// Hash over (ref, instruction hash, advisory payload).
    pub context_capsule_hash: String,
}

impl ContextCapsule {
    /// Domain prefix for the capsule hash.
    pub const HASH_DOMAIN: &'static str = "sddk.context_capsule.v1|";

    /// Seal a capsule, computing its hash deterministically.
    pub fn seal(
        effective_instructions_ref: impl Into<String>,
        effective_instruction_set_hash: impl Into<String>,
        advisory_context: AdvisoryContext,
    ) -> Self {
        let effective_instructions_ref = effective_instructions_ref.into();
        let effective_instruction_set_hash = effective_instruction_set_hash.into();
        let mut capsule = Self {
            effective_instructions_ref,
            effective_instruction_set_hash,
            advisory_context,
            context_capsule_hash: String::new(),
        };
        capsule.context_capsule_hash = capsule.recompute_hash();
        capsule
    }

    /// Recompute the capsule hash from the current fields.
    ///
    /// The instruction hash is an **input**, never recomputed here: the capsule
    /// does not know how to compile instructions, and recomputing them would put a
    /// second compiler in the delivery path.
    pub fn recompute_hash(&self) -> String {
        let mut h = Sha256::new();
        h.update(Self::HASH_DOMAIN.as_bytes());
        h.update(self.effective_instructions_ref.as_bytes());
        h.update(b"|");
        h.update(self.effective_instruction_set_hash.as_bytes());
        h.update(b"|");
        h.update(self.advisory_context.canonical_bytes());
        format!("{:064x}", h.finalize())
    }

    /// Re-seal after mutating the advisory payload.
    pub fn reseal(&mut self) {
        self.advisory_context.normalize();
        self.context_capsule_hash = self.recompute_hash();
    }

    /// Validate: the declared capsule hash matches the recomputed one.
    pub fn validate(&self) -> Result<(), CapsuleError> {
        if self.context_capsule_hash.len() != 64
            || !self
                .context_capsule_hash
                .chars()
                .all(|c| c.is_ascii_hexdigit() && !c.is_ascii_uppercase())
        {
            return Err(CapsuleError::Sha256Malformed(
                self.context_capsule_hash.clone(),
            ));
        }
        let computed = self.recompute_hash();
        if computed != self.context_capsule_hash {
            return Err(CapsuleError::Sha256Mismatch {
                expected: computed,
                got: self.context_capsule_hash.clone(),
            });
        }
        Ok(())
    }
}
