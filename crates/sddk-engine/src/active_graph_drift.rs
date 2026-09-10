// Copyright (c) SDDK contributors.
// SPDX-License-Identifier: MIT
//
// active_graph_drift.rs — T-07 (M8.7)
//
// Cross-input drift detection over two `ActiveGraphProjection`s.
// Given two projections (typically --cycle-a vs --cycle-b, or
// --from-cycle vs --from-input), reports:
//
//   - added nodes: in B, not in A
//   - removed nodes: in A, not in B
//   - changed nodes: in both, but kind/label/recorded_at/provenance differ
//   - added / removed / changed edges (same triad, keyed by sort_key)
//
// Drift is a pure graph operation; this module has no I/O. The CLI
// (sddk dev cockpit diff) wires the manifest loaders and renders
// the report.

use crate::active_graph::{ActiveGraphEdge, ActiveGraphNode, ActiveGraphProjection};
use sddk_domain::workflow_ir::NodeId;
use serde::{Deserialize, Serialize};

/// Audit guard. Must match the number of `NodeDeltaKind` + `EdgeDeltaKind`
/// variants. Update both together.
pub const DRIFT_DELTA_KIND_COUNT: usize = 6;

/// Per-node delta kind.
#[non_exhaustive]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum NodeDeltaKind {
    Added,
    Removed,
    Changed,
}

/// Per-edge delta kind.
#[non_exhaustive]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum EdgeDeltaKind {
    Added,
    Removed,
    Changed,
}

impl NodeDeltaKind {
    /// Canonical textual label (used in JSON output and audit logs).
    #[must_use]
    pub const fn label(self) -> &'static str {
        match self {
            Self::Added => "added",
            Self::Removed => "removed",
            Self::Changed => "changed",
        }
    }
}

impl EdgeDeltaKind {
    /// Canonical textual label (used in JSON output and audit logs).
    #[must_use]
    pub const fn label(self) -> &'static str {
        match self {
            Self::Added => "added",
            Self::Removed => "removed",
            Self::Changed => "changed",
        }
    }
}

/// One node-side delta.
#[non_exhaustive]
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct NodeDelta {
    pub kind: NodeDeltaKind,
    pub node_id: NodeId,
    /// The node as it appears in A (`None` for `Added`).
    pub before: Option<ActiveGraphNode>,
    /// The node as it appears in B (`None` for `Removed`).
    pub after: Option<ActiveGraphNode>,
    /// Field-level diff (e.g. "label", "provenance", "kind"). Empty
    /// for Added / Removed.
    pub changed_fields: Vec<String>,
}

/// One edge-side delta.
#[non_exhaustive]
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct EdgeDelta {
    pub kind: EdgeDeltaKind,
    /// Canonical sort key for the edge.
    pub sort_key: (String, NodeId, NodeId),
    pub before: Option<ActiveGraphEdge>,
    pub after: Option<ActiveGraphEdge>,
    pub changed_fields: Vec<String>,
}

/// Drift summary counts. Always emitted in the report even when zero
/// for predictable JSON shape.
#[non_exhaustive]
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct DriftSummary {
    pub nodes_added: usize,
    pub nodes_removed: usize,
    pub nodes_changed: usize,
    pub edges_added: usize,
    pub edges_removed: usize,
    pub edges_changed: usize,
}

/// Full drift report. Stable JSON shape — fields are always present.
#[non_exhaustive]
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct DriftReport {
    pub node_deltas: Vec<NodeDelta>,
    pub edge_deltas: Vec<EdgeDelta>,
    pub summary: DriftSummary,
}

/// Trait for drift computation. The default implementation
/// (`DefaultDriftEngine`) is deterministic — same inputs ⇒ same
/// outputs — and stateless.
pub trait DriftEngine {
    fn diff(&self, a: &ActiveGraphProjection, b: &ActiveGraphProjection) -> DriftReport;
}

/// Default deterministic engine. Field-by-field comparison:
/// `kind`, `label`, `recorded_at`, `provenance` for nodes;
/// `provenance` for edges (kind + endpoints are the sort key).
pub struct DefaultDriftEngine;

impl DriftEngine for DefaultDriftEngine {
    fn diff(&self, a: &ActiveGraphProjection, b: &ActiveGraphProjection) -> DriftReport {
        diff_projections(a, b)
    }
}

/// Free-function variant, used directly by tests and any caller that
/// does not need trait-object dispatch.
#[must_use]
pub fn diff_projections(
    a: &ActiveGraphProjection,
    b: &ActiveGraphProjection,
) -> DriftReport {
    let mut node_deltas: Vec<NodeDelta> = Vec::new();
    let mut edge_deltas: Vec<EdgeDelta> = Vec::new();
    let mut summary = DriftSummary::default();

    // ── Nodes ────────────────────────────────────────────────────────
    // All NodeIds in A or B, sorted, deduplicated.
    let mut all_ids: Vec<&NodeId> = a.nodes.keys().chain(b.nodes.keys()).collect();
    all_ids.sort();
    all_ids.dedup();
    for id in all_ids {
        match (a.nodes.get(id), b.nodes.get(id)) {
            (None, Some(after)) => {
                summary.nodes_added += 1;
                node_deltas.push(NodeDelta {
                    kind: NodeDeltaKind::Added,
                    node_id: id.clone(),
                    before: None,
                    after: Some(after.clone()),
                    changed_fields: Vec::new(),
                });
            }
            (Some(before), None) => {
                summary.nodes_removed += 1;
                node_deltas.push(NodeDelta {
                    kind: NodeDeltaKind::Removed,
                    node_id: id.clone(),
                    before: Some(before.clone()),
                    after: None,
                    changed_fields: Vec::new(),
                });
            }
            (Some(before), Some(after)) => {
                let changed = diff_node_fields(before, after);
                if !changed.is_empty() {
                    summary.nodes_changed += 1;
                    node_deltas.push(NodeDelta {
                        kind: NodeDeltaKind::Changed,
                        node_id: id.clone(),
                        before: Some(before.clone()),
                        after: Some(after.clone()),
                        changed_fields: changed,
                    });
                }
            }
            (None, None) => unreachable!("dedup invariant"),
        }
    }

    // ── Edges ────────────────────────────────────────────────────────
    // Edge identity is the (kind, source, target) sort key.
    let mut all_edge_keys: Vec<(String, NodeId, NodeId)> = a
        .edges
        .iter()
        .map(edge_sort_key_str)
        .chain(b.edges.iter().map(edge_sort_key_str))
        .collect();
    all_edge_keys.sort();
    all_edge_keys.dedup();
    for key in &all_edge_keys {
        let a_edge = a
            .edges
            .iter()
            .find(|e| edge_sort_key_str(e) == *key)
            .cloned();
        let b_edge = b
            .edges
            .iter()
            .find(|e| edge_sort_key_str(e) == *key)
            .cloned();
        match (a_edge, b_edge) {
            (None, Some(after)) => {
                summary.edges_added += 1;
                edge_deltas.push(EdgeDelta {
                    kind: EdgeDeltaKind::Added,
                    sort_key: key.clone(),
                    before: None,
                    after: Some(after),
                    changed_fields: Vec::new(),
                });
            }
            (Some(before), None) => {
                summary.edges_removed += 1;
                edge_deltas.push(EdgeDelta {
                    kind: EdgeDeltaKind::Removed,
                    sort_key: key.clone(),
                    before: Some(before),
                    after: None,
                    changed_fields: Vec::new(),
                });
            }
            (Some(before), Some(after)) => {
                let changed = diff_edge_fields(&before, &after);
                if !changed.is_empty() {
                    summary.edges_changed += 1;
                    edge_deltas.push(EdgeDelta {
                        kind: EdgeDeltaKind::Changed,
                        sort_key: key.clone(),
                        before: Some(before),
                        after: Some(after),
                        changed_fields: changed,
                    });
                }
            }
            (None, None) => unreachable!("dedup invariant"),
        }
    }

    DriftReport {
        node_deltas,
        edge_deltas,
        summary,
    }
}

fn edge_sort_key_str(e: &ActiveGraphEdge) -> (String, NodeId, NodeId) {
    let k = e.sort_key();
    (format!("{:?}", k.0), k.1, k.2)
}

fn diff_node_fields(a: &ActiveGraphNode, b: &ActiveGraphNode) -> Vec<String> {
    let mut changed = Vec::new();
    if a.kind != b.kind {
        changed.push("kind".to_string());
    }
    if a.label != b.label {
        changed.push("label".to_string());
    }
    if a.recorded_at != b.recorded_at {
        changed.push("recorded_at".to_string());
    }
    if a.provenance != b.provenance {
        changed.push("provenance".to_string());
    }
    changed
}

fn diff_edge_fields(a: &ActiveGraphEdge, b: &ActiveGraphEdge) -> Vec<String> {
    let mut changed = Vec::new();
    if a.provenance != b.provenance {
        changed.push("provenance".to_string());
    }
    changed
}

// ── Tests ───────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;
    use crate::active_graph::{ProvenanceRef, ProvenanceSourceKind};
    use sddk_domain::workflow_ir::NodeId;

    fn nid(s: &str) -> NodeId {
        NodeId(s.to_string())
    }

    fn prov(loc: &str) -> ProvenanceRef {
        ProvenanceRef::new(ProvenanceSourceKind::CycleManifest, loc.to_string())
    }

    fn node(id: &str, label: &str, kind: ActiveGraphNodeKind) -> ActiveGraphNode {
        ActiveGraphNode {
            id: nid(id),
            kind,
            label: label.to_string(),
            recorded_at: "2026-09-10T00:00:00Z".to_string(),
            provenance: None,
        }
    }

    use crate::active_graph::ActiveGraphNodeKind;
    use crate::active_graph::ActiveGraphEdgeKind;
    use std::collections::BTreeMap;

    fn empty_projection() -> ActiveGraphProjection {
        ActiveGraphProjection::default()
    }

    fn proj_with_nodes(nodes: Vec<ActiveGraphNode>) -> ActiveGraphProjection {
        let mut p = empty_projection();
        let mut m: BTreeMap<NodeId, ActiveGraphNode> = BTreeMap::new();
        for n in nodes {
            m.insert(n.id.clone(), n);
        }
        p.nodes = m;
        p.node_count = p.nodes.len();
        p
    }

    #[test]
    fn identical_projections_yield_empty_report() {
        let p = proj_with_nodes(vec![node("m9_1", "M9.1", ActiveGraphNodeKind::Workflow)]);
        let report = diff_projections(&p, &p);
        assert!(report.node_deltas.is_empty());
        assert!(report.edge_deltas.is_empty());
        assert_eq!(report.summary.nodes_added, 0);
        assert_eq!(report.summary.nodes_removed, 0);
        assert_eq!(report.summary.nodes_changed, 0);
        assert_eq!(report.summary.edges_added, 0);
        assert_eq!(report.summary.edges_removed, 0);
        assert_eq!(report.summary.edges_changed, 0);
    }

    #[test]
    fn added_node_is_reported() {
        let a = proj_with_nodes(vec![node("m9_1", "M9.1", ActiveGraphNodeKind::Workflow)]);
        let b = proj_with_nodes(vec![
            node("m9_1", "M9.1", ActiveGraphNodeKind::Workflow),
            node("m8_7", "M8.7", ActiveGraphNodeKind::Workflow),
        ]);
        let report = diff_projections(&a, &b);
        assert_eq!(report.summary.nodes_added, 1);
        assert_eq!(report.summary.nodes_removed, 0);
        assert_eq!(report.summary.nodes_changed, 0);
        assert_eq!(report.node_deltas.len(), 1);
        assert_eq!(report.node_deltas[0].kind, NodeDeltaKind::Added);
        assert_eq!(report.node_deltas[0].node_id, nid("m8_7"));
        assert!(report.node_deltas[0].before.is_none());
    }

    #[test]
    fn removed_node_is_reported() {
        let a = proj_with_nodes(vec![
            node("m9_1", "M9.1", ActiveGraphNodeKind::Workflow),
            node("m9_2", "M9.2", ActiveGraphNodeKind::Workflow),
        ]);
        let b = proj_with_nodes(vec![node("m9_1", "M9.1", ActiveGraphNodeKind::Workflow)]);
        let report = diff_projections(&a, &b);
        assert_eq!(report.summary.nodes_added, 0);
        assert_eq!(report.summary.nodes_removed, 1);
        assert_eq!(report.summary.nodes_changed, 0);
        let d = report
            .node_deltas
            .iter()
            .find(|d| d.kind == NodeDeltaKind::Removed)
            .unwrap();
        assert_eq!(d.node_id, nid("m9_2"));
        assert!(d.after.is_none());
    }

    #[test]
    fn changed_node_attribution_lists_field_names() {
        let mut n1 = node("m8_7", "M8.7 old", ActiveGraphNodeKind::Workflow);
        let n2 = node("m8_7", "M8.7 new", ActiveGraphNodeKind::Workflow);
        n1.provenance = Some(prov("Commits:row_1"));
        let n2 = ActiveGraphNode { provenance: Some(prov("Commits:row_2")), ..n2 };
        let a = proj_with_nodes(vec![n1]);
        let b = proj_with_nodes(vec![n2]);
        let report = diff_projections(&a, &b);
        assert_eq!(report.summary.nodes_changed, 1);
        let d = &report.node_deltas[0];
        assert_eq!(d.kind, NodeDeltaKind::Changed);
        assert!(d.changed_fields.contains(&"label".to_string()));
        assert!(d.changed_fields.contains(&"provenance".to_string()));
        assert!(!d.changed_fields.contains(&"kind".to_string()));
        assert!(!d.changed_fields.contains(&"recorded_at".to_string()));
    }

    #[test]
    fn added_removed_edges_reported() {
        let mut a = empty_projection();
        a.edges.push(ActiveGraphEdge {
            kind: ActiveGraphEdgeKind::ParentOf,
            source: nid("m9_1"),
            target: nid("m9_2"),
            provenance: None,
        });
        let mut b = empty_projection();
        b.edges.push(ActiveGraphEdge {
            kind: ActiveGraphEdgeKind::DelegatedTo,
            source: nid("orchestrator"),
            target: nid("m8_7"),
            provenance: None,
        });
        let report = diff_projections(&a, &b);
        assert_eq!(report.summary.edges_added, 1);
        assert_eq!(report.summary.edges_removed, 1);
        assert_eq!(report.summary.edges_changed, 0);
        assert_eq!(report.edge_deltas.len(), 2);
    }

    #[test]
    fn drift_delta_kind_count_matches_variants() {
        // 3 NodeDeltaKind variants + 3 EdgeDeltaKind variants.
        assert_eq!(
            DRIFT_DELTA_KIND_COUNT,
            3 + 3,
            "DRIFT_DELTA_KIND_COUNT must equal NodeDeltaKind variant count + EdgeDeltaKind variant count"
        );
    }

    #[test]
    fn node_delta_kind_label_strings() {
        assert_eq!(NodeDeltaKind::Added.label(), "added");
        assert_eq!(NodeDeltaKind::Removed.label(), "removed");
        assert_eq!(NodeDeltaKind::Changed.label(), "changed");
        assert_eq!(EdgeDeltaKind::Added.label(), "added");
        assert_eq!(EdgeDeltaKind::Removed.label(), "removed");
        assert_eq!(EdgeDeltaKind::Changed.label(), "changed");
    }

    #[test]
    fn deterministic_output_across_calls() {
        let a = proj_with_nodes(vec![node("m9_1", "M9.1", ActiveGraphNodeKind::Workflow)]);
        let b = proj_with_nodes(vec![node("m8_7", "M8.7", ActiveGraphNodeKind::Workflow)]);
        let r1 = diff_projections(&a, &b);
        let r2 = diff_projections(&a, &b);
        assert_eq!(r1, r2);
    }

    #[test]
    fn default_engine_matches_free_function() {
        let a = proj_with_nodes(vec![node("m9_1", "M9.1", ActiveGraphNodeKind::Workflow)]);
        let b = proj_with_nodes(vec![
            node("m9_1", "M9.1 RENAMED", ActiveGraphNodeKind::Workflow),
            node("m8_7", "M8.7", ActiveGraphNodeKind::Workflow),
        ]);
        let engine = DefaultDriftEngine;
        let r1 = engine.diff(&a, &b);
        let r2 = diff_projections(&a, &b);
        assert_eq!(r1, r2);
    }
}
