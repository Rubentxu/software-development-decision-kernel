// Copyright (c) SDDK contributors.
// SPDX-License-Identifier: MIT
//
// architecture_receipt/types.rs — A3-S9 / AC8 public types.
//
// The ArchitectureConformanceReceipt: the named Base-mode artefact that
// aggregates the native architecture-conformance capabilities.

use crate::architectural_contract::ContractId;
use crate::architecture_conformance::{ConformanceVector, DeltaContractStatus};
use crate::architecture_mutation::MutationId;
use crate::architecture_mutation::MutationKind;
use crate::evidence_ref::EvidenceRef;
use crate::knowledge::EventTime;
use crate::paradigm_profile::{EvidenceBasis, LensStatus, ParadigmLensKind};
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;

// ─────────────────────────────────────────────────────────────────────────────
// ReceiptVerdict (REQ-AC8-004)
// ─────────────────────────────────────────────────────────────────────────────

/// The closed verdict of an architecture-conformance receipt.
///
/// Deliberately not a score: the receipt reports a verdict plus the evidence,
/// never an aggregate quality number (AC-UAT-043 discipline).
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, PartialOrd, Ord, Serialize, Deserialize)]
pub enum ReceiptVerdict {
    /// No unresolved MUST findings.
    Pass,
    /// Unresolved MUST findings exist, all covered by governed waivers.
    PassWithWaivers,
    /// At least one unresolved MUST finding is not covered by a waiver.
    Blocked,
}

impl ReceiptVerdict {
    /// All variants.
    pub const ALL: [Self; 3] = [Self::Pass, Self::PassWithWaivers, Self::Blocked];

    /// Canonical short tag.
    pub fn canonical_tag(self) -> &'static str {
        match self {
            Self::Pass => "pass",
            Self::PassWithWaivers => "pass_with_waivers",
            Self::Blocked => "blocked",
        }
    }
}

// ─────────────────────────────────────────────────────────────────────────────
// HistoricalClass (REQ-AC8-011, AC-041-002)
// ─────────────────────────────────────────────────────────────────────────────

/// The A0/A1 historical finding classes the self-audit must reproduce
/// (`01-VISION-AND-ARCHITECTURE.md` §A0/A1 closeout).
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, PartialOrd, Ord, Serialize, Deserialize)]
pub enum HistoricalClass {
    /// Duplicate authorities (`01-VISION`: "duplicate authorities").
    DuplicateAuthority,
    /// Dependency boundary breach ("overly coarse policy semantics" class).
    DependencyBoundary,
    /// Stale compatibility ("stale compatibility").
    BoundedCompatibility,
    /// Projection/authority confusion ("projection/authority confusion").
    ProjectionOnly,
    /// Missing negative evidence ("missing negative evidence").
    MissingNegativeEvidence,
}

impl HistoricalClass {
    /// All classes.
    pub const ALL: [Self; 5] = [
        Self::DuplicateAuthority,
        Self::DependencyBoundary,
        Self::BoundedCompatibility,
        Self::ProjectionOnly,
        Self::MissingNegativeEvidence,
    ];

    /// Canonical short tag.
    pub fn canonical_tag(self) -> &'static str {
        match self {
            Self::DuplicateAuthority => "duplicate_authority",
            Self::DependencyBoundary => "dependency_boundary",
            Self::BoundedCompatibility => "bounded_compatibility",
            Self::ProjectionOnly => "projection_only",
            Self::MissingNegativeEvidence => "missing_negative_evidence",
        }
    }
}

/// Whether a historical class was reproduced natively, with its evidence.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct ClassCoverage {
    /// The historical class.
    pub class: HistoricalClass,
    /// Whether the native analysis reproduced it.
    pub reproduced: bool,
    /// Concrete evidence (finding subjects / contract ids / mutation ids).
    pub evidence: Vec<String>,
}

// ─────────────────────────────────────────────────────────────────────────────
// ReceiptBasis (REQ-AC8-001/002, AC-041-001)
// ─────────────────────────────────────────────────────────────────────────────

/// The exact basis a receipt was produced against.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct ReceiptBasis {
    /// Exact named revision (commit sha or equivalent).
    pub revision: String,
    /// AC4's contract-set digest.
    pub contract_set_digest: [u8; 32],
    /// AC2's semantic-graph digest.
    pub semantic_graph_digest: Vec<u8>,
    /// AC4's probe-plan digest.
    pub verification_plan_digest: [u8; 32],
    /// Human-readable knowledge basis (spec/ADR set).
    pub knowledge_basis: String,
}

// ─────────────────────────────────────────────────────────────────────────────
// Aggregated rows
// ─────────────────────────────────────────────────────────────────────────────

/// One contract's evaluated status.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct ClaimResult {
    /// The contract.
    pub contract: ContractId,
    /// Its status in the delta.
    pub status: DeltaContractStatus,
}

/// One mutation probe's fitness result (negative evidence, AC-041-002).
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct MutationResult {
    /// The mutation spec.
    pub mutation: MutationId,
    /// The mutation kind.
    pub kind: MutationKind,
    /// Whether the guard detected it.
    pub detected: bool,
}

/// One lens assessment's result (advisory, AC7).
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct LensResult {
    /// The declared lens kind.
    pub lens: ParadigmLensKind,
    /// The anchor (locator form).
    pub anchor: String,
    /// The assessed status.
    pub status: LensStatus,
    /// The basis AC3 records.
    pub basis: EvidenceBasis,
}

/// A compatibility-window row.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct CompatibilityEntry {
    /// The contract.
    pub contract: ContractId,
    /// Its status.
    pub status: DeltaContractStatus,
}

/// The change basis a scoped receipt was computed against.
///
/// `None` on the receipt means a global run (no change basis supplied); an
/// empty `changed_units` inside a present basis is a *real* zero.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct ChangeBasis {
    /// The resolved base revision the diff was taken against.
    pub base: String,
    /// The declared units a changed path overlapped (sorted).
    pub changed_units: Vec<String>,
}

/// One unresolved finding that may block the receipt.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct UnresolvedFinding {
    /// A short kind tag (`unknown`, `contradicted`, `shadow_authority`, ...).
    pub kind: String,
    /// The subjects involved (sorted).
    pub subjects: Vec<String>,
    /// Whether this finding blocks PASS (AC-041-003).
    pub mandatory: bool,
    /// Waiver references that cover this finding (sorted). Never invented.
    pub waiver_refs: Vec<String>,
}

// ─────────────────────────────────────────────────────────────────────────────
// ReceiptId (REQ-AC8-003)
// ─────────────────────────────────────────────────────────────────────────────

/// Derived identity for a receipt.
#[derive(Clone, Debug, PartialEq, Eq, Hash, PartialOrd, Ord, Serialize, Deserialize)]
pub struct ReceiptId(pub String);

impl ReceiptId {
    /// Domain prefix for the derivation.
    pub const DOMAIN_PREFIX: &'static str = "sddk.architecture_receipt.id.v1|";

    /// Derive the id from the basis + verdict.
    pub fn derive(basis: &ReceiptBasis, verdict: ReceiptVerdict) -> Self {
        use sha2::{Digest, Sha256};
        let mut h = Sha256::new();
        h.update(Self::DOMAIN_PREFIX.as_bytes());
        h.update(basis.revision.as_bytes());
        h.update(b"|");
        h.update(basis.contract_set_digest);
        h.update(b"|");
        h.update(&basis.semantic_graph_digest);
        h.update(b"|");
        h.update(basis.verification_plan_digest);
        h.update(b"|");
        h.update(basis.knowledge_basis.as_bytes());
        h.update(b"|");
        h.update(verdict.canonical_tag().as_bytes());
        let hex: String = h.finalize().iter().map(|b| format!("{b:02x}")).collect();
        Self(hex)
    }

    /// Borrow the raw string.
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl std::fmt::Display for ReceiptId {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(&self.0)
    }
}

// ─────────────────────────────────────────────────────────────────────────────
// ArchitectureConformanceReceipt
// ─────────────────────────────────────────────────────────────────────────────

/// The named Base-mode architecture-conformance receipt.
///
/// A **PROJECTION** (ADR-0095): derived from the outputs of AC2/AC4/AC5/AC6/AC7
/// plus a revision and knowledge basis. No IO, no persistence, no authority.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct ArchitectureConformanceReceipt {
    /// Derived identity.
    pub id: ReceiptId,
    /// Exact basis.
    pub basis: ReceiptBasis,
    /// Per-contract results (sorted by contract id).
    ///
    /// NOTE: these come from AC4's **change-scoped** delta, so a global run
    /// with no change basis legitimately has none. `audited_contracts` reports
    /// how many contracts the global pass actually inspected.
    pub claim_results: Vec<ClaimResult>,
    /// How many contracts the global DebVerify pass inspected (AC5).
    pub audited_contracts: usize,
    /// Evidence cited by the aggregated capabilities (sorted by locator).
    pub evidence_refs: Vec<EvidenceRef>,
    /// Provider contributions; empty in Base mode (AC-041-005).
    pub provider_basis: Vec<String>,
    /// Contracts whose AC4 status is `Unknown`.
    pub unknowns: Vec<ContractId>,
    /// Contracts whose AC4 status is `Contradicted`.
    pub contradictions: Vec<ContractId>,
    /// Compatibility-window rows.
    pub compatibility_delta: Vec<CompatibilityEntry>,
    /// Mutation fitness results (sorted by spec id).
    pub mutation_results: Vec<MutationResult>,
    /// Lens results (advisory; sorted).
    pub lens_results: Vec<LensResult>,
    /// AC5 finding counts per kind.
    pub audit_findings: BTreeMap<String, usize>,
    /// AC4's conformance vector (statuses, never a score).
    pub vector: ConformanceVector,
    /// Historical-class coverage (AC-041-002).
    pub class_coverage: Vec<ClassCoverage>,
    /// All unresolved findings (mandatory ones drive the verdict).
    pub unresolved: Vec<UnresolvedFinding>,
    /// The change basis, when the run was change-scoped (AC8 + AC4).
    pub change_basis: Option<ChangeBasis>,
    /// Waivers supplied by the caller (never invented).
    pub waivers: Vec<String>,
    /// The verdict.
    pub verdict: ReceiptVerdict,
    /// When the receipt was produced (caller-supplied).
    pub created_at: EventTime,
}

impl ArchitectureConformanceReceipt {
    /// The mandatory (blocking) unresolved findings.
    pub fn mandatory_unresolved(&self) -> Vec<&UnresolvedFinding> {
        self.unresolved.iter().filter(|f| f.mandatory).collect()
    }

    /// The mandatory findings that are not covered by any waiver.
    pub fn uncovered_unresolved(&self) -> Vec<&UnresolvedFinding> {
        self.unresolved
            .iter()
            .filter(|f| f.mandatory && f.waiver_refs.is_empty())
            .collect()
    }

    /// Whether every historical class was reproduced (AC-UAT-016).
    pub fn all_classes_reproduced(&self) -> bool {
        self.class_coverage.iter().all(|c| c.reproduced)
    }

    /// The classes that were not reproduced.
    pub fn unreproduced_classes(&self) -> Vec<HistoricalClass> {
        self.class_coverage
            .iter()
            .filter(|c| !c.reproduced)
            .map(|c| c.class)
            .collect()
    }
}
