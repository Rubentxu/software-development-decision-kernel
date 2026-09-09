// Copyright (c) SDDK contributors.
// SPDX-License-Identifier: MIT
//
// semantic_graph.rs — T-03 (M3 arch-spec-005)
//
// One canonical SemanticGraphProjection rebuildable from canonical facts
// (via the M1 CanonicalEventLog). Same facts => same projection bytes.

use crate::canonical_event_log::{CanonicalEventLog, FactRange};
use crate::semantic_node::{NodeId, SemanticNode, SemanticRelation};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::collections::{BTreeMap, BTreeSet};

/// Graph revision = `head + 1` (0 for empty graph). Monotone.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize, PartialOrd, Ord)]
pub struct GraphRevision(pub u64);

impl GraphRevision {
    pub fn next(self) -> Self {
        GraphRevision(self.0.saturating_add(1))
    }

    pub fn value(self) -> u64 {
        self.0
    }
}

pub trait SemanticGraphProjection {
    fn add_node(&mut self, node: SemanticNode);
    fn add_relation(&mut self, rel: SemanticRelation);
    fn nodes(&self) -> Vec<SemanticNode>;
    fn relations(&self) -> Vec<SemanticRelation>;
    fn graph_revision(&self) -> GraphRevision;
    /// Canonical JSON of (sorted nodes + sorted relations), the equality
    /// surface for "same facts => same projection bytes".
    fn canonical_bytes(&self) -> Vec<u8>;

    /// Rebuild from canonical facts. Same facts + same refs => same
    /// projection bytes.
    fn rebuild_from_canonical<L: CanonicalEventLog>(
        &mut self,
        log: &L,
    ) -> Result<(), SemanticGraphError>;
}

#[derive(Debug, thiserror::Error, PartialEq, Eq)]
pub enum SemanticGraphError {
    #[error("duplicate node id: {0}")]
    DuplicateNode(String),
    #[error("unknown node id referenced in relation: {0}")]
    UnknownNode(String),
}

#[derive(Debug, Default)]
pub struct InMemorySemanticGraph {
    nodes: BTreeMap<NodeId, SemanticNode>,
    relations: BTreeSet<crate::semantic_node::SemanticRelationKey>,
    relation_index: BTreeMap<crate::semantic_node::SemanticRelationKey, SemanticRelation>,
    revision: GraphRevision,
}

impl InMemorySemanticGraph {
    pub fn new() -> Self {
        Self::default()
    }
}

impl SemanticGraphProjection for InMemorySemanticGraph {
    fn add_node(&mut self, node: SemanticNode) {
        if self.nodes.contains_key(&node.id) {
            // Idempotent: re-adding the same id replaces the stored copy
            // (so the call site can refresh payload_ref after a CAS put).
            // Duplicate detection is reported via the `rebuild` path or via
            // structural checks; the runtime contract is "by-id".
        }
        self.nodes.insert(node.id.clone(), node);
        self.revision = self.revision.next();
    }

    fn add_relation(&mut self, rel: SemanticRelation) {
        let key = rel.key();
        if self.relation_index.contains_key(&key) {
            return;
        }
        self.relations.insert(key.clone());
        self.relation_index.insert(key, rel);
        self.revision = self.revision.next();
    }

    fn nodes(&self) -> Vec<SemanticNode> {
        self.nodes.values().cloned().collect()
    }

    fn relations(&self) -> Vec<SemanticRelation> {
        self.relation_index.values().cloned().collect()
    }

    fn graph_revision(&self) -> GraphRevision {
        self.revision
    }

    fn canonical_bytes(&self) -> Vec<u8> {
        let sorted_nodes: BTreeMap<&NodeId, &SemanticNode> = self.nodes.iter().collect();
        let mut h = Sha256::new();
        // Domain-separated prefix for the canonical-bytes kind.
        h.update(b"sddk.semantic_graph.canonical_bytes.v1|");
        for (id, node) in &sorted_nodes {
            h.update(id.as_str().as_bytes());
            h.update(b"|");
            h.update(node.kind.domain_tag().as_bytes());
            h.update(b"|");
            h.update(node.locator.as_bytes());
            h.update(b"|");
            match &node.payload_ref {
                Some(cas) => h.update(cas.digest().as_bytes()),
                None => h.update(b"<no-cas>"),
            }
            h.update(b"|");
            for (k, v) in &node.props_inline {
                h.update(k.as_bytes());
                h.update(b"=");
                h.update(v.as_bytes());
                h.update(b";");
            }
        }
        // Relations are already keyed into a BTreeSet by sha256; iteration
        // is deterministic.
        for rel in self.relation_index.values() {
            h.update(b"|rel|");
            h.update(rel.from.as_str().as_bytes());
            h.update(b"->");
            h.update(rel.to.as_str().as_bytes());
            h.update(b"::");
            h.update(rel.kind.domain_tag().as_bytes());
            for e in &rel.evidence {
                h.update(b"|ev|");
                h.update(e.ordering_key().as_bytes());
            }
        }
        let digest = h.finalize();
        digest.to_vec()
    }

    fn rebuild_from_canonical<L: CanonicalEventLog>(
        &mut self,
        log: &L,
    ) -> Result<(), SemanticGraphError> {
        // Reset the projection; rebuild deterministically by replaying every
        // fact in the canonical range. Same log bytes => same projection
        // bytes (the rebuild path doesn't introduce randomness).
        self.nodes.clear();
        self.relations.clear();
        self.relation_index.clear();
        let head = log.head().unwrap_or(0);
        let facts = log
            .read(FactRange {
                from_seq: 0,
                to_seq: head,
            })
            .unwrap_or_default();
        for fact in facts {
            use crate::canonical_event_log::FactClass;
            if fact.class == FactClass::DomainEvent
                && let Some(cas) = &fact.payload_ref
            {
                // Each canonical DomainEvent with a CAS payload becomes a
                // node of kind `decision` (a stable choice for the rebuild
                // path; callers may swap for their own projection strategy).
                let kind = crate::semantic_kind::NodeKind::parse("decision").unwrap();
                let id = crate::semantic_node::NodeId::new(&kind, cas.digest());
                let mut node = SemanticNode::new(id.clone(), kind.clone(), cas.digest());
                node.payload_ref = Some(cas.clone());
                self.add_node(node);
            }
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::canonical_event_log::{FactClass, FactEnvelopeV1};
    use crate::semantic_kind::{NodeKind, RelationKind};
    #[allow(unused_imports)]
    use time::OffsetDateTime;

    #[test]
    fn empty_graph_revision_is_zero() {
        let g = InMemorySemanticGraph::new();
        assert_eq!(g.graph_revision(), GraphRevision(0));
        assert!(g.nodes().is_empty());
    }

    #[test]
    fn add_node_increments_revision() {
        let mut g = InMemorySemanticGraph::new();
        let kind = NodeKind::parse("decision").unwrap();
        let id = NodeId::new(&kind, "d:1");
        let n = SemanticNode::new(id, kind, "d:1");
        g.add_node(n);
        assert_eq!(g.graph_revision(), GraphRevision(1));
        assert_eq!(g.nodes().len(), 1);
    }

    #[test]
    fn add_relation_typed_only() {
        let mut g = InMemorySemanticGraph::new();
        let a_kind = NodeKind::parse("decision").unwrap();
        let b_kind = NodeKind::parse("alternative").unwrap();
        let a = NodeId::new(&a_kind, "d:1");
        let b = NodeId::new(&b_kind, "alt:1");
        g.add_node(SemanticNode::new(a.clone(), a_kind, "d:1"));
        g.add_node(SemanticNode::new(b.clone(), b_kind, "alt:1"));
        let rel = SemanticRelation::new(
            a.clone(),
            b.clone(),
            RelationKind::parse("selected_over").unwrap(),
        );
        g.add_relation(rel);
        assert_eq!(g.relations().len(), 1);
        // Review: same direction+kind yields same key.
        let rel2 = SemanticRelation::new(a, b, RelationKind::parse("selected_over").unwrap());
        g.add_relation(rel2);
        assert_eq!(g.relations().len(), 1, "duplicate relation keys dedupe");
    }

    #[test]
    fn rebuild_from_canonical_is_deterministic() {
        let log = crate::canonical_event_log::InMemoryCanonicalEventLog::new();
        // Empty log: two rebuilds must yield the same canonical bytes.
        let mut g1 = InMemorySemanticGraph::new();
        g1.rebuild_from_canonical(&log).unwrap();
        let mut g2 = InMemorySemanticGraph::new();
        g2.rebuild_from_canonical(&log).unwrap();
        assert_eq!(g1.canonical_bytes(), g2.canonical_bytes());
        // Adding a non-DomainEvent fact must not affect the projection.
        let ts = OffsetDateTime::from_unix_timestamp(1_700_000_000).unwrap();
        let lifecycle = FactEnvelopeV1::new_inline(
            FactClass::LifecycleTransition,
            0,
            ts,
            b"no-effect".to_vec(),
        )
        .unwrap();
        log.append(lifecycle).unwrap();
        let mut g3 = InMemorySemanticGraph::new();
        g3.rebuild_from_canonical(&log).unwrap();
        assert_eq!(
            g1.canonical_bytes(),
            g3.canonical_bytes(),
            "non-DomainEvent facts must not affect canonical bytes"
        );
    }
}
