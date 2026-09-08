//! Cold-Start Recovery substrate — `AgentHost::cold_start` integrates
//! `RunStateView` + last `ContextCapsule` into a deterministic
//! recovery capsule per SPEC-021 / ADR-028 / CTX-COMPILER-002.
//!
//! Spec: ~/.sddk-knowledge/sddk-framework/specs/engine/REQ-ColdStartRecovery.md
//! ADR:  ~/.sddk-knowledge/sddk-framework/adrs/ADR-082-AGENT-HOST-COLD-START.md

use std::collections::{BTreeMap, BTreeSet};
use std::sync::Mutex;

use serde::{Deserialize, Serialize};

use crate::context_capsule::{
    CapsuleInputs, CompilerPolicy, ContextCapsule, NegativeKnowledge, Scope,
};
use crate::retry::Clock;
use crate::run_view::RunStateView;

// ── CapsuleStore ────────────────────────────────────────────────────────────

/// Durable store of `ContextCapsule` per workflow / node.
pub trait CapsuleStore: Send + Sync + std::fmt::Debug {
    fn last_capsule(&self, workflow_run: &str) -> Option<ContextCapsule>;
    fn last_capsule_for_node(&self, workflow_run: &str, node_run: &str) -> Option<ContextCapsule>;
    fn resolve_ref(&self, cid: &str) -> Option<ContextCapsule>;
    fn persist(&self, capsule: &ContextCapsule);
}

/// In-memory capsule store keyed by `<workflow>:<node>`.
#[derive(Debug, Default)]
pub struct InMemoryCapsuleStore {
    inner: Mutex<BTreeMap<String, ContextCapsule>>,
}

impl InMemoryCapsuleStore {
    pub fn new() -> Self {
        Self::default()
    }
}

impl CapsuleStore for InMemoryCapsuleStore {
    fn last_capsule(&self, workflow_run: &str) -> Option<ContextCapsule> {
        let g = self.inner.lock().expect("poisoned");
        g.values()
            .rev()
            .find(|c| c.for_target.workflow_run == workflow_run)
            .cloned()
    }
    fn last_capsule_for_node(&self, workflow_run: &str, node_run: &str) -> Option<ContextCapsule> {
        let g = self.inner.lock().expect("poisoned");
        let key = format!("{}:{}", workflow_run, node_run);
        g.get(&key).cloned().or_else(|| {
            g.values()
                .rev()
                .find(|c| {
                    c.for_target.workflow_run == workflow_run && c.for_target.node_run == node_run
                })
                .cloned()
        })
    }
    fn resolve_ref(&self, cid: &str) -> Option<ContextCapsule> {
        let g = self.inner.lock().expect("poisoned");
        g.values().find(|c| c.capsule_id == cid).cloned()
    }
    fn persist(&self, capsule: &ContextCapsule) {
        let mut g = self.inner.lock().expect("poisoned");
        let key = format!(
            "{}:{}",
            capsule.for_target.workflow_run, capsule.for_target.node_run
        );
        g.insert(key, capsule.clone());
    }
}

// ── RunStateViewInputs ──────────────────────────────────────────────────────

pub trait RunStateViewInputs: Send + Sync + std::fmt::Debug {
    fn run_state_view(&self, workflow_run: &str) -> Option<RunStateView>;
    fn frontier_node_runs(&self, workflow_run: &str) -> Vec<String>;
    fn blockers(&self, workflow_run: &str) -> Vec<String>;
    fn pending_decisions(&self, workflow_run: &str) -> Vec<String>;
}

#[derive(Debug, Default)]
pub struct InMemoryRunStateViewInputs {
    inner: Mutex<BTreeMap<String, RunStateView>>,
}

impl InMemoryRunStateViewInputs {
    pub fn new() -> Self {
        Self::default()
    }
    pub fn seed(&self, view: RunStateView) {
        let mut g = self.inner.lock().expect("poisoned");
        g.insert(view.run_id().to_string(), view);
    }
}

impl RunStateViewInputs for InMemoryRunStateViewInputs {
    fn run_state_view(&self, workflow_run: &str) -> Option<RunStateView> {
        self.inner
            .lock()
            .expect("poisoned")
            .get(workflow_run)
            .cloned()
    }
    fn frontier_node_runs(&self, workflow_run: &str) -> Vec<String> {
        self.inner
            .lock()
            .expect("poisoned")
            .get(workflow_run)
            .map(|v| v.frontier().to_vec())
            .unwrap_or_default()
    }
    fn blockers(&self, workflow_run: &str) -> Vec<String> {
        self.inner
            .lock()
            .expect("poisoned")
            .get(workflow_run)
            .map(|v| v.blockers().to_vec())
            .unwrap_or_default()
    }
    fn pending_decisions(&self, workflow_run: &str) -> Vec<String> {
        self.inner
            .lock()
            .expect("poisoned")
            .get(workflow_run)
            .map(|v| v.pending_decisions().to_vec())
            .unwrap_or_default()
    }
}

// ── CapsulePersistence hook ────────────────────────────────────────────────

pub trait CapsulePersistence: Send + Sync + std::fmt::Debug {
    fn persist(&self, capsule: &ContextCapsule);
}

/// No-op persistence. Default for production hosts.
#[derive(Debug, Default, Clone, Copy)]
pub struct NullCapsulePersistence;

impl CapsulePersistence for NullCapsulePersistence {
    fn persist(&self, _capsule: &ContextCapsule) {}
}

/// Recording persistence for tests.
#[derive(Debug, Default)]
pub struct RecordingCapsulePersistence {
    inner: Mutex<Vec<ContextCapsule>>,
}

impl RecordingCapsulePersistence {
    pub fn new() -> Self {
        Self::default()
    }
    pub fn records(&self) -> Vec<ContextCapsule> {
        self.inner.lock().expect("poisoned").clone()
    }
    pub fn len(&self) -> usize {
        self.inner.lock().expect("poisoned").len()
    }
    pub fn is_empty(&self) -> bool {
        self.len() == 0
    }
}

impl CapsulePersistence for RecordingCapsulePersistence {
    fn persist(&self, capsule: &ContextCapsule) {
        self.inner.lock().expect("poisoned").push(capsule.clone());
    }
}

// ── ColdStartOutput / Source / Error ───────────────────────────────────────

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ColdStartSource {
    FromRecovery,
    Fresh,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case", tag = "kind")]
#[non_exhaustive]
pub enum ColdStartError {
    RunNotFound { workflow_run: String },
    NoFrontier { workflow_run: String },
    CompileFailed(crate::context_capsule::CapsuleError),
    InvalidRecoveryState { reason: String },
}

impl std::fmt::Display for ColdStartError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::RunNotFound { workflow_run } => write!(f, "run not found: {workflow_run}"),
            Self::NoFrontier { workflow_run } => write!(f, "no frontier: {workflow_run}"),
            Self::CompileFailed(e) => write!(f, "compile failed: {e}"),
            Self::InvalidRecoveryState { reason } => write!(f, "invalid recovery: {reason}"),
        }
    }
}

impl std::error::Error for ColdStartError {}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ColdStartOutput {
    pub capsule: ContextCapsule,
    pub run_state_view: RunStateView,
    pub source: ColdStartSource,
}

// ── RecoveryCapsuleInputs ──────────────────────────────────────────────────

/// Adapter that composes a `CapsuleInputs` from `CapsuleStore` +
/// `RunStateViewInputs`. Used by `AgentHost::cold_start`.
pub struct RecoveryCapsuleInputs {
    parent: Option<ContextCapsule>,
    rsv: RunStateView,
}

impl std::fmt::Debug for RecoveryCapsuleInputs {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("RecoveryCapsuleInputs")
            .field(
                "parent",
                &self.parent.as_ref().map(|p| p.capsule_id.clone()),
            )
            .field("rsv_run", &self.rsv.run_id())
            .finish()
    }
}

impl RecoveryCapsuleInputs {
    pub fn new(parent: Option<ContextCapsule>, rsv: RunStateView) -> Self {
        Self { parent, rsv }
    }

    fn merge_must_read(&self) -> Vec<String> {
        let parent_reads = self
            .parent
            .as_ref()
            .map(|p| p.artifacts.must_read.clone())
            .unwrap_or_default();
        let blockers: Vec<String> = self.rsv.blockers().to_vec();
        let pending: Vec<String> = self
            .rsv
            .pending_decisions()
            .iter()
            .map(|d| format!("decision:{}", d))
            .collect();
        // Fresh cold-start fallback: if parent_reads + blockers + pending
        // is empty, seed must_read with the frontier node-run id so
        // CompileFailed(MissingRequired) does not fire.
        let frontier_seed: Vec<String> =
            if parent_reads.is_empty() && blockers.is_empty() && pending.is_empty() {
                self.rsv
                    .frontier()
                    .first()
                    .map(|n| vec![format!("node:{}", n)])
                    .unwrap_or_default()
            } else {
                Vec::new()
            };
        let mut seen: BTreeSet<String> = BTreeSet::new();
        let mut out = Vec::new();
        for r in parent_reads
            .iter()
            .chain(blockers.iter())
            .chain(pending.iter())
            .chain(frontier_seed.iter())
        {
            if seen.insert(r.clone()) {
                out.push(r.clone());
            }
        }
        out
    }
}

impl CapsuleInputs for RecoveryCapsuleInputs {
    fn objective(&self) -> &str {
        // Borrow target: parent first, else RSV pending decision, else static.
        // CapsuleInputs::objective returns &str; we cannot return a derived
        // owned String, so we store the chosen objective on the struct via
        // an internal String. To keep the API simple, use the parent's
        // objective or fall back to a static placeholder.
        static DEFAULT: &str = "cold-start recovery";
        self.parent
            .as_ref()
            .map(|p| p.objective.as_str())
            .unwrap_or(DEFAULT)
    }
    fn definition_of_done(&self) -> Vec<String> {
        self.parent
            .as_ref()
            .map(|p| p.definition_of_done.clone())
            .unwrap_or_default()
    }
    fn scope(&self) -> Scope {
        self.parent
            .as_ref()
            .map(|p| p.scope.clone())
            .unwrap_or_default()
    }
    fn constraints(&self) -> Vec<String> {
        self.parent
            .as_ref()
            .map(|p| p.constraints.clone())
            .unwrap_or_default()
    }
    fn accepted_decisions(&self) -> Vec<String> {
        self.parent
            .as_ref()
            .map(|p| p.decisions.accepted.clone())
            .unwrap_or_default()
    }
    fn rejected_decisions(&self) -> Vec<String> {
        self.parent
            .as_ref()
            .map(|p| p.decisions.rejected.clone())
            .unwrap_or_default()
    }
    fn assumptions(&self) -> Vec<crate::context_capsule::Assumption> {
        self.parent
            .as_ref()
            .map(|p| p.assumptions.clone())
            .unwrap_or_default()
    }
    fn must_read(&self) -> Vec<String> {
        self.merge_must_read()
    }
    fn relevant(&self) -> Vec<String> {
        self.parent
            .as_ref()
            .map(|p| p.artifacts.relevant.clone())
            .unwrap_or_default()
    }
    fn fetch_on_demand(&self) -> Vec<String> {
        self.parent
            .as_ref()
            .map(|p| p.artifacts.fetch_on_demand.clone())
            .unwrap_or_default()
    }
    fn negative_knowledge(&self) -> Vec<NegativeKnowledge> {
        self.parent
            .as_ref()
            .map(|p| p.negative_knowledge.clone())
            .unwrap_or_default()
    }
    fn previous_capsule(&self) -> Option<ContextCapsule> {
        self.parent.clone()
    }
    fn parent_target(&self) -> Option<crate::context_capsule::CapsuleTarget> {
        self.parent.as_ref().map(|p| p.for_target.clone())
    }
    fn provenance(&self) -> Vec<crate::run_view::ProvenanceLink> {
        self.parent
            .as_ref()
            .map(|p| p.provenance.clone())
            .unwrap_or_default()
    }
    fn last_seen_at_ms(&self, _ref_id: &str) -> Option<i64> {
        None
    }
    fn content_changed_since(&self, _ref_id: &str) -> Option<i64> {
        None
    }
    fn decision_overridden_at(&self, _ref_id: &str) -> Option<i64> {
        None
    }
    fn artifact_removed_at(&self, _ref_id: &str) -> Option<i64> {
        None
    }
    fn provider_revoked_at(&self, _ref_id: &str) -> Option<i64> {
        None
    }
    fn ref_tags(&self, _ref_id: &str) -> Vec<String> {
        Vec::new()
    }
}

// ── cold_start core (free function, testable independently) ───────────────

pub fn cold_start(
    workflow_run: &str,
    node_run: &str,
    rsv_inputs: &dyn RunStateViewInputs,
    store: &dyn CapsuleStore,
    clock: std::sync::Arc<dyn Clock>,
    policy: CompilerPolicy,
) -> Result<ColdStartOutput, ColdStartError> {
    let rsv =
        rsv_inputs
            .run_state_view(workflow_run)
            .ok_or_else(|| ColdStartError::RunNotFound {
                workflow_run: workflow_run.to_string(),
            })?;

    let frontier = rsv_inputs.frontier_node_runs(workflow_run);
    let resolved_node: String = if node_run == "*" || node_run.is_empty() {
        frontier
            .first()
            .cloned()
            .ok_or_else(|| ColdStartError::NoFrontier {
                workflow_run: workflow_run.to_string(),
            })?
    } else if frontier.iter().any(|n| n == node_run) {
        node_run.to_string()
    } else if !frontier.is_empty() {
        frontier[0].clone()
    } else {
        return Err(ColdStartError::NoFrontier {
            workflow_run: workflow_run.to_string(),
        });
    };

    let parent = store
        .last_capsule_for_node(workflow_run, &resolved_node)
        .or_else(|| store.last_capsule(workflow_run));

    let inputs = std::sync::Arc::new(RecoveryCapsuleInputs::new(parent.clone(), rsv.clone()))
        as std::sync::Arc<dyn CapsuleInputs>;
    let compiler = crate::context_capsule::ContextCompiler::with_policy(
        crate::context_capsule::ContextCompiler::new(inputs, clock.clone()),
        policy,
    );

    let target = crate::context_capsule::CapsuleTarget {
        workflow_run: workflow_run.to_string(),
        node_run: resolved_node,
        attempt: "cold-start".to_string(),
    };

    let mut capsule = if let Some(p) = &parent {
        compiler
            .compile_delta(target.clone(), p)
            .map_err(ColdStartError::CompileFailed)?
    } else {
        compiler
            .compile(target.clone())
            .map_err(ColdStartError::CompileFailed)?
    };

    let stamp = clock.now_ms() as i64;
    let parent_part = parent
        .as_ref()
        .map(|p| p.capsule_id.clone())
        .unwrap_or_else(|| format!("{}:{}:cold-start", workflow_run, target.node_run));
    capsule.capsule_id = format!("{}:{}", parent_part, stamp);

    store.persist(&capsule);

    Ok(ColdStartOutput {
        source: if parent.is_some() {
            ColdStartSource::FromRecovery
        } else {
            ColdStartSource::Fresh
        },
        capsule,
        run_state_view: rsv,
    })
}

// ── Clock adapter (no longer needed; Arc<dyn Clock> flows through) ─────────
