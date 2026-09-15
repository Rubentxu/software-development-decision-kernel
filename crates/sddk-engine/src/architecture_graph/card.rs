// Copyright (c) SDDK contributors.
// SPDX-License-Identifier: MIT
//
// architecture_graph/card.rs — A3 closeout: Software Unit cards.
//
// A bounded, rebuildable, **non-authoritative** projection of one software unit's
// knowledge. It exists so a consumer can ask for a unit-sized slice instead of a
// whole-graph dump, and so provenance and freshness survive the narrowing.
//
// Design rules (roadmap §A3 "Software Unit cards / progressive disclosure"):
//
// - projection, rebuildable from the same inputs the overlay is built from;
// - NOT authority: every field is derived, nothing here is canonical;
// - no second graph and no second DB: it reads the one projection read-only;
// - deterministic: fields are sorted, so two builds over equal inputs are equal;
// - provenance retained: decisions/specs/evidence carry their class;
// - no provider SDK types anywhere.

use serde::Serialize;

use crate::architectural_contract::ArchitecturalContract;
use crate::architecture_graph::ArchitectureGraphOverlay;
use crate::architecture_graph::SoftwareUnit;
use crate::knowledge::{KMT, KmtStatus, KnowledgeBasis};
use crate::semantic_graph::SemanticGraphProjection;

/// Where a card field's value came from.
///
/// Mirrors the provenance classes used elsewhere (`why architecture`): a reader
/// must be able to tell a declared fact from an observed one from a gap.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum CardProvenance {
    /// Declared by the architecture substrate.
    Declared,
    /// Reached through the projection's relations.
    Observed,
    /// Not present in the substrate. Stated, never inferred.
    Absent,
}

/// One dependency edge of the unit.
#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Serialize)]
pub struct CardDependency {
    /// The other end of the edge.
    pub counterpart: String,
    /// Relation tag, e.g. `ac2_rel_depends_on`.
    pub relation: String,
    /// `outbound` (unit → counterpart) or `inbound`.
    pub direction: String,
}

/// The unit's knowledge status, as a bounded statement about the *basis*.
///
/// A basis is global, so the honest per-unit statement is: which basis, whether
/// the basis carries an assertion keyed on this unit, and what the KMT says about
/// that assertion's freshness. Nothing here is invented per unit.
#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
pub struct CardKnowledgeStatus {
    /// The basis hash the card was built against.
    pub basis_hash: String,
    /// `true` iff the basis carries an assertion whose id matches the unit.
    pub has_assertion_for_unit: bool,
    /// KMT freshness tag, **only when an expected basis was supplied**.
    ///
    /// `None` means *not evaluated*, never "fresh". That distinction is exactly
    /// what A3's KMT work exists to keep: absence of an evaluation must not read
    /// as a passing one.
    pub freshness: Option<String>,
}

/// One software unit's bounded knowledge.
#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
pub struct SoftwareUnitCard {
    /// Unit identity.
    pub identity: String,
    /// Unit kind tag (module, crate, ...).
    pub kind: String,
    /// Source locator.
    pub locator: String,
    /// Module path, joined by `::`.
    pub module_path: String,
    /// Components/entities that own this unit, from `unique_owner` contracts
    /// naming it. Empty means no owner is declared — not that none exists.
    pub owned_by: Vec<String>,
    /// Declared relations touching this unit.
    pub dependencies: Vec<CardDependency>,
    /// Contracts constraining this unit.
    pub relevant_contracts: Vec<String>,
    /// Decisions those contracts are `decided_by`.
    pub relevant_decisions: Vec<String>,
    /// Specs those contracts are `specified_by`.
    pub relevant_specs: Vec<String>,
    /// Evidence references attached to this unit's relations.
    pub evidence_refs: Vec<String>,
    /// The unit's declared purpose, when the substrate states one.
    ///
    /// `None` today: `SoftwareUnit` has no purpose field and this projection
    /// refuses to invent one. A future cycle adding a declared purpose changes the
    /// producer, not the card shape.
    pub purpose: Option<String>,
    /// Projection node ids involved, so a consumer can go deeper on demand.
    pub graph_refs: Vec<String>,
    /// Knowledge status for this unit.
    pub knowledge_status: CardKnowledgeStatus,
    /// Provenance for the card as a whole.
    pub provenance: CardProvenance,
}

/// Build a card for one unit.
///
/// Read-only over the overlay. Returns `None` when the unit is not in the
/// projection, because reporting a card for a unit that is not there would be a
/// fabricated answer.
pub fn card_for_unit(
    unit: &SoftwareUnit,
    overlay: &ArchitectureGraphOverlay,
    contracts: &[ArchitecturalContract],
    basis: &KnowledgeBasis,
    expected: Option<&KnowledgeBasis>,
    now: crate::knowledge::EventTime,
) -> Option<SoftwareUnitCard> {
    let unit_ref = unit.id.as_str();
    let projection = overlay.projection();
    let nodes = projection.nodes();

    // The unit's own node, by locator (units are keyed on their id).
    let unit_node = nodes.iter().find(|n| n.locator == unit_ref)?;
    let unit_node_id = unit_node.id.as_str().to_string();

    // Declared relations touching the unit, in canonical order.
    let mut dependencies: Vec<CardDependency> = Vec::new();
    for rel in projection.relations() {
        let from = rel.from.as_str().to_string();
        let to = rel.to.as_str().to_string();
        if from != unit_node_id && to != unit_node_id {
            continue;
        }
        let direction = if from == unit_node_id {
            "outbound"
        } else {
            "inbound"
        };
        let counterpart = if from == unit_node_id { to } else { from };
        let counterpart_locator = nodes
            .iter()
            .find(|n| n.id.as_str() == counterpart)
            .map(|n| n.locator.clone())
            .unwrap_or(counterpart);
        dependencies.push(CardDependency {
            counterpart: counterpart_locator,
            relation: format!("{:?}", rel.kind),
            direction: direction.to_string(),
        });
    }
    dependencies.sort();
    dependencies.dedup();

    // Contracts constraining the unit, and their declared intent.
    let mut relevant_contracts: Vec<String> = Vec::new();
    for anchor in overlay.find_contracts_for_unit(&unit.id) {
        if let Some(n) = nodes.iter().find(|n| n.id == anchor)
            && let Some(cid) = n.props_inline.get("contract_id")
        {
            relevant_contracts.push(cid.clone());
        }
    }
    relevant_contracts.sort();
    relevant_contracts.dedup();

    let mut relevant_decisions: Vec<String> = Vec::new();
    let mut relevant_specs: Vec<String> = Vec::new();
    for cid in &relevant_contracts {
        if let Some(c) = contracts.iter().find(|c| c.id().as_str() == cid) {
            relevant_decisions.push(c.decided_by().render());
            relevant_specs.push(c.specified_by().render());
        }
    }
    relevant_decisions.sort();
    relevant_decisions.dedup();
    relevant_specs.sort();
    relevant_specs.dedup();

    // Owners: `unique_owner` contracts naming this unit.
    let mut owned_by: Vec<String> = contracts
        .iter()
        .filter(|c| match c.payload() {
            crate::architectural_contract::ContractPayload::UniqueOwner(e) => {
                e.as_str() == unit_ref
            }
            _ => false,
        })
        .map(|c| c.id().as_str().to_string())
        .collect();
    owned_by.sort();
    owned_by.dedup();

    // Evidence attached to this unit's relations (provider + reference).
    let mut evidence_refs: Vec<String> = Vec::new();
    for rel in projection.relations() {
        if rel.from.as_str() == unit_node_id || rel.to.as_str() == unit_node_id {
            for e in &rel.evidence {
                evidence_refs.push(format!("{e:?}"));
            }
        }
    }
    evidence_refs.sort();
    evidence_refs.dedup();

    // Graph refs: the unit node plus the anchors and neighbours reached.
    let mut graph_refs: Vec<String> = vec![unit_node_id.clone()];
    graph_refs.extend(dependencies.iter().map(|d| d.counterpart.clone()));
    graph_refs.sort();
    graph_refs.dedup();

    // Knowledge status, honestly scoped to the basis.
    let unit_assertions = basis
        .assertions()
        .keys()
        .filter(|k| k.as_str() == unit_ref)
        .count();
    // Freshness comes from the real KMT, and only when an expected basis is
    // supplied. No expected basis means NOT EVALUATED, which is reported as
    // `None` rather than defaulted to fresh.
    let freshness = expected.map(|exp| match KMT::evaluate(basis, exp, now) {
        KmtStatus::Fresh { .. } => "fresh".to_string(),
        KmtStatus::Stale { .. } => "stale".to_string(),
        KmtStatus::Invalidated { reason, .. } => format!("invalidated:{}", reason.canonical_tag()),
        KmtStatus::Unknown { reason, .. } => format!("unknown:{reason:?}"),
    });

    Some(SoftwareUnitCard {
        identity: unit_ref.to_string(),
        kind: format!("{:?}", unit.kind).to_lowercase(),
        locator: unit.locator.clone(),
        module_path: unit.module_path.join("::"),
        purpose: None,
        owned_by,
        dependencies,
        relevant_contracts,
        relevant_decisions,
        relevant_specs,
        evidence_refs,
        graph_refs,
        knowledge_status: CardKnowledgeStatus {
            basis_hash: basis.basis_hash().to_hex(),
            has_assertion_for_unit: unit_assertions > 0,
            freshness,
        },
        provenance: CardProvenance::Declared,
    })
}
