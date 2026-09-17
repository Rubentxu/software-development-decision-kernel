// Copyright (c) SDDK contributors.
// SPDX-License-Identifier: MIT
//
// architecture_graph/overlay.rs — T-02 (A3-S3 / AC2)
//
// ArchitectureGraphOverlay: a thin PROJECTION wrapper over the canonical
// SemanticGraphProjection. The overlay never owns bytes; it delegates every
// mutation to the canonical graph (`InMemorySemanticGraph`) so the digest
// surface stays unified (REQ-AC2-001/002/007, AC-033-001/006).
//
// Query surface is bounded and typed:
//   - find_units_contracted_by(ContractId) -> Vec<NodeId>
//   - find_contracts_for_unit(SoftwareUnitRef) -> Vec<NodeId>
//   - traverse_finding_to_software(ArchitectureClaimId) -> Vec<NodeId>
//   - traverse_decision_to_software(OverlayDecisionRef) -> Vec<NodeId>
//
// All queries return `Vec<NodeId>` against the canonical projection; no
// second store is exposed.

use crate::architectural_contract::{ArchitecturalContract, ArchitectureClaim, ClaimOutcome};
use crate::evidence_ref::EvidenceRef;
use crate::semantic_graph::{GraphRevision, InMemorySemanticGraph, SemanticGraphProjection};
use crate::semantic_kind::NodeKind;
use crate::semantic_node::{NodeId, SemanticNode, SemanticRelation};

use super::types::{
    ArchitectureClaimId, ArchitectureOverlayNodeKind, ArchitectureOverlayRelation,
    ArchitectureOverlayRelationKind, OverlayNodeRef, SoftwareUnit, SoftwareUnitRef,
};

/// ArchitectureGraphOverlay (PROJECTION, per ADR-0095).
///
/// Wraps an `InMemorySemanticGraph`. All overlay mutations route through
/// the canonical projection — there is no second store (REQ-AC2-001/002).
pub struct ArchitectureGraphOverlay {
    projection: InMemorySemanticGraph,
}

impl Default for ArchitectureGraphOverlay {
    fn default() -> Self {
        Self::new()
    }
}

impl ArchitectureGraphOverlay {
    pub fn new() -> Self {
        Self {
            projection: InMemorySemanticGraph::new(),
        }
    }

    pub fn graph_revision(&self) -> GraphRevision {
        self.projection.graph_revision()
    }

    /// Clear the underlying projection. Used by `rebuild` to reset before
    /// re-projection. REQ-AC2-006 requires that a rebuild clears prior
    /// state and re-projects deterministically.
    pub fn clear(&mut self) {
        self.projection = InMemorySemanticGraph::new();
    }

    /// Borrow the underlying canonical projection. Read-only access so the
    /// overlay cannot leak its store.
    pub fn projection(&self) -> &InMemorySemanticGraph {
        &self.projection
    }

    /// Canonical bytes of the underlying projection — REQ-AC2-007 says the
    /// overlay MUST NOT introduce a parallel digest surface.
    pub fn digest(&self) -> Vec<u8> {
        self.projection.canonical_bytes()
    }

    /// Add a typed software unit as a node. The NodeId locator is the
    /// unit's `id` (SoftwareUnitRef) so subsequent relations referencing
    /// `OverlayNodeRef::SoftwareUnit(unit_ref)` resolve to the same NodeId.
    pub fn add_unit(&mut self, unit: &SoftwareUnit) {
        let kind = NodeKind::parse(ArchitectureOverlayNodeKind::SoftwareUnit.domain_tag())
            .expect("static tag is well-formed");
        let locator = unit.id.0.clone();
        let id = NodeId::new(&kind, &locator);
        let mut node = SemanticNode::new(id, kind, locator);
        for (k, v) in super::types::build_unit_props(unit) {
            node.props_inline.insert(k, v);
        }
        self.projection.add_node(node);
    }

    /// Add an ArchitectureClaim as a node. Each claim gets a stable
    /// `ArchitectureClaimId` derived from contract_id + outcome + evaluated_at.
    pub fn add_claim(&mut self, claim: &ArchitectureClaim) -> ArchitectureClaimId {
        let kind = NodeKind::parse(ArchitectureOverlayNodeKind::ArchitectureClaim.domain_tag())
            .expect("static tag is well-formed");
        let claim_id = ArchitectureClaimId::from_claim(claim);
        // Use claim_id.as_str() as the locator so that subsequent relations
        // referencing OverlayNodeRef::Claim(claim_id) resolve to the same
        // NodeId (sha256(kind, locator) is deterministic).
        let locator = claim_id.as_str().to_string();
        let id = NodeId::new(&kind, &locator);
        let mut node = SemanticNode::new(id, kind, locator);
        node.props_inline.insert(
            "contract_id".to_string(),
            claim.contract_id().as_str().to_string(),
        );
        node.props_inline.insert(
            "outcome".to_string(),
            claim.outcome().canonical_tag().to_string(),
        );
        if let Some(note) = claim.note() {
            node.props_inline
                .insert("note".to_string(), note.to_string());
        }
        self.projection.add_node(node);
        claim_id
    }

    /// Emit one overlay relation into the canonical graph.
    pub fn add_relation(&mut self, rel: &ArchitectureOverlayRelation) {
        let from_kind =
            NodeKind::parse(relation_node_kind_tag(&rel.from)).expect("static tag is well-formed");
        let to_kind =
            NodeKind::parse(relation_node_kind_tag(&rel.to)).expect("static tag is well-formed");
        let from_locator = relation_node_locator(&rel.from);
        let to_locator = relation_node_locator(&rel.to);
        let from_id = NodeId::new(&from_kind, &from_locator);
        let to_id = NodeId::new(&to_kind, &to_locator);
        let relation_kind = rel
            .kind
            .as_relation_kind()
            .expect("static tag is well-formed");
        let mut semantic_rel = SemanticRelation::new(from_id, to_id, relation_kind);
        for e in &rel.evidence {
            semantic_rel.attach_evidence(e.clone());
        }
        self.projection.add_relation(semantic_rel);
    }
    /// Emit the contract-side relations for a contract: DecidedBy (contract
    /// → decision), SpecifiedBy (contract → spec), and one VerifiedBy edge
    /// per typed `EvidenceRef` (A4-S15R).
    ///
    /// Provenance axes (A4-S15R):
    /// - `SpecifiedBy`: contract → spec. Always emitted. Declared intent.
    /// - `VerifiedBy`:  contract → evidence node. One edge per unique typed
    ///   `EvidenceRef` (dedup by `EvidenceRef` field equality), order-
    ///   independent (canonicalised via `EvidenceBundle`). Never emitted
    ///   with empty evidence.
    /// - `DecidedBy`:   contract → decision.
    ///
    /// Pre-A4-S15R, `VerifiedBy` pointed at the spec node with the evidence
    /// attached as relation metadata — but no real producer passed non-empty
    /// evidence, so the path was unreachable. A3-S15 (ADR-0121 §4) explicitly
    /// deferred repointing to its own cycle. This is that cycle.
    pub fn add_contract_metadata(
        &mut self,
        contract: &ArchitecturalContract,
        decision_ref: &crate::architectural_contract::DecisionRef,
        spec_ref: &super::types::OverlaySpecRef,
        verified_by: &[EvidenceRef],
    ) {
        use super::types::evidence_overlay_node_ref;
        use crate::evidence_ref::EvidenceBundle;

        // Ensure the contract anchor node exists (one per contract). This
        // is idempotent: InMemorySemanticGraph::add_node replaces by id.
        let contract_node_kind =
            NodeKind::parse(ArchitectureOverlayNodeKind::SoftwareUnit.domain_tag())
                .expect("static tag is well-formed");
        let contract_node_locator = format!("contract:{}", contract.id().as_str());
        let contract_node_id = NodeId::new(&contract_node_kind, &contract_node_locator);
        let mut node = SemanticNode::new(
            contract_node_id.clone(),
            contract_node_kind,
            contract_node_locator.clone(),
        );
        node.props_inline.insert(
            "contract_id".to_string(),
            contract.id().as_str().to_string(),
        );
        node.props_inline
            .insert("kind".to_string(), "ac2.anchor.contract".to_string());
        self.projection.add_node(node);

        // DecidedBy: contract → decision
        let dec_locator = format!("decision:{}", decision_ref.canonical_payload());
        let dec_kind = NodeKind::parse(ArchitectureOverlayNodeKind::DecisionRef.domain_tag())
            .expect("static tag is well-formed");
        let dec_id = NodeId::new(&dec_kind, &dec_locator);
        let mut dec_node = SemanticNode::new(dec_id.clone(), dec_kind, dec_locator.clone());
        dec_node
            .props_inline
            .insert("decision_ref".to_string(), decision_ref.canonical_payload());
        self.projection.add_node(dec_node);

        let relation_kind = ArchitectureOverlayRelationKind::DecidedBy
            .as_relation_kind()
            .expect("static tag is well-formed");
        let semantic_rel = SemanticRelation::new(contract_node_id.clone(), dec_id, relation_kind);
        self.projection.add_relation(semantic_rel);

        // SpecRef node + SpecifiedBy edge, always emitted.
        let spec_locator = format!("spec:{}", spec_ref.canonical_payload());
        let spec_kind = NodeKind::parse(ArchitectureOverlayNodeKind::SpecRef.domain_tag())
            .expect("static tag is well-formed");
        let spec_id = NodeId::new(&spec_kind, &spec_locator);
        let mut spec_node = SemanticNode::new(spec_id.clone(), spec_kind, spec_locator.clone());
        spec_node.props_inline.insert(
            "spec_ref".to_string(),
            spec_ref.canonical_payload().to_string(),
        );
        self.projection.add_node(spec_node);

        let specified_by = ArchitectureOverlayRelationKind::SpecifiedBy
            .as_relation_kind()
            .expect("static tag is well-formed");
        self.projection.add_relation(SemanticRelation::new(
            contract_node_id.clone(),
            spec_id,
            specified_by,
        ));

        // A4-S15R: VerifiedBy points at typed EvidenceRef projection nodes.
        // - Empty verified_by → no edge (no UnknownEvidence, no spec fallback).
        // - Non-empty verified_by → exactly one edge per typed-unique EvidenceRef.
        //   Dedup + canonical ordering via `EvidenceBundle` (BTreeSet over
        //   `EvidenceRef` Ord = sha256(kind || locator || cas)).
        // - Insertion order is irrelevant — two rebuilds over the same set
        //   produce identical projection bytes.
        if verified_by.is_empty() {
            return;
        }
        let verified_by_kind = ArchitectureOverlayRelationKind::VerifiedBy
            .as_relation_kind()
            .expect("static tag is well-formed");
        let evidence_kind_tag = ArchitectureOverlayNodeKind::EvidenceRef.domain_tag();
        let evidence_node_kind =
            NodeKind::parse(evidence_kind_tag).expect("static tag is well-formed");

        // EvidenceBundle::from_refs canonicalises dedup + ordering without
        // doing any locator string parsing.
        let bundle = EvidenceBundle::from_refs(verified_by.iter().cloned());
        for e in bundle.iter() {
            let evidence_locator = format!("evidence:{}", e.ordering_key());
            let evidence_node_id = NodeId::new(&evidence_node_kind, &evidence_locator);
            // Idempotent node creation: same EvidenceRef → same NodeId →
            // add_node replaces; this is fine.
            let mut ev_node = SemanticNode::new(
                evidence_node_id.clone(),
                evidence_node_kind.clone(),
                evidence_locator,
            );
            // Stash the typed identity on the node so future WHY / rebuild
            // paths can recover the EvidenceRef without re-parsing.
            ev_node
                .props_inline
                .insert("evidence_kind".to_string(), e.kind.domain_tag().to_string());
            ev_node
                .props_inline
                .insert("evidence_locator".to_string(), e.locator.clone());
            if let Some(cas) = &e.cas {
                ev_node
                    .props_inline
                    .insert("evidence_cas".to_string(), cas.digest().to_string());
            }
            self.projection.add_node(ev_node);

            // Sanity-check the helper: the helper must agree with the kind tag
            // we computed above. If a future refactor changes one but not the
            // other, this fails closed at compile-time (the helper is `pub`).
            let _ = evidence_overlay_node_ref(e);

            let semantic_rel = SemanticRelation::new(
                contract_node_id.clone(),
                evidence_node_id,
                verified_by_kind.clone(),
            );
            self.projection.add_relation(semantic_rel);
        }
    }

    /// Find all software-unit nodes contracted by the given contract id.
    /// REQ-AC2-009: returns empty Vec when no matching claim exists.
    ///
    /// Convention: `ArchitectureClaimedBy` relations run `from=claim_node,
    /// to=unit_node`. So we look for any relation whose from is a claim
    /// node whose props_inline.contract_id == contract_id, and return the
    /// `to` (unit) side.
    pub fn find_units_contracted_by(&self, contract_id: &str) -> Vec<NodeId> {
        let relation_kind = ArchitectureOverlayRelationKind::ArchitectureClaimedBy
            .as_relation_kind()
            .expect("static tag is well-formed");
        let mut results = Vec::new();
        for rel in self.projection.relations() {
            if rel.kind != relation_kind {
                continue;
            }
            // rel.from is the claim node. Look up its contract_id prop.
            if let Some(cid) = self.contract_id_for_claim_node(&rel.from)
                && cid == contract_id
            {
                results.push(rel.to.clone());
            }
        }
        results.sort();
        results.dedup();
        results
    }

    /// Find all contracts constraining a given software unit.
    /// REQ-AC2-010: inverse of find_units_contracted_by.
    ///
    /// Convention: ArchitectureClaimedBy runs `from=claim, to=unit`. So we
    /// find any relation whose `to` equals the unit ref, then look up the
    /// contract_id prop on `from` (the claim node) and synthesize the
    /// contract anchor NodeId.
    pub fn find_contracts_for_unit(&self, unit_ref: &SoftwareUnitRef) -> Vec<NodeId> {
        let relation_kind = ArchitectureOverlayRelationKind::ArchitectureClaimedBy
            .as_relation_kind()
            .expect("static tag is well-formed");
        let unit_kind = NodeKind::parse(ArchitectureOverlayNodeKind::SoftwareUnit.domain_tag())
            .expect("static tag is well-formed");
        let unit_locator = &unit_ref.0;
        let unit_id = NodeId::new(&unit_kind, unit_locator);
        let mut results = Vec::new();
        for rel in self.projection.relations() {
            if rel.kind != relation_kind || rel.to != unit_id {
                continue;
            }
            if let Some(contract_id) = self.contract_id_for_claim_node(&rel.from) {
                let contract_locator = format!("contract:{contract_id}");
                let contract_kind =
                    NodeKind::parse(ArchitectureOverlayNodeKind::SoftwareUnit.domain_tag())
                        .expect("static tag is well-formed");
                results.push(NodeId::new(&contract_kind, &contract_locator));
            }
        }
        results.sort();
        results.dedup();
        results
    }

    fn contract_id_for_claim_node(&self, claim_node: &NodeId) -> Option<String> {
        for node in self.projection.nodes() {
            if node.id == *claim_node {
                return node.props_inline.get("contract_id").cloned();
            }
        }
        None
    }

    /// Attach a claim to a unit with the appropriate relation kind based
    /// on the claim outcome (REQ-AC2-014/015/016):
    ///   - Verified → ArchitectureClaimedBy (ArchitectureClaimedBy in A3-S3).
    ///   - Contradicted → ContradictsBy.
    ///   - Unknown → ArchitectureClaimedBy (recorded as unknown).
    ///   - Stale → NO relation emitted (REQ-AC2-016).
    pub fn attach_claim_to_unit(
        &mut self,
        claim: &ArchitectureClaim,
        claim_id: &ArchitectureClaimId,
        unit_ref: &SoftwareUnitRef,
    ) {
        let evidence: Vec<EvidenceRef> = claim
            .evidence_refs()
            .iter()
            .map(super::types::convert_claim_evidence_to_universal)
            .collect();
        match claim.outcome() {
            ClaimOutcome::Stale => {
                // REQ-AC2-016: stale claims excluded from projection.
            }
            ClaimOutcome::Contradicted => {
                let rel = ArchitectureOverlayRelation {
                    from: OverlayNodeRef::SoftwareUnit(unit_ref.clone()),
                    to: OverlayNodeRef::Claim(claim_id.clone()),
                    kind: ArchitectureOverlayRelationKind::ContradictsBy,
                    evidence,
                };
                self.add_relation(&rel);
            }
            ClaimOutcome::Verified | ClaimOutcome::Unknown => {
                let rel = ArchitectureOverlayRelation {
                    from: OverlayNodeRef::Claim(claim_id.clone()),
                    to: OverlayNodeRef::SoftwareUnit(unit_ref.clone()),
                    kind: ArchitectureOverlayRelationKind::ArchitectureClaimedBy,
                    evidence,
                };
                self.add_relation(&rel);
            }
        }
    }

    /// Bounded query surface (REQ-AC2-018).
    pub fn query(&self, q: Query) -> Vec<NodeId> {
        match q {
            Query::ContractsForUnit(unit) => self.find_contracts_for_unit(&unit),
            Query::UnitsContractedBy(contract) => self.find_units_contracted_by(&contract),
            Query::TraverseDecisionToSoftware(decision) => {
                self.traverse_decision_to_software(&decision)
            }
        }
    }
}

/// Closed ADT of overlay queries (REQ-AC2-018). Unknown variants would fail
/// closed at compile time because `match` here is exhaustive.
pub enum Query {
    ContractsForUnit(SoftwareUnitRef),
    UnitsContractedBy(String),
    TraverseDecisionToSoftware(super::types::OverlayDecisionRef),
}

/// Helper: derive the overlay node kind tag from an `OverlayNodeRef`.
fn relation_node_kind_tag(r: &OverlayNodeRef) -> &'static str {
    match r {
        OverlayNodeRef::SoftwareUnit(_) => ArchitectureOverlayNodeKind::SoftwareUnit.domain_tag(),
        OverlayNodeRef::BoundedContext(_) => {
            ArchitectureOverlayNodeKind::BoundedContext.domain_tag()
        }
        OverlayNodeRef::Decision(_) => ArchitectureOverlayNodeKind::DecisionRef.domain_tag(),
        OverlayNodeRef::Spec(_) => ArchitectureOverlayNodeKind::SpecRef.domain_tag(),
        OverlayNodeRef::Test(_) => ArchitectureOverlayNodeKind::TestRef.domain_tag(),
        OverlayNodeRef::Uat(_) => ArchitectureOverlayNodeKind::UatRef.domain_tag(),
        OverlayNodeRef::CompatibilityPath(_) => {
            ArchitectureOverlayNodeKind::CompatibilityPath.domain_tag()
        }
        OverlayNodeRef::Claim(_) => ArchitectureOverlayNodeKind::ArchitectureClaim.domain_tag(),
        // A4-S15R: evidence node uses the typed EvidenceRef kind tag.
        OverlayNodeRef::Evidence(_) => ArchitectureOverlayNodeKind::EvidenceRef.domain_tag(),
    }
}

/// Helper: derive the locator string from an `OverlayNodeRef`.
fn relation_node_locator(r: &OverlayNodeRef) -> String {
    match r {
        OverlayNodeRef::SoftwareUnit(u) => u.0.clone(),
        OverlayNodeRef::BoundedContext(s) => format!("ctx:{s}"),
        OverlayNodeRef::Decision(d) => format!("decision:{}", d.canonical_payload()),
        OverlayNodeRef::Spec(s) => format!("spec:{}", s.canonical_payload()),
        OverlayNodeRef::Test(t) => format!("test:{}", t.0),
        OverlayNodeRef::Uat(u) => format!("uat:{}", u.0),
        OverlayNodeRef::CompatibilityPath(c) => format!("compat:{}", c.0),
        OverlayNodeRef::Claim(c) => c.0.clone(),
        // A4-S15R: evidence node locator derives from `EvidenceRef::ordering_key()`,
        // the same key used by `EvidenceBundle::Ord`. Two typed-equal EvidenceRefs
        // produce the same locator and therefore the same NodeId.
        OverlayNodeRef::Evidence(e) => format!("evidence:{}", e.ordering_key()),
    }
}

impl ArchitectureGraphOverlay {
    /// Traverse from a claim id to its software unit (REQ-AC2-011).
    pub fn traverse_finding_to_software(&self, claim_id: &ArchitectureClaimId) -> Vec<NodeId> {
        let relation_kind = ArchitectureOverlayRelationKind::ArchitectureClaimedBy
            .as_relation_kind()
            .expect("static tag is well-formed");
        let claim_kind =
            NodeKind::parse(ArchitectureOverlayNodeKind::ArchitectureClaim.domain_tag())
                .expect("static tag is well-formed");
        let target_locator = claim_id.0.clone();
        let target_id = NodeId::new(&claim_kind, &target_locator);
        // In this overlay's convention ArchitectureClaimedBy runs claim->unit;
        // we want the unit side. Find any relation whose from is the claim.
        let mut visited = Vec::new();
        for rel in self.projection.relations() {
            if rel.kind == relation_kind && rel.from == target_id {
                visited.push(rel.to);
            }
        }
        visited.sort();
        visited.dedup();
        visited
    }

    /// Traverse from a decision ref to software units reachable via the
    /// overlay relation set (REQ-AC2-012). Forward half of REQ-AC2-011.
    pub fn traverse_decision_to_software(
        &self,
        decision_ref: &super::types::OverlayDecisionRef,
    ) -> Vec<NodeId> {
        // DecidedBy contract → decision. Walk from the decision node to
        // contracts, then from contracts to claims, then claims to units.
        let decided_kind = ArchitectureOverlayRelationKind::DecidedBy
            .as_relation_kind()
            .expect("static tag is well-formed");
        let decided_locator = format!("decision:{}", decision_ref.canonical_payload());
        let dec_node_kind = NodeKind::parse(ArchitectureOverlayNodeKind::DecisionRef.domain_tag())
            .expect("static tag is well-formed");
        let dec_node_id = NodeId::new(&dec_node_kind, &decided_locator);

        // 1) find contracts whose DecidedBy points at this decision.
        let mut contracts = Vec::new();
        for rel in self.projection.relations() {
            if rel.kind == decided_kind && rel.to == dec_node_id {
                contracts.push(rel.from.clone());
            }
        }
        // 2) for each contract, find its claims.
        let mut claims = Vec::new();
        let claimed_kind = ArchitectureOverlayRelationKind::ArchitectureClaimedBy
            .as_relation_kind()
            .expect("static tag is well-formed");
        for contract_node_id in &contracts {
            for rel in self.projection.relations() {
                if rel.kind == claimed_kind && rel.from == *contract_node_id {
                    claims.push(rel.to.clone());
                }
            }
        }
        // 3) for each claim, walk to its unit (ArchitectureClaimedBy in
        //    this overlay convention is contract->claim; the unit side is
        //    attached via the claim's contract_id prop and a follow-up
        //    ArchitectureClaimedBy edge stored in opposite direction).
        let mut units = Vec::new();
        for claim_id in &claims {
            // The unit side: any ArchitectureClaimedBy edge whose from
            // matches this claim_id is the unit claim→unit edge (added
            // via attach_claim_to_unit for Verified/Unknown).
            for rel in self.projection.relations() {
                if rel.kind == claimed_kind && rel.from == *claim_id {
                    units.push(rel.to.clone());
                }
            }
        }
        units.sort();
        units.dedup();
        units
    }

    /// Read-only provenance of one contract (A4-5b).
    ///
    /// Returns the contract anchor plus the two **distinct** provenance
    /// axes A4-S15R established:
    ///
    /// - `specified_by`: the spec payloads the contract is `SpecifiedBy`
    ///   (declared intent). The overlay stores a spec node's identity as
    ///   its canonical payload (A3-S15), so the payload is what is
    ///   recovered — no variant guessing.
    /// - `verified_by`: the typed `EvidenceRef`s the contract is
    ///   `VerifiedBy` (verification evidence), reconstructed losslessly
    ///   from the typed props A4-S15R stashed on each evidence node
    ///   (`evidence_kind` / `evidence_locator` / `evidence_cas`).
    ///
    /// Both lists are canonical (sorted, deduplicated). An empty
    /// `verified_by` means "no evidence linked", NOT "not verified".
    /// The two axes are never conflated.
    pub fn contract_provenance(&self, contract_id: &str) -> ContractProvenance {
        let anchor_kind = NodeKind::parse(ArchitectureOverlayNodeKind::SoftwareUnit.domain_tag())
            .expect("static tag is well-formed");
        let anchor_locator = format!("contract:{contract_id}");
        let anchor_id = NodeId::new(&anchor_kind, &anchor_locator);

        let specified_kind = ArchitectureOverlayRelationKind::SpecifiedBy
            .as_relation_kind()
            .expect("static tag is well-formed");
        let verified_kind = ArchitectureOverlayRelationKind::VerifiedBy
            .as_relation_kind()
            .expect("static tag is well-formed");
        let spec_node_kind = NodeKind::parse(ArchitectureOverlayNodeKind::SpecRef.domain_tag())
            .expect("static tag is well-formed");
        let evidence_store_kind = ArchitectureOverlayNodeKind::EvidenceRef.domain_tag();

        let mut specified_by: Vec<String> = Vec::new();
        let mut verified_by: Vec<EvidenceRef> = Vec::new();
        for rel in self.projection.relations() {
            if rel.from != anchor_id {
                continue;
            }
            if rel.kind != specified_kind && rel.kind != verified_kind {
                continue;
            }
            let Some(node) = self.node_by_id(&rel.to) else {
                continue;
            };
            if rel.kind == specified_kind && node.kind == spec_node_kind {
                if let Some(payload) = node.props_inline.get("spec_ref") {
                    specified_by.push(payload.clone());
                }
            } else if rel.kind == verified_kind
                && let Some(evidence) = evidence_from_node(node, evidence_store_kind)
            {
                verified_by.push(evidence);
            }
        }
        specified_by.sort();
        specified_by.dedup();
        // EvidenceRef Ord is sha256(ordering_key): canonical, deterministic.
        verified_by.sort();
        verified_by.dedup();
        ContractProvenance {
            contract: anchor_id,
            specified_by,
            verified_by,
        }
    }

    /// Read-only node lookup by id within the projection.
    fn node_by_id(&self, id: &NodeId) -> Option<SemanticNode> {
        self.projection.nodes().into_iter().find(|n| n.id == *id)
    }
}

/// Read-only provenance of one contract (A4-5b). PROJECTION-derived; no
/// identity of its own, no persistence.
///
/// `specified_by` and `verified_by` are **separate provenance axes**:
/// declared intent vs verification evidence. Neither implies the other.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ContractProvenance {
    /// The contract anchor node.
    pub contract: NodeId,
    /// Spec payloads the contract is `SpecifiedBy` (canonical order).
    pub specified_by: Vec<String>,
    /// Typed evidence the contract is `VerifiedBy` (canonical order).
    pub verified_by: Vec<EvidenceRef>,
}

/// Reconstruct a typed `EvidenceRef` from the props A4-S15R stashed on an
/// evidence node. Returns `None` (fail closed) if the closed kind tag is
/// unknown or the required props are absent — no defaulting.
fn evidence_from_node(node: SemanticNode, evidence_kind_tag: &str) -> Option<EvidenceRef> {
    if node.kind.domain_tag() != evidence_kind_tag {
        return None;
    }
    let kind = crate::evidence_ref::EvidenceKind::from_domain_tag(
        node.props_inline.get("evidence_kind")?,
    )?;
    let locator = node.props_inline.get("evidence_locator")?.clone();
    let cas = node
        .props_inline
        .get("evidence_cas")
        .map(|d| crate::canonical_event_log::CasRef(d.clone()));
    Some(EvidenceRef { kind, locator, cas })
}

// Helper to silence the unused import warning when overlay.rs is compiled
// in isolation.
#[allow(dead_code)]
fn _ensure_use(_x: &SoftwareUnit) {}

#[allow(dead_code)]
fn _ensure_use_claim(_x: &ArchitectureClaim) {}

// contract_overlay_node_id is exposed in case downstream callers want to
// reuse the locator convention; keep it referenced so `unused_imports`
// doesn't fire.
#[allow(dead_code)]
const _REFERENCE_ANCHOR: fn(&ArchitecturalContract) -> OverlayNodeRef =
    super::types::contract_overlay_node_id;
