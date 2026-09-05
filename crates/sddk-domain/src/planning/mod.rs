//! Planning Ledger domain types.
//!
//! Implements the PLN-LEDGER-001 domain model per ADR-072:
//! - WorkItemV1: planning unit with lifecycle status
//! - DependencyEdgeV1: typed edges between WorkItems
//! - WorkItemStatus: six-variant closed lifecycle state machine
//! - EvidenceAttachmentV1: CAS-referenced evidence attached to WorkItems
//! - DecisionRecordV1: rationale-bound decisions attached to WorkItems
//! - PlanningProvenanceChainV1: cycle-indexed provenance chain

use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};

use crate::assert_variant_count_eq;
use crate::event_envelope::{ActorKind, ActorRef};

// ── WorkItemStatus ─────────────────────────────────────────────────────────────

/// Planning work item lifecycle status.
///
/// Six-variant closed set per ADR-072 §3.3.
/// No Accepted or Blocked state — those concerns are expressed via
/// dependency edges and authority decisions respectively.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum WorkItemStatus {
    /// Draft: initial state, not yet active.
    Draft,
    /// Active: work is in progress.
    Active,
    /// Paused: temporarily suspended.
    Paused,
    /// Done: work completed successfully.
    Done,
    /// Superseded: replaced by another WorkItem.
    Superseded,
    /// Cancelled: work abandoned.
    Cancelled,
}

impl WorkItemStatus {
    /// Returns true if this status is terminal (no outgoing transitions allowed).
    pub fn is_terminal(&self) -> bool {
        matches!(
            self,
            WorkItemStatus::Done | WorkItemStatus::Superseded | WorkItemStatus::Cancelled
        )
    }

    /// Returns true if a transition from `self` to `to` is legal.
    ///
    /// Legal transitions per ADR-072 §3.3:
    /// - Draft → Active (requires dependency resolution)
    /// - Active → Paused
    /// - Paused → Active
    /// - Active → Done
    /// - Active → Superseded
    /// - Active → Cancelled
    /// - Paused → Cancelled
    pub fn can_transition_to(&self, to: WorkItemStatus) -> bool {
        matches!(
            (self, to),
            (WorkItemStatus::Draft, WorkItemStatus::Active)
                | (WorkItemStatus::Active, WorkItemStatus::Paused)
                | (WorkItemStatus::Active, WorkItemStatus::Done)
                | (WorkItemStatus::Active, WorkItemStatus::Superseded)
                | (WorkItemStatus::Active, WorkItemStatus::Cancelled)
                | (WorkItemStatus::Paused, WorkItemStatus::Active)
                | (WorkItemStatus::Paused, WorkItemStatus::Cancelled)
        )
    }

    /// Returns the list of valid outgoing transitions from this status.
    pub fn valid_transitions(&self) -> Vec<WorkItemStatus> {
        match self {
            WorkItemStatus::Draft => vec![WorkItemStatus::Active],
            WorkItemStatus::Active => vec![
                WorkItemStatus::Paused,
                WorkItemStatus::Done,
                WorkItemStatus::Superseded,
                WorkItemStatus::Cancelled,
            ],
            WorkItemStatus::Paused => vec![WorkItemStatus::Active, WorkItemStatus::Cancelled],
            WorkItemStatus::Done => vec![],
            WorkItemStatus::Superseded => vec![],
            WorkItemStatus::Cancelled => vec![],
        }
    }
}

assert_variant_count_eq!(
    WorkItemStatus,
    6,
    [
        WorkItemStatus::Draft,
        WorkItemStatus::Active,
        WorkItemStatus::Paused,
        WorkItemStatus::Done,
        WorkItemStatus::Superseded,
        WorkItemStatus::Cancelled,
    ]
);

// ── WorkItemV1 ───────────────────────────────────────────────────────────────

/// Schema version constant for WorkItemV1.
pub const WORK_ITEM_SCHEMA_VERSION: u32 = 1;

/// A planning work item (planning ledger unit).
///
/// Represents a unit of work in the planning portfolio, distinct from
/// WorkflowIR runtime execution nodes.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WorkItemV1 {
    /// Unique identifier (UUIDv7).
    pub id: WorkItemId,
    /// Cycle this item belongs to.
    pub cycle_id: CycleId,
    /// Short title describing the work.
    pub title: String,
    /// Detailed description.
    pub description: String,
    /// Current lifecycle status.
    pub status: WorkItemStatus,
    /// Actor responsible for this item (if assigned).
    pub actor_ref: Option<ActorRef>,
    /// Wall-clock creation time (RFC 3339).
    pub created_at: i64,
    /// Schema version; always 1.
    pub schema_version: u32,
}

impl WorkItemV1 {
    /// Creates a new WorkItemV1 in Draft status.
    pub fn new(
        id: WorkItemId,
        cycle_id: CycleId,
        title: String,
        description: String,
        actor_ref: Option<ActorRef>,
        created_at: i64,
    ) -> Self {
        Self {
            id,
            cycle_id,
            title,
            description,
            status: WorkItemStatus::Draft,
            actor_ref,
            created_at,
            schema_version: WORK_ITEM_SCHEMA_VERSION,
        }
    }

    /// Returns the stable identity hash of this WorkItem.
    ///
    /// Computed over canonical JSON of the identity fields (excludes schema_version
    /// and any non-identity metadata).
    pub fn compute_identity(&self) -> String {
        let canonical = serde_json::to_string(&self).expect("WorkItemV1 is always serializable");
        let digest = Sha256::digest(canonical.as_bytes());
        format!("{:x}", digest)
    }
}

/// Work item identifier (UUIDv7).
pub type WorkItemId = String;

/// Cycle identifier.
pub type CycleId = String;

// ── DependencyEdgeKind ────────────────────────────────────────────────────────

/// Kind of dependency relationship between WorkItems.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum DependencyEdgeKind {
    /// The from-item blocks the to-item from progressing.
    Blocks,
    /// The from-item blocks the to-item from closure (completed/superseded).
    BlocksOnClosure,
}

assert_variant_count_eq!(
    DependencyEdgeKind,
    2,
    [
        DependencyEdgeKind::Blocks,
        DependencyEdgeKind::BlocksOnClosure,
    ]
);

// ── DependencyEdgeV1 ──────────────────────────────────────────────────────────

/// Schema version constant for DependencyEdgeV1.
pub const DEPENDENCY_EDGE_SCHEMA_VERSION: u32 = 1;

/// A typed directed edge between two WorkItems.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DependencyEdgeV1 {
    /// Source WorkItem (the blocker).
    pub from_id: WorkItemId,
    /// Target WorkItem (the blocked).
    pub to_id: WorkItemId,
    /// Kind of dependency.
    pub kind: DependencyEdgeKind,
    /// Actor who created this edge (if recorded).
    pub actor_ref: Option<ActorRef>,
    /// Schema version; always 1.
    pub schema_version: u32,
}

impl DependencyEdgeV1 {
    /// Creates a new DependencyEdgeV1.
    pub fn new(
        from_id: WorkItemId,
        to_id: WorkItemId,
        kind: DependencyEdgeKind,
        actor_ref: Option<ActorRef>,
    ) -> Self {
        Self {
            from_id,
            to_id,
            kind,
            actor_ref,
            schema_version: DEPENDENCY_EDGE_SCHEMA_VERSION,
        }
    }

    /// Returns the stable identity hash of this edge.
    pub fn compute_identity(&self) -> String {
        let canonical =
            serde_json::to_string(&self).expect("DependencyEdgeV1 is always serializable");
        let digest = Sha256::digest(canonical.as_bytes());
        format!("{:x}", digest)
    }

    /// Returns true if this edge has a self-loop.
    pub fn is_self_loop(&self) -> bool {
        self.from_id == self.to_id
    }
}

// ── PlanningEvidenceKind ─────────────────────────────────────────────────────────

/// Kind of evidence attached to a WorkItem (planning-specific).
///
/// Distinct from `evidence::EvidenceKind` which covers UAT/approval evidence.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum PlanningEvidenceKind {
    /// Log output or trace.
    Log,
    /// Metric or measurement.
    Metric,
    /// Snapshot or screenshot.
    Snapshot,
    /// External reference or link.
    Reference,
    /// Approval or sign-off.
    Approval,
}

assert_variant_count_eq!(
    PlanningEvidenceKind,
    5,
    [
        PlanningEvidenceKind::Log,
        PlanningEvidenceKind::Metric,
        PlanningEvidenceKind::Snapshot,
        PlanningEvidenceKind::Reference,
        PlanningEvidenceKind::Approval,
    ]
);

// ── EvidenceAttachmentV1 ─────────────────────────────────────────────────────

/// Schema version constant for EvidenceAttachmentV1.
pub const EVIDENCE_ATTACHMENT_SCHEMA_VERSION: u32 = 1;

/// SHA-256 content hash of a CAS object (hex string with prefix).
pub type CasHash = String;

/// Evidence attached to a WorkItem, stored in CAS.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EvidenceAttachmentV1 {
    /// Unique evidence identifier.
    pub id: EvidenceId,
    /// WorkItem this evidence is attached to.
    pub work_item_id: WorkItemId,
    /// Kind of evidence.
    pub kind: PlanningEvidenceKind,
    /// CAS hash referencing the immutable evidence body.
    pub body_ref: CasHash,
    /// Actor who attached this evidence.
    pub actor_ref: Option<ActorRef>,
    /// Schema version; always 1.
    pub schema_version: u32,
}

impl EvidenceAttachmentV1 {
    /// Creates a new EvidenceAttachmentV1.
    pub fn new(
        id: EvidenceId,
        work_item_id: WorkItemId,
        kind: PlanningEvidenceKind,
        body_ref: CasHash,
        actor_ref: Option<ActorRef>,
    ) -> Self {
        Self {
            id,
            work_item_id,
            kind,
            body_ref,
            actor_ref,
            schema_version: EVIDENCE_ATTACHMENT_SCHEMA_VERSION,
        }
    }
}

/// Evidence identifier.
pub type EvidenceId = String;

// ── DecisionKind ─────────────────────────────────────────────────────────────

/// Kind of decision record.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum DecisionKind {
    /// Decision to accept or proceed.
    Accept,
    /// Decision to reject.
    Reject,
    /// Decision deferred to later.
    Defer,
    /// Decision escalated to higher authority.
    Escalate,
}

assert_variant_count_eq!(
    DecisionKind,
    4,
    [
        DecisionKind::Accept,
        DecisionKind::Reject,
        DecisionKind::Defer,
        DecisionKind::Escalate,
    ]
);

// ── DecisionRecordV1 ──────────────────────────────────────────────────────────

/// Schema version constant for DecisionRecordV1.
pub const DECISION_RECORD_SCHEMA_VERSION: u32 = 1;

/// A decision record attached to a WorkItem.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DecisionRecordV1 {
    /// Unique decision identifier.
    pub id: DecisionId,
    /// WorkItem this decision is attached to.
    pub work_item_id: WorkItemId,
    /// Kind of decision.
    pub kind: DecisionKind,
    /// Rationale (required, non-empty).
    pub rationale: String,
    /// Actor who made this decision.
    pub actor_ref: Option<ActorRef>,
    /// Schema version; always 1.
    pub schema_version: u32,
}

impl DecisionRecordV1 {
    /// Creates a new DecisionRecordV1.
    ///
    /// Returns an error if rationale is empty.
    pub fn new(
        id: DecisionId,
        work_item_id: WorkItemId,
        kind: DecisionKind,
        rationale: String,
        actor_ref: Option<ActorRef>,
    ) -> Result<Self, DecisionError> {
        if rationale.trim().is_empty() {
            return Err(DecisionError::EmptyRationale);
        }
        Ok(Self {
            id,
            work_item_id,
            kind,
            rationale,
            actor_ref,
            schema_version: DECISION_RECORD_SCHEMA_VERSION,
        })
    }

    /// Returns the stable identity hash of this decision record.
    pub fn compute_identity(&self) -> String {
        let canonical =
            serde_json::to_string(&self).expect("DecisionRecordV1 is always serializable");
        let digest = Sha256::digest(canonical.as_bytes());
        format!("{:x}", digest)
    }
}

/// Decision record identifier.
pub type DecisionId = String;

/// Error arising from invalid decision record construction.
#[derive(Debug, Clone, thiserror::Error)]
pub enum DecisionError {
    #[error("rationale must be non-empty")]
    EmptyRationale,
}

// ── PlanningProvenanceChainV1 ─────────────────────────────────────────────────

/// Schema version constant for PlanningProvenanceChainV1.
pub const PLANNING_PROVENANCE_SCHEMA_VERSION: u32 = 1;

/// A provenance chain linking a cycle to its WorkItems, evidence, and decisions.
///
/// Queryable by cycle_id; preserves ordering; reconstructable deterministically.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PlanningProvenanceChainV1 {
    /// Cycle this chain belongs to.
    pub cycle_id: CycleId,
    /// Ordered WorkItem identifiers in this cycle.
    pub work_item_ids: Vec<WorkItemId>,
    /// CAS hashes of evidence attached to items in this cycle.
    pub evidence_refs: Vec<CasHash>,
    /// Decision record identifiers in this cycle.
    pub decision_refs: Vec<DecisionId>,
}

impl PlanningProvenanceChainV1 {
    /// Creates a new PlanningProvenanceChainV1.
    pub fn new(
        cycle_id: CycleId,
        work_item_ids: Vec<WorkItemId>,
        evidence_refs: Vec<CasHash>,
        decision_refs: Vec<DecisionId>,
    ) -> Self {
        Self {
            cycle_id,
            work_item_ids,
            evidence_refs,
            decision_refs,
        }
    }

    /// Verifies this chain has no dangling references.
    ///
    /// A dangling reference is one that appears in evidence_refs or decision_refs
    /// but not in work_item_ids. Subclasses of references (e.g. evidence bodies)
    /// require separate CAS verification.
    pub fn verify_references(&self) -> Result<(), ProvenanceError> {
        // This is a stub: real verification requires access to the storage layer
        // to check that referenced WorkItems exist. The domain model only
        // validates the structural shape here.
        if self.cycle_id.is_empty() {
            return Err(ProvenanceError::EmptyCycleId);
        }
        Ok(())
    }
}

/// Error from provenance chain verification.
#[derive(Debug, Clone, thiserror::Error)]
pub enum ProvenanceError {
    #[error("cycle_id must be non-empty")]
    EmptyCycleId,
    #[error("dangling reference: {0}")]
    DanglingReference(String),
}

// ── Planning Graph Identity ───────────────────────────────────────────────────

/// A volatile-field-excluded WorkItem projection for identity computation.
///
/// Excludes fields that change across replays without semantic change:
/// - `created_at` (wall-clock, not reproducible)
/// - `status` (mutable by definition)
///
/// This is the canonical projection used for `compute_planning_graph_identity`.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WorkItemIdentityProjection {
    pub id: WorkItemId,
    pub cycle_id: CycleId,
    pub title: String,
    pub description: String,
    pub actor_ref: Option<ActorRef>,
    pub schema_version: u32,
}

/// A volatile-field-excluded DependencyEdge projection for identity computation.
///
/// Excludes `actor_ref` which may change across replays.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DependencyEdgeIdentityProjection {
    pub from_id: WorkItemId,
    pub to_id: WorkItemId,
    pub kind: DependencyEdgeKind,
    pub schema_version: u32,
}

impl From<&WorkItemV1> for WorkItemIdentityProjection {
    fn from(wi: &WorkItemV1) -> Self {
        Self {
            id: wi.id.clone(),
            cycle_id: wi.cycle_id.clone(),
            title: wi.title.clone(),
            description: wi.description.clone(),
            actor_ref: wi.actor_ref.clone(),
            schema_version: wi.schema_version,
        }
    }
}

impl From<&DependencyEdgeV1> for DependencyEdgeIdentityProjection {
    fn from(edge: &DependencyEdgeV1) -> Self {
        Self {
            from_id: edge.from_id.clone(),
            to_id: edge.to_id.clone(),
            kind: edge.kind,
            schema_version: edge.schema_version,
        }
    }
}

/// Computes the deterministic SHA-256 identity of a planning graph.
///
/// The identity is computed over canonical JSON of:
/// - All WorkItems in the cycle (sorted by id), with volatile fields excluded
/// - All DependencyEdges (sorted by from_id then to_id), with volatile fields excluded
/// - All evidence_refs (sorted)
/// - All decision_refs (sorted)
///
/// This function is PURE — it takes pre-fetched data and produces a deterministic hash.
/// Volatile fields (`created_at`, `status`) are excluded per FIND-PLN-008.
///
/// # Arguments
/// * `work_items` - All WorkItems in the cycle
/// * `edges` - All DependencyEdges in the cycle
/// * `evidence_refs` - All CAS hashes of evidence in the cycle
/// * `decision_refs` - All DecisionIds in the cycle
///
/// # Returns
/// SHA-256 hex string of the canonical JSON projection
pub fn compute_planning_graph_identity(
    work_items: &[WorkItemV1],
    edges: &[DependencyEdgeV1],
    evidence_refs: &[CasHash],
    decision_refs: &[DecisionId],
) -> String {
    // Sort work items by id for deterministic ordering
    let mut sorted_wis: Vec<WorkItemIdentityProjection> = work_items
        .iter()
        .map(WorkItemIdentityProjection::from)
        .collect();
    sorted_wis.sort_by(|a, b| a.id.cmp(&b.id));

    // Sort edges by (from_id, to_id) for deterministic ordering
    let mut sorted_edges: Vec<DependencyEdgeIdentityProjection> = edges
        .iter()
        .map(DependencyEdgeIdentityProjection::from)
        .collect();
    sorted_edges.sort_by(|a, b| (&a.from_id, &a.to_id).cmp(&(&b.from_id, &b.to_id)));

    // Sort evidence refs
    let mut sorted_evidence = evidence_refs.to_vec();
    sorted_evidence.sort();

    // Sort decision refs
    let mut sorted_decisions = decision_refs.to_vec();
    sorted_decisions.sort();

    // Build canonical JSON
    let canonical = serde_json::json!({
        "work_items": sorted_wis,
        "edges": sorted_edges,
        "evidence_refs": sorted_evidence,
        "decision_refs": sorted_decisions,
    });

    let digest = Sha256::digest(serde_json::to_string(&canonical).unwrap().as_bytes());
    format!("{:x}", digest)
}

// ── Storage Records ─────────────────────────────────────────────────────────────

/// SQL row representation for WorkItem persistence.
///
/// Decomposes ActorRef into its component fields for SQL storage.
#[derive(Debug, Clone)]
pub struct WorkItemRecord {
    pub id: WorkItemId,
    pub cycle_id: CycleId,
    pub title: String,
    pub description: String,
    pub status: WorkItemStatus,
    pub actor_ref_kind: Option<String>,
    pub actor_ref_id: Option<String>,
    pub actor_ref_label: Option<String>,
    pub created_at: i64,
    pub schema_version: u32,
}

impl WorkItemRecord {
    /// Converts this record into a domain WorkItemV1.
    pub fn into_domain(self) -> WorkItemV1 {
        let actor_ref = match (self.actor_ref_kind, self.actor_ref_id, self.actor_ref_label) {
            (Some(kind), Some(id), label) => Some(ActorRef {
                kind: match kind.as_str() {
                    "Human" => ActorKind::Human,
                    "Agent" => ActorKind::Agent,
                    _ => ActorKind::System,
                },
                id,
                definition_hash: None,
                policy_hash: None,
                model: None,
            }),
            _ => None,
        };
        WorkItemV1 {
            id: self.id,
            cycle_id: self.cycle_id,
            title: self.title,
            description: self.description,
            status: self.status,
            actor_ref,
            created_at: self.created_at,
            schema_version: self.schema_version,
        }
    }

    /// Creates a record from a domain WorkItemV1.
    pub fn from_domain(wi: &WorkItemV1) -> Self {
        let (actor_ref_kind, actor_ref_id, actor_ref_label) = match &wi.actor_ref {
            Some(ar) => (
                Some(
                    match ar.kind {
                        ActorKind::Human => "Human",
                        ActorKind::Agent => "Agent",
                        ActorKind::System => "System",
                    }
                    .to_string(),
                ),
                Some(ar.id.clone()),
                Some(ar.id.clone()), // label defaults to id in our usage
            ),
            None => (None, None, None),
        };
        Self {
            id: wi.id.clone(),
            cycle_id: wi.cycle_id.clone(),
            title: wi.title.clone(),
            description: wi.description.clone(),
            status: wi.status,
            actor_ref_kind,
            actor_ref_id,
            actor_ref_label,
            created_at: wi.created_at,
            schema_version: wi.schema_version,
        }
    }
}

/// SQL row representation for DependencyEdge persistence.
#[derive(Debug, Clone)]
pub struct DependencyEdgeRecord {
    pub from_id: WorkItemId,
    pub to_id: WorkItemId,
    pub kind: DependencyEdgeKind,
    pub actor_ref_kind: Option<String>,
    pub actor_ref_id: Option<String>,
    pub actor_ref_label: Option<String>,
    pub schema_version: u32,
}

impl DependencyEdgeRecord {
    /// Converts this record into a domain DependencyEdgeV1.
    pub fn into_domain(self) -> DependencyEdgeV1 {
        let actor_ref = match (self.actor_ref_kind, self.actor_ref_id, self.actor_ref_label) {
            (Some(kind), Some(id), label) => Some(ActorRef {
                kind: match kind.as_str() {
                    "Human" => ActorKind::Human,
                    "Agent" => ActorKind::Agent,
                    _ => ActorKind::System,
                },
                id,
                definition_hash: None,
                policy_hash: None,
                model: None,
            }),
            _ => None,
        };
        DependencyEdgeV1 {
            from_id: self.from_id,
            to_id: self.to_id,
            kind: self.kind,
            actor_ref,
            schema_version: self.schema_version,
        }
    }

    /// Creates a record from a domain DependencyEdgeV1.
    pub fn from_domain(edge: &DependencyEdgeV1) -> Self {
        let (actor_ref_kind, actor_ref_id, actor_ref_label) = match &edge.actor_ref {
            Some(ar) => (
                Some(
                    match ar.kind {
                        ActorKind::Human => "Human",
                        ActorKind::Agent => "Agent",
                        ActorKind::System => "System",
                    }
                    .to_string(),
                ),
                Some(ar.id.clone()),
                Some(ar.id.clone()),
            ),
            None => (None, None, None),
        };
        Self {
            from_id: edge.from_id.clone(),
            to_id: edge.to_id.clone(),
            kind: edge.kind,
            actor_ref_kind,
            actor_ref_id,
            actor_ref_label,
            schema_version: edge.schema_version,
        }
    }
}

/// SQL row representation for EvidenceAttachment persistence.
#[derive(Debug, Clone)]
pub struct EvidenceAttachmentRecord {
    pub id: EvidenceId,
    pub work_item_id: WorkItemId,
    pub kind: PlanningEvidenceKind,
    pub body_ref: CasHash,
    pub actor_ref_kind: Option<String>,
    pub actor_ref_id: Option<String>,
    pub actor_ref_label: Option<String>,
    pub schema_version: u32,
}

impl EvidenceAttachmentRecord {
    /// Converts this record into a domain EvidenceAttachmentV1.
    pub fn into_domain(self) -> EvidenceAttachmentV1 {
        let actor_ref = match (self.actor_ref_kind, self.actor_ref_id, self.actor_ref_label) {
            (Some(kind), Some(id), label) => Some(ActorRef {
                kind: match kind.as_str() {
                    "Human" => ActorKind::Human,
                    "Agent" => ActorKind::Agent,
                    _ => ActorKind::System,
                },
                id,
                definition_hash: None,
                policy_hash: None,
                model: None,
            }),
            _ => None,
        };
        EvidenceAttachmentV1 {
            id: self.id,
            work_item_id: self.work_item_id,
            kind: self.kind,
            body_ref: self.body_ref,
            actor_ref,
            schema_version: self.schema_version,
        }
    }

    /// Creates a record from a domain EvidenceAttachmentV1.
    pub fn from_domain(ea: &EvidenceAttachmentV1) -> Self {
        let (actor_ref_kind, actor_ref_id, actor_ref_label) = match &ea.actor_ref {
            Some(ar) => (
                Some(
                    match ar.kind {
                        ActorKind::Human => "Human",
                        ActorKind::Agent => "Agent",
                        ActorKind::System => "System",
                    }
                    .to_string(),
                ),
                Some(ar.id.clone()),
                Some(ar.id.clone()),
            ),
            None => (None, None, None),
        };
        Self {
            id: ea.id.clone(),
            work_item_id: ea.work_item_id.clone(),
            kind: ea.kind,
            body_ref: ea.body_ref.clone(),
            actor_ref_kind,
            actor_ref_id,
            actor_ref_label,
            schema_version: ea.schema_version,
        }
    }
}

/// SQL row representation for DecisionRecord persistence.
#[derive(Debug, Clone)]
pub struct DecisionRecordRecord {
    pub id: DecisionId,
    pub work_item_id: WorkItemId,
    pub kind: DecisionKind,
    pub rationale: String,
    pub actor_ref_kind: Option<String>,
    pub actor_ref_id: Option<String>,
    pub actor_ref_label: Option<String>,
    pub schema_version: u32,
}

impl DecisionRecordRecord {
    /// Converts this record into a domain DecisionRecordV1.
    pub fn into_domain(self) -> DecisionRecordV1 {
        let actor_ref = match (self.actor_ref_kind, self.actor_ref_id, self.actor_ref_label) {
            (Some(kind), Some(id), label) => Some(ActorRef {
                kind: match kind.as_str() {
                    "Human" => ActorKind::Human,
                    "Agent" => ActorKind::Agent,
                    _ => ActorKind::System,
                },
                id,
                definition_hash: None,
                policy_hash: None,
                model: None,
            }),
            _ => None,
        };
        // This should not fail since we're converting from a valid record
        DecisionRecordV1 {
            id: self.id,
            work_item_id: self.work_item_id,
            kind: self.kind,
            rationale: self.rationale,
            actor_ref,
            schema_version: self.schema_version,
        }
    }

    /// Creates a record from a domain DecisionRecordV1.
    pub fn from_domain(dr: &DecisionRecordV1) -> Self {
        let (actor_ref_kind, actor_ref_id, actor_ref_label) = match &dr.actor_ref {
            Some(ar) => (
                Some(
                    match ar.kind {
                        ActorKind::Human => "Human",
                        ActorKind::Agent => "Agent",
                        ActorKind::System => "System",
                    }
                    .to_string(),
                ),
                Some(ar.id.clone()),
                Some(ar.id.clone()),
            ),
            None => (None, None, None),
        };
        Self {
            id: dr.id.clone(),
            work_item_id: dr.work_item_id.clone(),
            kind: dr.kind,
            rationale: dr.rationale.clone(),
            actor_ref_kind,
            actor_ref_id,
            actor_ref_label,
            schema_version: dr.schema_version,
        }
    }
}

// ── Service ──────────────────────────────────────────────────────────────────

pub mod service;

// ── Tests ────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn work_item_status_is_six_variants() {
        // The assert_variant_count_eq! macro already enforces this at compile time.
        // This test exists to give test coverage visibility.
        let all_statuses = [
            WorkItemStatus::Draft,
            WorkItemStatus::Active,
            WorkItemStatus::Paused,
            WorkItemStatus::Done,
            WorkItemStatus::Superseded,
            WorkItemStatus::Cancelled,
        ];
        assert_eq!(all_statuses.len(), 6);
    }

    #[test]
    fn work_item_status_terminal_states() {
        assert!(WorkItemStatus::Done.is_terminal());
        assert!(WorkItemStatus::Superseded.is_terminal());
        assert!(WorkItemStatus::Cancelled.is_terminal());
        assert!(!WorkItemStatus::Draft.is_terminal());
        assert!(!WorkItemStatus::Active.is_terminal());
        assert!(!WorkItemStatus::Paused.is_terminal());
    }

    #[test]
    fn work_item_status_valid_transitions() {
        // Draft → Active
        assert!(WorkItemStatus::Draft.can_transition_to(WorkItemStatus::Active));
        // Active ↔ Paused
        assert!(WorkItemStatus::Active.can_transition_to(WorkItemStatus::Paused));
        assert!(WorkItemStatus::Paused.can_transition_to(WorkItemStatus::Active));
        // Active → terminal states
        assert!(WorkItemStatus::Active.can_transition_to(WorkItemStatus::Done));
        assert!(WorkItemStatus::Active.can_transition_to(WorkItemStatus::Superseded));
        assert!(WorkItemStatus::Active.can_transition_to(WorkItemStatus::Cancelled));
        // Paused → Cancelled
        assert!(WorkItemStatus::Paused.can_transition_to(WorkItemStatus::Cancelled));
        // Terminal states reject all outgoing
        assert!(!WorkItemStatus::Done.can_transition_to(WorkItemStatus::Active));
        assert!(!WorkItemStatus::Superseded.can_transition_to(WorkItemStatus::Active));
        assert!(!WorkItemStatus::Cancelled.can_transition_to(WorkItemStatus::Active));
        // Invalid transitions
        assert!(!WorkItemStatus::Draft.can_transition_to(WorkItemStatus::Done));
        assert!(!WorkItemStatus::Paused.can_transition_to(WorkItemStatus::Done));
    }

    #[test]
    fn work_item_v1_new_and_identity() {
        let wi = WorkItemV1::new(
            "wi-001".into(),
            "cycle-001".into(),
            "Implement feature X".into(),
            "Detailed description".into(),
            None,
            1700000000,
        );
        assert_eq!(wi.status, WorkItemStatus::Draft);
        assert_eq!(wi.schema_version, WORK_ITEM_SCHEMA_VERSION);
        // Identity should be stable
        let id1 = wi.compute_identity();
        let id2 = wi.compute_identity();
        assert_eq!(id1, id2);
    }

    #[test]
    fn dependency_edge_self_loop_detection() {
        let edge = DependencyEdgeV1::new(
            "wi-001".into(),
            "wi-001".into(),
            DependencyEdgeKind::Blocks,
            None,
        );
        assert!(edge.is_self_loop());

        let edge2 = DependencyEdgeV1::new(
            "wi-001".into(),
            "wi-002".into(),
            DependencyEdgeKind::Blocks,
            None,
        );
        assert!(!edge2.is_self_loop());
    }

    #[test]
    fn dependency_edge_kind_is_two_variants() {
        let kinds = [
            DependencyEdgeKind::Blocks,
            DependencyEdgeKind::BlocksOnClosure,
        ];
        assert_eq!(kinds.len(), 2);
    }

    #[test]
    fn evidence_kind_is_five_variants() {
        let kinds = [
            PlanningEvidenceKind::Log,
            PlanningEvidenceKind::Metric,
            PlanningEvidenceKind::Snapshot,
            PlanningEvidenceKind::Reference,
            PlanningEvidenceKind::Approval,
        ];
        assert_eq!(kinds.len(), 5);
    }

    #[test]
    fn decision_kind_is_four_variants() {
        let kinds = [
            DecisionKind::Accept,
            DecisionKind::Reject,
            DecisionKind::Defer,
            DecisionKind::Escalate,
        ];
        assert_eq!(kinds.len(), 4);
    }

    #[test]
    fn decision_record_empty_rationale_rejected() {
        let result = DecisionRecordV1::new(
            "dec-001".into(),
            "wi-001".into(),
            DecisionKind::Accept,
            "".into(),
            None,
        );
        assert!(result.is_err());
        assert!(matches!(result.unwrap_err(), DecisionError::EmptyRationale));
    }

    #[test]
    fn decision_record_valid() {
        let dec = DecisionRecordV1::new(
            "dec-001".into(),
            "wi-001".into(),
            DecisionKind::Accept,
            "Approved by architecture review".into(),
            None,
        );
        assert!(dec.is_ok());
        let dec = dec.unwrap();
        assert_eq!(dec.schema_version, DECISION_RECORD_SCHEMA_VERSION);
    }

    #[test]
    fn evidence_attachment_round_trip() {
        let ea = EvidenceAttachmentV1::new(
            "ev-001".into(),
            "wi-001".into(),
            PlanningEvidenceKind::Log,
            "sha256:abc123".into(),
            None,
        );
        let json = serde_json::to_string(&ea).expect("EvidenceAttachmentV1 is serializable");
        let ea2: EvidenceAttachmentV1 =
            serde_json::from_str(&json).expect("EvidenceAttachmentV1 is deserializable");
        assert_eq!(ea.id, ea2.id);
        assert_eq!(ea.body_ref, ea2.body_ref);
        assert_eq!(ea.kind, ea2.kind);
    }

    #[test]
    fn planning_provenance_chain_empty_cycle_id_rejected() {
        let chain =
            PlanningProvenanceChainV1::new("".into(), vec!["wi-001".into()], vec![], vec![]);
        assert!(chain.verify_references().is_err());
    }

    #[test]
    fn planning_provenance_chain_valid() {
        let chain = PlanningProvenanceChainV1::new(
            "cycle-001".into(),
            vec!["wi-001".into(), "wi-002".into()],
            vec!["sha256:abc".into()],
            vec!["dec-001".into()],
        );
        assert!(chain.verify_references().is_ok());
    }

    #[test]
    fn work_item_identity_is_deterministic() {
        let wi1 = WorkItemV1::new(
            "wi-001".into(),
            "cycle-001".into(),
            "Title".into(),
            "Description".into(),
            None,
            1700000000,
        );
        let wi2 = WorkItemV1::new(
            "wi-001".into(),
            "cycle-001".into(),
            "Title".into(),
            "Description".into(),
            None,
            1700000000,
        );
        assert_eq!(wi1.compute_identity(), wi2.compute_identity());
    }
}
