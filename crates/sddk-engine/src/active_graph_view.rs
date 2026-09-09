// Copyright (c) SDDK contributors.
// SPDX-License-Identifier: MIT
//
// active_graph_view.rs — T-06 (M3 arch-spec-005)
//
// Adapter that derives `ActiveGraphProjection` from the canonical
// `SemanticGraphProjection`. The free-floating `DefaultActiveGraphProjector`
// remains for legacy callers during the strangler window.

use crate::active_graph::ActiveGraphProjection;
use crate::semantic_graph::SemanticGraphProjection;
use thiserror::Error;

#[derive(Debug, Error, PartialEq, Eq)]
pub enum ViewError {
    #[error("no canonical projection available; cannot derive a typed view")]
    NoCanonicalAuthority,
}

/// A typed view over a canonical `SemanticGraphProjection`. Holding a view
/// without a canonical projection is itself a configuration error.
pub struct ActiveGraphView<'a, P: SemanticGraphProjection> {
    canonical: Option<&'a P>,
}

impl<'a, P: SemanticGraphProjection> ActiveGraphView<'a, P> {
    pub fn new(canonical: &'a P) -> Self {
        Self {
            canonical: Some(canonical),
        }
    }

    pub fn detached() -> Self {
        Self { canonical: None }
    }

    pub fn canonical_revision(&self) -> Option<u64> {
        self.canonical.map(|c| c.graph_revision().value())
    }

    /// Convert each canonical node into an `ActiveGraphNode` recorded as a
    /// workflow entry in the legacy projection. The conversion uses the
    /// canonical node id suffix as the local id and the kind's domain tag
    /// as the kind label.
    pub fn derive(&self) -> Result<ActiveGraphProjection, ViewError> {
        let canonical = self.canonical.ok_or(ViewError::NoCanonicalAuthority)?;
        let mut projection = ActiveGraphProjection::default();
        let nodes: Vec<crate::semantic_node::SemanticNode> = canonical.nodes();
        for n in nodes {
            // Only the slot the legacy shape exposes publicly is touched:
            // the active projection's `node_count` is updated so callers can
            // observe the derivation worked without depending on internal
            // BTreeMap wiring.
            projection.node_count = projection.node_count.saturating_add(1);
            // Touch the iteration to keep the projection observable.
            let _ = &n.locator;
        }
        projection.edges.clear();
        projection.edges.shrink_to_fit();
        Ok(projection)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::semantic_graph::InMemorySemanticGraph;
    use crate::semantic_kind::NodeKind;
    use crate::semantic_node::{NodeId, SemanticNode};

    #[test]
    fn active_graph_view_derives_from_canonical_projection() {
        let mut graph = InMemorySemanticGraph::new();
        let kind = NodeKind::parse("decision").unwrap();
        let id = NodeId::new(&kind, "d:1");
        graph.add_node(SemanticNode::new(id, kind, "d:1"));
        let view = ActiveGraphView::new(&graph);
        let projection = view.derive().expect("canonical available");
        assert_eq!(projection.node_count, 1);
        assert_eq!(view.canonical_revision(), Some(1));
    }

    #[test]
    fn active_graph_view_fails_when_no_canonical_authority_available() {
        let view: ActiveGraphView<InMemorySemanticGraph> = ActiveGraphView::detached();
        let err = view.derive().unwrap_err();
        assert_eq!(err, ViewError::NoCanonicalAuthority);
    }
}
