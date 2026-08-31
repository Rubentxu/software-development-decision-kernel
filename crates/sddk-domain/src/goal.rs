//! Goal aggregate and hash computation.
//!
//! Mirrors `plan_hash()` (engine/lib.rs:867) for the goal lifecycle.
//! A goal is the canonical intent descriptor for a cycle; its hash
//! provides stable identity across replays.

use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};

/// Scope binding that constrains a goal to a specific project context.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ScopeBinding {
    /// Project identifier this goal is bound to.
    pub project_id: String,
    /// Optional workspace qualifier within the project.
    pub workspace: Option<String>,
}

impl ScopeBinding {
    /// Creates a new scope binding.
    pub fn new(project_id: String, workspace: Option<String>) -> Self {
        Self {
            project_id,
            workspace,
        }
    }
}

/// The canonical goal descriptor — intent + identity + scope.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Goal {
    /// Stable goal identifier (unique within the project scope).
    pub goal_id: String,
    /// Human-readable goal description.
    pub description: String,
    /// Owner principal responsible for this goal.
    pub owner: String,
    /// Scope binding constraining this goal to a project context.
    pub scope_binding: ScopeBinding,
    /// Optional cycle association.
    pub cycle_id: Option<String>,
}

impl Goal {
    /// Creates a new goal.
    pub fn new(
        goal_id: String,
        description: String,
        owner: String,
        scope_binding: ScopeBinding,
    ) -> Self {
        Self {
            goal_id,
            description,
            owner,
            scope_binding,
            cycle_id: None,
        }
    }

    /// Creates a new goal with a cycle association.
    pub fn with_cycle(mut self, cycle_id: String) -> Self {
        self.cycle_id = Some(cycle_id);
        self
    }

    /// Computes the deterministic goal hash: `sha256:<64-hex-lowercase>`.
    ///
    /// Mirrors `engine::plan_hash()` structure (lib.rs:867) for stability
    /// across replays and re-openings.
    pub fn goal_hash(&self) -> String {
        let material = serde_json::json!({
            "goal_id": &self.goal_id,
            "description": &self.description,
            "owner": &self.owner,
            "scope_binding": &self.scope_binding,
            "cycle_id": &self.cycle_id,
        });
        let digest = Sha256::digest(material.to_string().as_bytes());
        format!("sha256:{digest:x}")
    }
}

/// Registered goal event payload — emitted when a goal is persisted to the ledger.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub struct GoalRegistered {
    /// Event type constant.
    pub event_type: &'static str,
    /// SHA-256 goal hash in `sha256:<hex>` form.
    pub goal_hash: String,
    /// SHA-256 plan hash this goal was registered with.
    pub plan_hash: String,
    /// SHA-256 evidence hash associated with the goal.
    pub evidence_hash: String,
    /// Scope binding at registration time.
    pub scope_binding: ScopeBinding,
    /// RFC 3339 timestamp of registration.
    pub registered_at: String,
    /// Frame correlation ID (format: `frame:<id>`).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub frame_id: Option<String>,
    /// Command correlation ID (format: `cmd:<id>`).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub command_id: Option<String>,
}

impl GoalRegistered {
    /// Creates a new goal registered event.
    pub fn new(
        goal_hash: String,
        plan_hash: String,
        evidence_hash: String,
        scope_binding: ScopeBinding,
        registered_at: String,
    ) -> Self {
        Self {
            event_type: "goal.registered",
            goal_hash,
            plan_hash,
            evidence_hash,
            scope_binding,
            registered_at,
            frame_id: None,
            command_id: None,
        }
    }

    /// Builder: sets the frame_id (format: `frame:<id>`).
    pub fn with_frame_id(mut self, frame_id: String) -> Self {
        self.frame_id = Some(frame_id);
        self
    }

    /// Builder: sets the command_id (format: `cmd:<id>`).
    pub fn with_command_id(mut self, command_id: String) -> Self {
        self.command_id = Some(command_id);
        self
    }
}

/// Result of checking whether a goal's evidence is current.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum GoalUpToDateResult {
    /// The goal evidence is current; inputs match the registered hash.
    UpToDate,
    /// The evidence hash does not match the current computed hash.
    HashMismatch {
        /// Hash that was registered.
        registered: String,
        /// Hash computed from current inputs.
        current: String,
    },
    /// The evidence could not be read (missing or corrupt).
    EvidenceUnreadable {
        /// Reason the evidence is unreadable.
        reason: String,
    },
}

impl GoalRegistered {
    /// Checks whether this registered goal is up-to-date with respect to
    /// a computed evidence hash.
    ///
    /// Fails closed: if evidence cannot be read, returns `EvidenceUnreadable`.
    /// If the computed hash differs from the registered hash, returns `HashMismatch`.
    /// If they match, returns `UpToDate`.
    pub fn is_up_to_date(
        &self,
        current_evidence_hash: Result<&str, &'static str>,
    ) -> GoalUpToDateResult {
        let current_hash = match current_evidence_hash {
            Ok(h) => h,
            Err(e) => {
                return GoalUpToDateResult::EvidenceUnreadable {
                    reason: e.to_string(),
                };
            }
        };
        if self.evidence_hash == current_hash {
            GoalUpToDateResult::UpToDate
        } else {
            GoalUpToDateResult::HashMismatch {
                registered: self.evidence_hash.clone(),
                current: current_hash.to_string(),
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    // ANTI-TAUTOLOGY RED test: this test verifies goal_hash produces a
    // non-placeholder hash. If the implementation were a no-op ("placeholder"
    // or empty string), this test would FAIL — exposing the tautology.
    //
    // REVERT EVIDENCE (cycle-36): a prior implementation returned
    // "sha256:placeholder" for all goals, making all hash-based lookups
    // collide silently. This test was added to prevent that regression.
    #[test]
    fn goal_hash_is_not_placeholder() {
        let goal = Goal::new(
            "g-1".into(),
            "test goal".into(),
            "owner-1".into(),
            ScopeBinding::new("p-1".into(), None),
        );
        let h = goal.goal_hash();
        assert!(
            h.starts_with("sha256:"),
            "goal_hash must start with sha256: prefix"
        );
        assert!(
            h.len() > "sha256:".len() + 10,
            "goal_hash must be a real digest, not truncated"
        );
        assert_ne!(
            h, "sha256:placeholder",
            "goal_hash must not return placeholder — revert evidence from cycle-36"
        );
    }

    #[test]
    fn goal_hash_is_deterministic() {
        let goal1 = Goal::new(
            "g-1".into(),
            "same description".into(),
            "same-owner".into(),
            ScopeBinding::new("p-1".into(), Some("w-1".into())),
        );
        let goal2 = Goal::new(
            "g-1".into(),
            "same description".into(),
            "same-owner".into(),
            ScopeBinding::new("p-1".into(), Some("w-1".into())),
        );
        assert_eq!(goal1.goal_hash(), goal2.goal_hash());
    }

    #[test]
    fn goal_hash_differs_on_different_input() {
        let goal_a = Goal::new(
            "g-a".into(),
            "description A".into(),
            "owner-a".into(),
            ScopeBinding::new("p-1".into(), None),
        );
        let goal_b = Goal::new(
            "g-b".into(),
            "description B".into(),
            "owner-b".into(),
            ScopeBinding::new("p-1".into(), None),
        );
        assert_ne!(goal_a.goal_hash(), goal_b.goal_hash());
    }

    #[test]
    fn goal_registered_event_type_is_goal_registered() {
        let registered = GoalRegistered::new(
            "sha256:abc123".into(),
            "sha256:def456".into(),
            "sha256:ghi789".into(),
            ScopeBinding::new("p-1".into(), None),
            "2026-08-27T12:00:00Z".into(),
        );
        assert_eq!(registered.event_type, "goal.registered");
    }

    #[test]
    fn goal_registered_frame_id_builder() {
        let registered = GoalRegistered::new(
            "sha256:abc123".into(),
            "sha256:def456".into(),
            "sha256:ghi789".into(),
            ScopeBinding::new("p-1".into(), None),
            "2026-08-27T12:00:00Z".into(),
        )
        .with_frame_id("frame:cmd-test-1".into());
        assert_eq!(registered.frame_id, Some("frame:cmd-test-1".into()));
        assert!(registered.command_id.is_none());
    }

    #[test]
    fn goal_registered_command_id_builder() {
        let registered = GoalRegistered::new(
            "sha256:abc123".into(),
            "sha256:def456".into(),
            "sha256:ghi789".into(),
            ScopeBinding::new("p-1".into(), None),
            "2026-08-27T12:00:00Z".into(),
        )
        .with_command_id("cmd:test-cmd-1".into());
        assert_eq!(registered.command_id, Some("cmd:test-cmd-1".into()));
        assert!(registered.frame_id.is_none());
    }

    #[test]
    fn goal_registered_both_ids_builder() {
        let registered = GoalRegistered::new(
            "sha256:abc123".into(),
            "sha256:def456".into(),
            "sha256:ghi789".into(),
            ScopeBinding::new("p-1".into(), None),
            "2026-08-27T12:00:00Z".into(),
        )
        .with_frame_id("frame:cmd-1".into())
        .with_command_id("cmd:build-1".into());
        assert_eq!(registered.frame_id, Some("frame:cmd-1".into()));
        assert_eq!(registered.command_id, Some("cmd:build-1".into()));
    }

    #[test]
    fn goal_up_to_date_matches() {
        let registered = GoalRegistered::new(
            "sha256:abc123".into(),
            "sha256:def456".into(),
            "sha256:evidence123".into(),
            ScopeBinding::new("p-1".into(), None),
            "2026-08-27T12:00:00Z".into(),
        );
        let result = registered.is_up_to_date(Ok("sha256:evidence123"));
        assert!(matches!(result, GoalUpToDateResult::UpToDate));
    }

    #[test]
    fn goal_up_to_date_hash_mismatch() {
        let registered = GoalRegistered::new(
            "sha256:abc123".into(),
            "sha256:def456".into(),
            "sha256:original".into(),
            ScopeBinding::new("p-1".into(), None),
            "2026-08-27T12:00:00Z".into(),
        );
        let result = registered.is_up_to_date(Ok("sha256:updated"));
        match result {
            GoalUpToDateResult::HashMismatch {
                registered: r,
                current: c,
            } => {
                assert_eq!(r, "sha256:original");
                assert_eq!(c, "sha256:updated");
            }
            other => panic!("expected HashMismatch, got {:?}", other),
        }
    }

    #[test]
    fn goal_up_to_date_evidence_unreadable() {
        let registered = GoalRegistered::new(
            "sha256:abc123".into(),
            "sha256:def456".into(),
            "sha256:evidence123".into(),
            ScopeBinding::new("p-1".into(), None),
            "2026-08-27T12:00:00Z".into(),
        );
        let result = registered.is_up_to_date(Err("evidence file missing"));
        match result {
            GoalUpToDateResult::EvidenceUnreadable { reason } => {
                assert_eq!(reason, "evidence file missing");
            }
            other => panic!("expected EvidenceUnreadable, got {:?}", other),
        }
    }
}
