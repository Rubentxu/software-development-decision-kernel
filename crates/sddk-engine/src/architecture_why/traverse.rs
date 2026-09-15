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
use crate::architecture_graph::{ArchitectureGraphOverlay, SoftwareUnitRef};
use crate::knowledge::MissingEvidence;
use crate::observation::{ObservationSet, ObservationSubject, SoftwareEntityRef};
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
    /// Observations available to close the `evidence → software relation` leg.
    ///
    /// `None` and an empty set mean the same thing for the answer: nothing observed
    /// the subject, so the leg stays unresolved.
    pub observations: Option<&'a ObservationSet>,
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
                observed_relations: vec![],
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
                // No claim means the overlay linked nothing. There are exactly
                // two causes, and naming the wrong one would be a false
                // explanation — the one thing this surface must not produce.
                // Found in the v1.169.37 release smoke: a `projection_only`
                // contract on a **declared** unit was reported as "subject is not
                // declared", which is false for it.
                why_not.push(WhyNotReason::ContractNotEvaluable {
                    contract: cid.clone(),
                    detail: unevaluable_reason(contract).to_string(),
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

        // OBSERVED provenance: relations an observation reports for this contract's
        // subject. This is what closes the `evidence → software relation` leg.
        let observed_relations: Vec<String> = match (input.observations, contract_entity(contract))
        {
            (Some(set), Some(entity)) => {
                let mut v: Vec<String> = set
                    .for_entity(&entity)
                    .into_iter()
                    .filter_map(|o| match &o.subject {
                        ObservationSubject::SoftwareRelation(r) => Some(r.render()),
                        _ => None,
                    })
                    .collect();
                v.sort();
                v.dedup();
                v
            }
            _ => Vec::new(),
        };

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
            observed_relations,
        });
    }

    // Deterministic order: by contract id, which is how the audit already sorts.
    contracts.sort_by(|a, b| a.contract.cmp(&b.contract));

    // The `evidence → software relation` leg is unresolved **only** when nothing
    // observed the subject. Since A4-0 the substrate can supply it, so reporting it
    // as structurally impossible would now be false.
    let any_observed = contracts.iter().any(|c| !c.observed_relations.is_empty());
    let unresolved_edges = if any_observed {
        Vec::new()
    } else {
        vec![WhyUnresolved {
            edge: ArchitectureWhy::EVIDENCE_TO_SOFTWARE_EDGE.to_string(),
            reason: ArchitectureWhy::EVIDENCE_TO_SOFTWARE_REASON.to_string(),
        }]
    };
    if !any_observed {
        why_not.push(WhyNotReason::UnknownRelation {
            edge: ArchitectureWhy::EVIDENCE_TO_SOFTWARE_EDGE.to_string(),
        });
    }
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

/// Why a contract has no AC1 claim, discriminating the two real causes.
///
/// The overlay links a contract only when its kind is unit-scoped **and** its
/// subject is a declared unit. Reporting the wrong one of those is worse than
/// reporting nothing, because a reader trusts the explanation.
fn unevaluable_reason(contract: &ArchitecturalContract) -> &'static str {
    if unit_scoped(contract) {
        "the contract is unit-scoped but its subject is not a declared unit, so the AC2 overlay \
         has no unit to attach a claim to"
    } else {
        "the contract's kind is not unit-scoped, so the AC2 overlay never links it (only \
         `single_authority` and `unique_owner` name a subject that is a unit id)"
    }
}

/// Whether a contract's kind names a subject that is also a unit id.
///
/// Mirrors `unit_subject` in the CLI's `declare_overlay`; both must agree or the
/// explanation below would describe a condition the overlay does not apply.
fn unit_scoped(contract: &ArchitecturalContract) -> bool {
    matches!(
        contract.payload(),
        ContractPayload::SingleAuthority(_) | ContractPayload::UniqueOwner(_)
    )
}

/// The contract's subject as a relation endpoint, when it has one.
///
/// This is the join key between a contract and the observations that involve its
/// subject. `BoundedCompatibility` and the other global kinds name no unit, so they
/// have no endpoint and therefore no observation can cover them.
fn contract_entity(contract: &ArchitecturalContract) -> Option<SoftwareEntityRef> {
    match contract.payload() {
        ContractPayload::SingleAuthority(c) => {
            Some(SoftwareEntityRef::Unit(SoftwareUnitRef::new(c.as_str())))
        }
        ContractPayload::UniqueOwner(e) => {
            Some(SoftwareEntityRef::Unit(SoftwareUnitRef::new(e.as_str())))
        }
        _ => None,
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
