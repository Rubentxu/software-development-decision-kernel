// Copyright (c) SDDK contributors.
// SPDX-License-Identifier: MIT
//
// semantic_node.rs — T-02 (M3 arch-spec-005)
//
// Typed nodes and relations. Payloads > 4 KiB route through CasRef,
// mirroring the canonical_fact_log conventions.

use crate::canonical_event_log::CasRef;
use crate::evidence_ref::EvidenceRef;
use crate::semantic_kind::{NodeKind, RelationKind};
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;

/// Stable identifier for a node. Sha256(domain || kind || locator).
#[derive(Clone, Debug, PartialEq, Eq, Hash, Serialize, Deserialize, PartialOrd, Ord)]
pub struct NodeId(pub String);

impl NodeId {
    pub fn new(kind: &NodeKind, locator: &str) -> Self {
        use sha2::{Digest, Sha256};
        let mut h = Sha256::new();
        h.update(b"sddk.semantic_node.node_id.v1|");
        h.update(kind.domain_tag().as_bytes());
        h.update(b"|");
        h.update(locator.as_bytes());
        let digest = h.finalize();
        let hex = digest
            .iter()
            .map(|b| format!("{:02x}", b))
            .collect::<String>();
        NodeId(format!("node:{hex}"))
    }
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

/// A typed semantic node. Inline `props_inline` for small ones; CAS-routed
/// `payload_ref` for payloads > 4 KiB.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct SemanticNode {
    pub id: NodeId,
    pub kind: NodeKind,
    pub locator: String,
    pub payload_ref: Option<CasRef>,
    pub props_inline: BTreeMap<String, String>,
}

impl SemanticNode {
    pub fn new(id: NodeId, kind: NodeKind, locator: impl Into<String>) -> Self {
        Self {
            id,
            kind,
            locator: locator.into(),
            payload_ref: None,
            props_inline: BTreeMap::new(),
        }
    }

    /// True when the node routes its payload through CAS, not inline.
    pub fn is_cas_routed(&self) -> bool {
        self.payload_ref.is_some()
    }
}

#[derive(Clone, Debug, PartialEq, Eq, Hash, Serialize, Deserialize, PartialOrd, Ord)]
pub struct SemanticRelationKey(pub String);

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct SemanticRelation {
    pub from: NodeId,
    pub to: NodeId,
    pub kind: RelationKind,
    /// Evidence references (M1 universal `EvidenceRef`) supporting the relation.
    pub evidence: Vec<EvidenceRef>,
}

impl SemanticRelation {
    pub fn new(from: NodeId, to: NodeId, kind: RelationKind) -> Self {
        Self {
            from,
            to,
            kind,
            evidence: Vec::new(),
        }
    }

    /// Stable key derived from sha256(from || relKind || to).
    pub fn key(&self) -> SemanticRelationKey {
        use sha2::{Digest, Sha256};
        let mut h = Sha256::new();
        h.update(b"sddk.semantic_graph.relation_key.v1|");
        h.update(self.from.as_str().as_bytes());
        h.update(b"|");
        h.update(self.kind.domain_tag().as_bytes());
        h.update(b"|");
        h.update(self.to.as_str().as_bytes());
        let digest = h.finalize();
        let hex = digest
            .iter()
            .map(|b| format!("{:02x}", b))
            .collect::<String>();
        SemanticRelationKey(format!("rel:{hex}"))
    }

    pub fn attach_evidence(&mut self, e: EvidenceRef) {
        self.evidence.push(e);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::canonical_event_log::CasRef;

    #[test]
    fn node_id_is_deterministic() {
        let kind = NodeKind::parse("decision").unwrap();
        let id1 = NodeId::new(&kind, "decision:42");
        let id2 = NodeId::new(&kind, "decision:42");
        assert_eq!(id1, id2);
        let id3 = NodeId::new(&kind, "decision:43");
        assert_ne!(id1, id3);
    }

    #[test]
    fn semantic_node_records_payload_route() {
        let kind = NodeKind::parse("decision").unwrap();
        let id = NodeId::new(&kind, "decision:1");
        let mut node = SemanticNode::new(id.clone(), kind.clone(), "decision:1");
        assert!(!node.is_cas_routed());
        node.payload_ref = Some(CasRef::from_bytes(b"big-payload"));
        assert!(node.is_cas_routed());
        assert_eq!(node.id, id);
        assert_eq!(node.kind, kind);
    }

    #[test]
    fn semantic_relation_key_is_deterministic() {
        let a_kind = NodeKind::parse("decision").unwrap();
        let b_kind = NodeKind::parse("alternative").unwrap();
        let a = NodeId::new(&a_kind, "decision:1");
        let b = NodeId::new(&b_kind, "alternative:1");
        let rel_kind = RelationKind::parse("selected_over").unwrap();
        let r1 = SemanticRelation::new(a.clone(), b.clone(), rel_kind.clone());
        let r2 = SemanticRelation::new(a.clone(), b.clone(), rel_kind.clone());
        assert_eq!(r1.key(), r2.key());
    }
}
