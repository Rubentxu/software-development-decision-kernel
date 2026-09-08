//! Agent Role Contract substrate — turns role boundaries from prompt
//! convention into machine-validatable contracts.
//!
//! Spec: ~/.sddk-knowledge/sddk-framework/specs/engine/REQ-AgentRoleContract.md
//! ADR:  ~/.sddk-knowledge/sddk-framework/adrs/ADR-083-AGENT-ROLE-CONTRACT.md

use std::collections::BTreeMap;
use std::sync::{Arc, Mutex};

use serde::{Deserialize, Serialize};

/// Functional role of an agent inside a workflow (CDD-ROLE-001 §RoleKind).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
#[non_exhaustive]
pub enum RoleKind {
    /// Top-level workflow orchestrator. Owns planning authority.
    Orchestrator,
    /// Dispatches to declared workers; never mutates planning.
    Coordinator,
    /// Executes a single capability; cannot delegate.
    Leaf,
    /// Read-only evaluator; emits verdicts but never side-effects.
    Evaluator,
    /// Advisory only; produces what-if / rejected branches.
    Advisor,
}

/// What this role may mutate at the planning/lifecycle level
/// (CDD-ROLE-001 §MutationAuthority).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum MutationAuthority {
    /// May mutate lifecycle and planning state.
    Lifecycle,
    /// May mutate only attempt-level state.
    Attempt,
    /// May NOT mutate anything (read-only path).
    None,
}

/// What the agent is asking to mutate (CDD-ROLE-001 §MutationKind).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum MutationKind {
    CreateWorkflow,
    DeleteWorkflow,
    MutatePlanning,
    MutateAttempt,
    MutateLifecycle,
    EmitDecision,
}

/// Mutation attempt: kind + target (path/ID).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct MutationRequest {
    pub kind: MutationKind,
    pub target: String,
}

impl MutationRequest {
    pub fn new(kind: MutationKind, target: impl Into<String>) -> Self {
        Self {
            kind,
            target: target.into(),
        }
    }
}

/// The role contract. Plain-data; static validation only.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub struct AgentRoleContract {
    pub role_id: String,
    pub kind: RoleKind,
    pub responsibility: String,
    /// Which leaf/coordinator roles this role may dispatch to.
    /// Empty for Leaf / Evaluator / Advisor.
    pub dispatch_allowlist: Vec<String>,
    /// Read scope (CIDs / path prefixes allowed).
    pub read_scopes: Vec<String>,
    /// Write scope (paths / mutations allowed).
    pub write_scopes: Vec<String>,
    /// Tool scope (tool IDs allowed).
    pub tool_scopes: Vec<String>,
    /// What this role may mutate at the planning/lifecycle level.
    pub mutation_authority: MutationAuthority,
    /// Max token budget per attempt.
    pub budget_tokens: u64,
    /// Schema IDs the role emits (e.g. return contracts).
    pub return_schemas: Vec<String>,
    /// Schema IDs the role accepts as input.
    pub input_schemas: Vec<String>,
    /// Who owns the final synthesis on join nodes.
    /// `None` if the role does not own any join.
    pub synthesis_owner: Option<String>,
    /// Hard-forbidden actions (e.g. "delete_workflow").
    pub forbidden_actions: Vec<String>,
    /// Capability ids this role can claim.
    pub capabilities: Vec<String>,
}

impl AgentRoleContract {
    /// Construct a minimal contract; builder-style via field access.
    pub fn new(
        role_id: impl Into<String>,
        kind: RoleKind,
        responsibility: impl Into<String>,
    ) -> Self {
        Self {
            role_id: role_id.into(),
            kind,
            responsibility: responsibility.into(),
            dispatch_allowlist: Vec::new(),
            read_scopes: Vec::new(),
            write_scopes: Vec::new(),
            tool_scopes: Vec::new(),
            mutation_authority: MutationAuthority::None,
            budget_tokens: 0,
            return_schemas: Vec::new(),
            input_schemas: Vec::new(),
            synthesis_owner: None,
            forbidden_actions: Vec::new(),
            capabilities: Vec::new(),
        }
    }

    /// Builder helper: set dispatch_allowlist.
    pub fn with_dispatch_allowlist(mut self, allowlist: Vec<String>) -> Self {
        self.dispatch_allowlist = allowlist;
        self
    }

    /// Builder helper: set mutation_authority.
    pub fn with_mutation_authority(mut self, ma: MutationAuthority) -> Self {
        self.mutation_authority = ma;
        self
    }

    /// Builder helper: set synthesis_owner.
    pub fn with_synthesis_owner(mut self, owner: impl Into<String>) -> Self {
        self.synthesis_owner = Some(owner.into());
        self
    }

    /// Builder helper: append a forbidden action.
    pub fn forbid(mut self, action: impl Into<String>) -> Self {
        self.forbidden_actions.push(action.into());
        self
    }

    /// Builder helper: set tool_scopes.
    pub fn with_tool_scopes(mut self, scopes: Vec<String>) -> Self {
        self.tool_scopes = scopes;
        self
    }
}

/// Error taxonomy (CDD-ROLE-001 §RoleError). Closed-set,
/// `#[non_exhaustive]` for forward compatibility.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case", tag = "kind")]
#[non_exhaustive]
pub enum RoleError {
    UnknownRole {
        role_id: String,
    },
    DelegationNotAllowed {
        from: String,
        to: String,
    },
    LeafCannotDelegate {
        role_id: String,
    },
    ForbiddenAction {
        role_id: String,
        action: String,
    },
    MutationNotAuthorized {
        mutation_kind: MutationKind,
        role_id: String,
    },
    ScopeViolation {
        role_id: String,
        scope_kind: String,
        target: String,
    },
    MultipleSynthesisOwners {
        join_id: String,
        owners: Vec<String>,
    },
    NoSynthesisOwner {
        join_id: String,
    },
    AuthorityCycle {
        cycle: Vec<String>,
    },
}

impl std::fmt::Display for RoleError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            RoleError::UnknownRole { role_id } => write!(f, "unknown role: {role_id}"),
            RoleError::DelegationNotAllowed { from, to } => {
                write!(f, "delegation {from} -> {to} not allowed")
            }
            RoleError::LeafCannotDelegate { role_id } => {
                write!(f, "leaf role cannot delegate: {role_id}")
            }
            RoleError::ForbiddenAction { role_id, action } => {
                write!(f, "role {role_id} forbids action {action}")
            }
            RoleError::MutationNotAuthorized {
                role_id,
                mutation_kind,
            } => write!(f, "role {role_id} cannot mutate {:?}", mutation_kind),
            RoleError::ScopeViolation {
                role_id,
                scope_kind,
                target,
            } => {
                write!(f, "role {role_id} scope {scope_kind} violated by {target}")
            }
            RoleError::MultipleSynthesisOwners { join_id, owners } => {
                write!(
                    f,
                    "join {join_id} has multiple synthesis owners: {owners:?}"
                )
            }
            RoleError::NoSynthesisOwner { join_id } => {
                write!(f, "join {join_id} has no synthesis owner")
            }
            RoleError::AuthorityCycle { cycle } => write!(f, "authority cycle: {cycle:?}"),
        }
    }
}

impl std::error::Error for RoleError {}

/// Registry of `AgentRoleContract`s.
pub trait RoleRegistry: Send + Sync + std::fmt::Debug {
    fn register(&self, contract: AgentRoleContract);
    fn get(&self, role_id: &str) -> Option<AgentRoleContract>;
    fn all(&self) -> Vec<AgentRoleContract>;
}

/// Reference in-memory implementation backed by a Mutex<BTreeMap>.
#[derive(Debug, Default)]
pub struct InMemoryRoleRegistry {
    inner: Mutex<BTreeMap<String, AgentRoleContract>>,
}

impl InMemoryRoleRegistry {
    pub fn new() -> Self {
        Self::default()
    }
}

impl RoleRegistry for InMemoryRoleRegistry {
    fn register(&self, contract: AgentRoleContract) {
        let mut g = self
            .inner
            .lock()
            .expect("InMemoryRoleRegistry mutex poisoned");
        g.insert(contract.role_id.clone(), contract);
    }
    fn get(&self, role_id: &str) -> Option<AgentRoleContract> {
        let g = self
            .inner
            .lock()
            .expect("InMemoryRoleRegistry mutex poisoned");
        g.get(role_id).cloned()
    }
    fn all(&self) -> Vec<AgentRoleContract> {
        let g = self
            .inner
            .lock()
            .expect("InMemoryRoleRegistry mutex poisoned");
        g.values().cloned().collect()
    }
}

/// Static validator for `AgentRoleContract`s.
#[derive(Debug, Clone)]
pub struct RoleValidator {
    registry: Arc<dyn RoleRegistry>,
}

impl RoleValidator {
    pub fn new(registry: Arc<dyn RoleRegistry>) -> Self {
        Self { registry }
    }

    pub fn registry(&self) -> Arc<dyn RoleRegistry> {
        self.registry.clone()
    }

    /// Validate a delegation request: from_role -> to_role.
    /// Implements invariants (1)–(3).
    pub fn validate_delegation(&self, from: &str, to: &str) -> Result<(), RoleError> {
        let from_role = self
            .registry
            .get(from)
            .ok_or_else(|| RoleError::UnknownRole {
                role_id: from.to_string(),
            })?;

        // Invariant 1: Leaf / Evaluator / Advisor cannot delegate.
        if matches!(
            from_role.kind,
            RoleKind::Leaf | RoleKind::Evaluator | RoleKind::Advisor
        ) {
            return Err(RoleError::LeafCannotDelegate {
                role_id: from.to_string(),
            });
        }

        let to_role = self
            .registry
            .get(to)
            .ok_or_else(|| RoleError::UnknownRole {
                role_id: to.to_string(),
            })?;

        // Coordinator must declare worker in allowlist.
        if matches!(from_role.kind, RoleKind::Coordinator) {
            if !from_role.dispatch_allowlist.iter().any(|s| s == to) {
                return Err(RoleError::DelegationNotAllowed {
                    from: from.to_string(),
                    to: to.to_string(),
                });
            }
        } else if matches!(from_role.kind, RoleKind::Orchestrator) {
            // Orchestrator may delegate to coordinators only.
            if !matches!(to_role.kind, RoleKind::Coordinator) {
                return Err(RoleError::DelegationNotAllowed {
                    from: from.to_string(),
                    to: to.to_string(),
                });
            }
        }

        Ok(())
    }

    /// Validate a mutation request against a role's authority.
    /// Implements invariants (6)–(9).
    pub fn validate_mutation(&self, role_id: &str, req: &MutationRequest) -> Result<(), RoleError> {
        let role = self
            .registry
            .get(role_id)
            .ok_or_else(|| RoleError::UnknownRole {
                role_id: role_id.to_string(),
            })?;

        // Forbidden actions win.
        if role.forbidden_actions.iter().any(|a| a == &req.target) {
            // Treat the action name as the target's identity (e.g. "delete_workflow").
            return Err(RoleError::ForbiddenAction {
                role_id: role_id.to_string(),
                action: req.target.clone(),
            });
        }

        // MutationAuthority controls what kinds.
        let allowed = match role.mutation_authority {
            MutationAuthority::None => false,
            MutationAuthority::Attempt => matches!(
                req.kind,
                MutationKind::MutateAttempt | MutationKind::EmitDecision
            ),
            MutationAuthority::Lifecycle => matches!(
                req.kind,
                MutationKind::MutateAttempt
                    | MutationKind::MutateLifecycle
                    | MutationKind::EmitDecision
                    | MutationKind::MutatePlanning
                    | MutationKind::CreateWorkflow
            ),
        };
        if !allowed {
            return Err(RoleError::MutationNotAuthorized {
                role_id: role_id.to_string(),
                mutation_kind: req.kind,
            });
        }

        // DeleteWorkflow always forbidden for non-Lifecycle even if allowed above.
        if matches!(req.kind, MutationKind::DeleteWorkflow)
            && !matches!(role.mutation_authority, MutationAuthority::Lifecycle)
        {
            return Err(RoleError::MutationNotAuthorized {
                role_id: role_id.to_string(),
                mutation_kind: req.kind,
            });
        }

        Ok(())
    }

    /// Validate that a join node has exactly one synthesis owner.
    /// Implements invariant (4).
    pub fn validate_synthesis_owner(
        &self,
        join_id: &str,
        candidates: &[String],
    ) -> Result<(), RoleError> {
        let mut owners: Vec<String> = Vec::new();
        for role in self.registry.all() {
            if let Some(ref so) = role.synthesis_owner
                && so == join_id
                && candidates.iter().any(|c| c == &role.role_id)
            {
                owners.push(role.role_id.clone());
            }
        }
        match owners.len() {
            0 => Err(RoleError::NoSynthesisOwner {
                join_id: join_id.to_string(),
            }),
            1 => Ok(()),
            _ => Err(RoleError::MultipleSynthesisOwners {
                join_id: join_id.to_string(),
                owners,
            }),
        }
    }

    /// Check that a tool call is allowed by the role's tool_scopes.
    /// Implements invariants (8)–(9).
    pub fn validate_tool(&self, role_id: &str, tool_id: &str) -> Result<(), RoleError> {
        let role = self
            .registry
            .get(role_id)
            .ok_or_else(|| RoleError::UnknownRole {
                role_id: role_id.to_string(),
            })?;
        if role.tool_scopes.iter().any(|s| s == tool_id) {
            Ok(())
        } else {
            Err(RoleError::ScopeViolation {
                role_id: role_id.to_string(),
                scope_kind: "tool".to_string(),
                target: tool_id.to_string(),
            })
        }
    }

    /// Detect cycles in the dispatch graph (DFS).
    /// Implements invariant (5).
    pub fn detect_cycles(&self) -> Result<(), RoleError> {
        let all = self.registry.all();
        // Build adjacency: role_id -> [role_id...]
        let adj: BTreeMap<String, Vec<String>> = all
            .iter()
            .map(|r| (r.role_id.clone(), r.dispatch_allowlist.clone()))
            .collect();

        let mut visited: std::collections::BTreeSet<String> = std::collections::BTreeSet::new();
        let mut on_stack: std::collections::BTreeSet<String> = std::collections::BTreeSet::new();
        let mut path: Vec<String> = Vec::new();

        for role in &all {
            if !visited.contains(&role.role_id)
                && let Some(cycle) =
                    dfs_cycle(&role.role_id, &adj, &mut visited, &mut on_stack, &mut path)
            {
                return Err(RoleError::AuthorityCycle { cycle });
            }
        }
        Ok(())
    }
}

fn dfs_cycle(
    node: &str,
    adj: &BTreeMap<String, Vec<String>>,
    visited: &mut std::collections::BTreeSet<String>,
    on_stack: &mut std::collections::BTreeSet<String>,
    path: &mut Vec<String>,
) -> Option<Vec<String>> {
    visited.insert(node.to_string());
    on_stack.insert(node.to_string());
    path.push(node.to_string());

    if let Some(neighbors) = adj.get(node) {
        for n in neighbors {
            if !visited.contains(n) {
                if let Some(cycle) = dfs_cycle(n, adj, visited, on_stack, path) {
                    return Some(cycle);
                }
            } else if on_stack.contains(n) {
                // Found back edge; report cycle path.
                if let Some(start) = path.iter().position(|p| p == n) {
                    let mut cycle: Vec<String> = path[start..].to_vec();
                    cycle.push(n.to_string());
                    return Some(cycle);
                }
            }
        }
    }

    on_stack.remove(node);
    path.pop();
    None
}

#[cfg(test)]
mod tests {
    use super::*;

    fn registry_with(roles: Vec<AgentRoleContract>) -> (Arc<InMemoryRoleRegistry>, RoleValidator) {
        let r = Arc::new(InMemoryRoleRegistry::new());
        for c in roles {
            r.register(c);
        }
        let v = RoleValidator::new(r.clone());
        (r, v)
    }

    fn leaf(id: &str) -> AgentRoleContract {
        AgentRoleContract::new(id, RoleKind::Leaf, "does leaf work")
    }

    fn coord(id: &str, allow: Vec<&str>) -> AgentRoleContract {
        AgentRoleContract::new(id, RoleKind::Coordinator, "coordinates")
            .with_dispatch_allowlist(allow.into_iter().map(String::from).collect())
    }

    #[allow(dead_code)]
    fn orch(id: &str) -> AgentRoleContract {
        AgentRoleContract::new(id, RoleKind::Orchestrator, "owns planning")
    }

    #[test]
    fn leaf_cannot_delegate() {
        let (_, v) = registry_with(vec![leaf("leaf-x")]);
        let err = v.validate_delegation("leaf-x", "anything").unwrap_err();
        assert!(matches!(err, RoleError::LeafCannotDelegate { .. }));
    }

    #[test]
    fn coord_dispatches_declared_worker() {
        let (_, v) = registry_with(vec![coord("coord-y", vec!["leaf-x"]), leaf("leaf-x")]);
        v.validate_delegation("coord-y", "leaf-x").unwrap();
    }

    #[test]
    fn coord_dispatches_unknown_role() {
        let (_, v) = registry_with(vec![coord("coord-y", vec!["leaf-missing"])]);
        let err = v
            .validate_delegation("coord-y", "leaf-missing")
            .unwrap_err();
        assert!(matches!(err, RoleError::UnknownRole { .. }));
    }

    #[test]
    fn one_synthesis_owner_per_join() {
        let c1 = leaf("leaf-a").with_synthesis_owner("join-1");
        let c2 = leaf("leaf-b").with_synthesis_owner("join-1");
        let (_, v) = registry_with(vec![c1, c2]);
        let err = v
            .validate_synthesis_owner("join-1", &["leaf-a".into(), "leaf-b".into()])
            .unwrap_err();
        assert!(matches!(err, RoleError::MultipleSynthesisOwners { .. }));
    }

    #[test]
    fn no_synthesis_owner_per_join() {
        let (_, v) = registry_with(vec![leaf("leaf-a")]);
        let err = v
            .validate_synthesis_owner("join-1", &["leaf-a".into()])
            .unwrap_err();
        assert!(matches!(err, RoleError::NoSynthesisOwner { .. }));
    }

    #[test]
    fn authority_cycle_detection() {
        let a = leaf("a").with_dispatch_allowlist(vec!["b".into()]);
        let b = leaf("b").with_dispatch_allowlist(vec!["a".into()]);
        // Note: leaves normally reject dispatch validation, but cycle detection
        // operates on raw adjacency; we test it as a structural probe.
        let (_, v) = registry_with(vec![a, b]);
        let err = v.detect_cycles().unwrap_err();
        assert!(matches!(err, RoleError::AuthorityCycle { .. }));
    }

    #[test]
    fn leaf_cannot_mutate_lifecycle() {
        let (_, v) = registry_with(vec![leaf("leaf-x")]);
        let req = MutationRequest::new(MutationKind::MutateLifecycle, "node-1");
        let err = v.validate_mutation("leaf-x", &req).unwrap_err();
        assert!(matches!(err, RoleError::MutationNotAuthorized { .. }));
    }

    #[test]
    fn forbidden_action_rejection() {
        let c = leaf("leaf-x").forbid("delete_workflow");
        let (_, v) = registry_with(vec![c]);
        let req = MutationRequest::new(MutationKind::DeleteWorkflow, "delete_workflow");
        let err = v.validate_mutation("leaf-x", &req).unwrap_err();
        assert!(matches!(err, RoleError::ForbiddenAction { .. }));
    }

    #[test]
    fn tool_scope_positive() {
        let c = leaf("leaf-x").with_tool_scopes(vec!["git.status".into()]);
        let (_, v) = registry_with(vec![c]);
        v.validate_tool("leaf-x", "git.status").unwrap();
    }

    #[test]
    fn tool_scope_violation() {
        let c = leaf("leaf-x").with_tool_scopes(vec!["git.status".into()]);
        let (_, v) = registry_with(vec![c]);
        let err = v.validate_tool("leaf-x", "shell.exec").unwrap_err();
        assert!(matches!(err, RoleError::ScopeViolation { .. }));
    }

    #[test]
    fn read_only_evaluator() {
        let mut c = AgentRoleContract::new("eval-x", RoleKind::Evaluator, "evaluates");
        c.mutation_authority = MutationAuthority::None;
        let (_, v) = registry_with(vec![c]);
        let req = MutationRequest::new(MutationKind::EmitDecision, "node-1");
        let err = v.validate_mutation("eval-x", &req).unwrap_err();
        assert!(matches!(err, RoleError::MutationNotAuthorized { .. }));
    }
}
