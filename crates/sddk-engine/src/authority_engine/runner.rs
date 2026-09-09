// Copyright (c) SDDK contributors.
// SPDX-License-Identifier: MIT
//
// runner.rs — SPEC-M5: High-level admission facade.
//
// Wraps `DefaultAuthorityEngine` and provides a single canonical entry point
// for CLI call sites to check admission before invoking an engine function.
// The legacy `AuthorityContext::validate(WritableSurface)` calls inside
// engine entry points remain as defense-in-depth (compat mirror, M9).

use sddk_domain::ActorKind as DomainActorKind;

use crate::authority::infer_actor_kind;
use crate::authority_engine::{
    ActionKind, ActionProposal, Actor, AdmissionDecision, AdmissionExplanation, AuthorityEngine,
    AuthorityEngineError, Capability, DefaultAuthorityEngine, Facts, PolicySnapshot, RiskBand,
    UtcTimestamp,
};

use super::bridge::default_policy_for_surface;

/// Outcome of a `admit_surface` call.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RunnerVerdict {
    pub decision: AdmissionDecision,
    pub explanation: AdmissionExplanation,
    pub capability: Capability,
}

/// High-level admission facade.
#[derive(Debug)]
pub struct AuthorityEngineRunner {
    engine: DefaultAuthorityEngine,
}

impl Default for AuthorityEngineRunner {
    fn default() -> Self {
        Self::new()
    }
}

impl AuthorityEngineRunner {
    /// Construct a runner with default policies registered.
    pub fn new() -> Self {
        Self {
            engine: DefaultAuthorityEngine::new(),
        }
    }

    /// Construct a runner with the given policies registered in order.
    pub fn with_policies(policies: Vec<PolicySnapshot>) -> Result<Self, AuthorityEngineError> {
        let mut engine = DefaultAuthorityEngine::new();
        for p in policies {
            engine.register_policy(p)?;
        }
        Ok(Self { engine })
    }

    /// Check admission for the given `(actor, surface)` pair with the supplied
    /// `action` kind and `target_id`. Returns a `RunnerVerdict` containing
    /// the engine's `AdmissionDecision`, the corresponding
    /// `AdmissionExplanation`, and the capability string for the surface.
    pub fn admit_surface(
        &self,
        actor: &str,
        surface: &str,
        action: ActionKind,
        target_id: &str,
        facts: Facts,
    ) -> Result<RunnerVerdict, AuthorityEngineError> {
        let policy = default_policy_for_surface(surface)?;
        let actor_kind = infer_actor_kind(actor);
        let actor_obj = Actor {
            kind: actor_kind_to_engine(&actor_kind),
            capabilities: derive_capabilities_for(actor_kind, surface),
            lease: None,
        };
        let proposal = ActionProposal {
            kind: action,
            target_id: target_id.to_string(),
            payload_digest: None,
            created_at: UtcTimestamp::now_utc(),
        };
        let decision = self.engine.admit(&proposal, &actor_obj, &facts, &policy);
        let explanation = self.engine.explain(&decision);
        let capability = format!("surface.{surface}");
        Ok(RunnerVerdict {
            decision,
            explanation,
            capability,
        })
    }
}

fn actor_kind_to_engine(k: &DomainActorKind) -> crate::authority_engine::ActorKind {
    use crate::authority_engine::ActorKind as E;
    match k {
        DomainActorKind::Human => E::Human {
            id: "caller".into(),
        },
        DomainActorKind::Agent => E::Agent {
            profile_id: "caller".into(),
            profile_version: 1,
        },
        DomainActorKind::System => E::System {
            service: "caller".into(),
        },
    }
}

fn derive_capabilities_for(_actor: DomainActorKind, surface: &str) -> Vec<Capability> {
    let band = default_band_for_surface(surface);
    let mut caps = vec![format!("surface.{surface}")];
    match band {
        RiskBand::High => {
            caps.push("cycle.lifecycle".to_string());
        }
        RiskBand::Medium => {
            caps.push("plan.write".to_string());
        }
        RiskBand::Low => {
            caps.push("cli.execute".to_string());
        }
    }
    caps
}

fn default_band_for_surface(surface: &str) -> RiskBand {
    match surface {
        "cycle_state" | "gate_receipts" | "plan_revisions" | "transition_records"
        | "framework_bundle" | "github_releases" => RiskBand::High,
        "knowledge_graph_vault" | "plan_item" | "evidence_attachment" | "decision_record" => {
            RiskBand::Medium
        }
        _ => RiskBand::Low,
    }
}

#[cfg(test)]
mod runner_tests {
    use super::*;

    fn runner() -> AuthorityEngineRunner {
        AuthorityEngineRunner::new()
    }

    #[test]
    fn admit_cycle_state_high_band_requires_approval() {
        let v = runner()
            .admit_surface(
                "user:alice",
                "cycle_state",
                ActionKind::CycleTransition,
                "c1",
                Facts::default(),
            )
            .expect("ok");
        assert!(
            matches!(v.decision, AdmissionDecision::RequireApproval { .. }),
            "expected RequireApproval, got {:?}",
            v.decision
        );
        assert_eq!(v.capability, "surface.cycle_state");
        assert!(!v.explanation.decision_digest.0.is_empty());
    }

    #[test]
    fn admit_evidence_attachment_medium_band_allows_human() {
        let v = runner()
            .admit_surface(
                "user:alice",
                "evidence_attachment",
                ActionKind::PlanEvidence,
                "p1",
                Facts::default(),
            )
            .expect("ok");
        // Medium band: capability `plan.write` is in default_low_risk matrix.
        assert!(
            matches!(v.decision, AdmissionDecision::Allow { .. }),
            "got {:?}",
            v.decision
        );
    }

    #[test]
    fn admit_unknown_surface_returns_error() {
        let r = runner().admit_surface(
            "user:alice",
            "does_not_exist",
            ActionKind::CycleStart,
            "x",
            Facts::default(),
        );
        assert!(matches!(
            r,
            Err(AuthorityEngineError::InternalContractBug { .. })
        ));
    }

    #[test]
    fn admit_decision_id_is_deterministic() {
        let r1 = runner()
            .admit_surface(
                "user:alice",
                "cycle_state",
                ActionKind::CycleTransition,
                "c1",
                Facts::default(),
            )
            .expect("ok");
        // decision_id is "approval-human-cycle_transition" per engine impl.
        assert_eq!(r1.decision.decision_id(), "approval-human-cycle_transition");
    }

    #[test]
    fn admit_low_band_system_allows() {
        let v = runner()
            .admit_surface(
                "system",
                "ledger_events",
                ActionKind::CliRun,
                "evt-1",
                Facts::default(),
            )
            .expect("ok");
        // Low band: `cli.execute` in default_low_risk matrix; system actor gets it.
        assert!(matches!(v.decision, AdmissionDecision::Allow { .. }));
    }

    #[test]
    fn admit_github_releases_human_without_cli_execute_denied() {
        // High band + Human actor without `cli.execute` capability → Deny.
        let v = runner()
            .admit_surface(
                "user:bob",
                "github_releases",
                ActionKind::CliRelease,
                "v0",
                Facts::default(),
            )
            .expect("ok");
        assert!(matches!(
            v.decision,
            AdmissionDecision::Deny {
                reason: crate::authority_engine::DenyReason::ActorKindNotPermitted,
                ..
            }
        ));
    }
}
