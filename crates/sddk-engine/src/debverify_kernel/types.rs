// Copyright (c) SDDK contributors.
// SPDX-License-Identifier: MIT
//
// debverify_kernel/types.rs — A4-2: closed ADTs for the DebVerify kernel.
//
// All types are EPHEMERAL (ADR-0095). No persistence authority.
// See `mod.rs` for the module-level contract.

use sha2::{Digest, Sha256};

/// A subject that a strategy can find something about.
///
/// Reuses the substrate `SoftwareEntityRef` vocabulary rather than inventing
/// parallel subject ids.
pub type SubjectId = crate::observation::SoftwareEntityRef;

/// A scope declares what the reconciliation pass is over.
///
/// `name` is a short user-facing label (e.g. "architecture", "all").
/// `subjects` constrains which subjects are in scope; empty means "all known".
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ReconciliationScope {
    /// Short label for the scope.
    pub name: String,
    /// Subjects in scope. Empty means "all known".
    pub subjects: Vec<SubjectId>,
}

impl ReconciliationScope {
    /// A scope named `name` over all known subjects.
    pub fn all(name: impl Into<String>) -> Self {
        Self {
            name: name.into(),
            subjects: Vec::new(),
        }
    }
    /// A scope over the given subjects only.
    pub fn over(name: impl Into<String>, subjects: Vec<SubjectId>) -> Self {
        Self {
            name: name.into(),
            subjects,
        }
    }
}

/// A baseline-challenge basis. NOT a change basis.
///
/// Identity is derived from `(scope_name, evidence_sha256)`. Wall clock,
/// rendering, and producer label are explicitly excluded.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Baseline {
    /// Scope name the baseline is over.
    pub scope_name: String,
    /// Hash of the evidence set the baseline was taken against.
    pub evidence_sha256: String,
}

impl Baseline {
    /// Construct a baseline.
    pub fn new(scope_name: impl Into<String>, evidence_sha256: impl Into<String>) -> Self {
        Self {
            scope_name: scope_name.into(),
            evidence_sha256: evidence_sha256.into(),
        }
    }
    /// Derive a baseline from a scope name + an observation set.
    pub fn from_observations(
        scope_name: impl Into<String>,
        evidence: &crate::observation::ObservationSet,
    ) -> Self {
        let mut h = Sha256::new();
        h.update(b"sddk.baseline.v1|");
        for o in evidence.observations() {
            h.update(o.id.as_str().as_bytes());
            h.update(b"|");
        }
        Self {
            scope_name: scope_name.into(),
            evidence_sha256: format!("{:064x}", h.finalize()),
        }
    }
    /// The baseline's deterministic identity.
    pub fn hash(&self) -> BaselineHash {
        let mut h = Sha256::new();
        h.update(b"sddk.baseline.id.v1|");
        h.update(self.scope_name.as_bytes());
        h.update(b"|");
        h.update(self.evidence_sha256.as_bytes());
        BaselineHash(format!("{:064x}", h.finalize()))
    }
}

/// Deterministic identity of a [`Baseline`].
#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct BaselineHash(pub String);

impl BaselineHash {
    /// Borrow the raw hex.
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

/// A trigger condition under which an accepted debt should be revisited.
///
/// `trigger` is a stable tag (e.g. "next_release", "when_X_happens"). It is
/// not evaluated by the kernel; it is metadata for the caller.
#[derive(Clone, Debug, PartialEq, Eq, serde::Serialize)]
pub struct TriggerCondition {
    /// Stable tag.
    pub trigger: String,
}

/// A debt item surfaced by a strategy.
///
/// Acceptance is **explicit**: an item is `accepted()` only when both
/// `decision_ref` and `revisit_trigger` are present.
#[derive(Clone, Debug, PartialEq, Eq, serde::Serialize)]
pub struct DebtItem {
    /// Debt class.
    pub kind: DebtItemKind,
    /// Where in the architecture this lives.
    pub location: SubjectId,
    /// Why this is debt (informational; never identity-relevant).
    pub description: String,
    /// Decision reference if and only if the debt has been accepted.
    pub decision_ref: Option<String>,
    /// Required for accepted debt. Specifies when the caller should revisit.
    pub revisit_trigger: Option<TriggerCondition>,
}

impl DebtItem {
    /// Is this debt accepted (decision-ref + revisit-trigger present)?
    pub fn accepted(&self) -> bool {
        self.decision_ref.is_some() && self.revisit_trigger.is_some()
    }
}

/// The closed vocabulary of debt classes. Aligned with the existing project
/// debt taxonomy (knowledge / implementation / architecture-design) but
/// scoped to DebVerify surface.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, PartialOrd, Ord, serde::Serialize)]
pub enum DebtItemKind {
    /// Implementation-level debt.
    Implementation,
    /// Architecture or design debt.
    ArchitectureDesign,
    /// Knowledge debt (a knowledge assertion has grown stale).
    Knowledge,
}

impl DebtItemKind {
    /// Canonical tag for receipts and identity.
    pub fn canonical_tag(self) -> &'static str {
        match self {
            Self::Implementation => "implementation",
            Self::ArchitectureDesign => "architecture_design",
            Self::Knowledge => "knowledge",
        }
    }
}

/// A delta of debt, all the items in one summary.
#[derive(Clone, Debug, PartialEq, Eq, serde::Serialize)]
pub struct DebtDelta {
    /// Debt items, in canonical order.
    pub items: Vec<DebtItem>,
}

/// The closed vocabulary of challenge-finding kinds. Aligned with A4-0's
/// `EvidenceResolution` and AC5's `DebVerifyFindingKind` where the semantics
/// genuinely match; new where DebVerify reports something the other layers
/// cannot.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, PartialOrd, Ord, serde::Serialize)]
pub enum ChallengeFindingKind {
    /// A subject has grown stale relative to the baseline.
    Stale,
    /// A strategy could not run (e.g., missing input).
    StrategyError,
}

impl ChallengeFindingKind {
    /// Canonical tag for receipts and identity.
    pub fn canonical_tag(self) -> &'static str {
        match self {
            Self::Stale => "stale",
            Self::StrategyError => "strategy_error",
        }
    }
}

/// A set of contradictions, each keyed by the relation that contradicts.
#[derive(Clone, Debug, PartialEq, Eq, serde::Serialize)]
pub struct ContradictionSet {
    /// The relation that has both affirming and denying observations.
    pub relation: crate::observation::RelationId,
    /// The affirming observation ids.
    pub supporting: Vec<crate::observation::ObservationId>,
    /// The denying observation ids.
    pub contradicting: Vec<crate::observation::ObservationId>,
}

/// A set of evidence gaps, each keyed by a subject that lacked evidence.
#[derive(Clone, Debug, PartialEq, Eq, serde::Serialize)]
pub struct GapSet {
    /// The subject that lacked evidence.
    pub subject: SubjectId,
    /// What was missing.
    pub gap: String,
}

/// A set of staleness findings.
#[derive(Clone, Debug, PartialEq, Eq, serde::Serialize)]
pub struct StaleSet {
    /// Findings, in canonical order.
    pub findings: Vec<super::strategy::ChallengeFinding>,
}

/// The closed 6-variant result of a reconciliation pass.
///
/// No `bool` + `Option<String>` and no numeric score. Distinct from
/// `verify_kernel::VerificationResult` — there is no conversion between them.
#[derive(Clone, Debug, PartialEq, Eq, serde::Serialize)]
pub enum ReconciliationSummary {
    /// Every applicable strategy found nothing to challenge.
    ConfirmedBaseline {
        /// How many strategies ran.
        strategies_run: usize,
    },
    /// At least one contradiction reconciled.
    Contradiction(Vec<ContradictionSet>),
    /// At least one stale subject was found.
    Staleness(Vec<super::strategy::ChallengeFinding>),
    /// Required evidence was missing for some subject.
    EvidenceGap(Vec<GapSet>),
    /// Known debt present and accepted by a decision.
    AcceptedDebt(DebtDelta),
    /// Scope declared but no strategies applicable.
    NotApplicable,
}

impl ReconciliationSummary {
    /// Is the baseline clean? True only for `ConfirmedBaseline` with no debt.
    pub fn is_clean(&self) -> bool {
        matches!(self, Self::ConfirmedBaseline { .. })
    }
}
