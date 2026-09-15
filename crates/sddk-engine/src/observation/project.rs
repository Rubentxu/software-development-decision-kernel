//! Project observations into the **one** `SemanticGraphProjection` (REQ-A4S0-014, 015).
//!
//! Extension-namespaced kinds, deterministic order, rebuildable. No second graph,
//! no second DB, nothing written by read paths.
//!
//! The edge this emits is the leg A3 recorded as missing:
//!
//! ```text
//! Evidence --OBSERVES--> SoftwareRelation
//! ```

use crate::semantic_graph::SemanticGraphProjection;
use crate::semantic_kind::{NodeKind, SemanticKindError};
use crate::semantic_node::{NodeId, SemanticNode};

use super::types::{ObservationSet, ObservationSubject};

/// Namespaced node kind for an observation.
pub const OBSERVATION_NODE_KIND: &str = "a4_node_software_observation";
/// Namespaced node kind for a software relation.
pub const RELATION_NODE_KIND: &str = "a4_node_software_relation";
/// Namespaced relation kind: observation → observed relation.
pub const OBSERVES_RELATION_KIND: &str = "a4_rel_observes";

/// Project every observation (and the relation it observes) into a projection.
///
/// Returns the observation node ids, in canonical order.
pub fn project_into<S: SemanticGraphProjection>(
    set: &ObservationSet,
    graph: &mut S,
) -> Result<Vec<NodeId>, SemanticKindError> {
    let observation_kind = NodeKind::parse(OBSERVATION_NODE_KIND)?;
    let relation_kind = NodeKind::parse(RELATION_NODE_KIND)?;
    let observes = crate::semantic_kind::RelationKind::parse(OBSERVES_RELATION_KIND)?;

    let mut ids = Vec::new();
    // `ObservationSet` is canonically ordered, so the projection is too.
    for observation in set.observations() {
        let locator = observation.id.as_str().to_string();
        let node_id = NodeId::new(&observation_kind, &locator);
        let mut node = SemanticNode::new(node_id.clone(), observation_kind.clone(), locator);
        node.props_inline
            .insert("subject".to_string(), observation.subject.canonical_tag());
        node.props_inline.insert(
            "origin".to_string(),
            observation.origin.canonical_tag().to_string(),
        );
        node.props_inline.insert(
            "stance".to_string(),
            observation.stance.canonical_tag().to_string(),
        );
        node.props_inline
            .insert("producer".to_string(), observation.producer.clone());
        node.props_inline.insert(
            "knowledge_basis".to_string(),
            observation.basis.knowledge_basis.to_hex(),
        );
        graph.add_node(node);

        // The observed relation becomes a node, and the observation points at it.
        if let ObservationSubject::SoftwareRelation(relation) = &observation.subject {
            let relation_id = relation.id();
            let relation_locator = relation_id.as_str().to_string();
            let relation_node_id = NodeId::new(&relation_kind, &relation_locator);
            let mut relation_node = SemanticNode::new(
                relation_node_id.clone(),
                relation_kind.clone(),
                relation_locator,
            );
            relation_node
                .props_inline
                .insert("from".to_string(), relation.from.canonical_tag());
            relation_node
                .props_inline
                .insert("kind".to_string(), relation.kind.domain_tag().to_string());
            relation_node
                .props_inline
                .insert("to".to_string(), relation.to.canonical_tag());
            graph.add_node(relation_node);

            graph.add_relation(crate::semantic_node::SemanticRelation::new(
                node_id.clone(),
                relation_node_id,
                observes.clone(),
            ));
        }
        ids.push(node_id);
    }
    Ok(ids)
}
