// Copyright (c) SDDK contributors.
// SPDX-License-Identifier: MIT
//
// intelligence_advisory — A4-5b: AdvisoryContext + WHY integration.
//
// This module is the seam that turns an already-computed A4-5a
// `IntelligenceLoopResult` into:
//
//   1. the existing A3 `AdvisoryContext` (sddk-domain) — the delivered
//      advisory payload; and
//   2. a typed, deterministic, provenance-backed `AdvisoryWhy` — the
//      answer to "why is this advisory item here?".
//
// It is NOT a new intelligence engine, NOT an orchestrator, and it does
// NOT open Governance. It receives the authority outputs that A4-5a
// composed; it never runs `VerifyKernel`, `DebVerifyKernel`,
// `AlignmentLensKernel`, `reduce_alignment`, the legacy `paradigm_lens`
// evaluator, or any provider.
//
// ## Semantics
//
// - `AdvisoryContext` stays advisory. Constructing it can never grant or
//   remove a permission, change a capability, produce an
//   `AuthorityDecision`, compile instructions, or mutate Governance.
// - `AlignmentState::Misaligned` yields advisory content and an
//   explanation; it NEVER yields `DENY` (directly or indirectly).
// - WHY explains an existing conclusion. It never creates a stronger one.
//   `ArchitectureWhy` siblings this: absence of an edge is reported, never
//   converted into a negative claim (`no edge != evidence of negation`).
//
// ## Two provenance axes (A4-S15R)
//
// ```text
// contract --SpecifiedBy--> spec        (declared intent)
// contract --VerifiedBy-->  evidence     (verification evidence)
// ```
//
// `SpecifiedBy` does not imply `VerifiedBy`. An empty `verified_by` means
// "no evidence shipped on that axis", not "not verified".

use sddk_domain::{AdvisoryContext, AdvisoryItem, AdvisoryKind, AdvisoryProvenance};

use crate::alignment_lens::not_evaluated::{NotEvaluated, NotEvaluatedReason};
use crate::alignment_lens::{LensContribution, LensContributionId};
use crate::architecture_graph::overlay::{ArchitectureGraphOverlay, ContractProvenance};
use crate::debverify_kernel::types::ReconciliationSummary;
use crate::intelligence_loop::{IntelligenceLoopReceiptId, IntelligenceLoopResult};
use crate::intent_universal_concern::types::UniversalConcern;
use crate::software_alignment::types::{
    AlignmentAssessment, AlignmentAssessmentId, AlignmentState,
};
use crate::verify_kernel::types::{VerificationClaim, VerificationResult};

// ─────────────────────────────────────────────────────────────────────────────
// Subject reference (EPHEMERAL selector)
// ─────────────────────────────────────────────────────────────────────────────

/// A typed selector for one advisory subject within an
/// [`IntelligenceLoopResult`]. EPHEMERAL: computed on demand, no identity,
/// no persistence.
///
/// A subject is rendered into `AdvisoryItem.subject` through the **single**
/// [`subject_key`] function. That key is never parsed back into a subject —
/// the producer and any lookup share the same typed value.
#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub enum AdvisorySubjectRef {
    /// The DebVerify reconciliation posture.
    Reconciliation,
    /// One `(VerificationClaim, VerificationResult)` pair, keyed by the
    /// contract the claim observes.
    VerificationClaim {
        /// The contract the claim is about.
        contract_id: String,
    },
    /// One lens contribution.
    LensContribution {
        /// Content-addressed contribution id.
        id: LensContributionId,
    },
    /// One lens coverage gap.
    LensGap {
        /// The concern that was not evaluated.
        concern: UniversalConcern,
        /// The typed reason (`NoRegisteredLens` / `LensExistsButRefused`).
        reason: NotEvaluatedReason,
    },
    /// The alignment assessment.
    Alignment {
        /// Content-addressed assessment id.
        id: AlignmentAssessmentId,
    },
}

impl AdvisorySubjectRef {
    /// The stable canonical key for this subject. This is the ONLY place a
    /// subject becomes a string; the result is used for ordering, for
    /// `AdvisoryItem.subject`, and for dedup — never parsed back.
    pub fn key(&self) -> String {
        subject_key(self)
    }
}

/// Canonical key for a subject reference. Deterministic; built from typed
/// fields only (no `Debug`, no `Display`, no locator parsing).
pub fn subject_key(s: &AdvisorySubjectRef) -> String {
    match s {
        AdvisorySubjectRef::Reconciliation => "reconciliation".to_string(),
        AdvisorySubjectRef::VerificationClaim { contract_id } => {
            format!("verification:{contract_id}")
        }
        AdvisorySubjectRef::LensContribution { id } => {
            format!("lens_contribution:{}", id.as_str())
        }
        AdvisorySubjectRef::LensGap { concern, reason } => {
            format!(
                "lens_gap:{}:{}",
                concern.canonical_tag(),
                reason.canonical_tag()
            )
        }
        AdvisorySubjectRef::Alignment { id } => format!("alignment:{}", id.as_str()),
    }
}

// ─────────────────────────────────────────────────────────────────────────────
// WHY model (EPHEMERAL)
// ─────────────────────────────────────────────────────────────────────────────

/// The substrate an [`AdvisoryWhy`] was computed against.
///
/// Clock-stable: it carries the composition identity and the two authority
/// anchors, but NOT `evaluation_time` (a hidden clock must never enter a
/// derived identity).
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct AdvisoryWhyBasis {
    /// The A4-5a composition identity.
    pub receipt_id: IntelligenceLoopReceiptId,
    /// Knowledge-basis anchor (hex).
    pub knowledge_basis_hash_hex: String,
    /// Observation-set anchor (canonical digest).
    pub observation_set_canonical_digest: String,
}

/// One typed leg of the provenance chain. Carries the exact typed value
/// from the loop — never a re-encoded or flattened form.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum AdvisoryWhyLeg {
    /// The verification pair, with its typed result.
    Verification {
        /// The claim.
        claim: VerificationClaim,
        /// The result for that claim.
        result: VerificationResult,
    },
    /// The DebVerify reconciliation.
    Reconciliation {
        /// Preserved verbatim (closed enum, 6 variants).
        summary: ReconciliationSummary,
    },
    /// A lens contribution (contribution + gaps preserved complete).
    LensContribution {
        /// The contribution.
        contribution: LensContribution,
    },
    /// A lens coverage gap.
    LensGap {
        /// The typed gap (`NotEvaluated`).
        gap: NotEvaluated,
    },
    /// The alignment assessment.
    Alignment {
        /// Preserved verbatim, including its id.
        assessment: AlignmentAssessment,
    },
    /// Read-only contract provenance from the AC2 overlay (A4-S15R axes).
    ContractProvenance {
        /// `SpecifiedBy` (declared intent) and `VerifiedBy` (evidence),
        /// kept as **separate** axes.
        provenance: ContractProvenance,
    },
}

/// A leg of the chain the substrate cannot supply.
///
/// Present so the absence is *visible* rather than silently inferred away.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct AdvisoryWhyUnresolved {
    /// The edge that is missing, spelled `from→relation→to`.
    pub edge: String,
    /// Why it is missing, in terms of the substrate.
    pub reason: String,
}

/// Structurally why an advisory conclusion is not the stronger one.
///
/// Only fires on a **typed** negative reason. A missing graph edge is NOT
/// a negative reason and never produces a `AdvisoryWhyNot`.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum AdvisoryWhyNot {
    /// The verification result is not `Verified`. The typed result carries
    /// the exact reason (`Contradicted` / `Unknown` / `Stale` /
    /// `NotApplicable`) — never flattened into a generic "failed".
    VerificationNotVerified {
        /// The contract the claim is about.
        contract_id: String,
        /// The typed result.
        result: VerificationResult,
    },
    /// A concern had no evaluable lens. `reason` keeps
    /// `NoRegisteredLens` distinct from `LensExistsButRefused`.
    ConcernNotEvaluated {
        /// The concern that was not evaluated.
        concern: UniversalConcern,
        /// The typed reason.
        reason: NotEvaluatedReason,
    },
    /// The alignment state is `Unknown`: not enough to decide. NEVER a
    /// denial.
    AlignmentUnknown {
        /// The assessment id.
        id: AlignmentAssessmentId,
    },
}

/// The typed, deterministic, provenance-backed answer to "why is this
/// advisory item here?". EPHEMERAL.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct AdvisoryWhy {
    /// The subject that was asked about.
    pub subject: AdvisorySubjectRef,
    /// The substrate the answer was computed against.
    pub basis: AdvisoryWhyBasis,
    /// Typed provenance legs. Empty when the subject is not present.
    pub legs: Vec<AdvisoryWhyLeg>,
    /// Legs the substrate cannot supply. Never converted into a negative.
    pub unresolved: Vec<AdvisoryWhyUnresolved>,
    /// Typed negative reasons. Empty when the conclusion is already the
    /// strongest one available.
    pub why_not: Vec<AdvisoryWhyNot>,
}

// ─────────────────────────────────────────────────────────────────────────────
// Derivation
// ─────────────────────────────────────────────────────────────────────────────

/// Enumerate the advisory subjects present in a composition result, in
/// canonical order (sorted by [`subject_key`], deduplicated by typed
/// identity).
pub fn advisory_subjects(result: &IntelligenceLoopResult) -> Vec<AdvisorySubjectRef> {
    let mut subjects: Vec<AdvisorySubjectRef> = Vec::new();
    subjects.push(AdvisorySubjectRef::Reconciliation);
    for (claim, _) in &result.verification_pairs {
        if let Some(contract_id) = claim_contract_id(claim) {
            subjects.push(AdvisorySubjectRef::VerificationClaim {
                contract_id: contract_id.to_string(),
            });
        }
    }
    for c in &result.lens_evaluation.contributions {
        subjects.push(AdvisorySubjectRef::LensContribution { id: c.id.clone() });
    }
    for gap in &result.lens_evaluation.gaps {
        subjects.push(AdvisorySubjectRef::LensGap {
            concern: gap.concern(),
            reason: gap.reason,
        });
    }
    subjects.push(AdvisorySubjectRef::Alignment {
        id: result.alignment_assessment.id.clone(),
    });
    canonicalize_subjects(subjects)
}

/// Sort by canonical key and dedup by typed identity. Deterministic
/// regardless of the input order.
fn canonicalize_subjects(mut v: Vec<AdvisorySubjectRef>) -> Vec<AdvisorySubjectRef> {
    v.sort_by_key(subject_key);
    v.dedup_by(|a, b| a == b);
    v
}

/// Derive the existing A3 [`AdvisoryContext`] from a composition result.
///
/// PROJECTION: a pure function of the result; deterministic; canonical
/// order and dedup are delegated to [`AdvisoryContext`]. It reuses the A3
/// type verbatim — there is no second advisory model.
///
/// Every authority output that A4-5a composed is represented, so nothing
/// is lost between the loop and the delivered advisory payload.
pub fn derive_advisory_context(result: &IntelligenceLoopResult) -> AdvisoryContext {
    let mut ctx = AdvisoryContext::none();

    // DebVerify reconciliation posture.
    ctx.push(AdvisoryItem {
        kind: AdvisoryKind::DebtReconciliation,
        subject: subject_key(&AdvisorySubjectRef::Reconciliation),
        note: reconciliation_tag(&result.reconciliation).to_string(),
        provenance: AdvisoryProvenance::Observed,
    });

    // Verification outcomes, one per pair.
    for (claim, outcome) in &result.verification_pairs {
        let Some(contract_id) = claim_contract_id(claim) else {
            continue;
        };
        ctx.push(AdvisoryItem {
            kind: AdvisoryKind::VerificationOutcome,
            subject: subject_key(&AdvisorySubjectRef::VerificationClaim {
                contract_id: contract_id.to_string(),
            }),
            note: verification_result_tag(outcome).to_string(),
            provenance: AdvisoryProvenance::Observed,
        });
    }

    // Lens contributions.
    for c in &result.lens_evaluation.contributions {
        ctx.push(AdvisoryItem {
            kind: AdvisoryKind::ParadigmObservation,
            subject: subject_key(&AdvisorySubjectRef::LensContribution { id: c.id.clone() }),
            note: format!(
                "{}:{}",
                c.concern.canonical_tag(),
                posture_tag(&c.evidence_resolution)
            ),
            provenance: AdvisoryProvenance::Observed,
        });
    }

    // Lens coverage gaps — typed reason preserved, not folded into a
    // generic unknown.
    for gap in &result.lens_evaluation.gaps {
        ctx.push(AdvisoryItem {
            kind: AdvisoryKind::LensCoverageGap,
            subject: subject_key(&AdvisorySubjectRef::LensGap {
                concern: gap.concern(),
                reason: gap.reason,
            }),
            note: gap.reason.canonical_tag().to_string(),
            provenance: AdvisoryProvenance::Observed,
        });
    }

    // Alignment posture.
    ctx.push(AdvisoryItem {
        kind: AdvisoryKind::AlignmentTension,
        subject: subject_key(&AdvisorySubjectRef::Alignment {
            id: result.alignment_assessment.id.clone(),
        }),
        note: result.alignment_assessment.state.canonical().to_string(),
        provenance: AdvisoryProvenance::Observed,
    });

    ctx
}

// ─────────────────────────────────────────────────────────────────────────────
// WHY
// ─────────────────────────────────────────────────────────────────────────────

/// Explain one advisory subject by its typed provenance.
///
/// - `result` and `receipt_id` are the A4-5a outputs (the receipt id is not
///   recoverable from the result alone, which is why it is passed in).
/// - `graph` is the read-only AC2 overlay, used to reach the two provenance
///   axes for verification subjects.
///
/// Never runs a producer, never invents a cause, never strengthens a
/// conclusion. If the subject is not present in the result, the absence is
/// reported via [`AdvisoryWhy::unresolved`] rather than a negative claim.
pub fn explain_advisory(
    result: &IntelligenceLoopResult,
    receipt_id: &IntelligenceLoopReceiptId,
    graph: &ArchitectureGraphOverlay,
    subject: &AdvisorySubjectRef,
) -> AdvisoryWhy {
    let basis = AdvisoryWhyBasis {
        receipt_id: receipt_id.clone(),
        knowledge_basis_hash_hex: result.knowledge_basis_hash_hex.clone(),
        observation_set_canonical_digest: result.observation_set_canonical_digest.clone(),
    };
    let mut legs: Vec<AdvisoryWhyLeg> = Vec::new();
    let mut unresolved: Vec<AdvisoryWhyUnresolved> = Vec::new();
    let mut why_not: Vec<AdvisoryWhyNot> = Vec::new();

    match subject {
        AdvisorySubjectRef::Reconciliation => {
            legs.push(AdvisoryWhyLeg::Reconciliation {
                summary: result.reconciliation.clone(),
            });
        }
        AdvisorySubjectRef::VerificationClaim { contract_id } => {
            let found = result
                .verification_pairs
                .iter()
                .find(|(c, _)| claim_contract_id(c) == Some(contract_id.as_str()));
            match found {
                Some((claim, outcome)) => {
                    legs.push(AdvisoryWhyLeg::Verification {
                        claim: claim.clone(),
                        result: outcome.clone(),
                    });
                    if !matches!(outcome, VerificationResult::Verified) {
                        why_not.push(AdvisoryWhyNot::VerificationNotVerified {
                            contract_id: contract_id.clone(),
                            result: outcome.clone(),
                        });
                    }
                    // Contract provenance: the two axes, kept separate.
                    let provenance = graph.contract_provenance(contract_id);
                    if provenance.verified_by.is_empty() {
                        // Absence of evidence is reported, NOT negated.
                        unresolved.push(AdvisoryWhyUnresolved {
                            edge: format!("contract:{contract_id}→VerifiedBy→evidence"),
                            reason: "no evidence is linked on the VerifiedBy axis; \
                                     this is an ABSENT leg, not a negative finding"
                                .to_string(),
                        });
                    }
                    legs.push(AdvisoryWhyLeg::ContractProvenance { provenance });
                }
                None => {
                    unresolved.push(AdvisoryWhyUnresolved {
                        edge: format!("advisory→verification:{contract_id}"),
                        reason: "no verification pair for this contract is present in \
                                 the composition result"
                            .to_string(),
                    });
                }
            }
        }
        AdvisorySubjectRef::LensContribution { id } => {
            match result
                .lens_evaluation
                .contributions
                .iter()
                .find(|c| &c.id == id)
            {
                Some(c) => legs.push(AdvisoryWhyLeg::LensContribution {
                    contribution: c.clone(),
                }),
                None => unresolved.push(AdvisoryWhyUnresolved {
                    edge: format!("advisory→lens_contribution:{}", id.as_str()),
                    reason: "no such contribution is present in the composition result".to_string(),
                }),
            }
        }
        AdvisorySubjectRef::LensGap { concern, reason } => {
            match result
                .lens_evaluation
                .gaps
                .iter()
                .find(|g| g.concern() == *concern && g.reason == *reason)
            {
                Some(gap) => {
                    legs.push(AdvisoryWhyLeg::LensGap { gap: gap.clone() });
                    why_not.push(AdvisoryWhyNot::ConcernNotEvaluated {
                        concern: *concern,
                        reason: *reason,
                    });
                }
                None => unresolved.push(AdvisoryWhyUnresolved {
                    edge: format!(
                        "advisory→lens_gap:{}:{}",
                        concern.canonical_tag(),
                        reason.canonical_tag()
                    ),
                    reason: "no such coverage gap is present in the composition result".to_string(),
                }),
            }
        }
        AdvisorySubjectRef::Alignment { id } => {
            if &result.alignment_assessment.id == id {
                legs.push(AdvisoryWhyLeg::Alignment {
                    assessment: result.alignment_assessment.clone(),
                });
                if result.alignment_assessment.state == AlignmentState::Unknown {
                    // Unknown is "not enough evidence to decide"; it is
                    // explicitly NOT a denial.
                    why_not.push(AdvisoryWhyNot::AlignmentUnknown { id: id.clone() });
                }
            } else {
                unresolved.push(AdvisoryWhyUnresolved {
                    edge: format!("advisory→alignment:{}", id.as_str()),
                    reason: "the composition result carries a different alignment \
                             assessment id"
                        .to_string(),
                });
            }
        }
    }

    AdvisoryWhy {
        subject: subject.clone(),
        basis,
        legs,
        unresolved,
        why_not,
    }
}

// ─────────────────────────────────────────────────────────────────────────────
// Typed tags (closed matches; no Debug/Display)
// ─────────────────────────────────────────────────────────────────────────────

/// The contract a verification claim observes, if the claim names one.
fn claim_contract_id(claim: &VerificationClaim) -> Option<&str> {
    match claim {
        VerificationClaim::ArchitectureConformance(c) => Some(c.contract_id.as_str()),
        VerificationClaim::StaticProvider(c) => Some(c.contract_id.as_str()),
        VerificationClaim::RuntimeProvider(c) => Some(c.contract_id.as_str()),
    }
}

/// Canonical tag for a verification result outcome.
fn verification_result_tag(result: &VerificationResult) -> &'static str {
    match result {
        VerificationResult::Verified => "verified",
        VerificationResult::Contradicted { .. } => "contradicted",
        VerificationResult::Unknown { .. } => "unknown",
        VerificationResult::Stale { .. } => "stale",
        VerificationResult::NotApplicable => "not_applicable",
    }
}

/// Canonical tag for a reconciliation summary verdict.
fn reconciliation_tag(summary: &ReconciliationSummary) -> &'static str {
    match summary {
        ReconciliationSummary::ConfirmedBaseline { .. } => "confirmed_baseline",
        ReconciliationSummary::Contradiction(_) => "contradiction",
        ReconciliationSummary::Staleness(_) => "staleness",
        ReconciliationSummary::EvidenceGap(_) => "evidence_gap",
        ReconciliationSummary::AcceptedDebt(_) => "accepted_debt",
        ReconciliationSummary::NotApplicable => "not_applicable",
    }
}

/// Canonical tag for a lens evidence posture.
fn posture_tag(p: &crate::observation::LensEvidenceResolution) -> &'static str {
    use crate::observation::EvidencePosture as P;
    match p {
        P::Supported { .. } => "supported",
        P::Contradicted { .. } => "contradicted",
        P::Conflicted { .. } => "conflicted",
        P::Insufficient { .. } => "insufficient",
    }
}
