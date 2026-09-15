// Copyright (c) SDDK contributors.
// SPDX-License-Identifier: MIT
//
// paradigm_profile/overlay.rs — T-02 (A3-S4 / AC3)
//
// ParadigmProfileOverlay: a thin PROJECTION wrapper over the canonical
// SemanticGraphProjection (REQ-AC3-008, ADR-0098). The overlay never owns
// bytes; it delegates every mutation to the canonical graph
// (`InMemorySemanticGraph`) so the digest surface stays unified.
//
// AC3 is **data-only**: lens evaluation is intentionally excluded; this
// overlay declares paradigm attachments and lens selections, and carries
// assessments with `status == Unknown, basis == Declared`. Verdict logic
// belongs to AC7.
//
// Query surface (closed ADT, REQ-AC3-016):
//   - ProfilesForAnchor(ParadigmAnchorRef) -> Vec<ParadigmProfileKind>
//   - AnchorsForParadigm(ParadigmProfileKind) -> Vec<ParadigmAnchorRef>
//   - AssessmentsForAnchor(ParadigmAnchorRef) -> Vec<LensAssessment>
// All queries return deterministic, sorted results.

use crate::semantic_graph::{GraphRevision, InMemorySemanticGraph, SemanticGraphProjection};
use crate::semantic_kind::{NamespacedKind, NodeKind, RelationKind};
use crate::semantic_node::{NodeId, SemanticNode, SemanticRelation};

use super::types::{
    BoundedContextRef, LensAssessment, ParadigmAnchorKind, ParadigmAnchorRef, ParadigmLensEdge,
    ParadigmOverlayRelationKind, ParadigmProfileEdge, ParadigmProfileKind, ProjectIntentRef,
    SoftwareUnitRef, build_anchor_props, build_assessment_props, build_lens_edge_props,
    build_profile_edge_props,
};

/// ParadigmProfileOverlay (PROJECTION, per ADR-0095).
pub struct ParadigmProfileOverlay {
    projection: InMemorySemanticGraph,
}

impl Default for ParadigmProfileOverlay {
    fn default() -> Self {
        Self::new()
    }
}

impl ParadigmProfileOverlay {
    pub fn new() -> Self {
        Self {
            projection: InMemorySemanticGraph::new(),
        }
    }

    pub fn graph_revision(&self) -> GraphRevision {
        self.projection.graph_revision()
    }

    /// Clear the underlying projection. Used by `rebuild` to reset before
    /// re-projection. REQ-AC3-015 requires that a rebuild clears prior
    /// state and re-projects deterministically.
    pub fn clear(&mut self) {
        self.projection = InMemorySemanticGraph::new();
    }

    /// Borrow the underlying canonical projection (read-only).
    pub fn projection(&self) -> &InMemorySemanticGraph {
        &self.projection
    }

    /// Canonical bytes of the underlying projection — REQ-AC3-008 says the
    /// overlay MUST NOT introduce a parallel digest surface.
    pub fn digest(&self) -> Vec<u8> {
        self.projection.canonical_bytes()
    }

    /// Add a typed anchor as a node. Locator uses the namespaced prefix
    /// from `ParadigmAnchorRef::locator()`.
    pub fn add_anchor(&mut self, anchor: &ParadigmAnchorRef) {
        let kind = anchor.kind().as_node_kind();
        let locator = anchor.locator();
        let id = NodeId::new(&kind, &locator);
        let mut node = SemanticNode::new(id, kind, locator);
        for (k, v) in build_anchor_props(anchor) {
            node.props_inline.insert(k, v);
        }
        self.projection.add_node(node);
    }

    /// Add a `uses_paradigm` edge (anchor → paradigm profile, REQ-AC3-010).
    pub fn add_profile(&mut self, edge: &ParadigmProfileEdge) {
        self.add_anchor(&edge.anchor);
        let from_kind = edge.anchor.kind().as_node_kind();
        let from = NodeId::new(&from_kind, &edge.anchor.locator());
        // Profile node lives in its own extension kind.
        let profile_kind = NodeKind::Extension(
            NamespacedKind::new(edge.profile.domain_tag()).expect("static tag is well-formed"),
        );
        let profile_locator = format!("{}|{}", edge.anchor.locator(), edge.profile.domain_tag());
        let profile_id = NodeId::new(&profile_kind, &profile_locator);
        let mut profile_node = SemanticNode::new(profile_id.clone(), profile_kind, profile_locator);
        for (k, v) in build_profile_edge_props(edge) {
            profile_node.props_inline.insert(k, v);
        }
        self.projection.add_node(profile_node);
        let rel_kind = ParadigmOverlayRelationKind::UsesParadigm.as_relation_kind();
        let rel = SemanticRelation::new(from, profile_id, rel_kind);
        self.projection.add_relation(rel);
    }

    /// Add a lens-selection edge (anchor → lens, REQ-AC3-011).
    pub fn add_lens(&mut self, edge: &ParadigmLensEdge) {
        self.add_anchor(&edge.anchor);
        let from_kind = edge.anchor.kind().as_node_kind();
        let from = NodeId::new(&from_kind, &edge.anchor.locator());
        let lens_kind = NodeKind::Extension(
            NamespacedKind::new(edge.lens.domain_tag()).expect("static tag is well-formed"),
        );
        let lens_locator = format!("lens|{}|{}", edge.anchor.locator(), edge.lens.domain_tag());
        let lens_id = NodeId::new(&lens_kind, &lens_locator);
        let mut lens_node = SemanticNode::new(lens_id.clone(), lens_kind, lens_locator);
        for (k, v) in build_lens_edge_props(edge) {
            lens_node.props_inline.insert(k, v);
        }
        self.projection.add_node(lens_node);
        let rel_kind = ParadigmOverlayRelationKind::EvaluatedBy.as_relation_kind();
        let rel = SemanticRelation::new(from, lens_id, rel_kind);
        self.projection.add_relation(rel);
    }

    /// Add an assessment node attached to the anchor (REQ-AC3-012).
    pub fn add_assessment(&mut self, assessment: &LensAssessment) {
        self.add_anchor(&assessment.anchor);
        let from_kind = assessment.anchor.kind().as_node_kind();
        let from = NodeId::new(&from_kind, &assessment.anchor.locator());
        let assess_kind = NodeKind::Extension(
            NamespacedKind::new("ac3_assessment").expect("static tag is well-formed"),
        );
        let assess_locator = format!(
            "assess|{}|{}|{}",
            assessment.anchor.locator(),
            assessment.lens.domain_tag(),
            assessment.evaluated_at_ms
        );
        let assess_id = NodeId::new(&assess_kind, &assess_locator);
        let mut assess_node = SemanticNode::new(assess_id.clone(), assess_kind, assess_locator);
        for (k, v) in build_assessment_props(assessment) {
            assess_node.props_inline.insert(k, v);
        }
        self.projection.add_node(assess_node);
        let rel_kind = ParadigmOverlayRelationKind::LensTarget.as_relation_kind();
        let rel = SemanticRelation::new(from, assess_id, rel_kind);
        self.projection.add_relation(rel);
    }

    pub fn add_project(&mut self, project: &ProjectIntentRef) {
        self.add_anchor(&ParadigmAnchorRef::ProjectIntent(project.clone()));
    }

    pub fn add_bounded_context(&mut self, ctx: &BoundedContextRef) {
        self.add_anchor(&ParadigmAnchorRef::BoundedContext(ctx.clone()));
    }

    pub fn add_software_unit(&mut self, unit: &SoftwareUnitRef) {
        self.add_anchor(&ParadigmAnchorRef::SoftwareUnit(unit.clone()));
    }

    /// Find paradigm profiles attached to a given anchor (REQ-AC3-017).
    /// Returns kinds sorted by `domain_tag()`. Deterministic.
    pub fn find_profiles_for_anchor(&self, anchor: &ParadigmAnchorRef) -> Vec<ParadigmProfileKind> {
        let target_anchor_id = NodeId::new(&anchor.kind().as_node_kind(), &anchor.locator());
        let rel_tag = ParadigmOverlayRelationKind::UsesParadigm.domain_tag();
        let nodes = self.projection.nodes();
        let mut found = Vec::new();
        for rel in self.projection.relations() {
            if rel.from != target_anchor_id {
                continue;
            }
            // Match relation kind by `domain_tag()` (returns String).
            if rel.kind.domain_tag() != rel_tag {
                continue;
            }
            // Find profile kind by inspecting to-node kind.
            if let Some(profile) = nodes.iter().find(|n| n.id == rel.to).and_then(|n| {
                ParadigmProfileKind::ALL
                    .iter()
                    .find(|k| k.as_node_kind() == n.kind)
                    .copied()
            }) {
                found.push(profile);
            }
        }
        found.sort_by_key(|k| k.domain_tag().to_string());
        found.dedup();
        found
    }

    /// Find anchors that declare the given paradigm (REQ-AC3-018).
    pub fn find_anchors_for_paradigm(
        &self,
        paradigm: ParadigmProfileKind,
    ) -> Vec<ParadigmAnchorRef> {
        let target_kind = paradigm.as_node_kind();
        let rel_tag = ParadigmOverlayRelationKind::UsesParadigm.domain_tag();
        let nodes = self.projection.nodes();
        // Build a lookup table from NodeId -> kind string (String) so
        // rel.to.kind can be resolved without storing a kind on NodeId.
        let kind_of: std::collections::HashMap<NodeId, NodeKind> = nodes
            .iter()
            .map(|n| (n.id.clone(), n.kind.clone()))
            .collect();
        let mut found = Vec::new();
        for rel in self.projection.relations() {
            if rel.kind.domain_tag() != rel_tag {
                continue;
            }
            let to_kind = match kind_of.get(&rel.to) {
                Some(k) => k,
                None => continue,
            };
            if *to_kind != target_kind {
                continue;
            }
            // Find anchor kind by inspecting from-node kind.
            if let Some(node) = nodes.iter().find(|n| n.id == rel.from) {
                for ak in ParadigmAnchorKind::ALL {
                    if node.kind == ak.as_node_kind()
                        && let Some(anchor) = anchor_from_node(ak, &node.locator)
                    {
                        found.push(anchor);
                    }
                }
            }
        }
        found.sort_by_key(|a| a.locator());
        found.dedup();
        found
    }

    /// Find assessments for a given anchor (REQ-AC3-019).
    pub fn find_assessments_for_anchor(&self, anchor: &ParadigmAnchorRef) -> Vec<LensAssessment> {
        let target_anchor_id = NodeId::new(&anchor.kind().as_node_kind(), &anchor.locator());
        let rel_tag = ParadigmOverlayRelationKind::LensTarget.domain_tag();
        let nodes = self.projection.nodes();
        let mut found = Vec::new();
        for rel in self.projection.relations() {
            if rel.from != target_anchor_id {
                continue;
            }
            if rel.kind.domain_tag() != rel_tag {
                continue;
            }
            if let Some(node) = nodes.iter().find(|n| n.id == rel.to)
                && let Some(a) = assessment_from_node_props(&node.props_inline, anchor)
            {
                found.push(a);
            }
        }
        found.sort_by(|a, b| {
            (a.lens.domain_tag().to_string(), a.evaluated_at_ms)
                .cmp(&(b.lens.domain_tag().to_string(), b.evaluated_at_ms))
        });
        found
    }

    /// Closed ADT query entry point (REQ-AC3-016).
    pub fn query(&self, q: Query) -> QueryResult {
        match q {
            Query::ProfilesForAnchor(anchor) => {
                QueryResult::Profiles(self.find_profiles_for_anchor(&anchor))
            }
            Query::AnchorsForParadigm(kind) => {
                QueryResult::Anchors(self.find_anchors_for_paradigm(kind))
            }
            Query::AssessmentsForAnchor(anchor) => {
                QueryResult::Assessments(self.find_assessments_for_anchor(&anchor))
            }
        }
    }
}

/// Decode a stored node locator back into the typed `ParadigmAnchorRef`.
fn anchor_from_node(ak: ParadigmAnchorKind, locator: &str) -> Option<ParadigmAnchorRef> {
    match ak {
        ParadigmAnchorKind::ProjectIntent => {
            let inner = locator.strip_prefix("ac3_pi:")?.to_string();
            Some(ParadigmAnchorRef::ProjectIntent(ProjectIntentRef(inner)))
        }
        ParadigmAnchorKind::BoundedContext => {
            let inner = locator.strip_prefix("ac2_bc:")?.to_string();
            Some(ParadigmAnchorRef::BoundedContext(BoundedContextRef(inner)))
        }
        ParadigmAnchorKind::SoftwareUnit => {
            let inner = locator.strip_prefix("ac2_su:")?.to_string();
            Some(ParadigmAnchorRef::SoftwareUnit(SoftwareUnitRef(inner)))
        }
    }
}

fn assessment_from_node_props(
    props: &std::collections::BTreeMap<String, String>,
    anchor: &ParadigmAnchorRef,
) -> Option<LensAssessment> {
    let lens_tag = props.get("ac3_lens_kind")?;
    let status_tag = props.get("ac3_status")?;
    let basis_tag = props.get("ac3_basis")?;
    let evaluated_at: i64 = props.get("ac3_evaluated_at_ms")?.parse().ok()?;
    let lens = lens_kind_from_tag(lens_tag)?;
    let status = status_from_tag(status_tag)?;
    let basis = basis_from_tag(basis_tag)?;
    Some(LensAssessment {
        lens,
        anchor: anchor.clone(),
        status,
        basis,
        evidence_refs: Vec::new(),
        notes: None,
        evaluated_at_ms: evaluated_at,
    })
}

fn lens_kind_from_tag(tag: &str) -> Option<super::types::ParadigmLensKind> {
    super::types::ParadigmLensKind::ALL
        .iter()
        .find(|k| k.domain_tag() == tag)
        .copied()
}

fn status_from_tag(tag: &str) -> Option<super::types::LensStatus> {
    super::types::LensStatus::ALL
        .iter()
        .find(|s| s.domain_tag() == tag)
        .copied()
}

fn basis_from_tag(tag: &str) -> Option<super::types::EvidenceBasis> {
    super::types::EvidenceBasis::ALL
        .iter()
        .find(|b| b.domain_tag() == tag)
        .copied()
}

// Re-expose RelationKind so call sites don't need a separate import.
#[allow(unused_imports)]
use RelationKind as _;

/// Closed query ADT (REQ-AC3-016). Three variants.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Query {
    ProfilesForAnchor(ParadigmAnchorRef),
    AnchorsForParadigm(ParadigmProfileKind),
    AssessmentsForAnchor(ParadigmAnchorRef),
}

/// Query result variants (mirror the Query ADT 1-to-1).
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum QueryResult {
    Profiles(Vec<ParadigmProfileKind>),
    Anchors(Vec<ParadigmAnchorRef>),
    Assessments(Vec<LensAssessment>),
}
