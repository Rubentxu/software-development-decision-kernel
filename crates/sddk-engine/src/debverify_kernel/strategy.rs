// Copyright (c) SDDK contributors.
// SPDX-License-Identifier: MIT
//
// debverify_kernel/strategy.rs — A4-2: the ChallengeStrategy trait + outcomes.

use crate::observation::ObservationSet;

use super::types::{
    ChallengeFindingKind, ContradictionSet, DebtItem, GapSet, ReconciliationScope, SubjectId,
};

/// What a strategy can return.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum ChallengeOutcome {
    /// Findings (stale subjects, debt, errors).
    Findings(Vec<ChallengeFinding>),
    /// Contradictions between affirming and denying observations.
    Contradictions(Vec<ContradictionSet>),
    /// Evidence gaps.
    Gaps(Vec<GapSet>),
}

/// A challenge finding. Strategies emit these for stale subjects, debt, and
/// unrecoverable strategy errors.
#[derive(Clone, Debug, PartialEq, Eq, serde::Serialize)]
pub struct ChallengeFinding {
    /// Kind of finding.
    pub kind: ChallengeFindingKind,
    /// Subject the finding is about.
    pub subject: SubjectId,
    /// Why this is a finding (informational).
    pub message: String,
    /// Optional debt, if the strategy is surfacing debt.
    pub debt: Option<DebtItem>,
}

/// A strategy that failed to run.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum ChallengeError {
    /// The strategy required an input that was missing.
    MissingInput(String),
}

/// A pluggable challenge strategy.
///
/// Implementations are stateless and pure. Adding a new strategy is a
/// registration-only operation: implement this trait and add it to
/// `default_strategy_set()`. The kernel does not need to change.
pub trait ChallengeStrategy: Send + Sync {
    /// Stable name for receipts and registry lookup.
    fn name(&self) -> &'static str;
    /// Is this strategy applicable to the given scope?
    fn applicable(&self, scope: &ReconciliationScope) -> bool;
    /// Run the strategy against the baseline + evidence.
    fn challenge(
        &self,
        baseline: &super::types::Baseline,
        evidence: &ObservationSet,
    ) -> Result<ChallengeOutcome, ChallengeError>;
}
