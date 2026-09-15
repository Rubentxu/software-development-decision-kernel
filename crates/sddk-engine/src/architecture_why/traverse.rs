//! The provenance traversal behind `why architecture`.
//!
//! Reads only what the caller already computed — the declared contracts, the AC1
//! claims the AC2 overlay registered, and AC5's audit. It performs **no
//! evaluation**: no `ContractEvaluation::evaluate`, no second audit, no graph
//! mutation. That is what makes it an explanation rather than a re-derivation.

use std::collections::BTreeMap;

use crate::architectural_contract::{ArchitecturalContract, ArchitectureClaim, ContractPayload};
use crate::architecture_conformance::DeltaContractStatus;
use crate::architecture_debverify::{DebVerifyAudit, FindingBasis};
use crate::architecture_graph::ArchitectureGraphOverlay;
use crate::knowledge::MissingEvidence;
use crate::semantic_graph::SemanticGraphProjection;

use super::types::{
    ArchitectureWhy, WhyAssessment, WhyBasis, WhyContract, WhyEvidence, WhyFinding, WhyIntent,
    WhyNotReason, WhyResolvedAs, WhyUnresolved,
};

/// Everything the traversal reads. All borrowed: `explain` owns nothing and
/// mutates nothing.
pub struct WhyInput<'a> {
    /// The argument exactly as given.
    pub query: &'a str,
    /// Which namespace the caller resolved it against.
    pub resolved_as: WhyResolvedAs,
    /// The finding basis (clock-stable).
    pub basis: &'a FindingBasis,
    /// The declared contracts.
    pub contracts: &'a [ArchitecturalContract],
    /// The AC1 claims the overlay registered, keyed by contract id.
    pub claims: &'a BTreeMap<String, ArchitectureClaim>,
    /// AC5's audit.
    pub audit: &'a DebVerifyAudit,
    /// The AC2 overlay.
    pub overlay: &'a ArchitectureGraphOverlay,
    /// The resolved finding, as `(id, index into audit.findings)`.
    ///
    /// The id is borrowed as a string because the traversal only reports it; it
    /// does not re-derive identity.
    pub finding: Option<(&'a str, usize)>,
}

/// Build the explanation.
pub fn explain(input: WhyInput<'_>) -> ArchitectureWhy {
    let basis = WhyBasis {
        revision: input.basis.revision.clone(),
        knowledge_basis: input.basis.knowledge_basis.clone(),
        finding_basis_digest: hex(&input.basis.contract_set_digest),
    };

    // The contracts to explain, in a deterministic order.
    let contract_ids: Vec<String> = match input.finding {
        Some((_, index)) => input
            .audit
            .findings
            .get(index)
            .map(|f| {
                f.contract_ids
                    .iter()
                    .map(|c| c.as_str().to_string())
                    .collect()
            })
            .unwrap_or_default(),
        // Resolved as a contract: explain exactly that one.
        None => vec![input.query.to_string()],
    };

    let finding = input.finding.and_then(|(id, index)| {
        input.audit.findings.get(index).map(|f| WhyFinding {
            id: id.to_string(),
            kind: f.kind.canonical_tag().to_string(),
            severity: f.severity.canonical_tag().to_string(),
            subjects: f.subjects.clone(),
            contract_ids: f
                .contract_ids
                .iter()
                .map(|c| c.as_str().to_string())
                .collect(),
        })
    });

    let mut contracts: Vec<WhyContract> = Vec::with_capacity(contract_ids.len());
    let mut why_not: Vec<WhyNotReason> = Vec::new();

    for cid in &contract_ids {
        let Some(contract) = input.contracts.iter().find(|c| c.id().as_str() == cid) else {
            // Declared by a finding but absent from the contract set: report the
            // gap rather than a leg with invented content.
            contracts.push(WhyContract {
                contract: cid.clone(),
                participates_because:
                    "named by the finding but not present in the validated contract set".to_string(),
                kind: "unknown".to_string(),
                subject: String::new(),
                revision: String::new(),
                assessment: WhyAssessment {
                    outcome: DeltaContractStatus::NotEvaluated.canonical_tag().to_string(),
                    evidence_present: false,
                    missing_evidence: vec![],
                },
                intent: WhyIntent {
                    decisions: vec![],
                    specs: vec![],
                },
                evidence: vec![],
                software_units: vec![],
            });
            why_not.push(WhyNotReason::ContractNotEvaluable {
                contract: cid.clone(),
                detail: "the finding names a contract that is not in the validated contract set"
                    .to_string(),
            });
            continue;
        };

        let claim = input.claims.get(cid);
        let assessment = match claim {
            Some(c) => {
                let outcome = DeltaContractStatus::from_claim_outcome(c.outcome());
                let missing: Vec<String> = c
                    .missing_evidence()
                    .iter()
                    .map(|m| match m {
                        MissingEvidence::NotProvided => "not_provided".to_string(),
                        MissingEvidence::FutureEvidence => "future_evidence".to_string(),
                    })
                    .collect();
                if outcome == DeltaContractStatus::Contradicted {
                    why_not.push(WhyNotReason::Contradiction {
                        contract: cid.clone(),
                    });
                } else if outcome == DeltaContractStatus::Stale {
                    why_not.push(WhyNotReason::Stale {
                        contract: cid.clone(),
                    });
                }
                for m in c.missing_evidence() {
                    why_not.push(match m {
                        MissingEvidence::NotProvided => WhyNotReason::MissingEvidence {
                            contract: cid.clone(),
                        },
                        MissingEvidence::FutureEvidence => WhyNotReason::FutureEvidence {
                            contract: cid.clone(),
                        },
                    });
                }
                WhyAssessment {
                    outcome: outcome.canonical_tag().to_string(),
                    evidence_present: !c.evidence_refs().is_empty(),
                    missing_evidence: missing,
                }
            }
            None => {
                // No claim means the overlay linked nothing, which happens when
                // the contract's subject unit is not declared. That is an
                // unanswerable leg, not an empty one.
                why_not.push(WhyNotReason::ContractNotEvaluable {
                    contract: cid.clone(),
                    detail: "no AC1 claim exists for this contract, because the AC2 overlay links \
                             a contract only when its subject is a declared unit"
                        .to_string(),
                });
                WhyAssessment {
                    outcome: DeltaContractStatus::NotEvaluated
                        .canonical_tag()
                        .to_string(),
                    evidence_present: false,
                    missing_evidence: vec![],
                }
            }
        };

        let evidence: Vec<WhyEvidence> = claim
            .map(|c| {
                c.evidence_refs()
                    .iter()
                    .map(|e| WhyEvidence {
                        provider: e.provider().to_string(),
                        reference: e.reference().to_string(),
                    })
                    .collect()
            })
            .unwrap_or_default();

        contracts.push(WhyContract {
            contract: cid.clone(),
            participates_because: participates(contract),
            kind: contract.payload().kind_tag().to_string(),
            subject: contract.payload().subject(contract.id().as_str()),
            revision: contract.revision().as_str().to_string(),
            assessment,
            intent: WhyIntent {
                decisions: vec![decision_ref(contract)],
                specs: vec![spec_ref(contract)],
            },
            evidence,
            software_units: units_for_contract(input.overlay, cid),
        });
    }

    // Deterministic order: by contract id, which is how the audit already sorts.
    contracts.sort_by(|a, b| a.contract.cmp(&b.contract));

    // The one leg the substrate cannot supply, always reported.
    let unresolved_edges = vec![WhyUnresolved {
        edge: ArchitectureWhy::EVIDENCE_TO_SOFTWARE_EDGE.to_string(),
        reason: ArchitectureWhy::EVIDENCE_TO_SOFTWARE_REASON.to_string(),
    }];
    why_not.push(WhyNotReason::UnknownRelation {
        edge: ArchitectureWhy::EVIDENCE_TO_SOFTWARE_EDGE.to_string(),
    });
    why_not.sort_by_key(|r| format!("{r:?}"));
    why_not.dedup();

    ArchitectureWhy {
        query: input.query.to_string(),
        resolved_as: input.resolved_as,
        basis,
        finding,
        contracts,
        unresolved_edges,
        why_not,
    }
}

/// Why a contract participates in a finding.
///
/// Derived from the finding kind and the contract's **typed payload**. The
/// finding's `message` is never read: it is informational and can change for a
/// redaction alone.
fn participates(contract: &ArchitecturalContract) -> String {
    match contract.payload() {
        ContractPayload::SingleAuthority(c) => format!(
            "claims exclusive authority over component `{c}`; two or more such claims on one \
             component are a shadow authority"
        ),
        ContractPayload::UniqueOwner(e) => format!(
            "claims unique ownership of entity `{e}`; an entity declared both owner and \
             projection-only is a contradiction"
        ),
        ContractPayload::ProjectionOnly { source_kind } => format!(
            "declares `{source_kind}` projection-only; declaring one subject both \
             authority-owned and projection-only is a contradiction"
        ),
        ContractPayload::ForbiddenDependency { from, to, .. } => format!(
            "forbids the dependency `{from} -> {to}`; the edge is present in the graph, which is \
             an authority bypass"
        ),
        ContractPayload::BoundedCompatibility { .. } => {
            "declares a bounded compatibility window; an elapsed window with no replacement is a \
             stale compatibility"
                .to_string()
        }
        ContractPayload::ProviderBoundary { surface, .. } => format!(
            "forbids inward provider types on surface `{surface}`; a violation of that boundary \
             is an authority bypass"
        ),
        ContractPayload::Extension { kind, .. } => format!(
            "extension contract of kind `{}`; it participates through its declared subject",
            kind.as_str()
        ),
    }
}

fn decision_ref(contract: &ArchitecturalContract) -> String {
    contract.decided_by().render()
}

fn spec_ref(contract: &ArchitecturalContract) -> String {
    contract.specified_by().render()
}

/// Software units reachable from a contract through the AC2 overlay.
///
/// `find_units_contracted_by` returns node ids; unit nodes carry the unit
/// reference as their locator (`add_unit` keys the node on `unit.id`), so the
/// locator is the unit ref.
fn units_for_contract(overlay: &ArchitectureGraphOverlay, contract_id: &str) -> Vec<String> {
    let ids: Vec<String> = overlay
        .find_units_contracted_by(contract_id)
        .iter()
        .map(|n| n.as_str().to_string())
        .collect();
    if ids.is_empty() {
        return Vec::new();
    }
    let projection = overlay.projection();
    let locators: BTreeMap<String, String> = projection
        .nodes()
        .into_iter()
        .map(|n| (n.id.as_str().to_string(), n.locator))
        .collect();
    let mut out: Vec<String> = ids
        .iter()
        .filter_map(|id| locators.get(id).cloned())
        .collect();
    out.sort();
    out.dedup();
    out
}

fn hex(bytes: &[u8]) -> String {
    bytes.iter().map(|b| format!("{b:02x}")).collect()
}
