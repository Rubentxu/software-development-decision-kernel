// Copyright (c) SDDK contributors.
// SPDX-License-Identifier: MIT
//
// verify_kernel/types.rs — A4-1: closed vocabularies for the Generic Verify kernel.
//
// Cycle: `p-63676b11dc0ef88f/a4-1-generic-verify`
// Spec: `docs/architecture/specs/arch-spec-043-generic-verify.md`
// ADR: `docs/architecture/adrs/ADR-0123-VERIFY-AND-DEBVERIFY-ARE-DISTINCT.md`
//
// # State classes (per ADR-0095)
//
// - `VerificationResult` — **EPHEMERAL** (evaluation outcome, no durable identity).
// - `VerificationClaim` — **EPHEMERAL** (question asked of the kernel).
// - `ProbePlan`, `ProbePlanStep`, `ProbeKind` — **EPHEMERAL** (planning artifacts).
// - `ChangeBasis`, `AffectedSubjects`, `ContradictionReason`, `EvidenceGap`,
//   `BasisHash` — **EPHEMERAL** (computation values).
//
// # Constraints
//
// - Closed result vocabulary, no `bool` + `Option<String>` (arch-spec-043 §Constraints).
// - `VerificationResult::ALL` pins the 5-variant invariant (no silent additions).
// - Reuses `SoftwareObservation` from `arch-spec-042`; no second evidence representation.

use crate::architecture_graph::SoftwareUnitRef;
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::collections::BTreeSet;

/// Closed taxonomy of verification outcomes (arch-spec-043 §Constraints, ADR-0123 §3).
///
/// `Verified | Contradicted | Unknown(gap) | Stale(basis) | NotApplicable`
///
/// Never use `bool` + `Option<String>`; this enum carries the closed vocabulary.
#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
pub enum VerificationResult {
    /// Evidence was provided and the claim holds.
    Verified,
    /// Evidence contradicts the claim.
    Contradicted {
        /// Why the evidence contradicts the claim.
        reason: ContradictionReason,
    },
    /// Evidence is missing or insufficient.
    Unknown {
        /// What evidence is needed to resolve the claim.
        gap: EvidenceGap,
    },
    /// The claim is past its applicability window.
    Stale {
        /// The basis that established the claim is no longer current.
        basis: BasisHash,
    },
    /// The claim does not apply to the given scope.
    NotApplicable,
}

impl VerificationResult {
    /// All variants, in canonical order (structural pin).
    ///
    /// If a new variant is added silently, the const assertion in the pin test
    /// `verification_result_has_5_closed_variants` will fail.
    pub const ALL: [Self; 5] = [
        Self::Verified,
        Self::Contradicted {
            reason: ContradictionReason::Sentinel,
        },
        Self::Unknown {
            gap: EvidenceGap::Sentinel,
        },
        Self::Stale {
            basis: BasisHash::SENTINEL,
        },
        Self::NotApplicable,
    ];

    /// Returns `true` if this result is terminal (no further evaluation needed).
    pub fn is_terminal(&self) -> bool {
        matches!(
            self,
            Self::Verified | Self::Contradicted { .. } | Self::NotApplicable
        )
    }

    /// Returns `true` if this result indicates a gap that can be closed.
    pub fn has_resolvable_gap(&self) -> bool {
        matches!(self, Self::Unknown { .. } | Self::Stale { .. })
    }
}

// ─────────────────────────────────────────────────────────────────────────────
// VerificationClaim (closed ADT; domain-specific variants via trait)
// ─────────────────────────────────────────────────────────────────────────────

/// Closed taxonomy of verification claims (arch-spec-043).
///
/// The trait `VerificationDomain` is bounded by this type: each domain
/// registers an `impl From<VerificationClaim> for DomainClaim`.
///
/// Q1 decision (design.md §Decision Q1): Closed ADT core + trait-bounded
/// extension. This enum is the closed core; domain-specific claim variants
/// are expressed as `impl From<VerificationClaim> for DomainClaim`.
#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
pub enum VerificationClaim {
    /// Architecture conformance: a contract holds after a change.
    ArchitectureConformance(ArchitectureConformanceClaim),
}

/// Architecture-conformance claim variant.
///
/// Wraps the contract ID and the change basis so the adapter can call
/// `compute_conformance_delta` with the correct inputs.
#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
pub struct ArchitectureConformanceClaim {
    /// The contract being verified.
    pub contract_id: String,
    /// The change basis: units that changed.
    pub basis: ChangeBasis,
}

impl ArchitectureConformanceClaim {
    /// Construct an architecture-conformance claim.
    pub fn new(contract_id: impl Into<String>, basis: ChangeBasis) -> Self {
        Self {
            contract_id: contract_id.into(),
            basis,
        }
    }
}

// ─────────────────────────────────────────────────────────────────────────────
// ProbePlan (structural minimality pin)
// ─────────────────────────────────────────────────────────────────────────────

/// A deterministic probe plan derived from a claim.
///
/// Minimality is **structurally pinned** in `VerifyKernel::evaluate`:
/// the kernel asserts `plan_probe_kinds == required_probe_kinds`.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct ProbePlan {
    /// Ordered, deduplicated probe steps.
    pub steps: Vec<ProbePlanStep>,
}

impl ProbePlan {
    /// Returns the probe kinds in this plan, in canonical order.
    pub fn probe_kinds(&self) -> BTreeSet<ProbeKind> {
        self.steps.iter().map(|s| s.kind).collect()
    }

    /// Returns `true` if this plan covers exactly the required probe kinds.
    pub fn covers_exactly(&self, required: &BTreeSet<ProbeKind>) -> bool {
        self.probe_kinds() == *required
    }

    /// Returns `true` if this plan has no duplicate steps.
    pub fn is_deduplicated(&self) -> bool {
        let kinds: BTreeSet<_> = self.steps.iter().map(|s| &s.kind).collect();
        kinds.len() == self.steps.len()
    }

    /// Returns `true` if steps are in canonical order.
    pub fn is_sorted(&self) -> bool {
        self.steps.windows(2).all(|w| w[0].kind <= w[1].kind)
    }
}

/// A single step in a probe plan.
#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
pub struct ProbePlanStep {
    /// The kind of probe to execute.
    pub kind: ProbeKind,
    /// Which subjects this probe operates on.
    pub subjects: AffectedSubjects,
}

/// Probe kinds: minimal deterministic set per claim.
///
/// This enum is **not** a 1:1 mapping with `ProbeRequirement` from AC4.
/// It is a higher-level planning label that the kernel uses to assert
/// minimality. Adapters map domain-specific probe requirements to these kinds.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
pub enum ProbeKind {
    /// Ownership / single-authority probe.
    Ownership,
    /// Forbidden-dependency probe.
    Dependency,
    /// Projection-only probe.
    Projection,
    /// Bounded-compatibility window probe.
    CompatibilityWindow,
    /// Provider-boundary probe.
    ProviderBoundary,
    /// Extension-contract probe.
    Extension,
    /// Unknown probe (used when a contract object is absent).
    Unknown,
}

impl ProbeKind {
    /// All variants, in canonical order.
    pub const ALL: [Self; 7] = [
        Self::Ownership,
        Self::Dependency,
        Self::Projection,
        Self::CompatibilityWindow,
        Self::ProviderBoundary,
        Self::Extension,
        Self::Unknown,
    ];
}

// ─────────────────────────────────────────────────────────────────────────────
// Supporting types
// ─────────────────────────────────────────────────────────────────────────────

/// The change basis: units that changed and when.
#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
pub struct ChangeBasis {
    /// Units that changed.
    pub units: Vec<SoftwareUnitRef>,
    /// Optional: the git revision or timestamp the change is based on.
    pub base: Option<String>,
    /// Optional: evaluation time in epoch-ms (defaults to wall clock).
    pub now_ms: Option<i64>,
}

impl ChangeBasis {
    /// Construct a change basis from changed units.
    pub fn new(units: impl Into<Vec<SoftwareUnitRef>>) -> Self {
        Self {
            units: units.into(),
            base: None,
            now_ms: None,
        }
    }

    /// Set the base revision.
    pub fn with_base(mut self, base: impl Into<String>) -> Self {
        self.base = Some(base.into());
        self
    }

    /// Set the evaluation time.
    pub fn with_now_ms(mut self, now_ms: i64) -> Self {
        self.now_ms = Some(now_ms);
        self
    }

    /// Compute the basis hash for staleness detection.
    pub fn basis_hash(&self) -> BasisHash {
        let mut hasher = Sha256::new();
        for unit in &self.units {
            hasher.update(unit.as_str().as_bytes());
        }
        if let Some(base) = &self.base {
            hasher.update(base.as_bytes());
        }
        let result = hasher.finalize();
        BasisHash(result.into())
    }
}

/// Subjects affected by a claim.
#[derive(Default, Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
pub struct AffectedSubjects {
    /// Units in scope.
    pub units: BTreeSet<SoftwareUnitRef>,
    /// Components in scope.
    pub components: BTreeSet<String>,
    /// Entities in scope.
    pub entities: BTreeSet<String>,
}

impl AffectedSubjects {
    /// Returns `true` if no subjects are in scope.
    pub fn is_empty(&self) -> bool {
        self.units.is_empty() && self.components.is_empty() && self.entities.is_empty()
    }
}

/// Why evidence contradicts a claim.
#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
pub enum ContradictionReason {
    /// Sentinel value for `VerificationResult::ALL` const construction.
    Sentinel,
    /// An ownership violation was observed.
    OwnershipViolation,
    /// A forbidden dependency was observed.
    ForbiddenDependency,
    /// A compatibility window violation was observed.
    CompatibilityViolation,
    /// A provider boundary violation was observed.
    ProviderBoundaryViolation,
    /// A contradiction between evidence sources.
    EvidenceContradiction,
    /// A custom domain-specific reason.
    Custom(String),
}

impl std::fmt::Display for ContradictionReason {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Sentinel => write!(f, "sentinel"),
            Self::OwnershipViolation => write!(f, "ownership_violation"),
            Self::ForbiddenDependency => write!(f, "forbidden_dependency"),
            Self::CompatibilityViolation => write!(f, "compatibility_violation"),
            Self::ProviderBoundaryViolation => write!(f, "provider_boundary_violation"),
            Self::EvidenceContradiction => write!(f, "evidence_contradiction"),
            Self::Custom(s) => write!(f, "custom:{}", s),
        }
    }
}

/// What evidence is missing or insufficient.
#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
pub enum EvidenceGap {
    /// Sentinel value for `VerificationResult::ALL` const construction.
    Sentinel,
    /// No observation covers the claimed subject.
    MissingForSubject(String),
    /// No evidence was provided for the claim.
    NoEvidenceProvided,
    /// Evidence is insufficient to resolve the claim.
    InsufficientEvidence,
    /// A custom domain-specific gap.
    Custom(String),
}

impl std::fmt::Display for EvidenceGap {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Sentinel => write!(f, "sentinel"),
            Self::MissingForSubject(s) => write!(f, "missing_for_subject:{}", s),
            Self::NoEvidenceProvided => write!(f, "no_evidence_provided"),
            Self::InsufficientEvidence => write!(f, "insufficient_evidence"),
            Self::Custom(s) => write!(f, "custom:{}", s),
        }
    }
}

/// A hash of the basis that established a claim (for staleness detection).
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
pub struct BasisHash([u8; 32]);

impl BasisHash {
    /// Sentinel value for `VerificationResult::ALL` const construction.
    pub const SENTINEL: Self = Self([0u8; 32]);

    /// Construct from a 32-byte array.
    pub fn new(bytes: [u8; 32]) -> Self {
        Self(bytes)
    }

    /// Return the raw bytes.
    pub fn as_bytes(&self) -> &[u8; 32] {
        &self.0
    }
}

impl std::fmt::Display for BasisHash {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        for byte in &self.0 {
            write!(f, "{:02x}", byte)?;
        }
        Ok(())
    }
}
