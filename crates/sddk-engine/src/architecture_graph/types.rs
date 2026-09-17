// Copyright (c) SDDK contributors.
// SPDX-License-Identifier: MIT
//
// architecture_graph/types.rs — T-01 (A3-S3 / AC2)
//
// Typed overlay node/relation kinds and SoftwareUnit inputs for the
// ArchitectureGraphOverlay. The overlay projects ArchitecturalContract
// (A3-S2 OBJECT) + ArchitectureClaim (A3-S2 PROJECTION) + typed
// SoftwareUnit/DecisionRef/SpecRef/TestRef/UatRef inputs into the one
// canonical SemanticGraphProjection (arch-spec-005).
//
// State classes (per ADR-0095):
// - ArchitectureGraphOverlay — PROJECTION (delegates to canonical graph)
// - SoftwareUnitRef, DecisionRef, SpecRef, TestRef, UatRef,
//   CompatibilityPathRef — typed newtypes (no persistence)
// - ArchitectureOverlayNodeKind, ArchitectureOverlayRelationKind — closed enums
//
// All overlay kinds use the `ac2.` namespace prefix to avoid growing
// CoreNodeKind / CoreRelationKind in this cycle.

use crate::architectural_contract::{
    ArchitecturalContract, ArchitectureClaim, DecisionRef, SpecRef,
};
use crate::evidence_ref::EvidenceRef;
use crate::semantic_kind::{NamespacedKind, NodeKind, RelationKind, SemanticKindError};
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;

/// Stable typed reference to a software unit (module / crate / function /
/// endpoint / service / schema / table). The overlay never owns bytes for
/// units; the locator is what matters for query semantics.
#[derive(Clone, Debug, PartialEq, Eq, Hash, Serialize, Deserialize, PartialOrd, Ord)]
pub struct SoftwareUnitRef(pub String);

impl SoftwareUnitRef {
    pub fn new(locator: impl Into<String>) -> Self {
        Self(locator.into())
    }
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

/// Closed enum of unit kinds the overlay understands. Each maps to a
/// stable domain_tag in the `ac2.unit.*` namespace.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Serialize, Deserialize, PartialOrd, Ord)]
pub enum UnitKind {
    Module,
    Crate,
    Function,
    Endpoint,
    Service,
    Schema,
    Table,
}

impl UnitKind {
    pub const ALL: [UnitKind; 7] = [
        UnitKind::Module,
        UnitKind::Crate,
        UnitKind::Function,
        UnitKind::Endpoint,
        UnitKind::Service,
        UnitKind::Schema,
        UnitKind::Table,
    ];

    pub fn domain_tag(&self) -> &'static str {
        match self {
            UnitKind::Module => "ac2_unit_module",
            UnitKind::Crate => "ac2_unit_crate",
            UnitKind::Function => "ac2_unit_function",
            UnitKind::Endpoint => "ac2_unit_endpoint",
            UnitKind::Service => "ac2_unit_service",
            UnitKind::Schema => "ac2_unit_schema",
            UnitKind::Table => "ac2_unit_table",
        }
    }
}

/// A software unit the overlay can reason about. PROJECTION input only —
/// the overlay never writes units to durable storage.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct SoftwareUnit {
    pub id: SoftwareUnitRef,
    pub locator: String,
    pub kind: UnitKind,
    pub module_path: Vec<String>,
}

impl SoftwareUnit {
    pub fn new(id: SoftwareUnitRef, kind: UnitKind, locator: impl Into<String>) -> Self {
        Self {
            id,
            locator: locator.into(),
            kind,
            module_path: Vec::new(),
        }
    }

    pub fn with_module_path(mut self, path: Vec<String>) -> Self {
        self.module_path = path;
        self
    }
}

/// Typed reference to a Decision Memory entry. Re-exported from A3-S2 substrate
/// for ergonomic access; the overlay never owns a Decision — it borrows one.
pub type OverlayDecisionRef = DecisionRef;

/// Typed reference to a spec. Re-exported from A3-S2 substrate.
pub type OverlaySpecRef = SpecRef;

/// Typed reference to a test entry (UAT scenario, regression test, etc.).
/// Overlay never owns tests.
#[derive(Clone, Debug, PartialEq, Eq, Hash, Serialize, Deserialize, PartialOrd, Ord)]
pub struct TestRef(pub String);

impl TestRef {
    pub fn new(s: impl Into<String>) -> Self {
        Self(s.into())
    }
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

/// Typed reference to a UAT scenario.
#[derive(Clone, Debug, PartialEq, Eq, Hash, Serialize, Deserialize, PartialOrd, Ord)]
pub struct UatRef(pub String);

impl UatRef {
    pub fn new(s: impl Into<String>) -> Self {
        Self(s.into())
    }
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

/// Typed reference to a compatibility path (deprecated -> replacement pair).
#[derive(Clone, Debug, PartialEq, Eq, Hash, Serialize, Deserialize, PartialOrd, Ord)]
pub struct CompatibilityPathRef(pub String);

impl CompatibilityPathRef {
    pub fn new(s: impl Into<String>) -> Self {
        Self(s.into())
    }
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

/// Closed enum of overlay node kinds. Each carries a domain_tag in the
/// `ac2.node.*` namespace so callers parse via `NodeKind::Extension(...)`
/// without growing `CoreNodeKind` in this cycle.
///
/// `EvidenceRef` is the projection node for the typed `evidence_ref::EvidenceRef`
/// (A4-S15R): a `VerifiedBy` edge from a contract anchor points at one of these.
/// Evidence is a PROJECTION (rebuildable) — not a new authority, store, CAS, or
/// semantic graph. Identity is the `EvidenceRef` itself (kind + locator + cas).
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Serialize, Deserialize, PartialOrd, Ord)]
pub enum ArchitectureOverlayNodeKind {
    SoftwareUnit,
    BoundedContext,
    DecisionRef,
    SpecRef,
    TestRef,
    UatRef,
    CompatibilityPath,
    ArchitectureClaim,
    /// A4-S15R: projection of an `evidence_ref::EvidenceRef` into the overlay.
    /// The node's locator is derived from `EvidenceRef::ordering_key()` so two
    /// typed-equal EvidenceRefs produce the same NodeId.
    EvidenceRef,
}

impl ArchitectureOverlayNodeKind {
    pub const ALL: [ArchitectureOverlayNodeKind; 9] = [
        ArchitectureOverlayNodeKind::SoftwareUnit,
        ArchitectureOverlayNodeKind::BoundedContext,
        ArchitectureOverlayNodeKind::DecisionRef,
        ArchitectureOverlayNodeKind::SpecRef,
        ArchitectureOverlayNodeKind::TestRef,
        ArchitectureOverlayNodeKind::UatRef,
        ArchitectureOverlayNodeKind::CompatibilityPath,
        ArchitectureOverlayNodeKind::ArchitectureClaim,
        ArchitectureOverlayNodeKind::EvidenceRef,
    ];

    pub fn domain_tag(&self) -> &'static str {
        match self {
            ArchitectureOverlayNodeKind::SoftwareUnit => "ac2_node_software_unit",
            ArchitectureOverlayNodeKind::BoundedContext => "ac2_node_bounded_context",
            ArchitectureOverlayNodeKind::DecisionRef => "ac2_node_decision_ref",
            ArchitectureOverlayNodeKind::SpecRef => "ac2_node_spec_ref",
            ArchitectureOverlayNodeKind::TestRef => "ac2_node_test_ref",
            ArchitectureOverlayNodeKind::UatRef => "ac2_node_uat_ref",
            ArchitectureOverlayNodeKind::CompatibilityPath => "ac2_node_compatibility_path",
            ArchitectureOverlayNodeKind::ArchitectureClaim => "ac2_node_architecture_claim",
            ArchitectureOverlayNodeKind::EvidenceRef => "ac2_node_evidence_ref",
        }
    }

    /// Parse a tag into either a core variant (closed) or an extension
    /// namespaced kind.
    pub fn as_node_kind(&self) -> NodeKind {
        NodeKind::Extension(
            NamespacedKind::new(self.domain_tag()).expect("static tag is well-formed"),
        )
    }
}

/// Closed enum of overlay relation kinds. Each carries a domain_tag in the
/// `ac2.rel.*` namespace.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Serialize, Deserialize, PartialOrd, Ord)]
pub enum ArchitectureOverlayRelationKind {
    Owns,
    DependsOn,
    Writes,
    Reads,
    Emits,
    Consumes,
    Projects,
    DerivesFrom,
    Implements,
    DecidedBy,
    /// Contract anchor → spec node. Emitted unconditionally since A3-S15: the
    /// spec node always existed but had no stable edge, so `contract → spec`
    /// (the declared intent behind a contract) was unreachable by traversal.
    SpecifiedBy,
    VerifiedBy,
    ContradictsBy,
    SupersedesBy,
    /// ArchitectureClaim → ArchitecturalContract (1:N per claim).
    /// Distinct from `VerifiedBy` because the claim observes the contract
    /// but does not necessarily verify it (claim outcome may be Unknown).
    ArchitectureClaimedBy,
}

impl ArchitectureOverlayRelationKind {
    pub const ALL: [ArchitectureOverlayRelationKind; 15] = [
        ArchitectureOverlayRelationKind::Owns,
        ArchitectureOverlayRelationKind::DependsOn,
        ArchitectureOverlayRelationKind::Writes,
        ArchitectureOverlayRelationKind::Reads,
        ArchitectureOverlayRelationKind::Emits,
        ArchitectureOverlayRelationKind::Consumes,
        ArchitectureOverlayRelationKind::Projects,
        ArchitectureOverlayRelationKind::DerivesFrom,
        ArchitectureOverlayRelationKind::Implements,
        ArchitectureOverlayRelationKind::DecidedBy,
        ArchitectureOverlayRelationKind::SpecifiedBy,
        ArchitectureOverlayRelationKind::VerifiedBy,
        ArchitectureOverlayRelationKind::ContradictsBy,
        ArchitectureOverlayRelationKind::SupersedesBy,
        ArchitectureOverlayRelationKind::ArchitectureClaimedBy,
    ];

    pub fn domain_tag(&self) -> &'static str {
        match self {
            ArchitectureOverlayRelationKind::Owns => "ac2_rel_owns",
            ArchitectureOverlayRelationKind::DependsOn => "ac2_rel_depends_on",
            ArchitectureOverlayRelationKind::Writes => "ac2_rel_writes",
            ArchitectureOverlayRelationKind::Reads => "ac2_rel_reads",
            ArchitectureOverlayRelationKind::Emits => "ac2_rel_emits",
            ArchitectureOverlayRelationKind::Consumes => "ac2_rel_consumes",
            ArchitectureOverlayRelationKind::Projects => "ac2_rel_projects",
            ArchitectureOverlayRelationKind::DerivesFrom => "ac2_rel_derives_from",
            ArchitectureOverlayRelationKind::Implements => "ac2_rel_implements",
            ArchitectureOverlayRelationKind::DecidedBy => "ac2_rel_decided_by",
            ArchitectureOverlayRelationKind::SpecifiedBy => "ac2_rel_specified_by",
            ArchitectureOverlayRelationKind::VerifiedBy => "ac2_rel_verified_by",
            ArchitectureOverlayRelationKind::ContradictsBy => "ac2_rel_contradicts_by",
            ArchitectureOverlayRelationKind::SupersedesBy => "ac2_rel_supersedes_by",
            ArchitectureOverlayRelationKind::ArchitectureClaimedBy => {
                "ac2_rel_architecture_claimed_by"
            }
        }
    }

    pub fn as_relation_kind(&self) -> Result<RelationKind, SemanticKindError> {
        RelationKind::parse(self.domain_tag())
    }
}

/// Overlay relation input. Carries EvidenceRef attachment per REQ-AC2-017.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct ArchitectureOverlayRelation {
    pub from: OverlayNodeRef,
    pub to: OverlayNodeRef,
    pub kind: ArchitectureOverlayRelationKind,
    pub evidence: Vec<EvidenceRef>,
}

/// Typed reference to any overlay node. Variants are 1:1 with
/// `ArchitectureOverlayNodeKind`. The overlay never owns bytes for these —
/// these are pointers into the canonical graph nodes.
///
/// `Evidence(EvidenceRef)` (A4-S15R) is the typed pointer for the projection
/// of a universal `evidence_ref::EvidenceRef` into the overlay graph.
#[derive(Clone, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum OverlayNodeRef {
    SoftwareUnit(SoftwareUnitRef),
    BoundedContext(String),
    Decision(OverlayDecisionRef),
    Spec(OverlaySpecRef),
    Test(TestRef),
    Uat(UatRef),
    CompatibilityPath(CompatibilityPathRef),
    Claim(ArchitectureClaimId),
    /// A4-S15R: typed pointer to the projection of an `EvidenceRef`. Identity
    /// is the `EvidenceRef` itself (kind + locator + cas).
    Evidence(EvidenceRef),
}

/// A4-S15R: build the typed overlay node ref for an `EvidenceRef`. Used by
/// `add_contract_metadata` so each `VerifiedBy` edge points at the right
/// projection node.
pub fn evidence_overlay_node_ref(e: &EvidenceRef) -> OverlayNodeRef {
    OverlayNodeRef::Evidence(e.clone())
}

/// Typed identifier for an ArchitectureClaim projected into the overlay.
/// Distinct from `ArchitecturalContract` id because claims are PROJECTION
/// and may be rebuilt without changing identity.
#[derive(Clone, Debug, PartialEq, Eq, Hash, Serialize, Deserialize, PartialOrd, Ord)]
pub struct ArchitectureClaimId(pub String);

impl ArchitectureClaimId {
    pub fn new(s: impl Into<String>) -> Self {
        Self(s.into())
    }
    pub fn from_claim(claim: &ArchitectureClaim) -> Self {
        // Claim identity derives from contract_id + outcome + evaluated_at,
        // matching the A3-S2 substrate's basis-hash discipline.
        use sha2::{Digest, Sha256};
        let mut h = Sha256::new();
        h.update(b"sddk.architecture_graph.claim_id.v1|");
        h.update(claim.contract_id().as_str().as_bytes());
        h.update(b"|");
        h.update(claim.outcome().canonical_tag().as_bytes());
        h.update(b"|");
        h.update(claim.evaluated_at().0.to_be_bytes());
        let digest = h.finalize();
        let hex = digest
            .iter()
            .map(|b| format!("{:02x}", b))
            .collect::<String>();
        Self(format!("claim:{hex}"))
    }
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

/// Helper: derive the overlay node ref for a contract, used during rebuild.
pub fn contract_overlay_node_id(contract: &ArchitecturalContract) -> OverlayNodeRef {
    // ArchitecturalContract nodes project as `ac2.node.software_unit` with
    // the contract id as locator — the contract *is* the source unit that
    // constrains others. This keeps the overlay's relation graph navigable
    // without inventing a new node kind for contracts (they already live
    // as CoreNodeKind::ArchitecturalContract in the canonical graph).
    OverlayNodeRef::SoftwareUnit(SoftwareUnitRef::new(format!(
        "contract:{}",
        contract.id().as_str()
    )))
}

/// Helper: build a stable props map for an overlay node.
pub fn build_unit_props(unit: &SoftwareUnit) -> BTreeMap<String, String> {
    let mut props = BTreeMap::new();
    props.insert("locator".to_string(), unit.locator.clone());
    props.insert("kind".to_string(), unit.kind.domain_tag().to_string());
    props.insert("module_path".to_string(), unit.module_path.join("::"));
    props
}

/// Convert an A3-S2 contract substrate `claim::EvidenceRef` (provider+reference)
/// to the universal `evidence_ref::EvidenceRef` (EvidenceKind+locator+cas).
/// Defaults to `EvidenceKind::Adhoc`; the provider tag is preserved as the
/// locator so the conversion is lossless (provider is recoverable).
pub fn convert_claim_evidence_to_universal(
    e: &crate::architectural_contract::claim::EvidenceRef,
) -> EvidenceRef {
    EvidenceRef::new(
        crate::evidence_ref::EvidenceKind::Adhoc,
        format!("{}:{}", e.provider(), e.reference()),
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::semantic_kind::RelationKind;

    #[test]
    fn all_overlay_node_tags_unique() {
        let tags: Vec<&str> = ArchitectureOverlayNodeKind::ALL
            .iter()
            .map(|k| k.domain_tag())
            .collect();
        let unique: std::collections::BTreeSet<&str> = tags.iter().copied().collect();
        assert_eq!(
            unique.len(),
            tags.len(),
            "overlay node tags must be distinct"
        );
    }

    #[test]
    fn all_overlay_relation_tags_unique() {
        let tags: Vec<&str> = ArchitectureOverlayRelationKind::ALL
            .iter()
            .map(|k| k.domain_tag())
            .collect();
        let unique: std::collections::BTreeSet<&str> = tags.iter().copied().collect();
        assert_eq!(
            unique.len(),
            tags.len(),
            "overlay relation tags must be distinct"
        );
    }

    #[test]
    fn all_overlay_node_tags_round_trip_through_extension_parser() {
        // REQ-AC2-003: overlay kinds route through NodeKind::Extension.
        for kind in ArchitectureOverlayNodeKind::ALL {
            let parsed = NodeKind::parse(kind.domain_tag()).expect("valid namespaced kind");
            match parsed {
                NodeKind::Extension(n) => assert_eq!(n.as_str(), kind.domain_tag()),
                NodeKind::Core(_) => panic!("overlay tag must not parse as core"),
            }
        }
    }

    #[test]
    fn all_overlay_relation_tags_round_trip_through_extension_parser() {
        // REQ-AC2-004: overlay relations route through RelationKind::Extension.
        for kind in ArchitectureOverlayRelationKind::ALL {
            let parsed = RelationKind::parse(kind.domain_tag()).expect("valid namespaced kind");
            match parsed {
                RelationKind::Extension(n) => assert_eq!(n.as_str(), kind.domain_tag()),
                RelationKind::Core(_) => panic!("overlay tag must not parse as core"),
            }
        }
    }

    #[test]
    fn unit_kind_tags_are_distinct() {
        let tags: Vec<&str> = UnitKind::ALL.iter().map(|k| k.domain_tag()).collect();
        let unique: std::collections::BTreeSet<&str> = tags.iter().copied().collect();
        assert_eq!(unique.len(), tags.len(), "unit kind tags must be distinct");
    }
}
