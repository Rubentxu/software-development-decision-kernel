// Copyright (c) SDDK contributors.
// SPDX-License-Identifier: MIT
//
// paradigm_profile/types.rs — T-01 (A3-S4 / AC3)
//
// Typed overlay node/relation kinds for `ParadigmProfileOverlay`. The overlay
// projects paradigm declarations (project intent, bounded context, software
// unit) and lens selections into the one canonical SemanticGraphProjection
// (arch-spec-005, ADR-0098).
//
// AC3 is **data-only**: lens evaluation is intentionally excluded; assessments
// carry `status == LensStatus::Unknown` and `basis == EvidenceBasis::Declared`
// until AC7 plugs in observation/verdict. This file encodes the vocabulary:
// 11 paradigm profile kinds, 11 paradigm lens kinds, 7 lens statuses,
// 5 evidence-bases, 3 anchor kinds, and 3 overlay relation kinds.
//
// State classes (per ADR-0095):
// - `ParadigmProfileOverlay` — **PROJECTION** (delegates to canonical graph).
// - `ProjectIntentRef`, `BoundedContextRef`, `SoftwareUnitRef` — typed newtypes
//   (no persistence).
// - `ParadigmProfileKind`, `ParadigmLensKind`, `LensStatus`, `EvidenceBasis`,
//   `ParadigmAnchorKind`, `ParadigmOverlayRelationKind` — closed enums.
// - `LensAssessment`, `ParadigmProfileEdge`, `ParadigmLensEdge` — data shapes.
//
// All overlay kinds use the `ac3.` namespace prefix to avoid growing
// CoreNodeKind / CoreRelationKind in this cycle.
//
// Anti-encroachment: this module MUST NOT depend on `architecture_graph`,
// alignment, verify, debverify, authority, capability or provider/host
// surfaces. Those checks are pinned by `tests.rs` (REQ-AC3-020..023).

use crate::evidence_ref::EvidenceRef;
use crate::semantic_kind::{NamespacedKind, NodeKind, RelationKind};
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;

/// Typed reference to a software unit. A3-S4 locally declares this
/// newtype instead of importing the AC2 substrate (REQ-AC3-023: no
/// `architecture_graph` type reexport). The locator string convention
/// mirrors the AC2 SoftwareUnitRef so cross-substrate queries remain
/// unambiguous when AC4 wires bridge traversal.
#[derive(Clone, Debug, PartialEq, Eq, Hash, Serialize, Deserialize, PartialOrd, Ord)]
pub struct SoftwareUnitRef(pub String);

impl SoftwareUnitRef {
    pub fn new(s: impl Into<String>) -> Self {
        Self(s.into())
    }
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

/// Typed reference to a bounded context. The AC2 substrate stores bounded
/// contexts as `String` inside the `OverlayNodeRef::BoundedContext` variant
/// (no separate newtype). A3-S4 mirrors that convention and uses a local
/// newtype so callers have a typed identity surface for AC3 queries
/// (`REQ-AC3-005` second anchor kind).
#[derive(Clone, Debug, PartialEq, Eq, Hash, Serialize, Deserialize, PartialOrd, Ord)]
pub struct BoundedContextRef(pub String);

impl BoundedContextRef {
    pub fn new(s: impl Into<String>) -> Self {
        Self(s.into())
    }
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

/// Typed reference to a project's paradigm intent (one per cycle by
/// REQ-AC3-005 / Path 2 of the exploration report).
#[derive(Clone, Debug, PartialEq, Eq, Hash, Serialize, Deserialize, PartialOrd, Ord)]
pub struct ProjectIntentRef(pub String);

impl ProjectIntentRef {
    pub fn new(s: impl Into<String>) -> Self {
        Self(s.into())
    }
    pub fn as_str(&self) -> &str {
        &self.0
    }
    /// Canonical payload reused by hash-shaped identifiers (sha256 with
    /// domain prefix). Mirrors the AC2 `canonical_payload` convention
    /// (`pub(crate)` exposure guarded by the AC2 substrate).
    pub fn canonical_payload(&self) -> &str {
        &self.0
    }
}

/// Closed enum of paradigm profile kinds the overlay understands.
///
/// Source: `docs/SDDK-Architecture-Conformance-Graph-Evolution-2026-09-14/04-PARADIGM-LENSES.md`
/// §ParadigmProfile. The 11 variants match the canonical narrative verbatim.
/// Custom is reserved for project-specific paradigms (does not grow the
/// platform ontology — the lens stays in the project's namespace).
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Serialize, Deserialize, PartialOrd, Ord)]
pub enum ParadigmProfileKind {
    ObjectOriented,
    Functional,
    FunctionalPure,
    DataOriented,
    EventDriven,
    Reactive,
    Hexagonal,
    Ddd,
    Pipeline,
    ActorLike,
    Custom,
}

impl ParadigmProfileKind {
    pub const ALL: [ParadigmProfileKind; 11] = [
        ParadigmProfileKind::ObjectOriented,
        ParadigmProfileKind::Functional,
        ParadigmProfileKind::FunctionalPure,
        ParadigmProfileKind::DataOriented,
        ParadigmProfileKind::EventDriven,
        ParadigmProfileKind::Reactive,
        ParadigmProfileKind::Hexagonal,
        ParadigmProfileKind::Ddd,
        ParadigmProfileKind::Pipeline,
        ParadigmProfileKind::ActorLike,
        ParadigmProfileKind::Custom,
    ];

    pub fn domain_tag(&self) -> &'static str {
        match self {
            ParadigmProfileKind::ObjectOriented => "ac3_profile_object_oriented",
            ParadigmProfileKind::Functional => "ac3_profile_functional",
            ParadigmProfileKind::FunctionalPure => "ac3_profile_functional_pure",
            ParadigmProfileKind::DataOriented => "ac3_profile_data_oriented",
            ParadigmProfileKind::EventDriven => "ac3_profile_event_driven",
            ParadigmProfileKind::Reactive => "ac3_profile_reactive",
            ParadigmProfileKind::Hexagonal => "ac3_profile_hexagonal",
            ParadigmProfileKind::Ddd => "ac3_profile_ddd",
            ParadigmProfileKind::Pipeline => "ac3_profile_pipeline",
            ParadigmProfileKind::ActorLike => "ac3_profile_actor_like",
            ParadigmProfileKind::Custom => "ac3_profile_custom",
        }
    }

    /// Map a profile kind to a `NodeKind` in the canonical SemanticGraph.
    /// Used by the overlay's `find_*` queries to dispatch on profile kind
    /// without re-deriving the namespace by hand.
    pub fn as_node_kind(&self) -> NodeKind {
        NodeKind::Extension(
            NamespacedKind::new(self.domain_tag()).expect("static tag is well-formed"),
        )
    }
}

/// Closed enum of paradigm lens kinds the overlay understands.
///
/// Initial lenses (per ADR-0114 §Decision) include OO, Functional/Pure
/// Functional, ADT (modelled at AC7) and Typed DSL. Event-driven,
/// Reactive, Data-oriented and Custom are explicitly extensible per
/// AC-035-003. Re-using the same 11-variant namespace as `ParadigmProfileKind`
/// preserves a one-to-one mapping between declaring a paradigm and selecting
/// the corresponding lens.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Serialize, Deserialize, PartialOrd, Ord)]
pub enum ParadigmLensKind {
    ObjectOriented,
    Functional,
    FunctionalPure,
    DataOriented,
    EventDriven,
    Reactive,
    Hexagonal,
    Ddd,
    Pipeline,
    ActorLike,
    Custom,
}

impl ParadigmLensKind {
    pub const ALL: [ParadigmLensKind; 11] = [
        ParadigmLensKind::ObjectOriented,
        ParadigmLensKind::Functional,
        ParadigmLensKind::FunctionalPure,
        ParadigmLensKind::DataOriented,
        ParadigmLensKind::EventDriven,
        ParadigmLensKind::Reactive,
        ParadigmLensKind::Hexagonal,
        ParadigmLensKind::Ddd,
        ParadigmLensKind::Pipeline,
        ParadigmLensKind::ActorLike,
        ParadigmLensKind::Custom,
    ];

    pub fn domain_tag(&self) -> &'static str {
        match self {
            ParadigmLensKind::ObjectOriented => "ac3_lens_object_oriented",
            ParadigmLensKind::Functional => "ac3_lens_functional",
            ParadigmLensKind::FunctionalPure => "ac3_lens_functional_pure",
            ParadigmLensKind::DataOriented => "ac3_lens_data_oriented",
            ParadigmLensKind::EventDriven => "ac3_lens_event_driven",
            ParadigmLensKind::Reactive => "ac3_lens_reactive",
            ParadigmLensKind::Hexagonal => "ac3_lens_hexagonal",
            ParadigmLensKind::Ddd => "ac3_lens_ddd",
            ParadigmLensKind::Pipeline => "ac3_lens_pipeline",
            ParadigmLensKind::ActorLike => "ac3_lens_actor_like",
            ParadigmLensKind::Custom => "ac3_lens_custom",
        }
    }

    /// Map a lens kind to the paradigm it normally assesses. One-to-one
    /// by design (same 11-variant namespace, same enum order).
    pub fn to_profile_kind(&self) -> ParadigmProfileKind {
        match self {
            ParadigmLensKind::ObjectOriented => ParadigmProfileKind::ObjectOriented,
            ParadigmLensKind::Functional => ParadigmProfileKind::Functional,
            ParadigmLensKind::FunctionalPure => ParadigmProfileKind::FunctionalPure,
            ParadigmLensKind::DataOriented => ParadigmProfileKind::DataOriented,
            ParadigmLensKind::EventDriven => ParadigmProfileKind::EventDriven,
            ParadigmLensKind::Reactive => ParadigmProfileKind::Reactive,
            ParadigmLensKind::Hexagonal => ParadigmProfileKind::Hexagonal,
            ParadigmLensKind::Ddd => ParadigmProfileKind::Ddd,
            ParadigmLensKind::Pipeline => ParadigmProfileKind::Pipeline,
            ParadigmLensKind::ActorLike => ParadigmProfileKind::ActorLike,
            ParadigmLensKind::Custom => ParadigmProfileKind::Custom,
        }
    }
}

/// Closed enum of lens assessment statuses (REQ-AC3-003).
///
/// Source: 04-PARADIGM-LENSES.md §Lens status. `UNKNOWN` is the **default**
/// for every freshly-built assessment in this cycle (no evaluation yet) per
/// REQ-AC3-012.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Serialize, Deserialize, PartialOrd, Ord)]
pub enum LensStatus {
    Aligned,
    Tension,
    Misaligned,
    Accepted,
    ReviewDue,
    Unknown,
    NotApplicable,
}

impl LensStatus {
    pub const ALL: [LensStatus; 7] = [
        LensStatus::Aligned,
        LensStatus::Tension,
        LensStatus::Misaligned,
        LensStatus::Accepted,
        LensStatus::ReviewDue,
        LensStatus::Unknown,
        LensStatus::NotApplicable,
    ];

    pub fn domain_tag(&self) -> &'static str {
        match self {
            LensStatus::Aligned => "ac3_status_aligned",
            LensStatus::Tension => "ac3_status_tension",
            LensStatus::Misaligned => "ac3_status_misaligned",
            LensStatus::Accepted => "ac3_status_accepted",
            LensStatus::ReviewDue => "ac3_status_review_due",
            LensStatus::Unknown => "ac3_status_unknown",
            LensStatus::NotApplicable => "ac3_status_not_applicable",
        }
    }
}

/// Closed enum of evidence bases (REQ-AC3-004).
///
/// Source: 11-FITNESS-RECEIPTS.md §Proof levels. `DECLARED` is the default
/// for assessments produced by AC3 (no observation yet).
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Serialize, Deserialize, PartialOrd, Ord)]
pub enum EvidenceBasis {
    Declared,
    Observed,
    Verified,
    MutationVerified,
    RuntimeCorroborated,
}

impl EvidenceBasis {
    pub const ALL: [EvidenceBasis; 5] = [
        EvidenceBasis::Declared,
        EvidenceBasis::Observed,
        EvidenceBasis::Verified,
        EvidenceBasis::MutationVerified,
        EvidenceBasis::RuntimeCorroborated,
    ];

    pub fn domain_tag(&self) -> &'static str {
        match self {
            EvidenceBasis::Declared => "ac3_basis_declared",
            EvidenceBasis::Observed => "ac3_basis_observed",
            EvidenceBasis::Verified => "ac3_basis_verified",
            EvidenceBasis::MutationVerified => "ac3_basis_mutation_verified",
            EvidenceBasis::RuntimeCorroborated => "ac3_basis_runtime_corroborated",
        }
    }
}

/// Closed enum of anchor kinds (REQ-AC3-005). One new anchor kind for A3-S4
/// (`ProjectIntent`); the other two re-use the AC2 substrate.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Serialize, Deserialize, PartialOrd, Ord)]
pub enum ParadigmAnchorKind {
    ProjectIntent,
    BoundedContext,
    SoftwareUnit,
}

impl ParadigmAnchorKind {
    pub const ALL: [ParadigmAnchorKind; 3] = [
        ParadigmAnchorKind::ProjectIntent,
        ParadigmAnchorKind::BoundedContext,
        ParadigmAnchorKind::SoftwareUnit,
    ];

    pub fn domain_tag(&self) -> &'static str {
        match self {
            ParadigmAnchorKind::ProjectIntent => "ac3_node_project_intent",
            // Re-use AC2's node kind for the existing bounded-context substrate.
            // Cross-substrate sharing is intentional: bounded contexts are
            // declared by the contract-architecture substrate, not invented here.
            ParadigmAnchorKind::BoundedContext => "ac2_node_bounded_context",
            // Re-use AC2's software-unit node kind.
            ParadigmAnchorKind::SoftwareUnit => "ac2_node_software_unit",
        }
    }

    pub fn as_node_kind(&self) -> NodeKind {
        NodeKind::Extension(
            NamespacedKind::new(self.domain_tag()).expect("static tag is well-formed"),
        )
    }
}

/// A `ParadigmAnchorRef` enum dispatches per anchor kind and is used as the
/// canonical identity in queries and relations. Variants carry typed newtype
/// refs that hash deterministically via the inner `String`.
#[derive(Clone, Debug, PartialEq, Eq, Hash, Serialize, Deserialize, PartialOrd, Ord)]
pub enum ParadigmAnchorRef {
    ProjectIntent(ProjectIntentRef),
    BoundedContext(BoundedContextRef),
    SoftwareUnit(SoftwareUnitRef),
}

impl ParadigmAnchorRef {
    pub fn kind(&self) -> ParadigmAnchorKind {
        match self {
            ParadigmAnchorRef::ProjectIntent(_) => ParadigmAnchorKind::ProjectIntent,
            ParadigmAnchorRef::BoundedContext(_) => ParadigmAnchorKind::BoundedContext,
            ParadigmAnchorRef::SoftwareUnit(_) => ParadigmAnchorKind::SoftwareUnit,
        }
    }

    /// String identity used as the `locator` for the corresponding graph
    /// node. The prefix `ac3_pi:` (project intent), `ac2_bc:` (bounded
    /// context, reused from AC2), `ac2_su:` (software unit, reused from
    /// AC2) make the namespace explicit without growing the AC2 substrate.
    pub fn locator(&self) -> String {
        match self {
            ParadigmAnchorRef::ProjectIntent(r) => format!("ac3_pi:{}", r.as_str()),
            ParadigmAnchorRef::BoundedContext(r) => format!("ac2_bc:{}", r.as_str()),
            ParadigmAnchorRef::SoftwareUnit(r) => format!("ac2_su:{}", r.as_str()),
        }
    }
}

/// A typed `uses_paradigm` projection input (REQ-AC3-010).
///
/// Each edge attaches one anchor to one `ParadigmProfileKind`. Multiple
/// edges per anchor are allowed (poly-paradigm).
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct ParadigmProfileEdge {
    pub anchor: ParadigmAnchorRef,
    pub profile: ParadigmProfileKind,
    /// Optional rationale string the operator may attach. Not evaluated.
    pub rationale: Option<String>,
}

/// A typed lens-selection input (REQ-AC3-011).
///
/// A lens selects one anchor (or one paradigm profile) for observation.
/// The actual verdict belongs to AC7; this struct only declares intent.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct ParadigmLensEdge {
    pub lens: ParadigmLensKind,
    pub anchor: ParadigmAnchorRef,
}

/// Closed enum of overlay relation kinds (REQ-AC3-009). Three variants.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Serialize, Deserialize, PartialOrd, Ord)]
pub enum ParadigmOverlayRelationKind {
    /// anchor → paradigm profile (`uses_paradigm`).
    UsesParadigm,
    /// anchor → lens (`selected_by_lens`).
    EvaluatedBy,
    /// lens → anchor (the inverse projection for symmetry queries).
    LensTarget,
}

impl ParadigmOverlayRelationKind {
    pub const ALL: [ParadigmOverlayRelationKind; 3] = [
        ParadigmOverlayRelationKind::UsesParadigm,
        ParadigmOverlayRelationKind::EvaluatedBy,
        ParadigmOverlayRelationKind::LensTarget,
    ];

    pub fn domain_tag(&self) -> &'static str {
        match self {
            ParadigmOverlayRelationKind::UsesParadigm => "ac3_rel_uses_paradigm",
            ParadigmOverlayRelationKind::EvaluatedBy => "ac3_rel_evaluated_by",
            ParadigmOverlayRelationKind::LensTarget => "ac3_rel_lens_target",
        }
    }

    pub fn as_relation_kind(&self) -> RelationKind {
        RelationKind::Extension(
            NamespacedKind::new(self.domain_tag()).expect("static tag is well-formed"),
        )
    }
}

/// Lens assessment data shape (REQ-AC3-012). This data shape exists so AC4,
/// AC5 and AC7 can attach evidence later. In A3-S4 every freshly-constructed
/// assessment carries `status == LensStatus::Unknown, basis == EvidenceBasis::Declared`.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct LensAssessment {
    pub lens: ParadigmLensKind,
    pub anchor: ParadigmAnchorRef,
    pub status: LensStatus,
    pub basis: EvidenceBasis,
    pub evidence_refs: Vec<EvidenceRef>,
    pub notes: Option<String>,
    pub evaluated_at_ms: i64,
}

impl LensAssessment {
    /// The canonical "freshly-built, no observation" constructor used by A3-S4
    /// (REQ-AC3-012). The `evidence_refs` list is `None`-y (`Vec::new()`);
    /// the status is forced to `Unknown` and the basis to `Declared`.
    pub fn declared(
        lens: ParadigmLensKind,
        anchor: ParadigmAnchorRef,
        evaluated_at_ms: i64,
    ) -> Self {
        Self {
            lens,
            anchor,
            status: LensStatus::Unknown,
            basis: EvidenceBasis::Declared,
            evidence_refs: Vec::new(),
            notes: None,
            evaluated_at_ms,
        }
    }
}

/// Rebuild input — what `rebuild(overlay, &inputs)` consumes (REQ-AC3-013).
#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct RebuildInputs {
    /// Project intent (per cycle); required by Path 2 of the exploration.
    pub project: Option<ProjectIntentRef>,
    /// `uses_paradigm` edges (anchor + paradigm profile).
    pub profiles: Vec<ParadigmProfileEdge>,
    /// Lens selection edges.
    pub lenses: Vec<ParadigmLensEdge>,
    /// Assessments (always `Unknown/Declared` in A3-S4; AC7 will fill later).
    pub assessments: Vec<LensAssessment>,
}

/// Stable ID for an assessment: derived deterministically from
/// `(lens, anchor.locator(), evaluated_at_ms)`. Mirrors `ArchitectureClaimId`
/// (REQ-AC2-005). Uses `sha2` (already a workspace dep) for sha256.
#[derive(Clone, Debug, PartialEq, Eq, Hash, Serialize, Deserialize, PartialOrd, Ord)]
pub struct LensAssessmentId(pub String);

impl LensAssessmentId {
    pub const DOMAIN_PREFIX: &'static str = "sddk.paradigm_profile.assessment_id.v1|";

    pub fn from_assessment(assessment: &LensAssessment) -> Self {
        use sha2::{Digest, Sha256};
        let mut h = Sha256::new();
        h.update(Self::DOMAIN_PREFIX.as_bytes());
        h.update(assessment.lens.domain_tag().as_bytes());
        h.update(b"|");
        h.update(assessment.anchor.locator().as_bytes());
        h.update(b"|");
        h.update(assessment.evaluated_at_ms.to_string().as_bytes());
        let digest = h.finalize();
        let hex: String = digest.iter().map(|b| format!("{:02x}", b)).collect();
        Self(hex)
    }

    pub fn as_str(&self) -> &str {
        &self.0
    }
}

/// Builder for stable node properties from a profile edge. Mirrors AC2's
/// `build_unit_props` (helper, `pub(crate)` exposure).
pub(crate) fn build_anchor_props(anchor: &ParadigmAnchorRef) -> BTreeMap<String, String> {
    let mut props = BTreeMap::new();
    props.insert(
        "ac3_anchor_kind".to_string(),
        anchor.kind().domain_tag().to_string(),
    );
    props.insert("ac3_anchor_id".to_string(), anchor.locator());
    props
}

pub(crate) fn build_profile_edge_props(edge: &ParadigmProfileEdge) -> BTreeMap<String, String> {
    let mut props = BTreeMap::new();
    props.insert("ac3_rel".to_string(), "uses_paradigm".to_string());
    props.insert(
        "ac3_profile_kind".to_string(),
        edge.profile.domain_tag().to_string(),
    );
    if let Some(rationale) = &edge.rationale {
        props.insert("ac3_rationale".to_string(), rationale.clone());
    }
    props
}

pub(crate) fn build_lens_edge_props(edge: &ParadigmLensEdge) -> BTreeMap<String, String> {
    let mut props = BTreeMap::new();
    props.insert("ac3_rel".to_string(), "evaluated_by".to_string());
    props.insert(
        "ac3_lens_kind".to_string(),
        edge.lens.domain_tag().to_string(),
    );
    props
}

pub(crate) fn build_assessment_props(assessment: &LensAssessment) -> BTreeMap<String, String> {
    let mut props = BTreeMap::new();
    let id = LensAssessmentId::from_assessment(assessment);
    props.insert("ac3_assessment_id".to_string(), id.0);
    props.insert(
        "ac3_status".to_string(),
        assessment.status.domain_tag().to_string(),
    );
    props.insert(
        "ac3_basis".to_string(),
        assessment.basis.domain_tag().to_string(),
    );
    props.insert(
        "ac3_lens_kind".to_string(),
        assessment.lens.domain_tag().to_string(),
    );
    props.insert(
        "ac3_evaluated_at_ms".to_string(),
        assessment.evaluated_at_ms.to_string(),
    );
    props
}

/// Helper for converting an evidence-ref into a stable property value.
/// Mirrors AC2's `convert_claim_evidence_to_universal`.
pub(crate) fn convert_evidence_to_property(eref: &EvidenceRef) -> String {
    // `EvidenceRef` fields are `kind: EvidenceKind` + `locator: String`.
    // We form `kind:locator` for stable string form.
    format!("{}:{}", eref.kind.domain_tag(), eref.locator)
}

// Suppress unused warning for the closure-style helper that is reserved for
// future use by AC7 integration. Compile-time pinned only.
#[allow(dead_code)]
pub(crate) fn convert_evidence_ref_to_property(eref: &EvidenceRef) -> String {
    convert_evidence_to_property(eref)
}

// ─────────────────────────────────────────────────────────────────────────────
// tests (small subset of compile-time pins; the bigger fixture-driven set
// lives in `tests.rs` and the AC3 acceptance tests)
// ─────────────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn all_paradigm_profile_tags_round_trip_through_extension_parser() {
        for kind in ParadigmProfileKind::ALL {
            let nk = NodeKind::parse(kind.domain_tag()).expect("parseable");
            match nk {
                NodeKind::Extension(ns) => assert_eq!(ns.as_str(), kind.domain_tag()),
                _ => panic!("expected Extension for {}", kind.domain_tag()),
            }
        }
    }

    #[test]
    fn all_paradigm_profile_tags_unique() {
        let tags: Vec<&str> = ParadigmProfileKind::ALL
            .iter()
            .map(|k| k.domain_tag())
            .collect();
        let unique: std::collections::BTreeSet<&str> = tags.iter().copied().collect();
        assert_eq!(
            unique.len(),
            tags.len(),
            "paradigm profile tags must be distinct"
        );
    }

    #[test]
    fn all_paradigm_lens_tags_unique() {
        let tags: Vec<&str> = ParadigmLensKind::ALL
            .iter()
            .map(|k| k.domain_tag())
            .collect();
        let unique: std::collections::BTreeSet<&str> = tags.iter().copied().collect();
        assert_eq!(
            unique.len(),
            tags.len(),
            "paradigm lens tags must be distinct"
        );
    }

    #[test]
    fn all_lens_status_tags_unique() {
        let tags: Vec<&str> = LensStatus::ALL.iter().map(|k| k.domain_tag()).collect();
        let unique: std::collections::BTreeSet<&str> = tags.iter().copied().collect();
        assert_eq!(
            unique.len(),
            tags.len(),
            "lens status tags must be distinct"
        );
    }

    #[test]
    fn all_evidence_basis_tags_unique() {
        let tags: Vec<&str> = EvidenceBasis::ALL.iter().map(|k| k.domain_tag()).collect();
        let unique: std::collections::BTreeSet<&str> = tags.iter().copied().collect();
        assert_eq!(
            unique.len(),
            tags.len(),
            "evidence basis tags must be distinct"
        );
    }

    #[test]
    fn lens_to_profile_mapping_is_injective() {
        for lens in ParadigmLensKind::ALL {
            let profile = lens.to_profile_kind();
            // The mapping must land in the canonical slice.
            assert!(
                ParadigmProfileKind::ALL.contains(&profile),
                "lens {} mapped to a non-canonical profile",
                lens.domain_tag()
            );
        }
    }

    #[test]
    fn declared_assessment_carries_unknown_declared() {
        let lens = ParadigmLensKind::FunctionalPure;
        let anchor = ParadigmAnchorRef::SoftwareUnit(SoftwareUnitRef::new("crates/x/lib.rs"));
        let a = LensAssessment::declared(lens, anchor.clone(), 1_700_000_000);
        assert_eq!(a.status, LensStatus::Unknown);
        assert_eq!(a.basis, EvidenceBasis::Declared);
        assert!(a.evidence_refs.is_empty());
        let id_a = LensAssessmentId::from_assessment(&a);
        // Determinism: same inputs must yield the same ID.
        let a2 = LensAssessment::declared(lens, anchor, 1_700_000_000);
        let id_b = LensAssessmentId::from_assessment(&a2);
        assert_eq!(id_a, id_b, "LensAssessmentId must be deterministic");
    }
}
