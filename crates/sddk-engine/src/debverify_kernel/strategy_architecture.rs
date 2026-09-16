// Copyright (c) SDDK contributors.
// SPDX-License-Identifier: MIT
//
// debverify_kernel/strategy_architecture.rs — A4-2 / A4-2M:
// ArchitectureChallengeStrategy.
//
// Wraps AC5's `run_debverify_audit`. Does NOT re-implement AC5's five
// detectors; calls them through the substrate (`audit_to_findings`).
//
// Why this is not "AC5 renombrado": the strategy is one of N. The kernel
// composes it with non-architecture strategies and reports results in the
// generic DebVerify vocabulary. AC5's `DebVerifyAudit` is reconstructed
// in-memory and converted into `ChallengeFinding`s.
//
// A4-2M semantics:
// - The audit executes **once**, at the call site (CLI / integration
//   layer), via `run_debverify_audit`. The strategy is a **port**, not
//   the executor.
// - `challenge()` returns `Gaps(...)` when the audit could not run
//   (no inputs available). It NEVER returns `Findings(vec![])`, because
//   that would be a false-clean trap — the kernel would promote it to
//   `ConfirmedBaseline` and the caller would conclude "all good" when
//   in fact nothing was checked.
// - The actual audit → finding conversion is `audit_to_findings`,
//   called by the CLI when the overlay + contracts are loaded. That
//   function is the **single execution** of the five detectors; the
//   strategy itself never calls them.

use crate::observation::ObservationSet;

use super::strategy::{ChallengeError, ChallengeOutcome, ChallengeStrategy};
use super::types::{Baseline, GapSet, ReconciliationScope, SubjectId};

/// Strategy name (stable, used in registry).
pub const STRATEGY_NAME: &str = "architecture_challenge";

/// Architecture challenge port.
///
/// The strategy itself does NOT execute the AC5 audit. It declares
/// that an architecture audit was required (the strategy is registered
/// for the "architecture" scope) and the call site is responsible for
/// running the audit and feeding the findings back through
/// `audit_to_findings`.
///
/// When the audit cannot run (no overlay, no contracts), the
/// strategy surfaces that absence as an explicit `GapSet`, NOT as
/// zero findings. This prevents the kernel from reporting
/// `ConfirmedBaseline` when nothing was actually checked.
pub struct ArchitectureChallengeStrategy;

impl ChallengeStrategy for ArchitectureChallengeStrategy {
    fn name(&self) -> &'static str {
        STRATEGY_NAME
    }

    fn applicable(&self, scope: &ReconciliationScope) -> bool {
        // Applies to any scope that names "architecture" (case-insensitive)
        // or is named "all" / "default". Other named scopes are not
        // architecture.
        let n = scope.name.to_lowercase();
        n == "architecture" || n == "all" || n == "default"
    }

    fn challenge(
        &self,
        _baseline: &Baseline,
        _evidence: &ObservationSet,
    ) -> Result<ChallengeOutcome, ChallengeError> {
        // The strategy cannot execute the audit by itself — it does not
        // have access to the overlay or contracts (those live in the CLI /
        // integration layer). The CLI is the call site that runs
        // `run_debverify_audit` and converts via `audit_to_findings`.
        //
        // When called directly without those inputs (e.g. from a unit
        // test that does not load a declaration), the strategy emits
        // a `GapSet` so the kernel surfaces `EvidenceGap` instead of
        // `ConfirmedBaseline`. This is the A4-2M false-clean guard.
        let scope_subject: SubjectId = crate::observation::SoftwareEntityRef::Component(
            crate::architectural_contract::ComponentRef::new("architecture".to_string())
                .unwrap_or_else(|_| {
                    crate::architectural_contract::ComponentRef::new("<invalid>".to_string())
                        .expect("ComponentRef::new always succeeds for ASCII")
                }),
        );
        Ok(ChallengeOutcome::Gaps(vec![GapSet {
            subject: scope_subject,
            gap: "architecture_challenge: audit inputs not provided; \
                  run_debverify_audit must be invoked at the call site \
                  and findings converted via audit_to_findings"
                .to_string(),
        }]))
    }
}

/// Bridge entry-point: convert a `DebVerifyAudit` into a vector of
/// `ChallengeFinding`s. This is the **single execution** of the five
/// detectors and the only place the AC5 types cross into the kernel
/// vocabulary.
///
/// Called by the CLI / integration layer after running
/// `architecture_debverify::run_debverify_audit(overlay, contracts, now)`.
pub fn audit_to_findings(
    audit: &crate::architecture_debverify::DebVerifyAudit,
) -> Vec<super::strategy::ChallengeFinding> {
    use super::strategy::ChallengeFinding;
    use super::types::{ChallengeFindingKind, DebtItem, DebtItemKind};
    audit
        .findings
        .iter()
        .map(|f| ChallengeFinding {
            kind: ChallengeFindingKind::Stale,
            subject: subject_for_audit_finding(f),
            message: format!("{}: {}", f.kind.canonical_tag(), f.message),
            debt: Some(DebtItem {
                kind: DebtItemKind::ArchitectureDesign,
                location: subject_for_audit_finding(f),
                description: f.message.clone(),
                decision_ref: None,
                revisit_trigger: None,
            }),
        })
        .collect()
}

fn subject_for_audit_finding(
    f: &crate::architecture_debverify::DebVerifyFinding,
) -> super::types::SubjectId {
    use crate::observation::SoftwareEntityRef;
    if let Some(first_contract) = f.contract_ids.first() {
        SoftwareEntityRef::Unit(crate::architecture_graph::SoftwareUnitRef::new(
            first_contract.as_str(),
        ))
    } else if let Some(first_subject) = f.subjects.first() {
        SoftwareEntityRef::Unit(crate::architecture_graph::SoftwareUnitRef::new(
            first_subject.clone(),
        ))
    } else {
        SoftwareEntityRef::Unit(crate::architecture_graph::SoftwareUnitRef::new(
            "<unknown>".to_string(),
        ))
    }
}
