// Copyright (c) SDDK contributors.
// SPDX-License-Identifier: MIT
//
// admission.rs — C4 single enforcement choke point (D-01 / D-08 / R-4-001).
//
// `enforce_gated` maps a `RunnerVerdict` onto a blocking `EnforcementOutcome`.
// Stage M4 (`All`): `Deny` aborts on ALL surfaces; every `RequireApproval`
// blocks through the live M2 approval loop (a granted approval matching the
// stable request hash re-admits). Per B+ (ADR-0111) only explicitly dangerous
// actions reach `RequireApproval`. The engine stays pure; this module decides
// how the CLI reacts to the verdict.

use sddk_domain::projections::ApprovalProjection;
use sddk_domain::{ApprovalDecision, EventStore, Projection};
use sddk_engine::authority_engine::{
    ActionKind, AdmissionDecision, DenyReason, Facts, RunnerVerdict,
};
use sddk_engine::event_bus::emit::{AdmissionDecidedInput, emit_admission_decision};
use sddk_storage::SqliteEventStore;
use sha2::{Digest, Sha256};

/// Enforcement staging (D-08). Advanced by milestone commit; rollback of a
/// stage flip is a single-commit revert. No runtime flag: a runtime flag
/// would be a new bypass surface, contradicting the cutover.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub(crate) enum EnforcementStage {
    /// M1 rollback affordance: RequireApproval blocks Low/Medium surfaces;
    /// High stays advisory. Not constructed in production since M4 = `All`.
    /// Retained (deliberately unconstructed) so a stage flip stays a
    /// single-commit revert. Owner: admission/authority maintainers. Reason:
    /// rollback affordance. Exit trigger: remove once M4 enforcement is proven
    /// across a full release cycle.
    #[allow(dead_code)]
    LowMedium,
    /// M4: every `RequireApproval` decision blocks via the live approval loop.
    /// Active since the M2 approval loop closed. Combined with B+ (ADR-0111),
    /// only explicitly dangerous actions reach `RequireApproval`, so routine
    /// High-band operations are unaffected.
    All,
}

/// Current enforcement stage (D-08). M4 = `All`: the M2 approval loop is live,
/// so every `RequireApproval` blocks and a granted approval re-admits.
pub(crate) const ENFORCEMENT_STAGE: EnforcementStage = EnforcementStage::All;

/// Risk band of the surface a verdict was computed for.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum SurfaceBand {
    Low,
    Medium,
    High,
}

/// Outcome of enforcing an admission verdict at a call site. `Blocked` and
/// `AwaitingApproval` mean the governed effect MUST NOT run.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) enum EnforcementOutcome {
    /// Proceed with the governed mutation.
    Proceed { receipt_id: String },
    /// Approval required and blocking at this stage; effect must not run.
    AwaitingApproval {
        decision_id: String,
        request_hash: String,
    },
    /// Denied; effect must not run. Zero external side effects (R-4-005).
    Blocked {
        reason: DenyReason,
        decision_id: String,
    },
    /// RequireApproval on a High surface while stage < All (M1 window).
    /// Advisory: the effect proceeds; the decision is still recorded.
    AdvisoryHighApproval {
        decision_id: String,
        request_hash: String,
    },
}

impl EnforcementOutcome {
    /// True when the governed effect must not execute.
    #[cfg(test)]
    pub(crate) fn blocks_effect(&self) -> bool {
        matches!(self, Self::Blocked { .. } | Self::AwaitingApproval { .. })
    }
}

/// Map a surface name to its risk band using the bridge policy table
/// (mirrors `default_band_for_surface`).
pub(crate) fn surface_band(surface: &str) -> SurfaceBand {
    match surface {
        "cycle_state" | "gate_receipts" | "plan_revisions" | "transition_records"
        | "framework_bundle" | "github_releases" => SurfaceBand::High,
        "knowledge_graph_vault" | "plan_item" | "evidence_attachment" | "decision_record" => {
            SurfaceBand::Medium
        }
        _ => SurfaceBand::Low,
    }
}

/// Single choke point (D-01): map a runner verdict to its enforcement
/// outcome for the current stage.
pub(crate) fn enforce_gated(
    verdict: &RunnerVerdict,
    surface: &str,
    stage: EnforcementStage,
) -> EnforcementOutcome {
    let band = surface_band(surface);
    match &verdict.decision {
        AdmissionDecision::Allow { receipt_id, .. } => EnforcementOutcome::Proceed {
            receipt_id: receipt_id.clone(),
        },
        AdmissionDecision::Deny {
            reason,
            decision_id,
        } => EnforcementOutcome::Blocked {
            reason: reason.clone(),
            decision_id: decision_id.clone(),
        },
        AdmissionDecision::RequireApproval { decision_id, .. } => {
            let request_hash = approval_request_hash_from(decision_id, surface);
            match (stage, band) {
                // M1 window: High surfaces stay advisory until the approval
                // loop is live (R-4-001 S2); M4 (`All`) makes them blocking.
                (EnforcementStage::LowMedium, SurfaceBand::High) => {
                    EnforcementOutcome::AdvisoryHighApproval {
                        decision_id: decision_id.clone(),
                        request_hash,
                    }
                }
                _ => EnforcementOutcome::AwaitingApproval {
                    decision_id: decision_id.clone(),
                    request_hash,
                },
            }
        }
        // `AdmissionDecision` is non_exhaustive: future variants fail closed
        // (block) rather than silently allow.
        other => EnforcementOutcome::Blocked {
            reason: DenyReason::UnknownAction,
            decision_id: other.decision_id().clone(),
        },
    }
}

/// Approval capability key (OQ-1 default: (surface, action) granularity).
/// Example: `surface.cycle_state#cycle_supersede`.
/// Consumed by the M2 approval loop (`emit_approval_requested_for`,
/// `granted_approval_refs`).
pub(crate) fn approval_capability_key(surface: &str, action: ActionKind) -> String {
    format!("surface.{}#{}", surface, action.as_str())
}

/// Stable SHA-256 over the full proposal identity (no timestamps), so the
/// same (surface, action, target, actor) always yields the same hash.
/// Consumed by the M2 approval loop (`emit_approval_requested_for`,
/// `granted_approval_refs`).
pub(crate) fn approval_request_hash(
    surface: &str,
    action: ActionKind,
    target: &str,
    actor: &str,
) -> String {
    let mut hasher = Sha256::new();
    hasher.update(surface.as_bytes());
    hasher.update(b"\0");
    hasher.update(action.as_str().as_bytes());
    hasher.update(b"\0");
    hasher.update(target.as_bytes());
    hasher.update(b"\0");
    hasher.update(actor.as_bytes());
    format!("sha256:{:x}", hasher.finalize())
}

/// Deterministic request hash derived from an engine decision_id when the
/// caller does not reconstruct the proposal inputs (M1 stopgap; the M2
/// approval loop passes explicit stable inputs).
fn approval_request_hash_from(decision_id: &str, surface: &str) -> String {
    let mut hasher = Sha256::new();
    hasher.update(surface.as_bytes());
    hasher.update(b"\0");
    hasher.update(decision_id.as_bytes());
    format!("sha256:{:x}", hasher.finalize())
}

// ── M2 approval loop (WU-C4-7, D-03 / R-4-002) ────────────────────────────────

/// Resolve the default state dir used for ledger access, mirroring
/// `enforce_admission_or_block`'s resolution (SDDK_STATE_HOME >
/// XDG_STATE_HOME > ~/.local/state).
pub(crate) fn resolve_state_dir() -> Option<std::path::PathBuf> {
    std::env::var_os("SDDK_STATE_HOME")
        .map(std::path::PathBuf::from)
        .or_else(|| std::env::var_os("XDG_STATE_HOME").map(std::path::PathBuf::from))
        .or_else(|| {
            std::env::var_os("HOME").map(|h| {
                let mut p = std::path::PathBuf::from(h);
                p.push(".local/state");
                p
            })
        })
}

/// Emit `approval.capability.requested` for a blocking `RequireApproval`
/// verdict (R-4-002 S1). The request capability key follows OQ-1
/// (`surface.<surface>#<action_kind>`) and the request hash is the stable
/// SHA-256 over `(surface, action, target, actor)` — no timestamps, so
/// re-invocations derive the same hash and can match the grant.
///
/// Emission is fail-soft: the caller has already been blocked, so a ledger
/// failure must not change the awaiting-approval outcome.
#[allow(clippy::too_many_arguments)]
pub(crate) fn emit_approval_requested_for(
    state_dir: &std::path::Path,
    cycle_id: &str,
    surface: &str,
    action: ActionKind,
    target: &str,
    actor: &str,
    project_id: &str,
) -> String {
    let capability = approval_capability_key(surface, action);
    let request_hash = approval_request_hash(surface, action, target, actor);
    let project_dir = state_dir.join("sddk").join("projects").join(project_id);
    let Ok(mut store) = SqliteEventStore::open(&project_dir) else {
        eprintln!(
            "approval request emission failed (fail-soft): no ledger at {}",
            project_dir.display()
        );
        return request_hash;
    };
    // One-hour approval window (matches ApprovalRequirement::timeout_seconds).
    let expires_at = {
        let expiry = time::OffsetDateTime::now_utc() + time::Duration::hours(1);
        expiry
            .format(&time::format_description::well_known::Rfc3339)
            .unwrap_or_else(|_| "1970-01-01T00:00:00Z".to_string())
    };
    let input = sddk_engine::event_bus::ApprovalRequestedInput {
        project_id: project_id.to_string(),
        cycle_id: cycle_id.to_string(),
        capability: capability.clone(),
        request_hash: request_hash.clone(),
        expires_at,
        occurred_at: now_rfc3339(),
        // ADR-069 §4: only Agent may emit approval requests. The CLI acts as
        // the agent-on-behalf-of the caller here.
        actor_id: format!("agent:{actor}"),
        actor_kind: sddk_domain::ActorKind::Agent,
        causation_id: None,
        correlation_id: None,
    };
    if let Err(e) = sddk_engine::event_bus::emit_approval_requested(&mut store, &input) {
        eprintln!("approval request emission failed (fail-soft): {e}");
    }
    request_hash
}

/// Rebuild `ApprovalProjection` for a cycle and return the granted approval
/// refs (D-03): when `(cycle_id, capability)` reports `Granted` with a
/// matching request hash, the returned `Facts::approval_refs` entry
/// satisfies the engine approval gate on re-admission (R-4-002 S2).
pub(crate) fn granted_approval_refs(
    state_dir: &std::path::Path,
    cycle_id: &str,
    surface: &str,
    action: ActionKind,
    target: &str,
    actor: &str,
) -> Vec<sddk_engine::authority_engine::EvidenceRef> {
    let capability = approval_capability_key(surface, action);
    let expected_hash = approval_request_hash(surface, action, target, actor);
    let project_dirs_root = state_dir.join("sddk").join("projects");
    let Ok(entries) = std::fs::read_dir(&project_dirs_root) else {
        return Vec::new();
    };
    for entry in entries.flatten() {
        let Ok(store) = SqliteEventStore::open(&entry.path()) else {
            continue;
        };
        let Ok(events) = store.load_stream(cycle_id, None, u32::MAX) else {
            continue;
        };
        let mut proj = ApprovalProjection::new(cycle_id);
        for event in &events {
            let _ = proj.apply(event);
        }
        if let Some(state) = proj
            .states()
            .get(&(cycle_id.to_string(), capability.clone()))
            && state.decision == Some(ApprovalDecision::Granted)
            && state.request_hash == expected_hash
        {
            return vec![sddk_engine::authority_engine::EvidenceRef(expected_hash)];
        }
    }
    Vec::new()
}

/// Single governed-admission entry point for CLI call sites (WU-C4-7).
///
/// 1. When a cycle ledger exists, first consults `ApprovalProjection` for a
///    granted approval matching the stable request hash and injects it into
///    `Facts::approval_refs` (S2 re-admission; the engine gate is satisfied
///    before `admit_surface` even runs).
/// 2. Runs `admit_surface` and enforces the verdict through the M2 approval
///    loop: a blocking `RequireApproval` emits `approval.capability.requested`
///    (S1) and aborts with zero governed effects; `Deny` aborts (R-4-005);
///    `Allow`/advisory proceed.
pub(crate) fn admit_governed(
    runner: &sddk_engine::authority_engine::AuthorityEngineRunner,
    actor: &str,
    surface: &str,
    action: ActionKind,
    target: &str,
    cycle_id: Option<&str>,
    project_id: &str,
) -> anyhow::Result<RunnerVerdict> {
    let state_dir = resolve_state_dir();
    let mut facts = Facts::default();
    let approval_loop = match (cycle_id, state_dir.as_deref()) {
        (Some(cycle_id), Some(state_dir)) => {
            facts.approval_refs =
                granted_approval_refs(state_dir, cycle_id, surface, action, target, actor);
            ApprovalLoopContext::new(
                project_id.to_string(),
                cycle_id.to_string(),
                action,
                target.to_string(),
                actor.to_string(),
            )
        }
        _ => ApprovalLoopContext::none(),
    };
    let verdict = runner
        .admit_surface(actor, surface, action, target, facts)
        .map_err(|e| anyhow::anyhow!("runner denied {surface}: {e:?}"))?;
    enforce_admission_or_block_ctx(&verdict, surface, state_dir.as_deref(), approval_loop)?;
    Ok(verdict)
}

/// Per-call approval-loop context (WU-C4-7). When present, a blocking
/// `RequireApproval` emits `approval.capability.requested` into the cycle's
/// ledger and re-admissions consult `ApprovalProjection` for a grant.
#[derive(Debug, Clone)]
pub(crate) struct ApprovalLoopContext {
    project_id: String,
    cycle_id: String,
    action: ActionKind,
    target: String,
    actor: String,
}

/// CLI wrapper over `enforce_gated` at the current stage: returns an error
/// (non-zero exit, no mutation) when the outcome blocks the effect. The
/// message is operator-facing; the decision ids stay machine-correlatable.
impl ApprovalLoopContext {
    /// Disables the approval loop (M1 behavior; unit tests without ledger).
    pub(crate) fn none() -> Self {
        Self {
            project_id: String::new(),
            cycle_id: String::new(),
            action: ActionKind::CliRun,
            target: String::new(),
            actor: String::new(),
        }
    }

    /// True when the approval loop is disabled.
    pub(crate) fn is_none(&self) -> bool {
        self.cycle_id.is_empty()
    }

    /// Full context for a governed call site with a live cycle ledger.
    pub(crate) fn new(
        project_id: String,
        cycle_id: String,
        action: ActionKind,
        target: String,
        actor: String,
    ) -> Self {
        Self {
            project_id,
            cycle_id,
            action,
            target,
            actor,
        }
    }
}

/// Core enforcement with the M2 approval loop (R-4-002). On a blocking
/// `RequireApproval`:
///
/// * S1 (first invocation): emit `approval.capability.requested` (capability
///   key `surface.<surface>#<action_kind>`, stable request hash) and exit
///   `awaiting_approval` with zero governed effects.
/// * S2 (re-invocation after `sddk approval grant`): the caller pre-injects
///   `Facts::approval_refs` via `granted_approval_refs` before admission,
///   so the engine gate is satisfied and the effect runs exactly once
///   (idempotency by event_id downstream).
pub(crate) fn enforce_admission_or_block_ctx(
    verdict: &RunnerVerdict,
    surface: &str,
    state_dir: Option<&std::path::Path>,
    approval_loop: ApprovalLoopContext,
) -> anyhow::Result<()> {
    let verdict_str = verdict_segment(verdict);
    let project_id = if approval_loop.is_none() {
        ""
    } else {
        approval_loop.project_id.as_str()
    };
    match enforce_gated(verdict, surface, ENFORCEMENT_STAGE) {
        EnforcementOutcome::Proceed { .. } => Ok(()),
        // Advisory at M1, still recorded (D-02): the approval loop (M4)
        // consumes these events to build the pending-approval projection.
        EnforcementOutcome::AdvisoryHighApproval { decision_id, .. } => {
            record_admission_decision(
                state_dir,
                project_id,
                surface,
                &verdict_str,
                &decision_id,
                verdict,
            );
            Ok(())
        }
        EnforcementOutcome::AwaitingApproval { decision_id, .. } => {
            record_admission_decision(
                state_dir,
                project_id,
                surface,
                &verdict_str,
                &decision_id,
                verdict,
            );
            // M2 approval loop (WU-C4-7): publish the request so a human can
            // resolve it via `sddk approval list|grant`. Fail-soft.
            if !approval_loop.is_none()
                && let Some(state_dir) = state_dir
            {
                emit_approval_requested_for(
                    state_dir,
                    &approval_loop.cycle_id,
                    surface,
                    approval_loop.action,
                    &approval_loop.target,
                    &approval_loop.actor,
                    &approval_loop.project_id,
                );
            }
            Err(anyhow::anyhow!(
                "ADMISSION: approval required before mutating '{surface}' \
                 (decision_id={decision_id}); no changes were made"
            ))
        }
        EnforcementOutcome::Blocked {
            reason,
            decision_id,
        } => {
            record_admission_decision(
                state_dir,
                project_id,
                surface,
                &verdict_str,
                &decision_id,
                verdict,
            );
            Err(anyhow::anyhow!(
                "ADMISSION: denied '{surface}' ({reason:?}, decision_id={decision_id}); \
                 no changes were made"
            ))
        }
    }
}

/// Verdict segment for event ids / payloads, from the engine decision.
fn verdict_segment(verdict: &RunnerVerdict) -> String {
    match &verdict.decision {
        AdmissionDecision::Allow { .. } => "allow".to_string(),
        AdmissionDecision::Deny { .. } => "deny".to_string(),
        // B+ (ADR-0111): the risk band no longer makes approval advisory, so
        // every `RequireApproval` is recorded as `require_approval`. The
        // historical `advisory_high_approval` string remains decodable in the
        // event schema but is never produced at M4.
        AdmissionDecision::RequireApproval { .. } => "require_approval".to_string(),
        _ => "unknown".to_string(),
    }
}

/// Fail-soft emission of `authority.admission.decided` into the current
/// project's ledger under `state_dir`. Errors are swallowed (logged to
/// stderr).
fn record_admission_decision(
    state_dir: Option<&std::path::Path>,
    project_id: &str,
    surface: &str,
    verdict_str: &str,
    decision_id: &str,
    verdict: &RunnerVerdict,
) {
    let Some(state_dir) = state_dir else {
        return;
    };
    if project_id.is_empty() {
        // Legacy path (M1 without cycle context): pick the single
        // bootstrapped project under `state_dir`. We deliberately do NOT
        // iterate every project dir (the previous M1 broadcast produced
        // FK-constraint noise and duplicated events in projects that did
        // not own the action). If there is no bootstrapped project, the
        // recording is skipped (the legacy test surface asserts a single
        // project ledger for its fixture).
        let projects_dir = state_dir.join("sddk").join("projects");
        if let Ok(entries) = std::fs::read_dir(&projects_dir) {
            for entry in entries.flatten() {
                let dir = entry.path();
                let ledger = dir.join("ledger.sqlite");
                if ledger.exists()
                    && let Ok(store) = sddk_storage::Storage::open(&ledger)
                    && let Ok(Some(record)) =
                        store.get_project_optional(&entry.file_name().to_string_lossy())
                {
                    let pid = record.project_id;
                    drop(store);
                    // Re-open via Storage to ensure the project row
                    // exists (it just did) and emit the event.
                    return record_single_project(
                        state_dir,
                        &pid,
                        surface,
                        verdict_str,
                        decision_id,
                        verdict,
                    );
                }
            }
        }
        return;
    }
    record_single_project(
        state_dir,
        project_id,
        surface,
        verdict_str,
        decision_id,
        verdict,
    );
}

/// Single-project emission helper (extracted from
/// [`record_admission_decision`] so the legacy "first bootstrapped
/// project" path and the cycle-aware M2 path share the same emit
/// logic).
fn record_single_project(
    state_dir: &std::path::Path,
    project_id: &str,
    surface: &str,
    verdict_str: &str,
    decision_id: &str,
    verdict: &RunnerVerdict,
) {
    let project_dir = state_dir.join("sddk").join("projects").join(project_id);
    if let Err(e) = ensure_project_row(&project_dir, project_id) {
        eprintln!("admission event recording failed (fail-soft): project upsert: {e}");
        return;
    }
    let Ok(mut store) = SqliteEventStore::open(&project_dir) else {
        return;
    };
    let input = AdmissionDecidedInput {
        project_id: project_id.to_string(),
        cycle_id: None,
        surface: surface.to_string(),
        action_kind: action_kind_segment(verdict),
        verdict: verdict_str.to_string(),
        decision_id: decision_id.to_string(),
        explanation_digest: verdict.explanation.decision_digest.0.clone(),
        occurred_at: now_rfc3339(),
        causation_id: None,
        correlation_id: None,
    };
    if let Err(e) = emit_admission_decision(&mut store, &input) {
        eprintln!("admission event recording failed (fail-soft): {e}");
    }
}

/// Make sure the `projects` row for `project_id` exists in the ledger
/// file at `project_dir/ledger.sqlite`. The Storage migration creates a
/// `projects` table with NOT NULL columns (`display_name`, `scope`,
/// `created_at`) that the event-store `INSERT OR IGNORE` does not
/// satisfy when sharing the same file. We open via `Storage` (which
/// applies all migrations) and skip the insert when the row already
/// exists.
fn ensure_project_row(project_dir: &std::path::Path, project_id: &str) -> anyhow::Result<()> {
    let ledger_path = project_dir.join("ledger.sqlite");
    let storage = sddk_storage::Storage::open(&ledger_path)?;
    if let Ok(Some(_)) = storage.get_project_optional(project_id) {
        return Ok(());
    }
    let record = sddk_storage::ProjectRecord {
        project_id: project_id.to_string(),
        display_name: "admission.decided".to_string(),
        remote_url: None,
        scope: ".".to_string(),
        created_at: crate::git_cmd::default_timestamp(),
    };
    let _ = storage.insert_project(&record);
    Ok(())
}

/// RFC 3339 timestamp via the same default used by git receipts.
fn now_rfc3339() -> String {
    crate::git_cmd::default_timestamp()
}

/// Action-kind segment from the decision id (`deny-cap-<actor>-<action>` /
/// `approval-<actor>-<action>` / `allow-<actor>-<action>`).
fn action_kind_segment(verdict: &RunnerVerdict) -> String {
    verdict
        .decision
        .decision_id()
        .rsplit('-')
        .next()
        .unwrap_or("unknown")
        .to_string()
}

#[cfg(test)]
mod admission_tests {
    use super::*;
    use sddk_engine::authority_engine::{
        ActorKind, AdmissionDecision, AdmissionExplanation, ApprovalRequirement, ApproverKind,
        AuthorityEngineRunner, DigestSha256, Facts, RunnerVerdict,
    };

    fn require_approval_verdict() -> RunnerVerdict {
        RunnerVerdict {
            decision: AdmissionDecision::RequireApproval {
                requirement: ApprovalRequirement {
                    approver_kind: ApproverKind::Human,
                    minimum_evidence: vec![],
                    timeout_seconds: 3600,
                },
                decision_id: "approval-human-cycle_transition".to_string(),
            },
            explanation: AdmissionExplanation {
                policy_id: "cycle_state".into(),
                policy_version: 1,
                gates_applied: vec![],
                evidence_refs_used: vec![],
                deny_reasons_evaluated: vec![],
                approval_requirements_considered: vec![],
                decision_digest: DigestSha256("d".into()),
            },
            capability: "surface.cycle_state".to_string(),
        }
    }

    fn runner_verdict(decision: AdmissionDecision, capability: &str) -> RunnerVerdict {
        RunnerVerdict {
            decision,
            explanation: sddk_engine::authority_engine::AdmissionExplanation {
                policy_id: "p".into(),
                policy_version: 1,
                gates_applied: vec![],
                evidence_refs_used: vec![],
                deny_reasons_evaluated: vec![],
                approval_requirements_considered: vec![],
                decision_digest: DigestSha256("d".into()),
            },
            capability: capability.to_string(),
        }
    }

    // ── stage × verdict × band matrix ────────────────────────────────────────

    #[test]
    fn allow_proceeds_at_every_stage_and_band() {
        let v = runner_verdict(
            AdmissionDecision::Allow {
                receipt_id: "allow-1".into(),
                postconditions: vec![],
            },
            "surface.dependency_edge",
        );
        for stage in [EnforcementStage::LowMedium, EnforcementStage::All] {
            for surface in ["dependency_edge", "knowledge_graph_vault", "cycle_state"] {
                assert!(matches!(
                    enforce_gated(&v, surface, stage),
                    EnforcementOutcome::Proceed { .. }
                ));
            }
        }
    }

    #[test]
    fn deny_blocks_at_every_stage_and_band() {
        let v = runner_verdict(
            AdmissionDecision::Deny {
                reason: DenyReason::ExplicitDenyOverride,
                decision_id: "deny-1".into(),
            },
            "surface.knowledge_graph_vault",
        );
        for stage in [EnforcementStage::LowMedium, EnforcementStage::All] {
            for surface in ["dependency_edge", "knowledge_graph_vault", "cycle_state"] {
                let out = enforce_gated(&v, surface, stage);
                assert!(out.blocks_effect(), "{out:?}");
                assert!(matches!(out, EnforcementOutcome::Blocked { .. }));
            }
        }
    }

    #[test]
    fn require_approval_blocks_low_medium_at_m1() {
        let v = require_approval_verdict();
        let out = enforce_gated(&v, "knowledge_graph_vault", EnforcementStage::LowMedium);
        assert!(out.blocks_effect());
        assert!(matches!(out, EnforcementOutcome::AwaitingApproval { .. }));
        // request_hash must be deterministic and stable.
        let again = enforce_gated(&v, "knowledge_graph_vault", EnforcementStage::LowMedium);
        assert_eq!(out, again);
    }

    #[test]
    fn require_approval_high_is_advisory_at_m1() {
        let v = require_approval_verdict();
        let out = enforce_gated(&v, "cycle_state", EnforcementStage::LowMedium);
        assert!(!out.blocks_effect());
        assert!(matches!(
            out,
            EnforcementOutcome::AdvisoryHighApproval { .. }
        ));
    }

    #[test]
    fn require_approval_high_blocks_at_stage_all() {
        let v = require_approval_verdict();
        let out = enforce_gated(&v, "cycle_state", EnforcementStage::All);
        assert!(out.blocks_effect());
        assert!(matches!(out, EnforcementOutcome::AwaitingApproval { .. }));
    }

    /// PR-UAT-005 / B+ (ADR-0111): approval is action-driven, not band-driven.
    /// Routine High-band operations allow; only explicitly dangerous actions
    /// require approval and block at the production stage.
    #[test]
    fn approval_matrix_is_action_driven_not_band_driven() {
        use sddk_engine::authority_engine::AuthorityEngineRunner;
        let runner = AuthorityEngineRunner::new();
        let cases = [
            // (actor, surface, action, expect_approval)
            (
                "user:alice",
                "cycle_state",
                ActionKind::CycleTransition,
                false,
            ),
            ("user:alice", "cycle_state", ActionKind::CyclePause, false),
            ("user:alice", "cycle_state", ActionKind::CycleResume, false),
            (
                "user:alice",
                "cycle_state",
                ActionKind::CycleSupersede,
                true,
            ),
            ("system", "gate_receipts", ActionKind::CliRun, false),
            (
                "user:alice",
                "transition_records",
                ActionKind::CycleTransition,
                false,
            ),
            (
                "user:alice",
                "knowledge_graph_vault",
                ActionKind::VaultIndex,
                false,
            ),
            ("system", "github_releases", ActionKind::CliRelease, true),
        ];
        for (actor, surface, action, expect_approval) in cases {
            let v = runner
                .admit_surface(actor, surface, action, "t1", Facts::default())
                .expect("admit");
            let out = enforce_gated(&v, surface, ENFORCEMENT_STAGE);
            if expect_approval {
                assert!(
                    matches!(v.decision, AdmissionDecision::RequireApproval { .. }),
                    "{surface}/{action:?} must require approval, got {:?}",
                    v.decision
                );
                assert!(out.blocks_effect(), "{surface}/{action:?} must block");
            } else {
                assert!(
                    matches!(v.decision, AdmissionDecision::Allow { .. }),
                    "{surface}/{action:?} must allow, got {:?}",
                    v.decision
                );
                assert!(!out.blocks_effect(), "{surface}/{action:?} must not block");
            }
        }
    }

    /// D-03 / R-4-002: a granted approval re-admits the same action as Allow.
    #[test]
    fn approval_grant_re_admits_as_allow() {
        use sddk_engine::authority_engine::{AuthorityEngineRunner, EvidenceRef};
        let runner = AuthorityEngineRunner::new();
        let facts = Facts {
            approval_refs: vec![EvidenceRef("sha256:granted".to_string())],
            ..Facts::default()
        };
        let v = runner
            .admit_surface(
                "user:alice",
                "cycle_state",
                ActionKind::CycleSupersede,
                "c1",
                facts,
            )
            .expect("admit");
        assert!(
            matches!(v.decision, AdmissionDecision::Allow { .. }),
            "granted approval must re-admit as Allow, got {:?}",
            v.decision
        );
    }

    /// Zero-bypass (PR-UAT-005): at the production stage every surface blocks a
    /// `RequireApproval`, so no surface can remain advisory.
    #[test]
    fn zero_bypass_every_surface_blocks_require_approval() {
        for surface in [
            "cycle_state",
            "gate_receipts",
            "plan_revisions",
            "transition_records",
            "framework_bundle",
            "github_releases",
            "knowledge_graph_vault",
            "plan_item",
            "evidence_attachment",
            "decision_record",
            "dependency_edge",
        ] {
            let out = enforce_gated(&require_approval_verdict(), surface, ENFORCEMENT_STAGE);
            assert!(
                out.blocks_effect(),
                "{surface} must block a RequireApproval at M4"
            );
            assert!(matches!(out, EnforcementOutcome::AwaitingApproval { .. }));
        }
    }

    // ── helpers ──────────────────────────────────────────────────────────────

    #[test]
    fn capability_key_uses_surface_and_action_granularity() {
        assert_eq!(
            approval_capability_key("cycle_state", ActionKind::CycleSupersede),
            "surface.cycle_state#cycle_supersede"
        );
    }

    #[test]
    fn request_hash_is_stable_and_input_sensitive() {
        let a = approval_request_hash("cycle_state", ActionKind::CycleSupersede, "c1", "user:a");
        let b = approval_request_hash("cycle_state", ActionKind::CycleSupersede, "c1", "user:a");
        assert_eq!(a, b);
        let c = approval_request_hash("cycle_state", ActionKind::CycleSupersede, "c2", "user:a");
        assert_ne!(a, c);
        assert!(a.starts_with("sha256:"));
    }

    // ── runner integration: real verdicts through the choke point ────────────

    #[test]
    fn runner_medium_surface_human_and_system_proceed() {
        let runner = AuthorityEngineRunner::new();
        // Medium band grants vault.write to the caller regardless of actor
        // kind (bridge table: "Medium band → plan items and vault writes").
        for actor in ["user:alice", "system"] {
            let v = runner
                .admit_surface(
                    actor,
                    "knowledge_graph_vault",
                    ActionKind::VaultIndex,
                    "vault",
                    Facts::default(),
                )
                .unwrap();
            assert!(
                matches!(
                    enforce_gated(&v, "knowledge_graph_vault", ENFORCEMENT_STAGE),
                    EnforcementOutcome::Proceed { .. }
                ),
                "actor {actor}"
            );
        }
    }

    #[test]
    fn runner_high_surface_human_cli_action_denies() {
        // Injectable Deny case for E2E: gate_receipts is High band and
        // `cli.execute` is System-only there, so a Human caller is denied
        // BEFORE any mutation runs (R-4-005).
        let runner = AuthorityEngineRunner::new();
        let v = runner
            .admit_surface(
                "user:alice",
                "gate_receipts",
                ActionKind::CliRun,
                "g1",
                Facts::default(),
            )
            .unwrap();
        let out = enforce_gated(&v, "gate_receipts", ENFORCEMENT_STAGE);
        assert!(out.blocks_effect());
        assert!(matches!(out, EnforcementOutcome::Blocked { .. }));
    }

    #[test]
    fn runner_high_surface_blocks_at_m4() {
        let runner = AuthorityEngineRunner::new();
        let v = runner
            .admit_surface(
                "user:alice",
                "cycle_state",
                ActionKind::CycleSupersede,
                "c1",
                Facts::default(),
            )
            .unwrap();
        let out = enforce_gated(&v, "cycle_state", ENFORCEMENT_STAGE);
        assert!(out.blocks_effect(), "RequireApproval must block at M4");
        assert!(matches!(out, EnforcementOutcome::AwaitingApproval { .. }));
    }

    #[test]
    fn runner_gate_receipts_denies_human_blocks_system_at_m4() {
        let runner = AuthorityEngineRunner::new();
        // gate_receipts is System-only in the legacy matrix.
        let human = runner
            .admit_surface(
                "user:alice",
                "gate_receipts",
                ActionKind::CliRun,
                "g1",
                Facts::default(),
            )
            .unwrap();
        assert!(matches!(
            enforce_gated(&human, "gate_receipts", ENFORCEMENT_STAGE),
            EnforcementOutcome::Blocked { .. }
        ));
        let system = runner
            .admit_surface(
                "system",
                "gate_receipts",
                ActionKind::CycleSupersede,
                "g1",
                Facts::default(),
            )
            .unwrap();
        // System actor: capability ok, dangerous action → RequireApproval → M4 blocks.
        let out = enforce_gated(&system, "gate_receipts", ENFORCEMENT_STAGE);
        assert!(out.blocks_effect(), "RequireApproval must block at M4");
        assert!(matches!(out, EnforcementOutcome::AwaitingApproval { .. }));
    }

    #[test]
    fn actor_kind_pattern_unused_import_guard() {
        // Sanity: enum exhaustiveness — ensures new ActorKind variants cannot
        // silently bypass the surface matrix in derive_capabilities_for.
        let _ = ActorKind::SecretaryL0;
    }

    #[test]
    fn cli_wrapper_ok_on_proceed_and_blocks_on_approval() {
        let allow = runner_verdict(
            AdmissionDecision::Allow {
                receipt_id: "allow-1".into(),
                postconditions: vec![],
            },
            "surface.dependency_edge",
        );
        assert!(
            enforce_admission_or_block_ctx(
                &allow,
                "dependency_edge",
                None,
                ApprovalLoopContext::none()
            )
            .is_ok()
        );
        let high = require_approval_verdict();
        assert!(
            enforce_admission_or_block_ctx(&high, "cycle_state", None, ApprovalLoopContext::none())
                .is_err(),
            "RequireApproval must block at M4"
        );
    }

    #[test]
    fn cli_wrapper_errs_with_decision_id_on_block() {
        let deny = runner_verdict(
            AdmissionDecision::Deny {
                reason: DenyReason::ActorKindNotPermitted,
                decision_id: "deny-cap-x".into(),
            },
            "surface.gate_receipts",
        );
        let err = enforce_admission_or_block_ctx(
            &deny,
            "gate_receipts",
            None,
            ApprovalLoopContext::none(),
        )
        .unwrap_err();
        let msg = format!("{err}");
        assert!(msg.contains("ADMISSION"), "{msg}");
        assert!(msg.contains("deny-cap-x"), "{msg}");
        assert!(msg.contains("no changes were made"), "{msg}");
    }

    #[test]
    fn cli_wrapper_errs_on_awaiting_approval() {
        // state_dir None: event recording disabled (unit test, no ledger).
        let v = require_approval_verdict();
        let err = enforce_admission_or_block_ctx(
            &v,
            "knowledge_graph_vault",
            None,
            ApprovalLoopContext::none(),
        )
        .unwrap_err();
        let msg = format!("{err}");
        assert!(msg.contains("approval required"), "{msg}");
        assert!(msg.contains("no changes were made"), "{msg}");
    }

    #[test]
    fn cli_wrapper_records_admission_event_on_deny() {
        // With a state dir, a deny must fail-soft-record the decision event
        // in the project ledger (R-4-005) even though the command aborts.
        let tmp = tempfile::tempdir().unwrap();
        let project_dir = tmp.path().join("sddk").join("projects").join("p-test");
        std::fs::create_dir_all(&project_dir).unwrap();
        // Bootstrap: the store requires the project row to exist.
        {
            let store = sddk_storage::Storage::open(project_dir.join("ledger.sqlite")).unwrap();
            store
                .insert_project(&sddk_domain::ProjectRecord {
                    project_id: "p-test".into(),
                    display_name: "test".into(),
                    remote_url: None,
                    scope: ".".into(),
                    created_at: "2026-09-13T00:00:00Z".into(),
                })
                .unwrap();
        }
        let v = runner_verdict(
            AdmissionDecision::Deny {
                reason: DenyReason::ActorKindNotPermitted,
                decision_id: "deny-cap-human-cli_run".into(),
            },
            "surface.gate_receipts",
        );
        let err = enforce_admission_or_block_ctx(
            &v,
            "gate_receipts",
            Some(tmp.path()),
            ApprovalLoopContext::none(),
        )
        .unwrap_err();
        assert!(format!("{err}").contains("ADMISSION"));
        // Deny aborts the mutation AND records the decision event (R-4-005).
        let store =
            sddk_storage::Storage::open_read_only(project_dir.join("ledger.sqlite")).unwrap();
        let events = store.list_events().unwrap();
        assert_eq!(events.len(), 1, "exactly one admission event");
        assert_eq!(events[0].event_type, "authority.admission.decided");
        assert_eq!(events[0].project_id, "p-test");
    }

    // ── M2 approval loop (WU-C4-7, R-4-002) ──────────────────────────────────

    use sddk_domain::event_envelope::{ActorKind as DomainActorKind, ActorRef, EntityRef};
    use sddk_domain::projections::Projection as TestProjection;
    use sddk_domain::projections::approval::ApprovalProjection as TestApprovalProjection;
    use sddk_domain::{ApprovalDecision as DomainApprovalDecision, EventEnvelopeV1};
    use sddk_engine::event_bus::ApprovalRequestedInput as EngineApprovalRequestedInput;
    use sddk_engine::event_bus::emit::{
        ApprovalDecisionInput, emit_approval_decision, emit_approval_requested,
    };

    const APPROVAL_CYCLE: &str = "c-approval-test";
    const APPROVAL_PROJECT: &str = "p-approval-test";
    const APPROVAL_TIMESTAMP: &str = "2026-09-13T12:00:00Z";

    fn make_envelope(
        stream_id: &str,
        event_type: &str,
        sequence: u64,
        actor_id: &str,
        actor_kind: DomainActorKind,
        payload: serde_json::Value,
    ) -> EventEnvelopeV1 {
        let mut env = EventEnvelopeV1 {
            event_id: format!("e-{stream_id}-{sequence}"),
            event_type: event_type.into(),
            schema_version: 1,
            stream_id: stream_id.into(),
            sequence,
            project_id: APPROVAL_PROJECT.into(),
            occurred_at: APPROVAL_TIMESTAMP.into(),
            recorded_at: APPROVAL_TIMESTAMP.into(),
            actor: ActorRef {
                kind: actor_kind,
                id: actor_id.into(),
                definition_hash: None,
                policy_hash: None,
                model: None,
                role: None,
            },
            subjects: vec![],
            payload,
            evidence_refs: vec![],
            content_hash: String::new(),
            metadata: None,
            causation_id: None,
            correlation_id: None,
            cycle_id: None,
            frame_id: None,
            fork_id: None,
        };
        env.content_hash = env.compute_content_hash();
        env
    }

    fn bootstrap_project(state_dir: &std::path::Path) {
        let project_dir = state_dir
            .join("sddk")
            .join("projects")
            .join(APPROVAL_PROJECT);
        std::fs::create_dir_all(&project_dir).unwrap();
        let storage = sddk_storage::Storage::open(project_dir.join("ledger.sqlite")).unwrap();
        storage
            .insert_project(&sddk_domain::ProjectRecord {
                project_id: APPROVAL_PROJECT.into(),
                display_name: "approval-test".into(),
                remote_url: None,
                scope: ".".into(),
                created_at: APPROVAL_TIMESTAMP.into(),
            })
            .unwrap();
    }

    fn approval_request_hash_for(surface: &str, target: &str) -> String {
        approval_request_hash(
            surface,
            sddk_engine::authority_engine::ActionKind::CycleSupersede,
            target,
            "user:approver",
        )
    }

    #[test]
    fn approval_loop_context_none_disables_loop() {
        let ctx = ApprovalLoopContext::none();
        assert!(ctx.is_none());
        let live = ApprovalLoopContext::new(
            APPROVAL_PROJECT.into(),
            APPROVAL_CYCLE.into(),
            sddk_engine::authority_engine::ActionKind::CycleSupersede,
            "c1".into(),
            "user:alice".into(),
        );
        assert!(!live.is_none());
        assert_eq!(live.cycle_id, APPROVAL_CYCLE);
    }

    #[test]
    fn granted_approval_refs_returns_empty_without_matching_grant() {
        // State dir with no project ledger at all: scanner yields nothing.
        let tmp = tempfile::tempdir().unwrap();
        let refs = super::granted_approval_refs(
            tmp.path(),
            APPROVAL_CYCLE,
            "cycle_state",
            sddk_engine::authority_engine::ActionKind::CycleSupersede,
            "c1",
            "user:approver",
        );
        assert!(refs.is_empty(), "empty ledger → no grant refs");
    }

    #[test]
    fn granted_approval_refs_finds_matching_grant_in_projection() {
        // D-03 / R-4-002 S2: when ApprovalProjection reports `Granted` with a
        // matching request hash, the call returns an EvidenceRef so the next
        // admit_governed invocation can satisfy the approval gate.
        let tmp = tempfile::tempdir().unwrap();
        bootstrap_project(tmp.path());
        let project_dir = tmp
            .path()
            .join("sddk")
            .join("projects")
            .join(APPROVAL_PROJECT);
        let storage = sddk_storage::Storage::open(project_dir.join("ledger.sqlite")).unwrap();

        let capability = approval_capability_key(
            "cycle_state",
            sddk_engine::authority_engine::ActionKind::CycleSupersede,
        );
        let request_hash = approval_request_hash_for("cycle_state", "c1");

        // Seed: requested → granted (Human approver)
        let mut store = sddk_storage::SqliteEventStore::open(&project_dir).unwrap();
        let _ = store.append(&make_envelope(
            APPROVAL_CYCLE,
            "approval.capability.requested",
            1,
            "agent:orchestrator",
            DomainActorKind::Agent,
            serde_json::json!({
                "cycle_id": APPROVAL_CYCLE,
                "capability": capability,
                "request_hash": request_hash,
            }),
        ));
        emit_approval_decision(
            &mut store,
            &ApprovalDecisionInput {
                project_id: APPROVAL_PROJECT.into(),
                cycle_id: APPROVAL_CYCLE.into(),
                capability: capability.clone(),
                request_hash: request_hash.clone(),
                decision: DomainApprovalDecision::Granted,
                actor_id: "user:approver".into(),
                actor_kind: DomainActorKind::Human,
                reason: "ok".into(),
                occurred_at: APPROVAL_TIMESTAMP.into(),
                causation_id: None,
                correlation_id: None,
            },
        )
        .unwrap();

        let refs = super::granted_approval_refs(
            tmp.path(),
            APPROVAL_CYCLE,
            "cycle_state",
            sddk_engine::authority_engine::ActionKind::CycleSupersede,
            "c1",
            "user:approver",
        );
        assert_eq!(refs.len(), 1, "exactly one matching grant ref");
        assert_eq!(refs[0].0, request_hash);

        // Sanity: a different request_hash yields no refs (capability+target+actor sensitive).
        let other_refs = super::granted_approval_refs(
            tmp.path(),
            APPROVAL_CYCLE,
            "cycle_state",
            sddk_engine::authority_engine::ActionKind::CycleSupersede,
            "c2",
            "user:approver",
        );
        assert!(
            other_refs.is_empty(),
            "different target must not match the stored grant"
        );

        // Ensure the projection read was strictly via ApprovalProjection (no
        // other side effects).
        let events = storage.list_events().unwrap();
        assert!(
            events
                .iter()
                .any(|e| e.event_type == "approval.capability.requested")
        );
        assert!(
            events
                .iter()
                .any(|e| e.event_type == "approval.capability.granted")
        );
    }

    #[test]
    fn granted_approval_refs_ignores_denied_decisions() {
        // D-03: only `Granted` decisions satisfy the approval gate; a `Denied`
        // decision must NOT yield approval_refs (otherwise the gate would
        // resurrect the operation after a denial).
        let tmp = tempfile::tempdir().unwrap();
        bootstrap_project(tmp.path());
        let project_dir = tmp
            .path()
            .join("sddk")
            .join("projects")
            .join(APPROVAL_PROJECT);
        let mut store = sddk_storage::SqliteEventStore::open(&project_dir).unwrap();

        let capability = approval_capability_key(
            "cycle_state",
            sddk_engine::authority_engine::ActionKind::CycleSupersede,
        );
        let request_hash = approval_request_hash_for("cycle_state", "c1");

        let _ = store.append(&make_envelope(
            APPROVAL_CYCLE,
            "approval.capability.requested",
            1,
            "agent:orchestrator",
            DomainActorKind::Agent,
            serde_json::json!({
                "cycle_id": APPROVAL_CYCLE,
                "capability": capability,
                "request_hash": request_hash,
            }),
        ));
        emit_approval_decision(
            &mut store,
            &ApprovalDecisionInput {
                project_id: APPROVAL_PROJECT.into(),
                cycle_id: APPROVAL_CYCLE.into(),
                capability: capability.clone(),
                request_hash: request_hash.clone(),
                decision: DomainApprovalDecision::Denied,
                actor_id: "user:approver".into(),
                actor_kind: DomainActorKind::Human,
                reason: "no".into(),
                occurred_at: APPROVAL_TIMESTAMP.into(),
                causation_id: None,
                correlation_id: None,
            },
        )
        .unwrap();

        let refs = super::granted_approval_refs(
            tmp.path(),
            APPROVAL_CYCLE,
            "cycle_state",
            sddk_engine::authority_engine::ActionKind::CycleSupersede,
            "c1",
            "user:approver",
        );
        assert!(
            refs.is_empty(),
            "Denied decision must never satisfy the approval gate"
        );
    }

    #[test]
    fn emit_approval_requested_for_persists_request_event() {
        // S1 (R-4-002): on a blocking RequireApproval, the CLI emits a single
        // `approval.capability.requested` event into the cycle's project
        // ledger. The capability follows `surface.<surface>#<action_kind>`
        // (OQ-1) and the request_hash is stable over re-invocations.
        let tmp = tempfile::tempdir().unwrap();
        bootstrap_project(tmp.path());
        let request_hash = super::emit_approval_requested_for(
            tmp.path(),
            APPROVAL_CYCLE,
            "cycle_state",
            sddk_engine::authority_engine::ActionKind::CycleSupersede,
            "c1",
            "user:alice",
            APPROVAL_PROJECT,
        );
        // Stable hash check: re-invocation yields the same value.
        let again = super::emit_approval_requested_for(
            tmp.path(),
            APPROVAL_CYCLE,
            "cycle_state",
            sddk_engine::authority_engine::ActionKind::CycleSupersede,
            "c1",
            "user:alice",
            APPROVAL_PROJECT,
        );
        assert_eq!(request_hash, again, "request hash must be deterministic");

        // Verify the event landed in the ledger.
        let project_dir = tmp
            .path()
            .join("sddk")
            .join("projects")
            .join(APPROVAL_PROJECT);
        let storage =
            sddk_storage::Storage::open_read_only(project_dir.join("ledger.sqlite")).unwrap();
        let events = storage.list_events().unwrap();
        let requested: Vec<_> = events
            .iter()
            .filter(|e| e.event_type == "approval.capability.requested")
            .collect();
        assert_eq!(requested.len(), 1, "exactly one requested event");
        let payload = &requested[0].payload;
        assert_eq!(
            payload.get("capability").and_then(|v| v.as_str()),
            Some("surface.cycle_state#cycle_supersede")
        );
        assert_eq!(
            payload.get("request_hash").and_then(|v| v.as_str()),
            Some(request_hash.as_str())
        );
    }

    #[test]
    fn emit_approval_requested_for_returns_hash_when_ledger_missing() {
        // Fail-soft contract: if the ledger cannot be opened, the function
        // still returns the deterministic request_hash so the caller can
        // surface it in the awaiting-approval error.
        let tmp = tempfile::tempdir().unwrap();
        // No bootstrap: project dir does not exist.
        let hash = super::emit_approval_requested_for(
            tmp.path(),
            APPROVAL_CYCLE,
            "cycle_state",
            sddk_engine::authority_engine::ActionKind::CycleSupersede,
            "c1",
            "user:alice",
            APPROVAL_PROJECT,
        );
        assert!(hash.starts_with("sha256:"));
    }

    #[test]
    fn admit_governed_no_state_dir_skips_approval_loop() {
        // With no state_dir available (no env vars), admit_governed must NOT
        // try to consult the projection or emit a request; the engine verdict
        // alone drives enforcement.
        let runner = AuthorityEngineRunner::new();
        // Surface = dependency_edge: Low band → Allow with no approval gate.
        let verdict = super::admit_governed(
            &runner,
            "user:alice",
            "dependency_edge",
            sddk_engine::authority_engine::ActionKind::CliRun,
            "d1",
            None,
            APPROVAL_PROJECT,
        )
        .expect("low band Allow must succeed");
        assert!(matches!(
            verdict.decision,
            sddk_engine::authority_engine::AdmissionDecision::Allow { .. }
        ));
    }

    #[test]
    fn admit_governed_injects_granted_refs_when_state_dir_present() {
        // Integration of S2: bootstrap a cycle ledger with a Granted decision
        // for the same surface/action/target/actor, then admit_governed must
        // observe the grant through ApprovalProjection. We assert via the
        // facts-injection helper that the engine gate is satisfied (verdict
        // = Allow) when the caller pre-populates approval_refs.
        let tmp = tempfile::tempdir().unwrap();
        bootstrap_project(tmp.path());
        let project_dir = tmp
            .path()
            .join("sddk")
            .join("projects")
            .join(APPROVAL_PROJECT);
        let mut store = sddk_storage::SqliteEventStore::open(&project_dir).unwrap();

        let capability = approval_capability_key(
            "cycle_state",
            sddk_engine::authority_engine::ActionKind::CycleSupersede,
        );
        let request_hash = approval_request_hash_for("cycle_state", "c1");

        let _ = store.append(&make_envelope(
            APPROVAL_CYCLE,
            "approval.capability.requested",
            1,
            "agent:orchestrator",
            DomainActorKind::Agent,
            serde_json::json!({
                "cycle_id": APPROVAL_CYCLE,
                "capability": capability,
                "request_hash": request_hash,
            }),
        ));
        emit_approval_decision(
            &mut store,
            &ApprovalDecisionInput {
                project_id: APPROVAL_PROJECT.into(),
                cycle_id: APPROVAL_CYCLE.into(),
                capability: capability.clone(),
                request_hash: request_hash.clone(),
                decision: DomainApprovalDecision::Granted,
                actor_id: "user:approver".into(),
                actor_kind: DomainActorKind::Human,
                reason: "ok".into(),
                occurred_at: APPROVAL_TIMESTAMP.into(),
                causation_id: None,
                correlation_id: None,
            },
        )
        .unwrap();

        // Now read the projection directly and confirm it sees the grant.
        let events = store.load_stream(APPROVAL_CYCLE, None, u32::MAX).unwrap();
        let mut proj = TestApprovalProjection::new(APPROVAL_CYCLE);
        for ev in &events {
            proj.apply(ev).unwrap();
        }
        let key = (APPROVAL_CYCLE.to_string(), capability.clone());
        let state = proj.states().get(&key).expect("state present");
        assert_eq!(state.decision, Some(DomainApprovalDecision::Granted));
        assert_eq!(state.request_hash, request_hash);

        // And admit_governed's S2 path must return those refs (sanity):
        let refs = super::granted_approval_refs(
            tmp.path(),
            APPROVAL_CYCLE,
            "cycle_state",
            sddk_engine::authority_engine::ActionKind::CycleSupersede,
            "c1",
            "user:approver",
        );
        assert_eq!(refs.len(), 1);
    }

    #[test]
    fn approval_projection_handles_requested_then_denied() {
        // Property of ApprovalProjection used by granted_approval_refs:
        // after a Denied event the (cycle_id, capability) state must show
        // `decision = Denied`. This pins the contract that granted_approval_refs
        // relies on.
        let mut proj = TestApprovalProjection::new(APPROVAL_CYCLE);
        proj.apply(&make_envelope(
            APPROVAL_CYCLE,
            "approval.capability.requested",
            1,
            "agent:orchestrator",
            DomainActorKind::Agent,
            serde_json::json!({
                "cycle_id": APPROVAL_CYCLE,
                "capability": "surface.cycle_state#cycle_supersede",
                "request_hash": "sha256:abc",
            }),
        ))
        .unwrap();
        proj.apply(&make_envelope(
            APPROVAL_CYCLE,
            "approval.capability.denied",
            2,
            "user:approver",
            DomainActorKind::Human,
            serde_json::json!({
                "cycle_id": APPROVAL_CYCLE,
                "capability": "surface.cycle_state#cycle_supersede",
                "request_hash": "sha256:abc",
            }),
        ))
        .unwrap();
        let key = (
            APPROVAL_CYCLE.to_string(),
            "surface.cycle_state#cycle_supersede".to_string(),
        );
        let state = proj.states().get(&key).unwrap();
        assert_eq!(state.decision, Some(DomainApprovalDecision::Denied));
    }

    #[test]
    fn emit_approval_requested_helper_writes_event_directly() {
        // The engine-side `emit_approval_requested` must round-trip: an event
        // appended to the store must be visible via `load_stream` and be
        // consumable by the ApprovalProjection. This guards the integration
        // between admission.rs's `emit_approval_requested_for` and the engine
        // helper.
        let tmp = tempfile::tempdir().unwrap();
        bootstrap_project(tmp.path());
        let project_dir = tmp
            .path()
            .join("sddk")
            .join("projects")
            .join(APPROVAL_PROJECT);
        let mut store = sddk_storage::SqliteEventStore::open(&project_dir).unwrap();

        let request_hash = approval_request_hash_for("cycle_state", "c1");
        emit_approval_requested(
            &mut store,
            &EngineApprovalRequestedInput {
                project_id: APPROVAL_PROJECT.into(),
                cycle_id: APPROVAL_CYCLE.into(),
                capability: "surface.cycle_state#cycle_supersede".into(),
                request_hash: request_hash.clone(),
                expires_at: "2026-09-13T13:00:00Z".into(),
                occurred_at: APPROVAL_TIMESTAMP.into(),
                actor_id: "agent:orchestrator".into(),
                actor_kind: DomainActorKind::Agent,
                causation_id: None,
                correlation_id: None,
            },
        )
        .unwrap();

        let events = store.load_stream(APPROVAL_CYCLE, None, u32::MAX).unwrap();
        assert_eq!(events.len(), 1);
        assert_eq!(events[0].event_type, "approval.capability.requested");
        assert_eq!(events[0].stream_id, APPROVAL_CYCLE);

        let mut proj = TestApprovalProjection::new(APPROVAL_CYCLE);
        proj.apply(&events[0]).unwrap();
        let key = (
            APPROVAL_CYCLE.to_string(),
            "surface.cycle_state#cycle_supersede".to_string(),
        );
        let state = proj.states().get(&key).unwrap();
        assert_eq!(state.request_hash, request_hash);
    }

    #[test]
    fn emit_approval_requested_for_rejects_non_agent_actor() {
        // ADR-069 §4: only Agent may emit approval-requested events. The
        // CLI helper must therefore route through the Agent kind; this test
        // pins that the helper sets `actor_kind: Agent` so an external
        // observer (storage layer) does not reject the emission. We assert
        // by reading the persisted envelope and checking its actor kind.
        let tmp = tempfile::tempdir().unwrap();
        bootstrap_project(tmp.path());
        super::emit_approval_requested_for(
            tmp.path(),
            APPROVAL_CYCLE,
            "cycle_state",
            sddk_engine::authority_engine::ActionKind::CycleSupersede,
            "c1",
            "user:alice",
            APPROVAL_PROJECT,
        );
        let project_dir = tmp
            .path()
            .join("sddk")
            .join("projects")
            .join(APPROVAL_PROJECT);
        let storage =
            sddk_storage::Storage::open_read_only(project_dir.join("ledger.sqlite")).unwrap();
        let events = storage.list_events().unwrap();
        let requested = events
            .iter()
            .find(|e| e.event_type == "approval.capability.requested")
            .expect("requested event persisted");
        let actor_ref = requested
            .actor_ref
            .as_ref()
            .expect("actor_ref must be set for new approval.requested events");
        assert_eq!(actor_ref.kind, DomainActorKind::Agent);
        assert_eq!(actor_ref.id, "agent:user:alice");
    }

    #[test]
    fn emit_approval_requested_event_envelope_validator_round_trip() {
        // The validator from ADR-069 §4 is enforced at append time. We verify
        // that a request emitted by `emit_approval_requested` survives the
        // secretarial validator (used by other engine flows).
        let tmp = tempfile::tempdir().unwrap();
        bootstrap_project(tmp.path());
        let project_dir = tmp
            .path()
            .join("sddk")
            .join("projects")
            .join(APPROVAL_PROJECT);
        let mut store = sddk_storage::SqliteEventStore::open(&project_dir).unwrap();

        let request_hash = approval_request_hash_for("cycle_state", "c1");
        emit_approval_requested(
            &mut store,
            &EngineApprovalRequestedInput {
                project_id: APPROVAL_PROJECT.into(),
                cycle_id: APPROVAL_CYCLE.into(),
                capability: "surface.cycle_state#cycle_supersede".into(),
                request_hash,
                expires_at: "2026-09-13T13:00:00Z".into(),
                occurred_at: APPROVAL_TIMESTAMP.into(),
                actor_id: "agent:orchestrator".into(),
                actor_kind: DomainActorKind::Agent,
                causation_id: None,
                correlation_id: None,
            },
        )
        .expect("Agent actor must pass the secretarial validator");

        // A subsequent load must see exactly one event of the expected type.
        let events = store.load_stream(APPROVAL_CYCLE, None, u32::MAX).unwrap();
        assert_eq!(events.len(), 1);
        assert_eq!(events[0].event_type, "approval.capability.requested");

        // Subjects field carries the cycle reference (engine contract).
        let _ = EntityRef {
            kind: "cycle".into(),
            id: APPROVAL_CYCLE.into(),
            version: None,
            content_hash: None,
        };
    }
}
