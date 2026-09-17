//! The typed answer to "why does SDDK claim this about the architecture?".
//!
//! Every field is deliberately separated by **provenance class**, so a reader (or
//! an agent) never has to guess how strongly a statement is held:
//!
//! | Class | Field | Meaning |
//! |---|---|---|
//! | ASSESSMENT | [`WhyContract::assessment`] | what the evaluator concluded |
//! | DECLARED | [`WhyContract::intent`] | what the declaration says originated this |
//! | OBSERVED | [`WhyContract::evidence`] | the concrete references that were supplied |
//! | UNKNOWN | [`ArchitectureWhy::unresolved_edges`] | legs the substrate cannot support |
//!
//! Nothing is merged into a single narrative blob, and no gap is filled by
//! inference: a leg the substrate does not have is *reported*, not assumed.

use serde::Serialize;

/// Which namespace the query resolved against.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum WhyResolvedAs {
    /// The argument is a declared contract id.
    Contract,
    /// The argument is a `FindingId` from the current audit.
    Finding,
}

impl WhyResolvedAs {
    /// Canonical tag.
    pub fn canonical_tag(self) -> &'static str {
        match self {
            WhyResolvedAs::Contract => "contract",
            WhyResolvedAs::Finding => "finding",
        }
    }
}

/// The substrate the answer was computed against.
///
/// All three fields are clock-stable, which is what makes a finding id a usable
/// handle across invocations (see [`FindingBasis`](crate::architecture_debverify::FindingBasis)).
#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
pub struct WhyBasis {
    /// Exact named revision.
    pub revision: String,
    /// Human-readable knowledge basis.
    pub knowledge_basis: String,
    /// Hex of the finding basis digest (revision + knowledge basis + contract set).
    pub finding_basis_digest: String,
}

/// The finding being explained, when the query resolved as one.
///
/// `severity` is reported for triage; `subjects` and `contract_ids` are reported
/// in full so cardinality is never collapsed.
#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
pub struct WhyFinding {
    /// Deterministic finding id.
    pub id: String,
    /// Canonical kind tag (`shadow_authority`, ...).
    pub kind: String,
    /// Deterministic severity tag (`critical` | `high` | `medium`).
    pub severity: String,
    /// The subjects involved (sorted by the audit).
    pub subjects: Vec<String>,
    /// The contracts involved (sorted by the audit).
    pub contract_ids: Vec<String>,
}

/// What the evaluator concluded about one contract.
///
/// `outcome` reuses AC4's [`DeltaContractStatus`](crate::architecture_conformance::DeltaContractStatus)
/// vocabulary rather than minting a second one: it already carries exactly the
/// five states this needs, including `not_evaluated` for "no claim exists".
#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
pub struct WhyAssessment {
    /// `verified` | `contradicted` | `unknown` | `stale` | `not_evaluated`.
    pub outcome: String,
    /// Whether any evidence reference was supplied.
    pub evidence_present: bool,
    /// Why the assessment is not stronger. Empty when evidence was supplied.
    pub missing_evidence: Vec<String>,
}

/// What the declaration says originated this contract. DECLARED intent — never
/// treated as authority (ADR-0120).
#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
pub struct WhyIntent {
    /// Decision references (ADRs, decisions, external authorities).
    pub decisions: Vec<String>,
    /// Specification references.
    pub specs: Vec<String>,
}

/// One concrete evidence reference. OBSERVED provenance.
#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
pub struct WhyEvidence {
    /// Provider tag (`runtime`, `static`, `manual`, ...).
    pub provider: String,
    /// Opaque reference within the provider's namespace.
    pub reference: String,
}

/// One contract in the explanation. The finding→contract cardinality is
/// preserved: a finding with N contracts produces N legs.
#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
pub struct WhyContract {
    /// The contract id.
    pub contract: String,
    /// Why this contract participates in the finding, derived from the finding
    /// kind and the contract's typed payload — never from the finding's
    /// free-text `message`, which is not authority.
    pub participates_because: String,
    /// The contract's declared kind tag.
    pub kind: String,
    /// The contract's subject (component, entity, ...).
    pub subject: String,
    /// The contract's declared revision.
    pub revision: String,
    /// ASSESSMENT.
    pub assessment: WhyAssessment,
    /// DECLARED intent.
    pub intent: WhyIntent,
    /// OBSERVED evidence.
    pub evidence: Vec<WhyEvidence>,
    /// Software units reached from this contract through the AC2 overlay.
    pub software_units: Vec<String>,
    /// Software relations **observed** to involve this contract's subject.
    ///
    /// OBSERVED provenance: these come from `SoftwareObservation`s (A4-0), not from
    /// the declaration. Empty means nothing observed this contract's subject — which
    /// is why the `evidence → software relation` leg stays unresolved.
    pub observed_relations: Vec<String>,
}

/// A leg of the chain the substrate cannot supply.
///
/// Present so the absence is *visible* rather than silently inferred away.
#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
pub struct WhyUnresolved {
    /// The edge that is missing, spelled as `from→relation→to`.
    pub edge: String,
    /// Why it is missing, in terms of the substrate.
    pub reason: String,
}

/// Structurally why a claim does not reach `VERIFIED`.
///
/// **Prepared, not surfaced.** A3-S15 creates no `why-not` command; this ADT is
/// the seam a future one would render. It is populated from the claim's own
/// `missing_evidence` and from the unresolved legs, so it never invents a reason.
#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
#[serde(tag = "reason", rename_all = "snake_case")]
pub enum WhyNotReason {
    /// No basis was supplied where the contract required one.
    MissingEvidence {
        /// The contract waiting on it.
        contract: String,
    },
    /// Expected basis was supplied but its declared time is in the future.
    FutureEvidence {
        /// The contract waiting on it.
        contract: String,
    },
    /// A relation the explanation would need is not in the graph.
    UnknownRelation {
        /// The missing edge.
        edge: String,
    },
    /// The contract's subject unit is not declared, so no claim links it.
    ContractNotEvaluable {
        /// The contract.
        contract: String,
        /// Why it cannot be evaluated.
        detail: String,
    },
    /// The evaluator recorded a contradiction.
    Contradiction {
        /// The contradicted contract.
        contract: String,
    },
    /// The evaluation window has elapsed.
    Stale {
        /// The contract past its window.
        contract: String,
    },
}

/// The answer.
#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
pub struct ArchitectureWhy {
    /// The argument exactly as given.
    pub query: String,
    /// Which namespace it resolved against.
    pub resolved_as: WhyResolvedAs,
    /// The substrate the answer was computed against.
    pub basis: WhyBasis,
    /// The finding, when the query resolved as one.
    pub finding: Option<WhyFinding>,
    /// One leg per contract. Cardinality preserved.
    pub contracts: Vec<WhyContract>,
    /// Legs the substrate cannot supply. Never empty-but-implied: the
    /// `evidence→software` leg is always reported here.
    pub unresolved_edges: Vec<WhyUnresolved>,
    /// Prepared WHY-NOT material. Always populated; has no surface yet.
    pub why_not: Vec<WhyNotReason>,
}

impl ArchitectureWhy {
    /// The unresolved leg this cycle documents.
    ///
    /// It stays constant because the substrate still has **no edge** running
    /// from an evidence reference to a software relation. A4-S15R gave
    /// evidence a first-class projection node kind and repointed `VerifiedBy`
    /// at it, but the `evidence → observes → software_relation` edge is a
    /// different relation (`FU-A3-S15-1`, still open). So the leg is a
    /// truthful gap, not a structural impossibility.
    pub const EVIDENCE_TO_SOFTWARE_EDGE: &'static str = "evidence→observes→software_relation";

    /// Fixed reason text for [`Self::EVIDENCE_TO_SOFTWARE_EDGE`].
    pub const EVIDENCE_TO_SOFTWARE_REASON: &'static str = "no supplied observation covers this subject's software relations, so the \
         evidence → software relation leg is UNKNOWN rather than inferred. Supply \
         observations (A4-0 provides the substrate) to close it. This is a truthful \
         gap for this subject, not a structural impossibility.";
}
