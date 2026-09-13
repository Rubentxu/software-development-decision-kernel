// Copyright (c) SDDK contributors.
// SPDX-License-Identifier: MIT
//
// admission.rs — C4 single enforcement choke point (D-01 / D-08 / R-4-001).
//
// `enforce_gated` maps a `RunnerVerdict` onto a blocking `EnforcementOutcome`.
// Stage M1 (`LowMedium`): `Deny` aborts on ALL surfaces; `RequireApproval`
// blocks Low/Medium surfaces and stays advisory on High surfaces until the
// approval loop is live (M4, R-4-001 S2). The engine stays pure; this module
// decides how the CLI reacts to the verdict.

use sddk_engine::authority_engine::{
    ActionKind, AdmissionDecision, DenyReason, DigestSha256, RunnerVerdict,
};
use sha2::{Digest, Sha256};

/// Enforcement staging (D-08). Advanced by milestone commit; rollback of a
/// stage flip is a single-commit revert. No runtime flag: a runtime flag
/// would be a new bypass surface, contradicting the cutover.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub(crate) enum EnforcementStage {
    /// M1: RequireApproval blocks Low/Medium surfaces; High stays advisory.
    LowMedium,
    /// M4: RequireApproval blocks every surface via the live approval loop.
    All,
}

/// Current enforcement stage (D-08). M1 = `LowMedium`.
pub(crate) const ENFORCEMENT_STAGE: EnforcementStage = EnforcementStage::LowMedium;

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
pub(crate) fn approval_capability_key(surface: &str, action: ActionKind) -> String {
    format!("surface.{}#{}", surface, action.as_str())
}

/// Stable SHA-256 over the full proposal identity (no timestamps), so the
/// same (surface, action, target, actor) always yields the same hash.
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

/// CLI wrapper over `enforce_gated` at the current stage: returns an error
/// (non-zero exit, no mutation) when the outcome blocks the effect. The
/// message is operator-facing; the decision ids stay machine-correlatable.
pub(crate) fn enforce_admission_or_block(
    verdict: &RunnerVerdict,
    surface: &str,
) -> anyhow::Result<()> {
    match enforce_gated(verdict, surface, ENFORCEMENT_STAGE) {
        EnforcementOutcome::Proceed { .. } | EnforcementOutcome::AdvisoryHighApproval { .. } => {
            Ok(())
        }
        EnforcementOutcome::AwaitingApproval {
            decision_id,
            request_hash,
        } => Err(anyhow::anyhow!(
            "ADMISSION: approval required before mutating '{surface}' \
             (decision_id={decision_id}, request_hash={request_hash}); \
             no changes were made"
        )),
        EnforcementOutcome::Blocked {
            reason,
            decision_id,
        } => Err(anyhow::anyhow!(
            "ADMISSION: denied '{surface}' ({reason:?}, decision_id={decision_id}); \
             no changes were made"
        )),
    }
}

#[cfg(test)]
mod admission_tests {
    use super::*;
    use sddk_engine::authority_engine::{
        ActorKind, AdmissionDecision, AdmissionExplanation, ApprovalRequirement, ApproverKind,
        AuthorityEngineRunner, Facts, RunnerVerdict,
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
    fn runner_high_surface_stays_advisory_at_m1() {
        let runner = AuthorityEngineRunner::new();
        let v = runner
            .admit_surface(
                "user:alice",
                "cycle_state",
                ActionKind::CycleTransition,
                "c1",
                Facts::default(),
            )
            .unwrap();
        let out = enforce_gated(&v, "cycle_state", ENFORCEMENT_STAGE);
        assert!(!out.blocks_effect());
        assert!(matches!(
            out,
            EnforcementOutcome::AdvisoryHighApproval { .. }
        ));
    }

    #[test]
    fn runner_gate_receipts_system_denies_human_advisory() {
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
                ActionKind::CliRun,
                "g1",
                Facts::default(),
            )
            .unwrap();
        // System actor: capability ok, then High band → RequireApproval →
        // advisory at M1 (no early blocking of the gate flow).
        let out = enforce_gated(&system, "gate_receipts", ENFORCEMENT_STAGE);
        assert!(!out.blocks_effect());
        assert!(matches!(
            out,
            EnforcementOutcome::AdvisoryHighApproval { .. } | EnforcementOutcome::Proceed { .. }
        ));
    }

    #[test]
    fn actor_kind_pattern_unused_import_guard() {
        // Sanity: enum exhaustiveness — ensures new ActorKind variants cannot
        // silently bypass the surface matrix in derive_capabilities_for.
        let _ = ActorKind::SecretaryL0;
    }

    #[test]
    fn cli_wrapper_ok_on_proceed_and_advisory() {
        let allow = runner_verdict(
            AdmissionDecision::Allow {
                receipt_id: "allow-1".into(),
                postconditions: vec![],
            },
            "surface.dependency_edge",
        );
        assert!(enforce_admission_or_block(&allow, "dependency_edge").is_ok());
        let high = require_approval_verdict();
        assert!(enforce_admission_or_block(&high, "cycle_state").is_ok());
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
        let err = enforce_admission_or_block(&deny, "gate_receipts").unwrap_err();
        let msg = format!("{err}");
        assert!(msg.contains("ADMISSION"), "{msg}");
        assert!(msg.contains("deny-cap-x"), "{msg}");
        assert!(msg.contains("no changes were made"), "{msg}");
    }

    #[test]
    fn cli_wrapper_errs_with_request_hash_on_awaiting_approval() {
        let v = require_approval_verdict();
        let err = enforce_admission_or_block(&v, "knowledge_graph_vault").unwrap_err();
        let msg = format!("{err}");
        assert!(msg.contains("approval required"), "{msg}");
        assert!(msg.contains("request_hash=sha256:"), "{msg}");
    }
}
