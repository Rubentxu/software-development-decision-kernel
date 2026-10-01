//! Dynamic Workflow Expansion — the **vertical** that joins an evidence gap to
//! a durable, incremental plan revision.
//!
//! Before this module, `aiw_s4_dynamic_expansion.rs` called
//! [`crate::Engine::cycle_replan`] directly. That path:
//!
//! - takes its `event_id` from the caller, so it has **no stable trigger
//!   identity** — two identical replans are two revisions, and the old W02 test
//!   *pinned that as expected*;
//! - never validates [`WritableSurface::PlanRevisions`];
//! - never produces a [`PlanRevisionV1`], so there is no revision N+1 with
//!   lineage; and
//! - has no notion of which nodes are new, so nothing can be "executed
//!   incrementally".
//!
//! This module implements the vertical the AIW-S4 contract requires:
//!
//! ```text
//! EvidenceGap → Secretary proposal → orchestration decision → authority
//!              → typed delta → PlanRevision N+1 → incremental execution
//!              → receipt
//! ```
//!
//! ## Idempotency
//!
//! The trigger's **content fingerprint** is the idempotency key. It travels in
//! the canonical `cycle.expansion.applied` payload, and the ledger is already
//! the append-only authority (ADR §2.7). A replay is detected by scanning the
//! cycle's own events for that fingerprint **before any step mutates
//! anything** — so a replay costs zero revisions, zero executions and zero
//! events. This also closes a projection-divergence gap: because the guard runs
//! before the snapshot write, a replay can never bump `replan_count` while its
//! event is being deduplicated.
//!
//! ## Lineage
//!
//! The parent link is the **ledger tip**, not a locally recomputed revision.
//! Each applied event stores the full [`PlanRevisionV1`], so ancestry is
//! reconstructible from the ledger alone and survives a restart.
//!
//! ## Declared limit (read before citing this as "execution")
//!
//! [`ExpansionOutcome::executed_node_ids`] records **which nodes were
//! dispatched** for this expansion and their per-node outcome. It is a
//! selection/accounting ledger, not an operator runtime: evaluating operators
//! is DW-RUNTIME-003/004/005, explicitly out of scope of
//! `execution_graph_compiler` ("compile-only"). Nothing here is evidence that
//! operator bodies ran.

use std::path::Path;

use serde::{Deserialize, Serialize};
use serde_json::json;
use sha2::{Digest, Sha256};

use sddk_domain::plan_revision::{NormalizedPlanV1, PlanMutation, PlanProvenanceV1, PlanRevisionV1};
use sddk_domain::workflow_ir::WorkflowIR;
use sddk_domain::{LedgerEvent, LedgerEventInput};

use crate::secretary_l1::{BoundedWindow, ClosedSetKind, ProposalTemplate, SecretaryId, SecretaryL1Engine, SecretaryProposal};
use crate::risk_approval_policy::RiskTier;
use crate::{
    Engine, EngineError, EventContext, RestageTo, authority::AuthorityContext,
    authority::WritableSurface, write_atomic,
};

/// Event type for the request half of an expansion.
pub const EVENT_REQUESTED: &str = "cycle.expansion.requested";
/// Event type for the applied half of an expansion.
pub const EVENT_APPLIED: &str = "cycle.expansion.applied";
/// Template id the convenience entry points register.
pub const DEFAULT_TEMPLATE_ID: &str = "sddk.template.dynamic-expansion";
/// Secretary id the convenience entry points use.
pub const DEFAULT_SECRETARY_ID: &str = "sddk.default";

const FP_KEY: &str = "trigger_fingerprint";

// ── Trigger ───────────────────────────────────────────────────────────────

/// An evidence-driven request to expand the active plan.
///
/// The trigger is **self-describing**: it carries both the plan it was
/// proposed against (`base_ir`) and the plan it proposes (`proposed_ir`), so
/// the pipeline never has to guess which nodes are new.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ExpansionTrigger {
    /// Cycle the expansion applies to.
    pub cycle_id: String,
    /// Human-readable description of the evidence gap that motivated it.
    pub gap_summary: String,
    /// Evidence references backing the gap.
    pub evidence_refs: Vec<String>,
    /// The plan the proposal was made against.
    pub base_ir: WorkflowIR,
    /// The plan the Secretary proposes.
    pub proposed_ir: WorkflowIR,
    /// Revision id the proposer believed was the tip. A mismatch fails closed.
    pub expected_base_revision: String,
    /// Phase to restage to on a successful expansion.
    pub restage_to: RestageTo,
    /// Secretary L1 template used to issue the proposal.
    pub template_id: String,
    /// Confidence carried by the proposal.
    pub confidence: f64,
}

impl ExpansionTrigger {
    /// Content-addressed identity of the expansion request.
    ///
    /// Two triggers with the same cycle, gap, evidence, template, confidence
    /// and proposed plan share a fingerprint **regardless of the base they were
    /// proposed against**: the base is a concurrency pin, not part of the
    /// request's identity.
    pub fn fingerprint(&self) -> String {
        let mut refs = self.evidence_refs.clone();
        refs.sort();
        let material = json!({
            "cycle_id": self.cycle_id,
            "gap_summary": self.gap_summary,
            "evidence_refs": refs,
            "template_id": self.template_id,
            "confidence": self.confidence,
            "proposed_ir": self.proposed_ir,
        });
        format!("{:064x}", Sha256::digest(material.to_string().as_bytes()))
    }
}

// ── Orchestration decision ────────────────────────────────────────────────

/// The decision made after seeing the Secretary's proposal and before any
/// authority check or mutation.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case", tag = "kind")]
#[non_exhaustive]
pub enum OrchestrationVerdict {
    /// Apply the expansion.
    Expand,
    /// Do not expand now.
    Defer { reason: String },
    /// Do not expand.
    Reject { reason: String },
}

// ── Outcome ───────────────────────────────────────────────────────────────

/// Result of one expansion attempt.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ExpansionOutcome {
    /// Content fingerprint identifying this expansion.
    pub trigger_fingerprint: String,
    /// `true` only when this call produced the revision.
    pub applied: bool,
    /// `true` when orchestration deferred or rejected (no canonical change).
    pub deferred: bool,
    /// Revision id of the resulting (or previously applied) revision.
    pub revision_id: String,
    /// Parent revision id.
    pub parent_revision_id: Option<String>,
    /// Typed mutation that produced the revision.
    pub mutation: PlanMutation,
    /// Lineage length: root + one entry per applied expansion.
    pub lineage_len: usize,
    /// Nodes dispatched by **this** expansion. Never includes pre-existing
    /// nodes.
    pub executed_node_ids: Vec<String>,
    /// Dispatched nodes that succeeded.
    pub new_node_execution_count: usize,
    /// Whether at least one dispatched node failed.
    pub new_node_failed: bool,
    /// Dispatched nodes that failed.
    pub failed_node_ids: Vec<String>,
    /// Durable total of successful new-node executions across the lineage.
    pub total_new_node_execution_count: usize,
    /// Secretary proposal id, when a proposal was issued.
    pub proposal_id: String,
    /// Secretary identity, when a proposal was issued.
    pub secretary_id: Option<String>,
    /// Receipt path, when this call applied.
    pub receipt_path: Option<String>,
}

// ── Errors ────────────────────────────────────────────────────────────────

/// Fail-closed error taxonomy for the expansion vertical.
#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error, Serialize, Deserialize)]
#[serde(rename_all = "snake_case", tag = "kind")]
#[non_exhaustive]
pub enum ExpansionError {
    /// The authority is not admitted on `plan_revisions`.
    #[error("authority denied on plan_revisions: {actor_kind:?} not admitted")]
    AuthorityDenied { actor_kind: String },
    /// The proposed delta is not a usable plan.
    #[error("invalid delta: {reason}")]
    InvalidDelta { reason: String },
    /// The plan moved after the proposal was formed.
    #[error("stale base revision: trigger pinned {expected}, lineage tip is {actual}")]
    StaleBaseRevision { expected: String, actual: String },
    /// The Secretary could not issue a proposal.
    #[error("no proposal issued: {reason}")]
    NoProposal { reason: String },
    /// The plan-revision substrate rejected the derived revision.
    #[error("plan revision rejected: {reason}")]
    PlanRevision { reason: String },
    /// The cycle does not exist in the ledger.
    #[error("cycle not found: {cycle_id}")]
    CycleNotFound { cycle_id: String },
    /// A storage/ledger failure.
    #[error("ledger failure: {0}")]
    Storage(String),
}

// ── Injected collaborators ───────────────────────────────────────────────

/// The collaborators the vertical needs beyond the engine itself.
pub struct ExpansionDeps<'a> {
    /// Secretary that issues the proposal.
    pub secretary: &'a SecretaryL1Engine,
    /// Identity of that Secretary.
    pub secretary_id: SecretaryId,
    /// Orchestration decision over the issued proposal.
    pub decide: &'a dyn Fn(&SecretaryProposal) -> OrchestrationVerdict,
    /// Node execution hook: `true` on success, `false` on failure.
    pub exec: &'a dyn Fn(&str) -> bool,
}

impl<L: sddk_domain::Ledger> Engine<L> {
    /// The current plan revision id for a cycle, rebuilt from its canonical
    /// events so it survives a restart. A cycle with no applied expansion
    /// returns its content-derived root id.
    pub fn current_plan_revision_id(&self, cycle_id: &str) -> Result<String, ExpansionError> {
        let events = self.cycle_events(cycle_id)?;
        if events.is_empty() {
            return Err(ExpansionError::CycleNotFound {
                cycle_id: cycle_id.to_string(),
            });
        }
        Ok(tip_revision_id(&events))
    }

    /// Applies an expansion with a default orchestration (always expand) and a
    /// default execution hook (always succeeds).
    pub fn apply_dynamic_expansion(
        &mut self,
        trigger: &ExpansionTrigger,
        authority: &AuthorityContext,
        ctx: &EventContext,
        receipt_path: &Path,
    ) -> Result<ExpansionOutcome, ExpansionError> {
        let sec = default_secretary(&trigger.template_id);
        self.apply_dynamic_expansion_inner(
            trigger,
            ExpansionDeps {
                secretary: &sec,
                secretary_id: SecretaryId(DEFAULT_SECRETARY_ID.into()),
                decide: &|_| OrchestrationVerdict::Expand,
                exec: &|_| true,
            },
            authority,
            ctx,
            Some(receipt_path),
        )
    }

    /// As [`Self::apply_dynamic_expansion`] with a caller-supplied
    /// orchestration decision.
    pub fn apply_dynamic_expansion_deciding(
        &mut self,
        trigger: &ExpansionTrigger,
        authority: &AuthorityContext,
        ctx: &EventContext,
        receipt_path: &Path,
        decide: &dyn Fn(&SecretaryProposal) -> OrchestrationVerdict,
    ) -> Result<ExpansionOutcome, ExpansionError> {
        let sec = default_secretary(&trigger.template_id);
        self.apply_dynamic_expansion_inner(
            trigger,
            ExpansionDeps {
                secretary: &sec,
                secretary_id: SecretaryId(DEFAULT_SECRETARY_ID.into()),
                decide,
                exec: &|_| true,
            },
            authority,
            ctx,
            Some(receipt_path),
        )
    }

    /// As [`Self::apply_dynamic_expansion`] with a caller-supplied node
    /// execution hook.
    pub fn apply_dynamic_expansion_with(
        &mut self,
        trigger: &ExpansionTrigger,
        authority: &AuthorityContext,
        ctx: &EventContext,
        receipt_path: &Path,
        exec: &dyn Fn(&str) -> bool,
    ) -> Result<ExpansionOutcome, ExpansionError> {
        let sec = default_secretary(&trigger.template_id);
        self.apply_dynamic_expansion_inner(
            trigger,
            ExpansionDeps {
                secretary: &sec,
                secretary_id: SecretaryId(DEFAULT_SECRETARY_ID.into()),
                decide: &|_| OrchestrationVerdict::Expand,
                exec,
            },
            authority,
            ctx,
            Some(receipt_path),
        )
    }

    /// The full vertical with an injected Secretary.
    pub fn apply_dynamic_expansion_with_secretary(
        &mut self,
        trigger: &ExpansionTrigger,
        secretary: &SecretaryL1Engine,
        secretary_id: SecretaryId,
        authority: &AuthorityContext,
        ctx: &EventContext,
        receipt_path: &Path,
        decide: &dyn Fn(&SecretaryProposal) -> OrchestrationVerdict,
    ) -> Result<ExpansionOutcome, ExpansionError> {
        self.apply_dynamic_expansion_inner(
            trigger,
            ExpansionDeps {
                secretary,
                secretary_id,
                decide,
                exec: &|_| true,
            },
            authority,
            ctx,
            Some(receipt_path),
        )
    }

    fn cycle_events(&self, cycle_id: &str) -> Result<Vec<LedgerEvent>, ExpansionError> {
        self.ledger
            .list_cycle_events(cycle_id)
            .map_err(|e| ExpansionError::Storage(e.to_string()))
    }

    /// The vertical itself. Every guard runs before the first mutation, so a
    /// rejected or stale expansion leaves the ledger and the projection
    /// untouched.
    pub fn apply_dynamic_expansion_inner(
        &mut self,
        trigger: &ExpansionTrigger,
        deps: ExpansionDeps<'_>,
        authority: &AuthorityContext,
        ctx: &EventContext,
        receipt_path: Option<&Path>,
    ) -> Result<ExpansionOutcome, ExpansionError> {
        let fingerprint = trigger.fingerprint();
        let cycle_id = trigger.cycle_id.clone();

        let events = self.cycle_events(&cycle_id)?;
        if events.is_empty() {
            return Err(ExpansionError::CycleNotFound {
                cycle_id: cycle_id.clone(),
            });
        }

        // ── Step 0: replay guard, before every mutation.
        if let Some(prev) = find_applied(&events, &fingerprint) {
            return Ok(replay_outcome(prev, &events, &fingerprint));
        }

        // ── Step 1: authority, before any state access.
        authority
            .validate(WritableSurface::PlanRevisions)
            .map_err(|_| ExpansionError::AuthorityDenied {
                actor_kind: format!("{:?}", authority.actor_kind),
            })?;

        // ── Step 2: the delta must be a usable plan.
        if trigger.proposed_ir.operators.is_empty() {
            return Err(ExpansionError::InvalidDelta {
                reason: "proposed plan has no operators".into(),
            });
        }

        // ── Step 3: the base pin must still be the tip.
        let tip = tip_revision_id(&events);
        if tip != trigger.expected_base_revision {
            return Err(ExpansionError::StaleBaseRevision {
                expected: trigger.expected_base_revision.clone(),
                actual: tip,
            });
        }

        // ── Step 4: Secretary issues the proposal.
        let proposal = deps
            .secretary
            .propose(
                0,
                &trigger.template_id,
                trigger.evidence_refs.clone(),
                Vec::new(),
                vec![trigger.gap_summary.clone()],
                trigger.gap_summary.clone(),
                trigger.confidence,
            )
            .map_err(|e| ExpansionError::NoProposal {
                reason: e.to_string(),
            })?;

        // ── Step 5: orchestration decision.
        if !matches!((deps.decide)(&proposal), OrchestrationVerdict::Expand) {
            return Ok(ExpansionOutcome {
                trigger_fingerprint: fingerprint,
                applied: false,
                deferred: true,
                revision_id: tip,
                parent_revision_id: None,
                mutation: PlanMutation::Initial,
                lineage_len: lineage_len(&events),
                executed_node_ids: Vec::new(),
                new_node_execution_count: 0,
                new_node_failed: false,
                failed_node_ids: Vec::new(),
                total_new_node_execution_count: total_executions(&events),
                proposal_id: proposal.proposal_id.clone(),
                secretary_id: Some(deps.secretary_id.0.clone()),
                receipt_path: None,
            });
        }

        // ── Step 6: typed delta → PlanRevision N+1, parented on the ledger tip.
        let provenance =
            PlanProvenanceV1::new(&deps.secretary_id.0, env!("CARGO_PKG_VERSION")).map_err(
                |e| ExpansionError::PlanRevision {
                    reason: e.to_string(),
                },
            )?;
        let normalized = NormalizedPlanV1::from_workflow_ir(&trigger.proposed_ir);
        let revision = PlanRevisionV1::new(
            Some(tip.clone()),
            PlanMutation::NodesChanged,
            provenance,
            normalized,
        )
        .map_err(|e| ExpansionError::PlanRevision {
            reason: e.to_string(),
        })?;

        // ── Step 7: incremental execution — only nodes absent from the base.
        let base_nodes: Vec<String> =
            trigger.base_ir.operators.keys().map(|k| k.0.clone()).collect();
        let mut executed_node_ids: Vec<String> = Vec::new();
        let mut failed_node_ids: Vec<String> = Vec::new();
        for id in trigger.proposed_ir.operators.keys() {
            if base_nodes.contains(&id.0) {
                continue; // pre-existing node: never re-dispatched
            }
            executed_node_ids.push(id.0.clone());
            if !(deps.exec)(id.0.as_str()) {
                failed_node_ids.push(id.0.clone());
            }
        }
        let new_node_execution_count = executed_node_ids.len() - failed_node_ids.len();

        // ── Step 8: canonical events. The ledger is the authority; the cycle
        // manifest is untouched because a plan revision is its own surface.
        let manifest = self
            .ledger
            .get_cycle(&cycle_id)
            .map_err(|e| ExpansionError::Storage(e.to_string()))?
            .manifest;

        let requested = ledger_input(
            &cycle_id,
            &manifest.project_id,
            &ctx.event_id,
            ctx,
            EVENT_REQUESTED,
            json!({
                FP_KEY: fingerprint,
                "base_revision": trigger.expected_base_revision,
                "proposal_id": proposal.proposal_id,
                "gap_summary": trigger.gap_summary,
            }),
        );
        self.ledger
            .update_cycle_with_event(&manifest, &ctx.occurred_at, &requested, false)
            .map_err(|e| ExpansionError::Storage(e.to_string()))?;

        let applied_evt = ledger_input(
            &cycle_id,
            &manifest.project_id,
            &format!("{}-expansion-applied", ctx.event_id),
            ctx,
            EVENT_APPLIED,
            json!({
                FP_KEY: fingerprint,
                "revision_id": revision.revision_id,
                "parent_revision_id": revision.parent_revision_id,
                "mutation": revision.mutation,
                "provenance": revision.provenance,
                "normalized": revision.normalized,
                "executed_node_ids": executed_node_ids,
                "failed_node_ids": failed_node_ids,
                "new_node_execution_count": new_node_execution_count,
                "proposal_id": proposal.proposal_id,
                "secretary_id": deps.secretary_id.0,
                "evidence_refs": trigger.evidence_refs,
                "gap_summary": trigger.gap_summary,
                "restage_to": trigger.restage_to.as_str(),
            }),
        );
        self.ledger
            .update_cycle_with_event(&manifest, &ctx.occurred_at, &applied_evt, false)
            .map_err(|e| ExpansionError::Storage(e.to_string()))?;

        // ── Step 9: durable receipt, written after the events land.
        let mut written = None;
        if let Some(dir) = receipt_path {
            let file = dir.join(&cycle_id).join("expansion-receipt.json");
            if let Some(parent) = file.parent() {
                std::fs::create_dir_all(parent)
                    .map_err(|e| ExpansionError::Storage(e.to_string()))?;
            }
            let body = json!({
                FP_KEY: fingerprint,
                "revision_id": revision.revision_id,
                "parent_revision_id": revision.parent_revision_id,
                "mutation": revision.mutation,
                "lineage_len": lineage_len(&events) + 1,
                "executed_node_ids": executed_node_ids,
                "failed_node_ids": failed_node_ids,
                "new_node_execution_count": new_node_execution_count,
                "proposal_id": proposal.proposal_id,
                "secretary_id": deps.secretary_id.0,
            });
            write_atomic(
                &file,
                serde_json::to_string_pretty(&body)
                    .map_err(|e| ExpansionError::Storage(e.to_string()))?
                    .as_bytes(),
            )
            .map_err(|e| ExpansionError::Storage(e.to_string()))?;
            written = Some(file.to_string_lossy().to_string());
        }

        Ok(ExpansionOutcome {
            trigger_fingerprint: fingerprint,
            applied: true,
            deferred: false,
            revision_id: revision.revision_id,
            parent_revision_id: revision.parent_revision_id,
            mutation: revision.mutation,
            lineage_len: lineage_len(&events) + 1,
            executed_node_ids,
            new_node_execution_count,
            new_node_failed: !failed_node_ids.is_empty(),
            failed_node_ids,
            total_new_node_execution_count: total_executions(&events) + new_node_execution_count,
            proposal_id: proposal.proposal_id,
            secretary_id: Some(deps.secretary_id.0.clone()),
            receipt_path: written,
        })
    }
}

fn ledger_input(
    cycle_id: &str,
    project_id: &str,
    event_id: &str,
    ctx: &EventContext,
    event_type: &str,
    payload: serde_json::Value,
) -> LedgerEventInput {
    LedgerEventInput {
        event_id: event_id.to_owned(),
        project_id: project_id.to_owned(),
        cycle_id: Some(cycle_id.to_owned()),
        frame_id: ctx.frame_id.clone(),
        command_id: ctx.command_id.clone(),
        actor: ctx.actor.clone(),
        actor_ref: ctx.actor_ref.clone(),
        event_type: event_type.to_owned(),
        occurred_at: ctx.occurred_at.clone(),
        state_before: None,
        state_after: None,
        payload,
        causation_id: ctx.causation_id.clone(),
        correlation_id: ctx.correlation_id.clone(),
    }
}

// ── lineage reconstruction from the ledger ────────────────────────────────

fn applied_events(events: &[LedgerEvent]) -> Vec<&LedgerEvent> {
    events
        .iter()
        .filter(|e| e.event_type == EVENT_APPLIED)
        .collect()
}

/// Content-derived root revision id: the manifest recorded at `cycle.start`.
///
/// Recomputable from the ledger alone, so it needs no extra durable state.
fn root_revision_id(events: &[LedgerEvent]) -> String {
    let anchor = events
        .iter()
        .find(|e| e.event_type == "cycle.start")
        .and_then(|e| e.state_after.clone())
        .unwrap_or_else(|| json!({ "root": true }));
    format!(
        "{:064x}",
        Sha256::digest(format!("plan-root|{anchor}").as_bytes())
    )
}

fn tip_revision_id(events: &[LedgerEvent]) -> String {
    applied_events(events)
        .last()
        .and_then(|e| e.payload.get("revision_id"))
        .and_then(|v| v.as_str())
        .map(str::to_owned)
        .unwrap_or_else(|| root_revision_id(events))
}

fn lineage_len(events: &[LedgerEvent]) -> usize {
    applied_events(events).len() + 1
}

fn total_executions(events: &[LedgerEvent]) -> usize {
    applied_events(events)
        .iter()
        .filter_map(|e| e.payload.get("new_node_execution_count"))
        .filter_map(|v| v.as_u64())
        .map(|v| v as usize)
        .sum()
}

fn find_applied<'a>(events: &'a [LedgerEvent], fingerprint: &str) -> Option<&'a LedgerEvent> {
    applied_events(events)
        .into_iter()
        .find(|e| e.payload.get(FP_KEY).and_then(|v| v.as_str()) == Some(fingerprint))
}

/// Rebuilds the outcome of an already-applied expansion, reporting it as a
/// replay (`applied: false`) with zero new executions.
fn replay_outcome(
    prev: &LedgerEvent,
    events: &[LedgerEvent],
    fingerprint: &str,
) -> ExpansionOutcome {
    let s = |k: &str| {
        prev.payload
            .get(k)
            .and_then(|v| v.as_str())
            .unwrap_or_default()
            .to_owned()
    };
    let l = |k: &str| -> Vec<String> {
        prev.payload
            .get(k)
            .and_then(|v| v.as_array())
            .map(|a| {
                a.iter()
                    .filter_map(|x| x.as_str().map(str::to_owned))
                    .collect()
            })
            .unwrap_or_default()
    };
    let failed = l("failed_node_ids");
    ExpansionOutcome {
        trigger_fingerprint: fingerprint.to_owned(),
        applied: false,
        deferred: false,
        revision_id: s("revision_id"),
        parent_revision_id: prev
            .payload
            .get("parent_revision_id")
            .and_then(|v| v.as_str())
            .map(str::to_owned),
        mutation: serde_json::from_value(
            prev.payload
                .get("mutation")
                .cloned()
                .unwrap_or_else(|| json!("initial")),
        )
        .unwrap_or(PlanMutation::Initial),
        lineage_len: lineage_len(events),
        executed_node_ids: l("executed_node_ids"),
        new_node_execution_count: 0,
        new_node_failed: !failed.is_empty(),
        failed_node_ids: failed,
        total_new_node_execution_count: total_executions(events),
        proposal_id: s("proposal_id"),
        secretary_id: Some(s("secretary_id")),
        receipt_path: None,
    }
}

// ── default Secretary ─────────────────────────────────────────────────────

/// A minimal Secretary carrying the expansion template named by the trigger,
/// used by the convenience entry points that take no injected Secretary.
fn default_secretary(template_id: &str) -> SecretaryL1Engine {
    let e = SecretaryL1Engine::new();
    let _ = e.register_template(ProposalTemplate::new(
        template_id,
        ClosedSetKind::SuggestCandidate,
        "expand the plan to cover the evidence gap",
        RiskTier::Medium,
        BoundedWindow::new(SecretaryId(DEFAULT_SECRETARY_ID.into()), 0, i64::MAX, 5),
    ));
    e
}

/// Engine errors surface through the storage variant of [`ExpansionError`].
impl From<EngineError> for ExpansionError {
    fn from(e: EngineError) -> Self {
        ExpansionError::Storage(e.to_string())
    }
}
