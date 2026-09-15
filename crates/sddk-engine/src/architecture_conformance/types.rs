// Copyright (c) SDDK contributors.
// SPDX-License-Identifier: MIT
//
// architecture_conformance/types.rs — A3-S5 / AC4 public types.
//
// Closed vocabularies and data shapes for the ArchitectureConformanceDelta.
// See `mod.rs` for the state-class contract.

use crate::architectural_contract::{
    ArchitecturalContract, ArchitectureClaim, ClaimOutcome, ContractId, ContractKind, EvidenceRef,
};
use crate::architecture_graph::SoftwareUnitRef;
use crate::knowledge::EventTime;
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;

// ─────────────────────────────────────────────────────────────────────────────
// DeltaContractStatus (REQ-AC4-001..004)
// ─────────────────────────────────────────────────────────────────────────────

/// Closed taxonomy of per-contract conformance statuses (AC-034-003).
///
/// `NotEvaluated` is produced only when a contract is affected by the change
/// basis but no [`crate::architectural_contract::ArchitecturalContract`]
/// object was supplied for it (provider-dependent probe deferred).
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, PartialOrd, Ord, Serialize, Deserialize)]
pub enum DeltaContractStatus {
    /// Evidence was provided and the contract holds.
    Verified,
    /// Evidence contradicts the contract.
    Contradicted,
    /// Evidence is missing or insufficient.
    Unknown,
    /// The contract is past its applicability window.
    Stale,
    /// The contract is affected but no contract object was supplied.
    NotEvaluated,
}

impl DeltaContractStatus {
    /// All variants, in canonical order (REQ-AC4-001).
    pub const ALL: [Self; 5] = [
        Self::Verified,
        Self::Contradicted,
        Self::Unknown,
        Self::Stale,
        Self::NotEvaluated,
    ];

    /// Total mapping from the AC1 claim outcome (REQ-AC4-002).
    ///
    /// `NotEvaluated` is never produced here; it is a delta-level distinction
    /// (contract object absent), not an evaluation outcome.
    pub fn from_claim_outcome(outcome: ClaimOutcome) -> Self {
        match outcome {
            ClaimOutcome::Verified => Self::Verified,
            ClaimOutcome::Contradicted => Self::Contradicted,
            ClaimOutcome::Unknown => Self::Unknown,
            ClaimOutcome::Stale => Self::Stale,
        }
    }

    /// Canonical short tag for receipts (REQ-AC4-003).
    pub fn canonical_tag(self) -> &'static str {
        match self {
            Self::Verified => "verified",
            Self::Contradicted => "contradicted",
            Self::Unknown => "unknown",
            Self::Stale => "stale",
            Self::NotEvaluated => "not_evaluated",
        }
    }
}

// ─────────────────────────────────────────────────────────────────────────────
// ProbeRequirement (REQ-AC4-009..011)
// ─────────────────────────────────────────────────────────────────────────────

/// The minimal deterministic probe a contract kind requires (AC-034-001).
///
/// The probe *kind* is a planning label: AC4 does not execute arbitrary
/// probes — it derives the required probe set so that downstream Verify
/// tooling knows the minimum deterministic work for a change basis.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, PartialOrd, Ord, Serialize, Deserialize)]
pub enum ProbeRequirement {
    /// Ownership / single-authority probe.
    OwnershipProbe,
    /// Forbidden-dependency probe.
    DependencyProbe,
    /// Projection-only probe (entity is rebuilt, not stored).
    ProjectionProbe,
    /// Bounded-compatibility window probe.
    CompatibilityWindowProbe,
    /// Provider-boundary probe (no inward provider SDK types).
    ProviderBoundaryProbe,
    /// Extension-contract probe (declared-only until an evaluator exists).
    ExtensionProbe,
    /// A contract kind AC4 does not yet map. Reserved; unused today.
    Unknown,
}

impl ProbeRequirement {
    /// All variants, in canonical order (REQ-AC4-009).
    pub const ALL: [Self; 7] = [
        Self::OwnershipProbe,
        Self::DependencyProbe,
        Self::ProjectionProbe,
        Self::CompatibilityWindowProbe,
        Self::ProviderBoundaryProbe,
        Self::ExtensionProbe,
        Self::Unknown,
    ];

    /// Total, deterministic mapping from the AC1 contract kind (REQ-AC4-010).
    ///
    /// Never returns [`Self::Unknown`] for the current AC1 kind set; that
    /// variant is reserved for a future kind that AC4 has not yet mapped.
    pub fn for_contract_kind(kind: &ContractKind) -> Self {
        match kind {
            ContractKind::SingleAuthority => Self::OwnershipProbe,
            ContractKind::UniqueOwner => Self::OwnershipProbe,
            ContractKind::ForbiddenDependency => Self::DependencyProbe,
            ContractKind::ProjectionOnly => Self::ProjectionProbe,
            ContractKind::BoundedCompatibility => Self::CompatibilityWindowProbe,
            ContractKind::ProviderBoundary => Self::ProviderBoundaryProbe,
            ContractKind::Extension(_) => Self::ExtensionProbe,
        }
    }

    /// Canonical short tag for receipts.
    pub fn canonical_tag(self) -> &'static str {
        match self {
            Self::OwnershipProbe => "ownership_probe",
            Self::DependencyProbe => "dependency_probe",
            Self::ProjectionProbe => "projection_probe",
            Self::CompatibilityWindowProbe => "compatibility_window_probe",
            Self::ProviderBoundaryProbe => "provider_boundary_probe",
            Self::ExtensionProbe => "extension_probe",
            Self::Unknown => "unknown_probe",
        }
    }
}

// ─────────────────────────────────────────────────────────────────────────────
// AffectedContract (REQ-AC4-005, 011)
// ─────────────────────────────────────────────────────────────────────────────

/// A contract reached by the change basis, plus the minimum probe plan.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct AffectedContract {
    /// The contract's declared kind, when the contract object was supplied.
    ///
    /// `None` means the contract is reachable from the graph but no
    /// [`ArchitecturalContract`](crate::architectural_contract::ArchitecturalContract)
    /// object was provided; its status is `NotEvaluated`.
    pub contract_kind: Option<ContractKind>,
    /// The changed units that reached this contract (sorted, deduplicated).
    pub triggering_units: Vec<SoftwareUnitRef>,
    /// The minimal deterministic probe set (sorted, deduplicated).
    pub probes: Vec<ProbeRequirement>,
}

// ─────────────────────────────────────────────────────────────────────────────
// ConformanceVector (REQ-AC4-023..025)
// ─────────────────────────────────────────────────────────────────────────────

/// Per-dimension conformance status. **No numeric weight exists.**
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, PartialOrd, Ord, Serialize, Deserialize)]
pub enum VectorStatus {
    /// Every contributing contract is `Verified`.
    Verified,
    /// Some evidence is present but the dimension is not fully verified
    /// (e.g. a stale compatibility window).
    Partial,
    /// Evidence is missing or insufficient.
    Unknown,
    /// At least one contributing contract is contradicted.
    Contradicted,
    /// The dimension was not evaluated in this cycle/scope.
    NotEvaluated,
}

impl VectorStatus {
    /// All variants (REQ-AC4-024).
    pub const ALL: [Self; 5] = [
        Self::Verified,
        Self::Partial,
        Self::Unknown,
        Self::Contradicted,
        Self::NotEvaluated,
    ];

    /// Canonical short tag for receipts.
    pub fn canonical_tag(self) -> &'static str {
        match self {
            Self::Verified => "VERIFIED",
            Self::Partial => "PARTIAL",
            Self::Unknown => "UNKNOWN",
            Self::Contradicted => "CONTRADICTED",
            Self::NotEvaluated => "NOT_EVALUATED",
        }
    }

    /// Fully-verified wins, then contradicted, then unknown, then partial,
    /// then not-evaluated. Deterministic aggregation with **no weights**.
    pub(crate) fn combine(self, other: Self) -> Self {
        use VectorStatus::*;
        // Precedence: Contradicted > Unknown > Partial > Verified > NotEvaluated
        // (a single contradiction is the strongest signal; verified only when
        // everything verified).
        fn rank(s: VectorStatus) -> u8 {
            match s {
                Contradicted => 0,
                Unknown => 1,
                Partial => 2,
                Verified => 3,
                NotEvaluated => 4,
            }
        }
        if rank(self) <= rank(other) {
            self
        } else {
            other
        }
    }
}

/// The conformance vector: one status per architectural dimension.
///
/// The kernel emits statuses only; it never fabricates a universal weighted
/// score (AC-034-008, AC-UAT-043).
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct ConformanceVector {
    /// Single-authority dimension (`SingleAuthority` contracts).
    pub authority: VectorStatus,
    /// Unique-owner dimension (`UniqueOwner` contracts).
    pub ownership: VectorStatus,
    /// Forbidden-dependency dimension (`ForbiddenDependency` contracts).
    pub dependencies: VectorStatus,
    /// Bounded-compatibility dimension (`BoundedCompatibility` contracts).
    pub compatibility: VectorStatus,
    /// Negative-path dimension (`ProviderBoundary` contracts).
    pub negative_paths: VectorStatus,
    /// Paradigm alignment (AC7). Always `NotEvaluated` in AC4.
    pub paradigm_alignment: VectorStatus,
    /// Runtime evidence (AC11). Always `NotEvaluated` in AC4.
    pub runtime_evidence: VectorStatus,
}

impl ConformanceVector {
    /// The seven dimensions in canonical order (REQ-AC4-023).
    pub fn dimensions(&self) -> [(VectorDimension, VectorStatus); 7] {
        [
            (VectorDimension::Authority, self.authority),
            (VectorDimension::Ownership, self.ownership),
            (VectorDimension::Dependencies, self.dependencies),
            (VectorDimension::Compatibility, self.compatibility),
            (VectorDimension::NegativePaths, self.negative_paths),
            (VectorDimension::ParadigmAlignment, self.paradigm_alignment),
            (VectorDimension::RuntimeEvidence, self.runtime_evidence),
        ]
    }

    /// An all-`NotEvaluated` vector.
    pub fn empty() -> Self {
        Self {
            authority: VectorStatus::NotEvaluated,
            ownership: VectorStatus::NotEvaluated,
            dependencies: VectorStatus::NotEvaluated,
            compatibility: VectorStatus::NotEvaluated,
            negative_paths: VectorStatus::NotEvaluated,
            paradigm_alignment: VectorStatus::NotEvaluated,
            runtime_evidence: VectorStatus::NotEvaluated,
        }
    }
}

/// Named vector dimensions (REQ-AC4-023).
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, PartialOrd, Ord, Serialize, Deserialize)]
pub enum VectorDimension {
    /// `SingleAuthority`.
    Authority,
    /// `UniqueOwner`.
    Ownership,
    /// `ForbiddenDependency`.
    Dependencies,
    /// `BoundedCompatibility`.
    Compatibility,
    /// `ProviderBoundary`.
    NegativePaths,
    /// Owned by AC7.
    ParadigmAlignment,
    /// Owned by AC11.
    RuntimeEvidence,
}

// ─────────────────────────────────────────────────────────────────────────────
// ArchitectureConformanceDelta (REQ-AC4-006..008, 012, 020, 021)
// ─────────────────────────────────────────────────────────────────────────────

/// Stable identity for a delta: sha256 over the canonical delta payload.
#[derive(Clone, Debug, PartialEq, Eq, Hash, PartialOrd, Ord, Serialize, Deserialize)]
pub struct ArchitectureConformanceDeltaId(pub String);

impl ArchitectureConformanceDeltaId {
    /// Domain prefix for the delta id derivation.
    pub const DOMAIN_PREFIX: &'static str = "sddk.architecture_conformance.delta_id.v1|";

    /// Derive the id from the plan + contract-set + graph digests.
    pub fn derive(
        plan_digest: &[u8; 32],
        contract_set_digest: &[u8; 32],
        graph_digest: &[u8],
    ) -> Self {
        use sha2::{Digest, Sha256};
        let mut h = Sha256::new();
        h.update(Self::DOMAIN_PREFIX.as_bytes());
        h.update(plan_digest);
        h.update(b"|");
        h.update(contract_set_digest);
        h.update(b"|");
        h.update(graph_digest);
        let hex: String = h.finalize().iter().map(|b| format!("{b:02x}")).collect();
        Self(hex)
    }
}

/// The evidence-backed conformance delta for a change basis (AC4 exit).
///
/// A **PROJECTION** value (ADR-0095): reconstructible from the graph revision,
/// the contract set, the evidence and `now`. Never a persistence authority;
/// this module performs no IO.
///
/// `claims` holds only outcomes produced by AC1's
/// [`ContractEvaluation::evaluate`](crate::architectural_contract::ContractEvaluation::evaluate)
/// (`Verified` / `Unknown` / `Stale`). `Contradicted` is a *delta-level*
/// distinction carried by `contradictions`: AC1's evaluator deliberately never
/// produces it ("A4 Verify is responsible for `Contradicted`"). AC4 records
/// probe-observed violations supplied by the caller (the AC6 mutation seam)
/// without ever constructing a claim itself.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct ArchitectureConformanceDelta {
    /// Affected contracts keyed by id (deterministic iteration order).
    pub affected: BTreeMap<ContractId, AffectedContract>,
    /// One claim per evaluated contract (absent for contradicted/not-evaluated).
    pub claims: BTreeMap<ContractId, ArchitectureClaim>,
    /// Affected contracts with a probe-observed violation (sorted).
    pub contradictions: Vec<ContractId>,
    /// Affected contracts with no supplied contract object (sorted).
    pub not_evaluated: Vec<ContractId>,
    /// sha256 over the canonical probe plan.
    pub plan_digest: [u8; 32],
    /// sha256 over the sorted supplied contract basis hashes.
    pub contract_set_digest: [u8; 32],
    /// The overlay digest at compute time.
    pub graph_digest: Vec<u8>,
    /// The evaluation timestamp passed by the caller.
    pub evaluated_at: EventTime,
    /// Per-dimension statuses (no aggregate score).
    pub vector: ConformanceVector,
    /// Identity derived from the three digests.
    pub id: ArchitectureConformanceDeltaId,
}

impl ArchitectureConformanceDelta {
    /// Contracts whose claim outcome is `Unknown` (sorted).
    pub fn unknowns(&self) -> Vec<ContractId> {
        self.by_outcome(ClaimOutcome::Unknown)
    }

    /// Contracts with a probe-observed violation (sorted).
    pub fn contradictions(&self) -> &[ContractId] {
        &self.contradictions
    }

    /// Contracts whose claim outcome is `Stale` (sorted).
    pub fn stale(&self) -> Vec<ContractId> {
        self.by_outcome(ClaimOutcome::Stale)
    }

    /// Contracts that were affected but not evaluated (sorted).
    pub fn not_evaluated(&self) -> &[ContractId] {
        &self.not_evaluated
    }

    /// Resolve the status of one affected contract, if present.
    ///
    /// Precedence: `Contradicted` > `NotEvaluated` > claim outcome.
    pub fn status_of(&self, contract: &ContractId) -> Option<DeltaContractStatus> {
        if self.contradictions.contains(contract) {
            return Some(DeltaContractStatus::Contradicted);
        }
        if self.not_evaluated.contains(contract) {
            return Some(DeltaContractStatus::NotEvaluated);
        }
        self.claims
            .get(contract)
            .map(|c| DeltaContractStatus::from_claim_outcome(c.outcome()))
    }

    /// All statuses, sorted by contract id.
    pub fn statuses(&self) -> Vec<(ContractId, DeltaContractStatus)> {
        let mut out: Vec<(ContractId, DeltaContractStatus)> = Vec::new();
        for id in self.affected.keys() {
            if let Some(status) = self.status_of(id) {
                out.push((id.clone(), status));
            }
        }
        out
    }

    fn by_outcome(&self, outcome: ClaimOutcome) -> Vec<ContractId> {
        self.claims
            .iter()
            .filter(|(_, c)| c.outcome() == outcome)
            .map(|(id, _)| id.clone())
            .collect()
    }
}

/// Evidence supplied per contract for the evaluation.
pub type ContractEvidence = BTreeMap<ContractId, Vec<EvidenceRef>>;

/// The complete input basis for a conformance computation.
#[derive(Clone, Debug)]
pub struct ConformanceInputs<'a> {
    /// The AC1 contract objects (authority).
    pub contracts: &'a [ArchitecturalContract],
    /// Evidence refs supplied per contract id.
    pub evidence: &'a ContractEvidence,
    /// Contracts for which a deterministic probe observed a violation.
    ///
    /// This is the seam the AC6 mutation probes will drive. AC4 does not
    /// execute arbitrary probes; it records the witnesses it is given.
    pub contradiction_witnesses: &'a [ContractId],
    /// When set, only this contract may enter the delta's scope.
    ///
    /// A *filter*, not a widened scope: it can only narrow. `--contract` on the
    /// CLI sets it; leaving it `None` reproduces pre-A3-S13 behaviour exactly.
    pub contract_filter: Option<&'a ContractId>,
}

// ─────────────────────────────────────────────────────────────────────────────
// Errors
// ─────────────────────────────────────────────────────────────────────────────

/// Errors surfaced while computing a conformance delta.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum ConformanceError {
    /// An anchor node returned by the overlay could not be resolved to a
    /// `contract_id` (AC2 `add_contract_metadata` convention violated).
    UnresolvedContractAnchor {
        /// The anchor node id that could not be resolved.
        anchor: String,
    },
    /// An anchor carried a `contract_id` prop that is not a valid
    /// [`ContractId`].
    InvalidContractId {
        /// The raw prop value.
        raw: String,
    },
}

impl std::fmt::Display for ConformanceError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::UnresolvedContractAnchor { anchor } => write!(
                f,
                "architecture_conformance: contract anchor `{anchor}` has no `contract_id` prop \
                 (was `ArchitectureGraphOverlay::add_contract_metadata` called?)"
            ),
            Self::InvalidContractId { raw } => write!(
                f,
                "architecture_conformance: contract anchor carried an invalid contract id `{raw}`"
            ),
        }
    }
}

impl std::error::Error for ConformanceError {}
