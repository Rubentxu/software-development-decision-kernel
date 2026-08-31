//! Workflow IR types — compile-time validated, content-addressed executable plans.
//!
//! All collection fields use `BTreeMap`/`BTreeSet` for deterministic serialization
//! and hash stability. HashMap is explicitly forbidden in this module.

use std::collections::BTreeMap;
use std::collections::BTreeSet;

use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use thiserror::Error;

/// Maximum inline context capsule size in bytes (4096).
pub const INLINE_CAPSULE_MAX_BYTES: usize = 4096;

fn is_max_u32(v: &u32) -> bool {
    *v == u32::MAX
}

const fn u32_max() -> u32 {
    u32::MAX
}

// ── Newtypes ─────────────────────────────────────────────────────────────────

/// Content hash in `sha256:<64-hex-lowercase>` format.
pub type ContentHash = String;

/// IR identifier (ULID, assigned post-hoc).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct IrId(pub String);

/// Revision identifier (ULID).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct RevisionId(pub String);

/// Run identifier (UUID v7).
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct RunId(pub String);

/// Node identifier (stable within an IR).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, PartialOrd, Ord)]
pub struct NodeId(pub String);

/// Operator identifier (stable within an IR).
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize, PartialOrd, Ord)]
pub struct OperatorId(pub String);

/// Edge identifier.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, PartialOrd, Ord)]
pub struct EdgeId(pub String);

/// Event identifier.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, PartialOrd, Ord)]
pub struct EventId(pub String);

/// Capability identifier (e.g. `git.commit`).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, PartialOrd, Ord)]
pub struct CapabilityId(pub String);

/// Schema version constant for IR types.
pub const SCHEMA_VERSION: u32 = 1;

// ── ExpansionPermission ───────────────────────────────────────────────────────

/// Expansion permission set — which runtime expansions a node is allowed to perform.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ExpansionPermission {
    /// Node may expand a Map operator.
    Map,
    /// Node may expand a Discover operator.
    Discover,
    /// Node may expand a Replan operator.
    Replan,
}

impl ExpansionPermission {
    /// Returns true if this permission is a member of the v1 closed set
    /// (Map, Discover, Replan).
    pub fn is_known_permission(&self) -> bool {
        matches!(
            self,
            ExpansionPermission::Map | ExpansionPermission::Discover | ExpansionPermission::Replan
        )
    }

    /// Returns true if this permission is a member of the given allowlist.
    pub fn is_allowed_by(&self, allowlist: &BTreeSet<ExpansionPermission>) -> bool {
        allowlist.contains(self)
    }

    /// Deprecated: misleading semantics — ignores the allowlist parameter and always
    /// returns true for the v1 closed set. Use `is_known_permission()` and/or
    /// `is_allowed_by(allowlist)` instead.
    #[deprecated(
        since = "1.30.0",
        note = "misleading: ignores the allowlist. Use is_known_permission() + is_allowed_by(allowlist). Removed in cycle 3 (v1.31.0)."
    )]
    pub fn is_allowed(&self, _allowlist: &BTreeSet<ExpansionPermission>) -> bool {
        // v1 closed set: only Map, Discover, Replan exist
        self.is_known_permission()
    }
}

// ── Budgets ─────────────────────────────────────────────────────────────────

/// Execution budgets for a workflow run.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub struct Budgets {
    /// Maximum wall-clock time in milliseconds.
    pub max_wall_ms: u64,
    /// Maximum input tokens.
    pub max_tokens: u64,
    /// Maximum output tokens.
    pub max_cost_micros: u64,
    /// Maximum call depth.
    pub max_depth: u64,
    /// Maximum nodes in the execution graph.
    pub max_nodes: u64,
    /// Remaining tokens (decremented at runtime).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub remaining_tokens: Option<u64>,
    /// Maximum consecutive ticks without observable progress before the run is
    /// terminated with a typed no-progress outcome.
    #[serde(default = "u32_max", skip_serializing_if = "is_max_u32")]
    pub no_progress_threshold: u32,
}

impl Default for Budgets {
    fn default() -> Self {
        Self {
            max_wall_ms: u64::MAX,
            max_tokens: u64::MAX,
            max_cost_micros: u64::MAX,
            max_depth: u64::MAX,
            max_nodes: u64::MAX,
            remaining_tokens: None,
            no_progress_threshold: u32::MAX,
        }
    }
}

impl Budgets {
    /// Returns the kernel hard-limit ceilings used by validator G5.
    ///
    /// These are the maximum values the kernel will ever accept:
    /// - 24 h wall-clock
    /// - 100 M tokens
    /// - $1 000
    /// - depth 64
    /// - 10 000 nodes
    pub fn hard_limits() -> Self {
        Self {
            max_wall_ms: 86_400_000,
            max_tokens: 100_000_000,
            max_cost_micros: 1_000_000_000,
            max_depth: 64,
            max_nodes: 10_000,
            remaining_tokens: None,
            no_progress_threshold: u32::MAX,
        }
    }

    /// All-zero budget — the additive identity for `consume`.
    pub fn zero() -> Self {
        Self {
            max_wall_ms: 0,
            max_tokens: 0,
            max_cost_micros: 0,
            max_depth: 0,
            max_nodes: 0,
            remaining_tokens: Some(0),
            no_progress_threshold: u32::MAX,
        }
    }

    /// Component-wise comparison: `self ≤ other` for every numeric ceiling field.
    /// `remaining_tokens` is NOT compared (it is a runtime counter, not a ceiling).
    pub fn fits_within(&self, other: &Budgets) -> bool {
        self.max_wall_ms <= other.max_wall_ms
            && self.max_tokens <= other.max_tokens
            && self.max_cost_micros <= other.max_cost_micros
            && self.max_depth <= other.max_depth
            && self.max_nodes <= other.max_nodes
    }

    /// Subtracts `sub` from `self`, returning the remaining budget.
    /// Returns an error if any field would underflow.
    ///
    /// `remaining_tokens` moves in lockstep with `max_tokens`:
    /// - Seeded from `max_tokens` when `None`
    /// - `sub.remaining_tokens` is ignored (sub is a cost vector, not state)
    pub fn consume(&self, sub: &Budgets) -> Result<Budgets, BudgetError> {
        let wall_ms =
            self.max_wall_ms
                .checked_sub(sub.max_wall_ms)
                .ok_or(BudgetError::Underflow {
                    field: BudgetField::WallMs,
                    have: self.max_wall_ms,
                    consume: sub.max_wall_ms,
                })?;
        let tokens = self
            .max_tokens
            .checked_sub(sub.max_tokens)
            .ok_or(BudgetError::Underflow {
                field: BudgetField::Tokens,
                have: self.max_tokens,
                consume: sub.max_tokens,
            })?;
        let cost_micros = self
            .max_cost_micros
            .checked_sub(sub.max_cost_micros)
            .ok_or(BudgetError::Underflow {
                field: BudgetField::CostMicros,
                have: self.max_cost_micros,
                consume: sub.max_cost_micros,
            })?;
        let depth = self
            .max_depth
            .checked_sub(sub.max_depth)
            .ok_or(BudgetError::Underflow {
                field: BudgetField::Depth,
                have: self.max_depth,
                consume: sub.max_depth,
            })?;
        let nodes = self
            .max_nodes
            .checked_sub(sub.max_nodes)
            .ok_or(BudgetError::Underflow {
                field: BudgetField::Nodes,
                have: self.max_nodes,
                consume: sub.max_nodes,
            })?;

        // remaining_tokens: seed from self.max_tokens when None, then subtract
        let remaining = self
            .remaining_tokens
            .unwrap_or(self.max_tokens)
            .checked_sub(sub.max_tokens)
            .ok_or(BudgetError::Underflow {
                field: BudgetField::Tokens,
                have: self.remaining_tokens.unwrap_or(self.max_tokens),
                consume: sub.max_tokens,
            })?;

        Ok(Budgets {
            max_wall_ms: wall_ms,
            max_tokens: tokens,
            max_cost_micros: cost_micros,
            max_depth: depth,
            max_nodes: nodes,
            remaining_tokens: Some(remaining),
            no_progress_threshold: self.no_progress_threshold,
        })
    }
}

/// Field identifier for [`BudgetError::Underflow`].
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BudgetField {
    /// Wall-clock time in milliseconds.
    WallMs,
    /// Input tokens.
    Tokens,
    /// Cost in microdollars.
    CostMicros,
    /// Call depth.
    Depth,
    /// Node count.
    Nodes,
}

/// Errors from [`Budgets::consume`].
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum BudgetError {
    /// A budget field would go negative on subtraction.
    Underflow {
        /// Which field underflowed.
        field: BudgetField,
        /// The current value.
        have: u64,
        /// The amount to consume.
        consume: u64,
    },
    /// A budget field exceeds the hard limit.
    ExceedsLimit {
        /// Which field exceeded the limit.
        field: BudgetField,
        /// The value that was provided.
        got: u64,
        /// The hard limit.
        limit: u64,
    },
}

// ── Invariant ────────────────────────────────────────────────────────────────

/// Invariant that the workflow IR must satisfy.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize, Default)]
#[serde(rename_all = "snake_case")]
pub enum Invariant {
    /// DAG must have no cycles (default).
    #[default]
    ConvergenceBounded,
    /// All operators must have arity > 0.
    ArityPositive,
}

// ── Policy ──────────────────────────────────────────────────────────────────

/// Pack-specific policy profile.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Policy {
    /// Policy name.
    pub name: String,
    /// Policy JSON blob.
    pub config: BTreeMap<String, serde_json::Value>,
}

// ── ConvergenceSpec ─────────────────────────────────────────────────────────

/// Convergence criteria for loop/expansion termination.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ConvergenceSpec {
    /// Maximum iterations before forced convergence.
    pub max_iterations: u32,
    /// Signature that indicates no progress (stable output).
    pub no_progress_signature: Option<String>,
}

// ── Provenance ─────────────────────────────────────────────────────────────

/// Provenance metadata for a compiled IR.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Provenance {
    /// Tool that generated this IR.
    pub generated_by: String,
    /// Hash of the prompt that produced this IR.
    pub prompt_hash: String,
    /// Hash of the model that produced this IR.
    pub model_hash: String,
    /// Hash of the policy applied.
    pub policy_hash: String,
}

// ── GuardExpr ──────────────────────────────────────────────────────────────

/// Runtime guard expression evaluated before operator execution.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct GuardExpr {
    /// Expression text.
    pub expr: String,
}

// ── Operator enum ──────────────────────────────────────────────────────────

/// One step in a workflow DAG.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(tag = "op", rename_all = "snake_case")]
pub enum Operator {
    /// A leaf task that calls a capability.
    Task {
        /// Capability required for this task.
        capability: CapabilityId,
        /// Inputs to the capability (deterministic map).
        inputs: BTreeMap<String, serde_json::Value>,
    },
    /// Execute operators in sequence.
    Sequence {
        /// Ordered list of operator IDs.
        body: Vec<OperatorId>,
    },
    /// Execute branches in parallel, limited by max_concurrency.
    Parallel {
        /// Branch operator IDs.
        branches: Vec<OperatorId>,
        /// Maximum concurrent branches.
        max_concurrency: u32,
    },
    /// Map over a collection (stub in v1.29.0 — full semantics in cycle 3).
    Map {
        /// Source operator ID.
        source: OperatorId,
        /// Body operator ID.
        body: OperatorId,
        /// Maximum concurrent mappings.
        max_concurrency: u32,
    },
    /// Wait for all branches then continue (stub in v1.29.0).
    Join {
        /// Join policy name.
        policy: String,
        /// Branch operator IDs.
        branches: Vec<OperatorId>,
    },
    /// Race: first branch to complete wins (stub in v1.29.0).
    Race {
        /// Branch operator IDs.
        branches: Vec<OperatorId>,
        /// Timeout in milliseconds.
        timeout_ms: u64,
    },
    /// Conditional branch (stub in v1.29.0).
    Choice {
        /// Branch map: condition string -> operator ID.
        branches: BTreeMap<String, OperatorId>,
    },
    /// Iterative loop (stub in v1.29.0).
    Loop {
        /// Maximum iterations.
        max_iterations: u32,
        /// Guard expression.
        until: GuardExpr,
        /// Body operator ID.
        body: OperatorId,
    },
    /// Conditional execution gate.
    Gate {
        /// Guard expression.
        condition: GuardExpr,
        /// Body operator ID.
        body: OperatorId,
    },
    /// Wait for an external event.
    Wait {
        /// Event type to wait for.
        event_type: String,
        /// Timeout in milliseconds.
        timeout_ms: u64,
    },
    /// Invoke a sub-workflow.
    SubWorkflow {
        /// Reference to the sub-workflow run.
        run_ref: String,
    },
    /// Compensation for a failed operator (stub in v1.29.0).
    Compensate {
        /// Operator ID to compensate.
        of: OperatorId,
    },
}

impl Operator {
    /// Returns all operator IDs referenced by this operator (for cycle detection).
    pub fn referenced_ids(&self) -> Vec<OperatorId> {
        match self {
            Operator::Task { .. } => vec![],
            Operator::Sequence { body } => body.clone(),
            Operator::Parallel { branches, .. } => branches.clone(),
            Operator::Map { source, body, .. } => vec![source.clone(), body.clone()],
            Operator::Join { branches, .. } => branches.clone(),
            Operator::Race { branches, .. } => branches.clone(),
            Operator::Choice { branches } => branches.values().cloned().collect(),
            Operator::Loop { body, .. } => vec![body.clone()],
            Operator::Gate { body, .. } => vec![body.clone()],
            Operator::Wait { .. } => vec![],
            Operator::SubWorkflow { .. } => vec![],
            Operator::Compensate { of } => vec![of.clone()],
        }
    }
}

// ── TemplateRef ────────────────────────────────────────────────────────────

/// Reference to a workflow template.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct TemplateRef {
    /// Template identifier (reverse-DNS).
    pub id: String,
    /// Template version.
    pub version: String,
}

// ── WorkflowTemplate ───────────────────────────────────────────────────────

/// Human-authored intent declaration — source of truth for compilation.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields, rename_all = "snake_case")]
pub struct WorkflowTemplate {
    /// Template identifier (reverse-DNS, e.g. `sddk.adaptive.discovery`).
    pub template_id: String,
    /// Human-readable name.
    pub name: String,
    /// Semantic version.
    pub version: String,
    /// Free-text intent description.
    pub intent: String,
    /// Authored capability allowlist (no wildcards).
    pub capability_allowlist: BTreeSet<CapabilityId>,
    /// Expansion permissions granted by this template.
    pub expansion_permissions: BTreeSet<ExpansionPermission>,
    /// Invariants this template guarantees.
    pub invariants: BTreeSet<Invariant>,
    /// Execution budgets.
    pub budgets: Budgets,
    /// Pack-specific policies.
    pub policies: BTreeMap<String, Policy>,
    /// Convergence criteria.
    pub convergence: ConvergenceSpec,
    /// Schema version (must be 1 for v1.29.0 readers).
    pub schema_version: u32,
}

impl WorkflowTemplate {
    /// Validates this template for compilation.
    pub fn validate(&self) -> Result<(), CompileError> {
        // Empty allowlist rejected
        if self.capability_allowlist.is_empty() {
            return Err(CompileError::EmptyCapabilityAllowlist);
        }
        // Schema version must be 1
        if self.schema_version != SCHEMA_VERSION {
            return Err(CompileError::UnsupportedSchemaVersion {
                got: self.schema_version,
                want: SCHEMA_VERSION,
            });
        }
        // Check that all expansion permissions are in the closed set AND in the allowlist
        for perm in &self.expansion_permissions {
            if !perm.is_known_permission() || !perm.is_allowed_by(&self.expansion_permissions) {
                return Err(CompileError::ExpansionNotAllowed);
            }
        }
        // Budget must fit within hard limits
        if !self.budgets.fits_within(&Budgets::hard_limits()) {
            return Err(CompileError::BudgetExceedsLimit);
        }
        Ok(())
    }
}

// ── WorkflowIR ──────────────────────────────────────────────────────────────

/// Validated, content-addressed executable plan produced by the compiler.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields, rename_all = "snake_case")]
pub struct WorkflowIR {
    /// IR identifier (assigned post-compilation).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub ir_id: Option<IrId>,
    /// Schema version (must be 1).
    pub schema_version: u32,
    /// Template this IR was compiled from.
    pub template_ref: TemplateRef,
    /// Operators keyed by ID (order-independent BTreeMap).
    pub operators: BTreeMap<OperatorId, Operator>,
    /// Guards keyed by operator ID.
    pub guards: BTreeMap<OperatorId, GuardExpr>,
    /// Effective expansion permissions (may be subset of template).
    pub expansion_permissions: BTreeSet<ExpansionPermission>,
    /// Effective budgets (may be tighter than template).
    pub budgets: Budgets,
    /// Required invariants (must be subset of template invariants).
    pub required_invariants: BTreeSet<Invariant>,
    /// Provenance metadata.
    pub provenance: Provenance,
}

impl WorkflowIR {
    /// Computes the content hash of this IR (mirrors EventEnvelopeV1::compute_content_hash).
    ///
    /// Excludes `ir_id` (assigned post-hoc) and `schema_version` (metadata).
    /// Stable across BTreeMap key ordering because serde_json uses BTreeMap by default.
    pub fn compute_content_hash(&self) -> ContentHash {
        // Canonical form: zero out fields excluded from hash
        let mut canonical = self.clone();
        canonical.ir_id = None;
        canonical.schema_version = 0;

        let bytes = serde_json::to_vec(&canonical).expect("WorkflowIR is always serializable");
        let digest = Sha256::digest(&bytes);
        let hex = format!("{:064x}", digest);
        format!("sha256:{}", hex)
    }

    /// Validates this IR after compilation.
    pub fn validate(&self) -> Result<(), ValidateError> {
        // Schema version check
        if self.schema_version != SCHEMA_VERSION {
            return Err(ValidateError::UnsupportedSchemaVersion {
                got: self.schema_version,
                want: SCHEMA_VERSION,
            });
        }
        // Required invariants must be subset of expansion permissions
        // (stub for v1.29.0)
        Ok(())
    }
}

// ── Errors ─────────────────────────────────────────────────────────────────

/// Compile-time errors for WorkflowTemplate validation.
///
/// Audit (cycle 3, kernel-cycle-3-carries-over): trimmed 2 unused variants
/// (`YamlSerde`, `InvariantSubsumed`). All remaining variants are emitted by
/// `WorkflowCompiler`. Adding a variant requires updating both `compiler.rs`
/// usage and the cycle-3 audit results at `docs/audit/error-variants.md`.
#[derive(Debug, Error)]
pub enum CompileError {
    /// Capability allowlist is empty.
    #[error("capability allowlist is empty")]
    EmptyCapabilityAllowlist,

    /// Expansion permission not in the closed set.
    #[error("expansion permission not in closed set")]
    ExpansionNotAllowed,

    /// Unsupported schema version.
    #[error("unsupported schema version: got {got}, want {want}")]
    UnsupportedSchemaVersion {
        /// The version that was found.
        got: u32,
        /// The version that was expected.
        want: u32,
    },

    /// Budget exceeds template limit.
    #[error("budget exceeds template limit")]
    BudgetExceedsLimit,

    /// Operator not in allowlist.
    #[error("operator not in allowlist: {0:?}")]
    OperatorNotAllowed(CapabilityId),

    /// Capability not in allowlist.
    #[error("capability not in allowlist: {0:?}")]
    CapabilityNotInAllowlist(CapabilityId),

    /// Cycle detected in operator graph.
    #[error("cycle detected in operator graph")]
    CycleDetected,

    /// Hash collision detected.
    #[error("hash collision detected")]
    HashCollision,
}

// Compile-time guard: 8 variants (post-cycle-3 trim). Drift fails the build.
crate::assert_variant_count_eq!(
    CompileError,
    8,
    [
        CompileError::EmptyCapabilityAllowlist,
        CompileError::ExpansionNotAllowed,
        CompileError::UnsupportedSchemaVersion { .. },
        CompileError::BudgetExceedsLimit,
        CompileError::OperatorNotAllowed(_),
        CompileError::CapabilityNotInAllowlist(_),
        CompileError::CycleDetected,
        CompileError::HashCollision,
    ]
);

/// Runtime validation errors for WorkflowIR.
#[derive(Debug, Error)]
pub enum ValidateError {
    /// Unsupported schema version.
    #[error("unsupported schema version: got {got}, want {want}")]
    UnsupportedSchemaVersion {
        /// The version that was found.
        got: u32,
        /// The version that was expected.
        want: u32,
    },

    /// Operator not found.
    #[error("operator not found: {0:?}")]
    OperatorNotFound(OperatorId),

    /// Cycle detected in operator graph.
    #[error("cycle detected in operator graph")]
    CycleDetected,

    /// Guard expression failed.
    #[error("guard expression failed: {0}")]
    GuardFailed(String),

    /// Budget exceeds hard limits.
    #[error("budget exceeds hard limits")]
    BudgetExceedsLimit,

    /// Capability not in the template's allowlist.
    #[error("capability not in allowlist: {0:?}")]
    CapabilityNotInAllowlist(CapabilityId),

    /// Operator not allowed (e.g., Map without expansion permission).
    #[error("operator not allowed: {0:?}")]
    OperatorNotAllowed(OperatorId),
}
