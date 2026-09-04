//! Engine-side authority enforcement per ADR-070.
//!
//! This module declares:
//! - AuthorityContext: the canonical caller-supplied authority record
//! - infer_actor_kind: the locked v1.81.x prefix heuristic helper
//! - WritableSurface: the 12-surface enum (8 from ADR-069 §3 + 4 planning surfaces)
//! - WRITABLE_SURFACE_MATRIX: const table mapping surface → admitted ActorKind
//! - validate: AuthorityContext::validate(surface) returns Result<(), EngineError>
//!
//! Domain layer remains unchanged; engine-side only.

use crate::EngineError;
use sddk_domain::ActorKind;

/// The 12 writable surfaces (8 from ADR-069 §3 + 4 planning surfaces from ADR-072).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum WritableSurface {
    CycleState,
    LedgerEvents,
    GateReceipts,
    PlanRevisions,
    TransitionRecords,
    FrameworkBundle,
    GithubReleases,
    KnowledgeGraphVault,
    // Planning surfaces (ADR-072)
    /// A planning work item.
    PlanItem,
    /// A dependency edge between planning work items.
    DependencyEdge,
    /// An evidence attachment on a planning work item.
    EvidenceAttachment,
    /// A decision record on a planning work item.
    DecisionRecord,
}

impl WritableSurface {
    /// Returns the canonical snake_case name of the surface.
    pub fn name(self) -> &'static str {
        match self {
            WritableSurface::CycleState => "cycle_state",
            WritableSurface::LedgerEvents => "ledger_events",
            WritableSurface::GateReceipts => "gate_receipts",
            WritableSurface::PlanRevisions => "plan_revisions",
            WritableSurface::TransitionRecords => "transition_records",
            WritableSurface::FrameworkBundle => "framework_bundle",
            WritableSurface::GithubReleases => "github_releases",
            WritableSurface::KnowledgeGraphVault => "knowledge_graph_vault",
            WritableSurface::PlanItem => "plan_item",
            WritableSurface::DependencyEdge => "dependency_edge",
            WritableSurface::EvidenceAttachment => "evidence_attachment",
            WritableSurface::DecisionRecord => "decision_record",
        }
    }
}

/// Caller-supplied authority record.
///
/// Constructed at CLI call sites via `AuthorityContext::for_cli` and passed
/// to engine entry points. Engine-side validation consults the writable-surface
/// matrix before any ledger mutation.
#[derive(Debug, Clone)]
pub struct AuthorityContext {
    /// Kind of actor making the request.
    pub actor_kind: ActorKind,
    /// Stable identifier of the actor within the kind namespace.
    pub actor_id: String,
    /// Lease owner string (required for cycle pause/resume/supersede).
    pub lease_owner: Option<String>,
    /// Fencing token associated with the lease.
    pub fencing_token: Option<i64>,
}

impl AuthorityContext {
    /// Canonical constructor for CLI callers.
    ///
    /// Use this at every CLI entry point that calls engine methods.
    pub fn for_cli(
        actor_id: impl Into<String>,
        actor_kind: ActorKind,
        lease_owner: Option<String>,
        fencing_token: Option<i64>,
    ) -> Self {
        Self {
            actor_kind,
            actor_id: actor_id.into(),
            lease_owner,
            fencing_token,
        }
    }

    /// Constructs an AuthorityContext for testing or internal use.
    ///
    /// For CLI call sites, use `for_cli` instead.
    pub fn for_test(actor_kind: ActorKind, actor_id: impl Into<String>) -> Self {
        Self {
            actor_kind,
            actor_id: actor_id.into(),
            lease_owner: None,
            fencing_token: None,
        }
    }

    /// Validate this authority context against the writable-surface matrix.
    ///
    /// Fail-closed: returns `Err(EngineError::AuthorityContextRejected)` when
    /// `self.actor_kind` is not admitted for the given surface.
    pub fn validate(&self, surface: WritableSurface) -> Result<(), EngineError> {
        let admitted = WRITABLE_SURFACE_MATRIX
            .iter()
            .find(|(s, _)| *s == surface)
            .map(|(_, kinds)| kinds)
            .ok_or_else(|| EngineError::AuthorityContextRejected {
                surface: surface.name().to_string(),
                kind: format!("{:?}", self.actor_kind),
                reason: "unknown surface".to_string(),
            })?;

        if admitted.contains(&self.actor_kind) {
            Ok(())
        } else {
            Err(EngineError::AuthorityContextRejected {
                surface: surface.name().to_string(),
                kind: format!("{:?}", self.actor_kind),
                reason: format!(
                    "actor_kind {:?} not admitted on surface {} (admitted: {:?})",
                    self.actor_kind,
                    surface.name(),
                    admitted
                ),
            })
        }
    }
}

/// Locked v1.81.x prefix heuristic from CLI.
///
/// Matches the mapping documented in ADR-069 §5 and locked as the v1.81.x
/// contract. Replaces the inline chain previously at `cycle.rs:1217-1223`.
pub fn infer_actor_kind(actor_id: &str) -> ActorKind {
    if actor_id.starts_with("user:") {
        ActorKind::Human
    } else if actor_id.starts_with("agent:") {
        ActorKind::Agent
    } else {
        ActorKind::System
    }
}

/// The 12-surface matrix (ADR-069 §3 + ADR-072 planning surfaces).
///
/// Each row maps a `WritableSurface` to the set of `ActorKind` values
/// that are admitted to write to that surface.
pub const WRITABLE_SURFACE_MATRIX: &[(WritableSurface, &[ActorKind])] = &[
    (
        WritableSurface::CycleState,
        &[ActorKind::Human, ActorKind::Agent, ActorKind::System],
    ),
    (
        WritableSurface::LedgerEvents,
        &[ActorKind::Human, ActorKind::Agent, ActorKind::System],
    ),
    (WritableSurface::GateReceipts, &[ActorKind::System]),
    (
        WritableSurface::PlanRevisions,
        &[ActorKind::Human, ActorKind::Agent],
    ),
    (
        WritableSurface::TransitionRecords,
        &[ActorKind::Human, ActorKind::Agent, ActorKind::System],
    ),
    (WritableSurface::FrameworkBundle, &[ActorKind::System]),
    (WritableSurface::GithubReleases, &[ActorKind::System]),
    (WritableSurface::KnowledgeGraphVault, &[ActorKind::Human]),
    // Planning surfaces (ADR-072)
    (
        WritableSurface::PlanItem,
        &[ActorKind::Human, ActorKind::Agent],
    ),
    (WritableSurface::DependencyEdge, &[ActorKind::System]),
    (
        WritableSurface::EvidenceAttachment,
        &[ActorKind::Human, ActorKind::Agent],
    ),
    (WritableSurface::DecisionRecord, &[ActorKind::Human]),
];

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn infer_human_prefix() {
        assert_eq!(infer_actor_kind("user:alice"), ActorKind::Human);
    }

    #[test]
    fn infer_agent_prefix() {
        assert_eq!(infer_actor_kind("agent:orchestrator"), ActorKind::Agent);
    }

    #[test]
    fn infer_system_fallback() {
        assert_eq!(infer_actor_kind("system"), ActorKind::System);
        assert_eq!(infer_actor_kind("anything-else"), ActorKind::System);
    }

    #[test]
    fn validate_admitted_kinds() {
        let ctx = AuthorityContext::for_test(ActorKind::Human, "user:test");
        assert!(ctx.validate(WritableSurface::CycleState).is_ok());
        assert!(ctx.validate(WritableSurface::PlanRevisions).is_ok());
        assert!(ctx.validate(WritableSurface::KnowledgeGraphVault).is_ok());
    }

    #[test]
    fn validate_rejected_kinds() {
        let ctx = AuthorityContext::for_test(ActorKind::Human, "user:test");
        assert!(ctx.validate(WritableSurface::FrameworkBundle).is_err());
        assert!(ctx.validate(WritableSurface::GithubReleases).is_err());

        let ctx2 = AuthorityContext::for_test(ActorKind::System, "sys");
        assert!(ctx2.validate(WritableSurface::KnowledgeGraphVault).is_err());
    }

    #[test]
    fn matrix_covers_twelve_surfaces() {
        assert_eq!(WRITABLE_SURFACE_MATRIX.len(), 12);
    }
}
