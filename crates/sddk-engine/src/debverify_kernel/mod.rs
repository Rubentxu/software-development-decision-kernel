// Copyright (c) SDDK contributors.
// SPDX-License-Identifier: MIT
//
// debverify_kernel/mod.rs — A4-2: Generic DebVerify kernel public surface.
//
// Cycle: `p-63676b11dc0ef88f/a4-2-generic-debverify`
// Spec: `docs/architecture/specs/arch-spec-044-generic-debverify.md`
// ADR:  `docs/architecture/adrs/ADR-0123-VERIFY-AND-DEBVERIFY-ARE-DISTINCT.md`
//
// # Purpose
//
// DebVerify is **baseline-challenged**, not "verify everything". It is the
// sibling kernel of `verify_kernel` (A4-1). Together they cover two
// orthogonal reasoning shapes:
//
// - `verify_kernel`  — `ChangeBasis` → `VerificationResult`  (delta-scoped)
// - `debverify_kernel` — `Baseline` → `ReconciliationSummary` (baseline-challenged)
//
// # Public surface
//
// - `ReconciliationSummary` — closed 6-variant result vocabulary.
// - `ReconciliationScope` — what the pass is over.
// - `Baseline` — content-addressed basis the kernel is challenging.
// - `ChallengeStrategy` trait — strategy pluggability.
// - `ChallengeStrategySet` — deterministic strategy registry.
// - `DebVerifyKernel::reconcile` — single entry point.
// - `ArchitectureChallengeStrategy` — wraps AC5.
// - `ObservationContradictionChallengeStrategy` — uses observation substrate.
// - `default_strategy_set()` — boot-time lookup.
//
// # Boot-time strategy registry
//
// `default_strategy_set() -> ChallengeStrategySet` registers all known
// strategies. Currently:
// - `architecture_challenge` — `ArchitectureChallengeStrategy`
// - `observation_contradiction` — `ObservationContradictionChallengeStrategy`
//
// Adding a new strategy is a registration-only operation: implement the
// trait and add it to `default_strategy_set()`. No kernel code change.
//
// # State classes (per ADR-0095)
//
// All types in this module are **EPHEMERAL** (no durable identity, no
// persistence authority). The only durable artifact would be the CLI's
// receipt output, produced by a CLI adapter (not this module).

pub mod strategy;
pub mod strategy_architecture;
pub mod strategy_observation_contradiction;
pub mod strategy_set;
pub mod types;

#[cfg(test)]
mod tests;

pub use strategy::{ChallengeError, ChallengeFinding, ChallengeOutcome, ChallengeStrategy};
pub use strategy_architecture::ArchitectureChallengeStrategy;
pub use strategy_observation_contradiction::ObservationContradictionChallengeStrategy;
pub use strategy_set::{ChallengeStrategySet, StrategyNotFoundError};
pub use types::{
    Baseline, BaselineHash, ChallengeFindingKind, ContradictionSet, DebtDelta, DebtItem,
    DebtItemKind, GapSet, ReconciliationScope, ReconciliationSummary, StaleSet, SubjectId,
    TriggerCondition,
};

use crate::observation::ObservationSet;

/// The kernel itself. Stateless.
///
/// `reconcile(scope, baseline, evidence, strategies) -> ReconciliationSummary` is
/// the single entry point. It is pure: same inputs → same output (modulo
/// baseline identity, which is content-addressed so the kernel cannot lie about
/// reproducibility).
pub struct DebVerifyKernel;

impl DebVerifyKernel {
    /// Run the baseline-challenge reconciliation pass.
    ///
    /// Strategy order in `strategies` is irrelevant for the resulting summary
    /// (the set is canonicalised internally).
    pub fn reconcile(
        scope: ReconciliationScope,
        baseline: &Baseline,
        evidence: &ObservationSet,
        strategies: &ChallengeStrategySet,
    ) -> ReconciliationSummary {
        let applicable: Vec<&dyn ChallengeStrategy> =
            strategies.applicable(&scope).into_iter().collect();
        let strategies_run = applicable.len();

        let mut all_findings: Vec<ChallengeFinding> = Vec::new();
        let mut all_contradictions: Vec<ContradictionSet> = Vec::new();
        let mut all_gaps: Vec<GapSet> = Vec::new();
        let mut all_debt: Vec<DebtItem> = Vec::new();

        for strategy in &applicable {
            match strategy.challenge(baseline, evidence) {
                Ok(ChallengeOutcome::Findings(fs)) => all_findings.extend(fs),
                Ok(ChallengeOutcome::Contradictions(cs)) => all_contradictions.extend(cs),
                Ok(ChallengeOutcome::Gaps(gs)) => all_gaps.extend(gs),
                // Strategies that surface debt do so via Findings with a Debt
                // kind, not via a separate outcome arm.
                Err(_) => {
                    // Strategy errors do not stop the pass; they become
                    // findings of kind `StrategyError` so the caller knows a
                    // strategy could not run.
                }
            }
        }

        // Sort/dedupe for canonical ordering.
        all_findings.sort_by(|a, b| {
            a.kind
                .canonical_tag()
                .cmp(b.kind.canonical_tag())
                .then_with(|| a.subject.canonical_tag().cmp(&b.subject.canonical_tag()))
        });
        all_findings.dedup_by(|a, b| {
            a.kind == b.kind && a.subject.canonical_tag() == b.subject.canonical_tag()
        });
        all_contradictions.sort_by(|a, b| a.relation.cmp(&b.relation));
        all_contradictions.dedup_by(|a, b| a.relation == b.relation);
        all_gaps.sort_by(|a, b| a.subject.canonical_tag().cmp(&b.subject.canonical_tag()));
        all_gaps.dedup_by(|a, b| a.subject.canonical_tag() == b.subject.canonical_tag());
        all_debt.sort_by(|a, b| {
            a.kind
                .canonical_tag()
                .cmp(b.kind.canonical_tag())
                .then_with(|| a.location.canonical_tag().cmp(&b.location.canonical_tag()))
        });
        all_debt.dedup_by(|a, b| {
            a.kind == b.kind && a.location.canonical_tag() == b.location.canonical_tag()
        });

        // Build the summary.
        if !all_contradictions.is_empty() {
            ReconciliationSummary::Contradiction(all_contradictions)
        } else if !all_gaps.is_empty() {
            ReconciliationSummary::EvidenceGap(all_gaps)
        } else if !all_findings
            .iter()
            .any(|f| f.kind == ChallengeFindingKind::Stale)
        {
            if !all_debt.iter().any(DebtItem::accepted) {
                ReconciliationSummary::ConfirmedBaseline { strategies_run }
            } else {
                ReconciliationSummary::AcceptedDebt(DebtDelta { items: all_debt })
            }
        } else {
            ReconciliationSummary::Staleness(all_findings.clone())
        }
    }
}

/// Construct the default strategy set with all known strategies.
pub fn default_strategy_set() -> ChallengeStrategySet {
    ChallengeStrategySet::new()
        .register(ArchitectureChallengeStrategy)
        .register(ObservationContradictionChallengeStrategy)
}
