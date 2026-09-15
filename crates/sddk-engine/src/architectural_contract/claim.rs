// Copyright (c) SDDK contributors.
// SPDX-License-Identifier: MIT
//
// architectural_contract/claim.rs — `EvidenceRef`, `EvaluatorRef`,
// `ClaimOutcome`, `ArchitectureClaim` (PROJECTION), and `ContractEvaluation`
// (EPHEMERAL canonical entry point).
//
// # State classes
//
//   - `EvidenceRef`, `EvaluatorRef`, `ClaimOutcome` — variant enums / typed newtypes
//   - `ArchitectureClaim` — PROJECTION (rebuildable from contract + evidence + now)
//   - `ContractEvaluation::evaluate` — EPHEMERAL (pure transform)

use serde::{Deserialize, Serialize};
use std::fmt;

use crate::knowledge::{EventTime, MissingEvidence};

use super::error::ContractError;
use super::payload::ContractPayload;
use super::types::ContractId;
use crate::architectural_contract::ArchitecturalContract;

// ─────────────────────────────────────────────────────────────────────────────
// EvidenceRef
// ─────────────────────────────────────────────────────────────────────────────

/// A reference to one piece of evidence used to evaluate a contract.
///
/// Typed as a `(provider, ref)` tuple. The provider tag scopes the
/// reference kind (e.g. `"runtime"` for an observed trace, `"static"` for
/// a CogniCode call-graph node, `"manual"` for a human-provided record).
#[derive(Clone, Debug, PartialEq, Eq, Hash, PartialOrd, Ord, Serialize, Deserialize)]
pub struct EvidenceRef {
    /// Provider tag (e.g. `"runtime"`, `"static"`, `"manual"`).
    provider: String,
    /// Opaque reference within the provider's namespace.
    reference: String,
}

impl EvidenceRef {
    /// Construct a new `EvidenceRef`. Empty fields are rejected.
    pub fn new(
        provider: impl Into<String>,
        reference: impl Into<String>,
    ) -> Result<Self, ContractError> {
        let p: String = provider.into();
        let r: String = reference.into();
        if p.trim().is_empty() {
            return Err(ContractError::InvalidRef {
                kind: "EvidenceRef.provider".into(),
                reason: "empty or whitespace".into(),
            });
        }
        if r.trim().is_empty() {
            return Err(ContractError::InvalidRef {
                kind: "EvidenceRef.reference".into(),
                reason: "empty or whitespace".into(),
            });
        }
        Ok(Self {
            provider: p,
            reference: r,
        })
    }

    /// Borrow the provider tag.
    pub fn provider(&self) -> &str {
        &self.provider
    }

    /// Borrow the reference.
    pub fn reference(&self) -> &str {
        &self.reference
    }
}

// ─────────────────────────────────────────────────────────────────────────────
// EvaluatorRef
// ─────────────────────────────────────────────────────────────────────────────

/// Identifier of the entity that produced an [`ArchitectureClaim`].
///
/// Typed so we can later introduce engine / static / runtime / human
/// authority splits without leaking string bags.
#[derive(Clone, Debug, PartialEq, Eq, Hash, PartialOrd, Ord, Serialize, Deserialize)]
pub struct EvaluatorRef(String);

impl EvaluatorRef {
    /// Construct a new `EvaluatorRef`.
    pub fn new(raw: impl Into<String>) -> Result<Self, ContractError> {
        let s: String = raw.into();
        if s.trim().is_empty() {
            return Err(ContractError::InvalidRef {
                kind: "EvaluatorRef".into(),
                reason: "empty or whitespace".into(),
            });
        }
        Ok(Self(s))
    }

    /// Borrow the inner string.
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl fmt::Display for EvaluatorRef {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.0)
    }
}

impl std::str::FromStr for EvaluatorRef {
    type Err = ContractError;
    fn from_str(s: &str) -> Result<Self, Self::Err> {
        Self::new(s.to_string())
    }
}

// ─────────────────────────────────────────────────────────────────────────────
// ClaimOutcome
// ─────────────────────────────────────────────────────────────────────────────

/// Closed taxonomy of claim outcomes (REQ-A3S2-011).
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, PartialOrd, Ord, Serialize, Deserialize)]
pub enum ClaimOutcome {
    /// All required evidence was provided and the contract holds.
    Verified,
    /// Evidence contradicts the contract.
    Contradicted,
    /// Evidence is missing or insufficient.
    Unknown,
    /// The contract is past its applicability window.
    Stale,
}

impl ClaimOutcome {
    /// Canonical short tag for receipts.
    #[allow(dead_code)] // reserved for receipt emission (A4 cycle)
    pub(crate) fn canonical_tag(self) -> &'static str {
        match self {
            ClaimOutcome::Verified => "verified",
            ClaimOutcome::Contradicted => "contradicted",
            ClaimOutcome::Unknown => "unknown",
            ClaimOutcome::Stale => "stale",
        }
    }
}

// ─────────────────────────────────────────────────────────────────────────────
// ArchitectureClaim (PROJECTION)
// ─────────────────────────────────────────────────────────────────────────────

/// The result of evaluating an [`ArchitecturalContract`] against a set of
/// [`EvidenceRef`]s at a given point in time.
///
/// Construction is internal to [`ContractEvaluation::evaluate`]; the field
/// shape is public so downstream consumers can read it, but the
/// constructor is private so callers cannot forge a `Verified` outcome
/// (REQ-A3S2-003, REQ-A3S2-012, REQ-A3S2-015).
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct ArchitectureClaim {
    contract_id: ContractId,
    outcome: ClaimOutcome,
    evidence_refs: Vec<EvidenceRef>,
    evaluated_at: EventTime,
    evaluator: EvaluatorRef,
    missing_evidence: Vec<MissingEvidence>,
    /// Rationale (informational only; NEVER runtime authority).
    note: Option<String>,
}

impl ArchitectureClaim {
    /// The contract this claim evaluates.
    pub fn contract_id(&self) -> &ContractId {
        &self.contract_id
    }

    /// The claim outcome.
    pub fn outcome(&self) -> ClaimOutcome {
        self.outcome
    }

    /// The evidence references used (may be empty for `Unknown`).
    pub fn evidence_refs(&self) -> &[EvidenceRef] {
        &self.evidence_refs
    }

    /// When the claim was produced.
    pub fn evaluated_at(&self) -> EventTime {
        self.evaluated_at
    }

    /// The evaluator that produced the claim.
    pub fn evaluator(&self) -> &EvaluatorRef {
        &self.evaluator
    }

    /// Why evidence was missing (populated for `Unknown` outcomes).
    pub fn missing_evidence(&self) -> &[MissingEvidence] {
        &self.missing_evidence
    }

    /// Optional rationale (informational only).
    pub fn note(&self) -> Option<&str> {
        self.note.as_deref()
    }
}

// ─────────────────────────────────────────────────────────────────────────────
// ContractEvaluation (EPHEMERAL canonical entry)
// ─────────────────────────────────────────────────────────────────────────────

/// Canonical entry point for evaluating [`ArchitecturalContract`]s.
///
/// Pure: no IO, no clock reads (the `now` argument is data). See
/// [`ContractEvaluation::evaluate`] (REQ-A3S2-003).
pub struct ContractEvaluation;

impl ContractEvaluation {
    /// Evaluate a contract against the provided evidence at `now`.
    ///
    /// Rules (in evaluation order, REQ-A3S2-011..014):
    ///
    /// 1. If `evidence_refs` is empty ⇒ `Unknown` with `NotProvided`.
    /// 2. If `now < declared_at` ⇒ `Stale`.
    /// 3. If the contract is `BoundedCompatibility`, `now >= deprecated_after`,
    ///    and there is no `replaced_by` ⇒ `Stale`.
    /// 4. Otherwise ⇒ `Verified` (the presence of any `EvidenceRef` is
    ///    treated as sufficient — the policy of what counts as a
    ///    contradiction is delegated to A4 Verify, which is out of scope
    ///    here; this module MUST NOT mark `Contradicted`, only A4 may).
    pub fn evaluate(
        contract: &ArchitecturalContract,
        evidence_refs: Vec<EvidenceRef>,
        now: EventTime,
        evaluator: EvaluatorRef,
        note: Option<String>,
    ) -> ArchitectureClaim {
        // Rule 1: no evidence ⇒ Unknown.
        if evidence_refs.is_empty() {
            return ArchitectureClaim {
                contract_id: contract.id().clone(),
                outcome: ClaimOutcome::Unknown,
                evidence_refs,
                evaluated_at: now,
                evaluator,
                missing_evidence: vec![MissingEvidence::NotProvided],
                note,
            };
        }

        // Rule 2: time travel ⇒ Stale.
        if now.0 < contract.declared_at().0 {
            return ArchitectureClaim {
                contract_id: contract.id().clone(),
                outcome: ClaimOutcome::Stale,
                evidence_refs,
                evaluated_at: now,
                evaluator,
                missing_evidence: Vec::new(),
                note,
            };
        }

        // Rule 3: bounded compatibility window elapsed without replacement.
        if let ContractPayload::BoundedCompatibility {
            deprecated_after,
            replaced_by,
        } = contract.payload()
            && now.0 >= deprecated_after.0
            && replaced_by.is_none()
        {
            return ArchitectureClaim {
                contract_id: contract.id().clone(),
                outcome: ClaimOutcome::Stale,
                evidence_refs,
                evaluated_at: now,
                evaluator,
                missing_evidence: Vec::new(),
                note,
            };
        }

        // Rule 4: default. A4 Verify is responsible for `Contradicted`.
        ArchitectureClaim {
            contract_id: contract.id().clone(),
            outcome: ClaimOutcome::Verified,
            evidence_refs,
            evaluated_at: now,
            evaluator,
            missing_evidence: Vec::new(),
            note,
        }
    }
}

/// Test-only helpers exposed via `claim::test_helpers::*`. Sibling modules
/// in the engine (e.g. `architecture_graph`) use these to construct claims
/// with arbitrary outcomes for fixture-level testing — the substrate's
/// `evaluate()` only reaches Verified/Unknown/Stale outcomes.
#[cfg(test)]
pub(crate) mod test_helpers {
    use super::*;

    /// Build an `ArchitectureClaim` with a specified outcome. Used by
    /// architecture_graph tests to exercise Contradicted and Stale relation
    /// emission paths that `evaluate()` cannot reach on its own.
    pub fn claim_with_outcome(
        contract_id: crate::architectural_contract::ContractId,
        outcome: ClaimOutcome,
        provider: &str,
    ) -> ArchitectureClaim {
        ArchitectureClaim {
            contract_id,
            outcome,
            evidence_refs: vec![
                EvidenceRef::new(provider, "ref:test").expect("valid evidence ref"),
            ],
            evaluated_at: EventTime(1_700_000_002),
            evaluator: EvaluatorRef::new("test:helper").unwrap(),
            missing_evidence: Vec::new(),
            note: Some("test-only claim constructed via test_helpers".to_string()),
        }
    }
}
