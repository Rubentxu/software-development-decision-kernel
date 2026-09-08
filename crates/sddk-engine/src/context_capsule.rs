//! Context Capsule Compiler — produces deterministic, bounded,
//! provenance-aware context capsules per SPEC-021 / ADR-028.
//!
//! Spec: ~/.sddk-knowledge/sddk-framework/specs/engine/REQ-ContextCapsuleCompiler.md
//! ADR:  ~/.sddk-knowledge/sddk-framework/adrs/ADR-081-CONTEXT-CAPSULE-COMPILER.md

use std::collections::HashMap;
use std::sync::Arc;

use serde::{Deserialize, Serialize};

use crate::retry::Clock;
use crate::run_view::ProvenanceLink;

// ── Capsule value types (SPEC-021 §Schema) ──────────────────────────────────

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ContextCapsule {
    pub capsule_id: String,
    pub for_target: CapsuleTarget,
    pub objective: String,
    pub definition_of_done: Vec<String>,
    pub scope: Scope,
    pub constraints: Vec<String>,
    pub decisions: CapsuleDecisions,
    pub assumptions: Vec<Assumption>,
    pub open_questions: Vec<String>,
    pub artifacts: ArtifactBundle,
    pub negative_knowledge: Vec<NegativeKnowledge>,
    pub changes_since_parent: Vec<ChangeEntry>,
    pub evidence_required: Vec<String>,
    pub tools_allowed: Vec<String>,
    pub budget: CapsuleBudget,
    pub return_contract: String,
    pub recovery: RecoveryState,
    pub provenance: Vec<ProvenanceLink>,
    pub staleness: StalenessReport,
}

#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct CapsuleTarget {
    pub workflow_run: String,
    pub node_run: String,
    pub attempt: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Default, Serialize, Deserialize)]
pub struct Scope {
    pub include: Vec<String>,
    pub exclude: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct CapsuleDecisions {
    pub accepted: Vec<String>,
    pub rejected: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Assumption {
    pub text: String,
    pub confidence: f64,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ArtifactBundle {
    pub must_read: Vec<String>,
    pub relevant: Vec<String>,
    pub fetch_on_demand: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct NegativeKnowledge {
    pub claim: String,
    pub status: NegativeStatus,
    pub reason: String,
    pub evidence_ref: Option<String>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum NegativeStatus {
    RuledOut,
    Pending,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ChangeEntry {
    pub kind: ChangeKind,
    pub ref_id: String,
    pub summary: String,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ChangeKind {
    Added,
    Modified,
    Invalidated,
    UnchangedReusable,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct CapsuleBudget {
    pub max_tokens: u64,
    pub actual_tokens: u64,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct RecoveryState {
    pub previous_attempt: Option<String>,
    pub previous_capsule: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct StalenessReport {
    pub stale_refs: Vec<StaleRef>,
    pub as_of_ms: i64,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct StaleRef {
    pub ref_id: String,
    pub reason: StaleReason,
    pub detected_at_ms: i64,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum StaleReason {
    ContentChangedSinceCompile,
    DecisionOverridden,
    ArtifactRemoved,
    ProviderRevoked,
    BeyondTtl,
}

// ── CapsuleInputs trait ─────────────────────────────────────────────────────

/// Input seam for the compiler. Mirrors `LeaseStore` /
/// `CircuitBreakerStore` (Send + Sync + Debug).
pub trait CapsuleInputs: Send + Sync + std::fmt::Debug {
    fn objective(&self) -> &str;
    fn definition_of_done(&self) -> Vec<String>;
    fn scope(&self) -> Scope;
    fn constraints(&self) -> Vec<String>;
    fn accepted_decisions(&self) -> Vec<String>;
    fn rejected_decisions(&self) -> Vec<String>;
    fn assumptions(&self) -> Vec<Assumption>;
    fn must_read(&self) -> Vec<String>;
    fn relevant(&self) -> Vec<String>;
    fn fetch_on_demand(&self) -> Vec<String>;
    fn negative_knowledge(&self) -> Vec<NegativeKnowledge>;
    fn previous_capsule(&self) -> Option<ContextCapsule>;
    fn parent_target(&self) -> Option<CapsuleTarget>;
    fn provenance(&self) -> Vec<ProvenanceLink>;
    /// Timestamp the ref was last seen. Used by TTL staleness.
    fn last_seen_at_ms(&self, ref_id: &str) -> Option<i64>;
    /// `Some(ts)` if content changed since compile.
    fn content_changed_since(&self, ref_id: &str) -> Option<i64>;
    /// `Some(ts)` if a decision was overridden.
    fn decision_overridden_at(&self, ref_id: &str) -> Option<i64>;
    /// `Some(ts)` if an artifact was removed.
    fn artifact_removed_at(&self, ref_id: &str) -> Option<i64>;
    /// `Some(ts)` if a provider was revoked.
    fn provider_revoked_at(&self, ref_id: &str) -> Option<i64>;
    /// Tags a ref so scope filtering can decide inclusion.
    fn ref_tags(&self, ref_id: &str) -> Vec<String>;
}

// ── CapsuleError ────────────────────────────────────────────────────────────

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case", tag = "kind")]
#[non_exhaustive]
pub enum CapsuleError {
    BudgetExceeded { actual: u64, max: u64 },
    MissingRequired { field: String },
    StaleNegativeKnowledge { claim: String },
    InvalidParent { reason: String },
}

impl std::fmt::Display for CapsuleError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::BudgetExceeded { actual, max } => {
                write!(f, "budget exceeded: actual={actual} max={max}")
            }
            Self::MissingRequired { field } => write!(f, "missing required field: {field}"),
            Self::StaleNegativeKnowledge { claim } => {
                write!(f, "stale negative knowledge: {claim}")
            }
            Self::InvalidParent { reason } => write!(f, "invalid parent capsule: {reason}"),
        }
    }
}

impl std::error::Error for CapsuleError {}

// ── CompilerPolicy ──────────────────────────────────────────────────────────

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct CompilerPolicy {
    pub max_tokens: u64,
    pub staleness_ttl_ms: i64,
    pub preserve_negative_knowledge: bool,
}

impl Default for CompilerPolicy {
    fn default() -> Self {
        Self {
            max_tokens: 80_000,
            staleness_ttl_ms: 3_600_000,
            preserve_negative_knowledge: true,
        }
    }
}

// ── ContextCompiler ────────────────────────────────────────────────────────

pub struct ContextCompiler {
    inputs: Arc<dyn CapsuleInputs>,
    clock: Arc<dyn Clock>,
    policy: CompilerPolicy,
}

impl std::fmt::Debug for ContextCompiler {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("ContextCompiler")
            .field("inputs", &"...")
            .field("policy", &self.policy)
            .finish()
    }
}

impl ContextCompiler {
    pub fn new(inputs: Arc<dyn CapsuleInputs>, clock: Arc<dyn Clock>) -> Self {
        Self {
            inputs,
            clock,
            policy: CompilerPolicy::default(),
        }
    }

    pub fn with_policy(mut self, policy: CompilerPolicy) -> Self {
        self.policy = policy;
        self
    }

    pub fn policy(&self) -> &CompilerPolicy {
        &self.policy
    }

    pub fn inputs(&self) -> &Arc<dyn CapsuleInputs> {
        &self.inputs
    }

    /// Compile a fresh capsule. Deterministic given identical inputs +
    /// clock + policy.
    pub fn compile(&self, target: CapsuleTarget) -> Result<ContextCapsule, CapsuleError> {
        let objective = self.inputs.objective().to_string();
        if objective.trim().is_empty() {
            return Err(CapsuleError::MissingRequired {
                field: "objective".to_string(),
            });
        }

        let must_read = self.filter_scope(self.inputs.must_read());
        if must_read.is_empty() {
            return Err(CapsuleError::MissingRequired {
                field: "must_read".to_string(),
            });
        }
        let relevant = self.filter_scope(self.inputs.relevant());
        let fetch_on_demand = self.inputs.fetch_on_demand();

        let acceptance = CapsuleDecisions {
            accepted: self.inputs.accepted_decisions(),
            rejected: self.inputs.rejected_decisions(),
        };
        let assumptions = self.inputs.assumptions();
        let negative_knowledge = self.inputs.negative_knowledge();
        if let Some(stale) = self.find_stale_negative(&negative_knowledge) {
            return Err(CapsuleError::StaleNegativeKnowledge { claim: stale });
        }
        let provenance = self.inputs.provenance();
        let recovery = RecoveryState {
            previous_attempt: self
                .inputs
                .parent_target()
                .as_ref()
                .map(|t| t.attempt.clone()),
            previous_capsule: self
                .inputs
                .previous_capsule()
                .as_ref()
                .map(|p| p.capsule_id.clone()),
        };

        let capsule_id = format!(
            "{}:{}:{}",
            target.workflow_run, target.node_run, target.attempt
        );
        let as_of_ms = self.clock.now_ms() as i64;

        let budget = self.compute_budget(
            &must_read,
            &relevant,
            &fetch_on_demand,
            &negative_knowledge,
            &assumptions,
            &provenance,
            target.workflow_run.len() as u64,
        )?;
        if budget.actual_tokens > self.policy.max_tokens {
            return Err(CapsuleError::BudgetExceeded {
                actual: budget.actual_tokens,
                max: self.policy.max_tokens,
            });
        }

        let stale_refs = self.collect_stale_refs(&must_read, as_of_ms);

        Ok(ContextCapsule {
            capsule_id,
            for_target: target,
            objective,
            definition_of_done: self.inputs.definition_of_done(),
            scope: self.inputs.scope(),
            constraints: self.inputs.constraints(),
            decisions: acceptance,
            assumptions,
            open_questions: Vec::new(),
            artifacts: ArtifactBundle {
                must_read,
                relevant,
                fetch_on_demand,
            },
            negative_knowledge,
            changes_since_parent: Vec::new(),
            evidence_required: Vec::new(),
            tools_allowed: Vec::new(),
            budget,
            return_contract: String::new(),
            recovery,
            provenance,
            staleness: StalenessReport {
                stale_refs,
                as_of_ms,
            },
        })
    }

    /// Compile a delta from `parent` to current inputs (SPEC-021 §Delta).
    pub fn compile_delta(
        &self,
        target: CapsuleTarget,
        parent: &ContextCapsule,
    ) -> Result<ContextCapsule, CapsuleError> {
        if parent.capsule_id.is_empty() {
            return Err(CapsuleError::InvalidParent {
                reason: "empty capsule_id".to_string(),
            });
        }

        let mut current = self.compile(target.clone())?;

        let attempt_seq = target
            .attempt
            .rsplit('-')
            .next()
            .and_then(|s| s.parse::<u32>().ok())
            .unwrap_or(0);
        current.capsule_id = format!("{}:delta:{}", parent.capsule_id, attempt_seq);
        current.recovery.previous_attempt = Some(parent.for_target.attempt.clone());
        current.recovery.previous_capsule = Some(parent.capsule_id.clone());
        current.changes_since_parent = self.diff_artifacts(parent, &current);
        current.negative_knowledge = if self.policy.preserve_negative_knowledge {
            self.merge_negative(parent, &current)
        } else {
            current.negative_knowledge
        };
        Ok(current)
    }

    fn filter_scope(&self, refs: Vec<String>) -> Vec<String> {
        let scope = self.inputs.scope();
        let excluded: std::collections::HashSet<&str> =
            scope.exclude.iter().map(|s| s.as_str()).collect();
        refs.into_iter()
            .filter(|r| {
                let tags = self.inputs.ref_tags(r);
                !tags.iter().any(|t| excluded.contains(t.as_str()))
            })
            .collect()
    }

    fn find_stale_negative(&self, items: &[NegativeKnowledge]) -> Option<String> {
        items
            .iter()
            .find(|n| n.status == NegativeStatus::RuledOut && n.evidence_ref.is_none())
            .map(|n| n.claim.clone())
    }

    #[allow(clippy::too_many_arguments)]
    fn compute_budget(
        &self,
        must_read: &[String],
        relevant: &[String],
        fetch_on_demand: &[String],
        negative: &[NegativeKnowledge],
        assumptions: &[Assumption],
        provenance: &[ProvenanceLink],
        overhead: u64,
    ) -> Result<CapsuleBudget, CapsuleError> {
        let chars: u64 = must_read
            .iter()
            .chain(relevant)
            .chain(fetch_on_demand)
            .map(|s| s.len() as u64)
            .sum::<u64>()
            + negative
                .iter()
                .map(|n| n.claim.len() as u64 + n.reason.len() as u64)
                .sum::<u64>()
            + assumptions.iter().map(|a| a.text.len() as u64).sum::<u64>()
            + overhead;
        let provenance_chars: u64 = provenance
            .iter()
            .map(|p| p.reference().len() as u64 + 32)
            .sum();
        let actual = (chars + provenance_chars) / 4 + 64;
        Ok(CapsuleBudget {
            max_tokens: self.policy.max_tokens,
            actual_tokens: actual,
        })
    }

    fn collect_stale_refs(&self, refs: &[String], as_of_ms: i64) -> Vec<StaleRef> {
        let mut out = Vec::new();
        for r in refs {
            if let Some(ts) = self.inputs.content_changed_since(r) {
                out.push(StaleRef {
                    ref_id: r.clone(),
                    reason: StaleReason::ContentChangedSinceCompile,
                    detected_at_ms: ts,
                });
                continue;
            }
            if let Some(ts) = self.inputs.decision_overridden_at(r) {
                out.push(StaleRef {
                    ref_id: r.clone(),
                    reason: StaleReason::DecisionOverridden,
                    detected_at_ms: ts,
                });
                continue;
            }
            if let Some(ts) = self.inputs.artifact_removed_at(r) {
                out.push(StaleRef {
                    ref_id: r.clone(),
                    reason: StaleReason::ArtifactRemoved,
                    detected_at_ms: ts,
                });
                continue;
            }
            if let Some(ts) = self.inputs.provider_revoked_at(r) {
                out.push(StaleRef {
                    ref_id: r.clone(),
                    reason: StaleReason::ProviderRevoked,
                    detected_at_ms: ts,
                });
                continue;
            }
            if let Some(last) = self.inputs.last_seen_at_ms(r)
                && as_of_ms.saturating_sub(last) >= self.policy.staleness_ttl_ms
            {
                out.push(StaleRef {
                    ref_id: r.clone(),
                    reason: StaleReason::BeyondTtl,
                    detected_at_ms: last,
                });
            }
        }
        out
    }

    fn diff_artifacts(
        &self,
        parent: &ContextCapsule,
        current: &ContextCapsule,
    ) -> Vec<ChangeEntry> {
        let mut out = Vec::new();
        let parent_set: std::collections::HashSet<&String> =
            parent.artifacts.must_read.iter().collect();
        let current_set: std::collections::HashSet<&String> =
            current.artifacts.must_read.iter().collect();
        for r in &current.artifacts.must_read {
            if !parent_set.contains(r) {
                out.push(ChangeEntry {
                    kind: ChangeKind::Added,
                    ref_id: r.clone(),
                    summary: "added since parent".to_string(),
                });
            }
        }
        for r in &parent.artifacts.must_read {
            if current_set.contains(r) {
                out.push(ChangeEntry {
                    kind: ChangeKind::UnchangedReusable,
                    ref_id: r.clone(),
                    summary: "unchanged reusable".to_string(),
                });
            } else {
                out.push(ChangeEntry {
                    kind: ChangeKind::Invalidated,
                    ref_id: r.clone(),
                    summary: "no longer in must_read".to_string(),
                });
            }
        }
        out
    }

    fn merge_negative(
        &self,
        parent: &ContextCapsule,
        current: &ContextCapsule,
    ) -> Vec<NegativeKnowledge> {
        let mut out: Vec<NegativeKnowledge> = parent.negative_knowledge.clone();
        let existing: std::collections::HashSet<String> =
            out.iter().map(|n| n.claim.clone()).collect();
        for n in &current.negative_knowledge {
            if !existing.contains(&n.claim) {
                out.push(n.clone());
            }
        }
        out
    }
}

// ── InMemoryCapsuleInputs (test adapter) ────────────────────────────────────

#[derive(Debug, Default, Clone)]
pub struct InMemoryCapsuleInputs {
    objective: String,
    definition_of_done: Vec<String>,
    scope: Scope,
    constraints: Vec<String>,
    accepted_decisions: Vec<String>,
    rejected_decisions: Vec<String>,
    assumptions: Vec<Assumption>,
    must_read: Vec<String>,
    relevant: Vec<String>,
    fetch_on_demand: Vec<String>,
    negative_knowledge: Vec<NegativeKnowledge>,
    previous_capsule: Option<ContextCapsule>,
    parent_target: Option<CapsuleTarget>,
    provenance: Vec<ProvenanceLink>,
    last_seen: HashMap<String, i64>,
    content_changed: HashMap<String, i64>,
    decision_overridden: HashMap<String, i64>,
    artifact_removed: HashMap<String, i64>,
    provider_revoked: HashMap<String, i64>,
    ref_tags: HashMap<String, Vec<String>>,
}

impl InMemoryCapsuleInputs {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn with_objective(mut self, s: impl Into<String>) -> Self {
        self.objective = s.into();
        self
    }
    pub fn with_dod(mut self, v: Vec<String>) -> Self {
        self.definition_of_done = v;
        self
    }
    pub fn with_scope(mut self, s: Scope) -> Self {
        self.scope = s;
        self
    }
    pub fn with_constraints(mut self, v: Vec<String>) -> Self {
        self.constraints = v;
        self
    }
    pub fn with_accepted(mut self, v: Vec<String>) -> Self {
        self.accepted_decisions = v;
        self
    }
    pub fn with_rejected(mut self, v: Vec<String>) -> Self {
        self.rejected_decisions = v;
        self
    }
    pub fn with_assumptions(mut self, v: Vec<Assumption>) -> Self {
        self.assumptions = v;
        self
    }
    pub fn with_must_read(mut self, v: Vec<String>) -> Self {
        self.must_read = v;
        self
    }
    pub fn with_relevant(mut self, v: Vec<String>) -> Self {
        self.relevant = v;
        self
    }
    pub fn with_fetch_on_demand(mut self, v: Vec<String>) -> Self {
        self.fetch_on_demand = v;
        self
    }
    pub fn with_negative_knowledge(mut self, v: Vec<NegativeKnowledge>) -> Self {
        self.negative_knowledge = v;
        self
    }
    pub fn with_previous_capsule(mut self, c: ContextCapsule) -> Self {
        self.parent_target = Some(c.for_target.clone());
        self.previous_capsule = Some(c);
        self
    }
    pub fn with_provenance(mut self, v: Vec<ProvenanceLink>) -> Self {
        self.provenance = v;
        self
    }
    pub fn with_last_seen(mut self, k: &str, ts: i64) -> Self {
        self.last_seen.insert(k.to_string(), ts);
        self
    }
    pub fn with_content_changed(mut self, k: &str, ts: i64) -> Self {
        self.content_changed.insert(k.to_string(), ts);
        self
    }
    pub fn with_decision_overridden(mut self, k: &str, ts: i64) -> Self {
        self.decision_overridden.insert(k.to_string(), ts);
        self
    }
    pub fn with_artifact_removed(mut self, k: &str, ts: i64) -> Self {
        self.artifact_removed.insert(k.to_string(), ts);
        self
    }
    pub fn with_provider_revoked(mut self, k: &str, ts: i64) -> Self {
        self.provider_revoked.insert(k.to_string(), ts);
        self
    }
    pub fn with_ref_tags(mut self, k: &str, tags: Vec<String>) -> Self {
        self.ref_tags.insert(k.to_string(), tags);
        self
    }
}

impl CapsuleInputs for InMemoryCapsuleInputs {
    fn objective(&self) -> &str {
        &self.objective
    }
    fn definition_of_done(&self) -> Vec<String> {
        self.definition_of_done.clone()
    }
    fn scope(&self) -> Scope {
        self.scope.clone()
    }
    fn constraints(&self) -> Vec<String> {
        self.constraints.clone()
    }
    fn accepted_decisions(&self) -> Vec<String> {
        self.accepted_decisions.clone()
    }
    fn rejected_decisions(&self) -> Vec<String> {
        self.rejected_decisions.clone()
    }
    fn assumptions(&self) -> Vec<Assumption> {
        self.assumptions.clone()
    }
    fn must_read(&self) -> Vec<String> {
        self.must_read.clone()
    }
    fn relevant(&self) -> Vec<String> {
        self.relevant.clone()
    }
    fn fetch_on_demand(&self) -> Vec<String> {
        self.fetch_on_demand.clone()
    }
    fn negative_knowledge(&self) -> Vec<NegativeKnowledge> {
        self.negative_knowledge.clone()
    }
    fn previous_capsule(&self) -> Option<ContextCapsule> {
        self.previous_capsule.clone()
    }
    fn parent_target(&self) -> Option<CapsuleTarget> {
        self.parent_target.clone()
    }
    fn provenance(&self) -> Vec<ProvenanceLink> {
        self.provenance.clone()
    }
    fn last_seen_at_ms(&self, ref_id: &str) -> Option<i64> {
        self.last_seen.get(ref_id).copied()
    }
    fn content_changed_since(&self, ref_id: &str) -> Option<i64> {
        self.content_changed.get(ref_id).copied()
    }
    fn decision_overridden_at(&self, ref_id: &str) -> Option<i64> {
        self.decision_overridden.get(ref_id).copied()
    }
    fn artifact_removed_at(&self, ref_id: &str) -> Option<i64> {
        self.artifact_removed.get(ref_id).copied()
    }
    fn provider_revoked_at(&self, ref_id: &str) -> Option<i64> {
        self.provider_revoked.get(ref_id).copied()
    }
    fn ref_tags(&self, ref_id: &str) -> Vec<String> {
        self.ref_tags.get(ref_id).cloned().unwrap_or_default()
    }
}
