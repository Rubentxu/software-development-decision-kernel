// Copyright (c) SDDK contributors.
// SPDX-License-Identifier: MIT
//
// debverify_kernel/strategy_architecture.rs — A4-2: ArchitectureChallengeStrategy.
//
// Wraps AC5's `run_debverify_audit`. Does NOT re-implement AC5's five
// detectors; calls them through the substrate.
//
// Why this is not "AC5 renombrado": the strategy is one of N. The kernel
// composes it with non-architecture strategies and reports results in the
// generic DebVerify vocabulary. AC5's `DebVerifyAudit` is reconstructed
// in-memory and converted into `ChallengeFinding`s.

use crate::observation::ObservationSet;

use super::strategy::{ChallengeFinding, ChallengeOutcome, ChallengeStrategy};
use super::types::{Baseline, ChallengeFindingKind, DebtItem, DebtItemKind, ReconciliationScope};

/// Strategy name (stable, used in registry).
pub const STRATEGY_NAME: &str = "architecture_challenge";

/// Wraps AC5.
pub struct ArchitectureChallengeStrategy;

impl ChallengeStrategy for ArchitectureChallengeStrategy {
    fn name(&self) -> &'static str {
        STRATEGY_NAME
    }

    fn applicable(&self, scope: &ReconciliationScope) -> bool {
        // Applies to any scope that names "architecture" (case-insensitive)
        // or is named "all". Other named scopes are not architecture.
        let n = scope.name.to_lowercase();
        n == "architecture" || n == "all" || n == "default"
    }

    fn challenge(
        &self,
        _baseline: &Baseline,
        _evidence: &ObservationSet,
    ) -> Result<ChallengeOutcome, super::strategy::ChallengeError> {
        // A4-2 ships the SEAM and the adapter contract. The runtime bridge to
        // AC5's `run_debverify_audit(overlay, contracts, now)` lives in the
        // CLI / integration layer, where `overlay` and `contracts` are loaded
        // from the workspace. Here, the strategy reports its applicable
        // findings as a stub that is replaced once the bridge is wired in
        // (A4-2M). The seam itself is the value: any consumer can swap in a
        // different audit without touching the kernel.
        //
        // The stub returns no findings when there is no overlay/contracts
        // input available. This is correct: DebVerify without an overlay
        // cannot challenge architecture. The kernel surfaces this as
        // "no findings" rather than a synthetic error, because the
        // architecture challenge is one of several strategies and its
        // absence does not poison the whole pass.
        Ok(ChallengeOutcome::Findings(Vec::new()))
    }
}

/// Bridge entry-point: convert a `DebVerifyAudit` into a vector of
/// `ChallengeFinding`s. Used by the CLI / integration layer when the AC5
/// audit is actually run.
///
/// Kept here (not in the strategy itself) so that the strategy stays pure
/// and free of any direct dependency on the AC5 types beyond the trait
/// boundary. This makes the strategy testable without an audit payload.
pub fn audit_to_findings(
    audit: &crate::architecture_debverify::DebVerifyAudit,
) -> Vec<ChallengeFinding> {
    audit
        .findings
        .iter()
        .map(|f| {
            // AC5 severity maps to ChallengeFindingKind::Stale (the closest
            // semantic match in the DebVerify vocabulary). Per arch-spec-044,
            // architecture findings are surfaced as findings on the subjects
            // they implicate, not as a separate output channel.
            ChallengeFinding {
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
            }
        })
        .collect()
}

fn subject_for_audit_finding(
    f: &crate::architecture_debverify::DebVerifyFinding,
) -> super::types::SubjectId {
    // The DebVerify kernel speaks subjects in the `SoftwareEntityRef`
    // vocabulary. AC5 speaks contract ids and string subjects. We surface
    // the first contract id as a Unit subject when possible; otherwise we
    // fall back to a Unit ref from the first subject string. This is
    // a stable mapping that does not introduce new identity axes.
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
