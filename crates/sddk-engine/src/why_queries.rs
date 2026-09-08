//! Causal Why Queries on Active Graph
//!
//! Implements three verbs (`why`, `debt_why`, `decision_why`) on top of
//! [`ActiveGraphProjection`]. Returns plain-data result sets sorted
//! canonically. Closure is bounded to [`WHY_MAX_DEPTH`] hops.
//!
//! Pattern: P-a (struct + trait + plain-data evaluator). Determinism is
//! caller-controlled via `recorded_at`.

use crate::active_graph::{
    ActiveGraphEdge, ActiveGraphEdgeKind, ActiveGraphNode, ActiveGraphNodeKind,
    ActiveGraphProjection,
};
use sddk_domain::workflow_ir::NodeId;
use std::collections::{BTreeMap, BTreeSet, VecDeque};

// ── Audit guard constants ────────────────────────────────────────────────

/// Closed-set size of [`WhyQueryKind`]. Bump when a variant is added.
pub const WHY_QUERY_KIND_COUNT: usize = 3;

/// Maximum closure depth for `why` queries. Cycles beyond this depth
/// are truncated with `…(truncated)`.
pub const WHY_MAX_DEPTH: usize = 32;

// ── Kinds ────────────────────────────────────────────────────────────────

/// Closed-set taxonomy of why-query verbs.
#[non_exhaustive]
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum WhyQueryKind {
    /// Walk inbound edges (transitive closure).
    Why,
    /// Return gating edges only.
    DebtWhy,
    /// Return evidence + memory refs + promotions.
    DecisionWhy,
}

impl WhyQueryKind {
    #[must_use]
    pub const fn label(self) -> &'static str {
        match self {
            Self::Why => "why",
            Self::DebtWhy => "debt_why",
            Self::DecisionWhy => "decision_why",
        }
    }
}

// ── Causal step ──────────────────────────────────────────────────────────

/// One step in a causal explanation.
#[non_exhaustive]
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum WhyCausalStep {
    Node {
        id: NodeId,
        kind: ActiveGraphNodeKind,
        label: String,
    },
    Edge {
        kind: ActiveGraphEdgeKind,
        source: NodeId,
        target: NodeId,
    },
}

impl WhyCausalStep {
    /// Canonical sort key for deterministic ordering.
    #[must_use]
    pub fn sort_key(&self) -> (u8, String, String) {
        match self {
            Self::Node { id, .. } => (0, id.0.clone(), String::new()),
            Self::Edge {
                kind,
                source,
                target,
            } => (1, source.0.clone(), format!("{:?}:{}", kind, target.0)),
        }
    }
}

// ── Result ───────────────────────────────────────────────────────────────

/// Output of a `why`-family query.
#[non_exhaustive]
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct WhyQueryResult {
    pub kind: WhyQueryKind,
    pub target: NodeId,
    pub matched: bool,
    /// Canonical-sorted (Nodes first, then Edges by source+target+kind).
    pub causal_path: Vec<WhyCausalStep>,
    /// Canonical textual summary.
    pub summary: String,
    /// Caller-supplied `recorded_at`.
    pub generated_at: String,
    /// True if the closure was truncated by [`WHY_MAX_DEPTH`].
    pub truncated: bool,
}

// ── Trait + default evaluator ────────────────────────────────────────────

/// Evaluates a why query against an active graph projection.
pub trait WhyQueryEngine {
    fn query(
        &self,
        projection: &ActiveGraphProjection,
        target: &NodeId,
        kind: WhyQueryKind,
        recorded_at: &str,
    ) -> WhyQueryResult;
}

/// Default plain-data evaluator.
#[derive(Debug, Default, Clone, Copy, PartialEq, Eq)]
pub struct DefaultWhyQueryEngine;

impl WhyQueryEngine for DefaultWhyQueryEngine {
    fn query(
        &self,
        projection: &ActiveGraphProjection,
        target: &NodeId,
        kind: WhyQueryKind,
        recorded_at: &str,
    ) -> WhyQueryResult {
        // Unknown target ⇒ trivial result.
        if !projection.nodes.contains_key(target) {
            return WhyQueryResult {
                kind,
                target: target.clone(),
                matched: false,
                causal_path: Vec::new(),
                summary: "unknown target".to_string(),
                generated_at: recorded_at.to_string(),
                truncated: false,
            };
        }

        let (causal_path, truncated) = match kind {
            WhyQueryKind::Why => {
                let (steps, trunc) = walk_inbound(projection, target);
                (steps, trunc)
            }
            WhyQueryKind::DebtWhy => (
                filter_inbound_by_kinds(
                    projection,
                    target,
                    &[ActiveGraphEdgeKind::Gates, ActiveGraphEdgeKind::EvidenceOf],
                ),
                false,
            ),
            WhyQueryKind::DecisionWhy => (
                filter_inbound_by_kinds(
                    projection,
                    target,
                    &[
                        ActiveGraphEdgeKind::Promotes,
                        ActiveGraphEdgeKind::References,
                        ActiveGraphEdgeKind::EvidenceOf,
                    ],
                ),
                false,
            ),
        };

        let summary = build_summary(kind, target, &causal_path, truncated);

        WhyQueryResult {
            kind,
            target: target.clone(),
            matched: true,
            causal_path,
            summary,
            generated_at: recorded_at.to_string(),
            truncated,
        }
    }
}

// ── Helpers ──────────────────────────────────────────────────────────────

/// BFS inbound closure from `target` (limited to [`WHY_MAX_DEPTH`] hops).
/// Returns nodes + edges, plus a truncation flag.
fn walk_inbound(projection: &ActiveGraphProjection, target: &NodeId) -> (Vec<WhyCausalStep>, bool) {
    let mut visited: BTreeSet<NodeId> = BTreeSet::new();
    let mut nodes_out: BTreeMap<NodeId, ActiveGraphNode> = BTreeMap::new();
    let mut edges_out: BTreeMap<(NodeId, NodeId, ActiveGraphEdgeKind), ActiveGraphEdge> =
        BTreeMap::new();
    let mut frontier: VecDeque<(NodeId, usize)> = VecDeque::new();
    let mut truncated = false;

    // Seed with the target itself.
    visited.insert(target.clone());
    frontier.push_back((target.clone(), 0));

    while let Some((current, depth)) = frontier.pop_front() {
        if depth >= WHY_MAX_DEPTH {
            truncated = true;
            continue;
        }
        for edge in &projection.edges {
            if edge.target == current {
                // Add the edge.
                let key = (edge.source.clone(), edge.target.clone(), edge.kind);
                edges_out.entry(key).or_insert_with(|| edge.clone());
                // Visit the source if not seen.
                if !visited.contains(&edge.source) {
                    visited.insert(edge.source.clone());
                    frontier.push_back((edge.source.clone(), depth + 1));
                }
            }
        }
    }

    // Materialise nodes from visited.
    for nid in &visited {
        if let Some(node) = projection.nodes.get(nid) {
            nodes_out.insert(nid.clone(), node.clone());
        }
    }

    let mut steps: Vec<WhyCausalStep> = Vec::new();
    for node in nodes_out.values() {
        steps.push(WhyCausalStep::Node {
            id: node.id.clone(),
            kind: node.kind,
            label: node.label.clone(),
        });
    }
    for edge in edges_out.values() {
        steps.push(WhyCausalStep::Edge {
            kind: edge.kind,
            source: edge.source.clone(),
            target: edge.target.clone(),
        });
    }
    steps.sort_by_key(WhyCausalStep::sort_key);
    (steps, truncated)
}

/// Filter inbound edges by allowed kinds; include their source nodes too.
fn filter_inbound_by_kinds(
    projection: &ActiveGraphProjection,
    target: &NodeId,
    allowed: &[ActiveGraphEdgeKind],
) -> Vec<WhyCausalStep> {
    let mut nodes_out: BTreeMap<NodeId, ActiveGraphNode> = BTreeMap::new();
    let mut edges_out: BTreeMap<(NodeId, NodeId, ActiveGraphEdgeKind), ActiveGraphEdge> =
        BTreeMap::new();

    // Always include the target node itself.
    if let Some(node) = projection.nodes.get(target) {
        nodes_out.insert(node.id.clone(), node.clone());
    }

    for edge in &projection.edges {
        if edge.target == *target && allowed.contains(&edge.kind) {
            let key = (edge.source.clone(), edge.target.clone(), edge.kind);
            edges_out.entry(key).or_insert_with(|| edge.clone());
            if let Some(node) = projection.nodes.get(&edge.source) {
                nodes_out.insert(node.id.clone(), node.clone());
            }
        }
    }

    let mut steps: Vec<WhyCausalStep> = Vec::new();
    for node in nodes_out.values() {
        steps.push(WhyCausalStep::Node {
            id: node.id.clone(),
            kind: node.kind,
            label: node.label.clone(),
        });
    }
    for edge in edges_out.values() {
        steps.push(WhyCausalStep::Edge {
            kind: edge.kind,
            source: edge.source.clone(),
            target: edge.target.clone(),
        });
    }
    steps.sort_by_key(WhyCausalStep::sort_key);
    steps
}

fn build_summary(
    kind: WhyQueryKind,
    target: &NodeId,
    steps: &[WhyCausalStep],
    truncated: bool,
) -> String {
    let node_count = steps
        .iter()
        .filter(|s| matches!(s, WhyCausalStep::Node { .. }))
        .count();
    let edge_count = steps
        .iter()
        .filter(|s| matches!(s, WhyCausalStep::Edge { .. }))
        .count();
    let suffix = if truncated { " …(truncated)" } else { "" };
    format!(
        "{} {}: {} node(s), {} edge(s){}",
        kind.label(),
        target.0,
        node_count,
        edge_count,
        suffix
    )
}

// ── Tests ────────────────────────────────────────────────────────────────

#[cfg(test)]
#[allow(clippy::collapsible_if)]
mod tests {
    use super::WhyQueryEngine;
    use super::*;
    use crate::active_graph::{
        ActiveGraphEdge, ActiveGraphEdgeKind, ActiveGraphInput, ActiveGraphNode,
        ActiveGraphProjection, ActiveGraphProjector, DefaultActiveGraphProjector,
    };

    fn projector() -> DefaultActiveGraphProjector {
        DefaultActiveGraphProjector
    }

    fn engine() -> DefaultWhyQueryEngine {
        DefaultWhyQueryEngine
    }

    fn make_projection() -> ActiveGraphProjection {
        // root → child → leaf (parent edges)
        // DelegatedTo: orch → worker
        // Gates: gate1 → leaf
        // EvidenceOf: ev1 → leaf
        // Promotes: promo1 → child
        // References: ref1 → root
        let input = ActiveGraphInput {
            workflow_nodes: vec![
                NodeId("root".to_string()),
                NodeId("child".to_string()),
                NodeId("leaf".to_string()),
            ],
            workflow_edges: vec![
                (NodeId("root".to_string()), NodeId("child".to_string())),
                (NodeId("child".to_string()), NodeId("leaf".to_string())),
            ],
            delegations: vec![(NodeId("orch".to_string()), NodeId("worker".to_string()))],
            memory_refs: vec![("ref1".to_string(), NodeId("root".to_string()))],
            assurance_labels: vec!["ev1".to_string()],
            evidence_links: vec![("ev1".to_string(), NodeId("leaf".to_string()))],
            lab_promotions: vec![
                ("gate1".to_string(), NodeId("leaf".to_string()), false),
                ("promo1".to_string(), NodeId("child".to_string()), true),
            ],
            ..Default::default()
        };
        projector().project(&input, "t0")
    }

    // ── S-1: why follows parent chain ──────────────────────────────────

    #[test]
    fn s1_why_follows_parent_chain_to_root() {
        let p = make_projection();
        let r = engine().query(&p, &NodeId("leaf".to_string()), WhyQueryKind::Why, "t0");
        assert!(r.matched);
        // Should include root, child, leaf, plus the two parent edges.
        let node_ids: Vec<NodeId> = r
            .causal_path
            .iter()
            .filter_map(|s| match s {
                WhyCausalStep::Node { id, .. } => Some(id.clone()),
                _ => None,
            })
            .collect();
        assert!(node_ids.contains(&NodeId("root".to_string())));
        assert!(node_ids.contains(&NodeId("child".to_string())));
        assert!(node_ids.contains(&NodeId("leaf".to_string())));
        // Edge kinds present.
        let edge_count = r
            .causal_path
            .iter()
            .filter(|s| matches!(s, WhyCausalStep::Edge { .. }))
            .count();
        assert!(edge_count >= 2);
        assert!(!r.truncated);
        assert!(r.summary.contains("why"));
    }

    // ── S-2: debt_why returns gating edges ────────────────────────────

    #[test]
    fn s2_debt_why_returns_gating_edges_only() {
        let p = make_projection();
        let r = engine().query(&p, &NodeId("leaf".to_string()), WhyQueryKind::DebtWhy, "t0");
        assert!(r.matched);
        // Must include the Gates edge from gate1 → leaf, plus leaf itself.
        let sources: Vec<NodeId> = r
            .causal_path
            .iter()
            .filter_map(|s| match s {
                WhyCausalStep::Edge { source, .. } => Some(source.clone()),
                _ => None,
            })
            .collect();
        assert!(sources.contains(&NodeId("promotion:gate1".to_string())));
        // Must NOT include References or Promotes or DelegatedTo.
        for s in &r.causal_path {
            if let WhyCausalStep::Edge { kind, .. } = s {
                assert!(
                    matches!(
                        kind,
                        ActiveGraphEdgeKind::Gates | ActiveGraphEdgeKind::EvidenceOf
                    ),
                    "unexpected edge kind in debt_why: {:?}",
                    kind
                );
            }
        }
        // Must include the EvidenceOf edge too (ev1 → leaf).
        assert!(sources.contains(&NodeId("evidence:ev1".to_string())));
    }

    // ── S-3: decision_why returns refs + evidence + promotions ────────

    #[test]
    fn s3_decision_why_returns_evidence_refs_promotions() {
        let p = make_projection();
        let r = engine().query(
            &p,
            &NodeId("child".to_string()),
            WhyQueryKind::DecisionWhy,
            "t0",
        );
        assert!(r.matched);
        // Should include the Promotes edge promo1 → child.
        let mut has_promotes = false;
        for s in &r.causal_path {
            if let WhyCausalStep::Edge {
                kind,
                source,
                target,
            } = s
            {
                if matches!(kind, ActiveGraphEdgeKind::Promotes)
                    && source == &NodeId("promotion:promo1".to_string())
                    && target == &NodeId("child".to_string())
                {
                    has_promotes = true;
                }
            }
        }
        assert!(has_promotes);
        // Should also include the child + promo1 nodes themselves.
        let node_ids: Vec<NodeId> = r
            .causal_path
            .iter()
            .filter_map(|s| match s {
                WhyCausalStep::Node { id, .. } => Some(id.clone()),
                _ => None,
            })
            .collect();
        assert!(node_ids.contains(&NodeId("child".to_string())));
        assert!(node_ids.contains(&NodeId("promotion:promo1".to_string())));
    }

    // ── S-4: unknown target ───────────────────────────────────────────

    #[test]
    fn s4_unknown_target_matched_false() {
        let p = make_projection();
        let r = engine().query(&p, &NodeId("ghost".to_string()), WhyQueryKind::Why, "t0");
        assert!(!r.matched);
        assert!(r.causal_path.is_empty());
        assert_eq!(r.summary, "unknown target");
    }

    // ── S-5: max depth clamp ──────────────────────────────────────────

    #[test]
    fn s5_max_depth_truncates_chain() {
        // Build a chain longer than WHY_MAX_DEPTH.
        let mut nodes: Vec<NodeId> = Vec::new();
        let mut edges: Vec<(NodeId, NodeId)> = Vec::new();
        for i in 0..(WHY_MAX_DEPTH + 5) {
            let id = NodeId(format!("n{i}"));
            nodes.push(id.clone());
            if i > 0 {
                edges.push((NodeId(format!("n{}", i - 1)), id));
            }
        }
        let input = ActiveGraphInput {
            workflow_nodes: nodes,
            workflow_edges: edges,
            ..Default::default()
        };
        let p = projector().project(&input, "t0");
        let target = NodeId(format!("n{}", WHY_MAX_DEPTH + 4));
        let r = engine().query(&p, &target, WhyQueryKind::Why, "t0");
        assert!(r.matched);
        assert!(
            r.truncated,
            "expected truncation, got steps={:?}",
            r.causal_path
        );
        assert!(r.summary.contains("(truncated)"));
    }

    // ── S-6: closed-set audit guard ──────────────────────────────────

    #[test]
    fn s6_closed_set_audit_guard() {
        assert_eq!(WHY_QUERY_KIND_COUNT, 3);
        let all_kinds = [
            WhyQueryKind::Why,
            WhyQueryKind::DebtWhy,
            WhyQueryKind::DecisionWhy,
        ];
        assert_eq!(all_kinds.len(), WHY_QUERY_KIND_COUNT);
    }

    // ── Bonus: deterministic across runs ──────────────────────────────

    #[test]
    fn s7_deterministic_output() {
        let p = make_projection();
        let r1 = engine().query(&p, &NodeId("leaf".to_string()), WhyQueryKind::Why, "t-A");
        let r2 = engine().query(&p, &NodeId("leaf".to_string()), WhyQueryKind::Why, "t-B");
        // Same causal_path; only generated_at differs.
        assert_eq!(r1.causal_path, r2.causal_path);
        assert_ne!(r1.generated_at, r2.generated_at);
    }

    // ── Bonus: empty projection with valid target query returns empty path

    #[test]
    fn s8_empty_projection_unknown_target() {
        let p = ActiveGraphProjection::default();
        let r = engine().query(&p, &NodeId("x".to_string()), WhyQueryKind::Why, "t0");
        assert!(!r.matched);
        assert!(r.causal_path.is_empty());
    }

    // ── Bonus: same projection, no inbound edges ⇒ just the target node

    #[test]
    fn s9_isolated_target_returns_self_only() {
        let input = ActiveGraphInput {
            workflow_nodes: vec![NodeId("solo".to_string())],
            ..Default::default()
        };
        let p = projector().project(&input, "t0");
        let r = engine().query(&p, &NodeId("solo".to_string()), WhyQueryKind::Why, "t0");
        assert!(r.matched);
        let node_count = r
            .causal_path
            .iter()
            .filter(|s| matches!(s, WhyCausalStep::Node { .. }))
            .count();
        let edge_count = r
            .causal_path
            .iter()
            .filter(|s| matches!(s, WhyCausalStep::Edge { .. }))
            .count();
        assert_eq!(node_count, 1);
        assert_eq!(edge_count, 0);
    }

    // ── (silence unused-import warning for ActiveGraphNode + ActiveGraphEdge in tests)
    #[allow(dead_code)]
    fn _types_used(_n: ActiveGraphNode, _e: ActiveGraphEdge) {}
}
