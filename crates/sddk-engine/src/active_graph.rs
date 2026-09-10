//! Active Graph Projection (Typed)
//!
//! Plain-data projection of the current cycle's run/memory/assurance/lab state
//! into a deterministic, typed graph. Foundation for the Cockpit surface
//! (orders 510-520) and the `why`/`debt why`/`decision why` causal queries
//! (order 500).
//!
//! Pattern: P-a (struct + trait + plain-data evaluator). Determinism is
//! caller-controlled via `recorded_at`; iteration is canonical via
//! `BTreeMap`/`BTreeSet`.

use sddk_domain::workflow_ir::NodeId;
use serde::{Deserialize, Serialize};
use std::collections::{BTreeMap, BTreeSet};

// ── Audit guard constants ────────────────────────────────────────────────

/// Closed-set size of [`ActiveGraphNodeKind`]. Bump when a variant is added.
pub const ACTIVE_GRAPH_NODE_KIND_COUNT: usize = 10;

/// Closed-set size of [`ActiveGraphEdgeKind`]. Bump when a variant is added.
pub const ACTIVE_GRAPH_EDGE_KIND_COUNT: usize = 6;

/// Closed-set size of [`ProvenanceSourceKind`]. Bump when a variant is added.
pub const PROVENANCE_SOURCE_KIND_COUNT: usize = 3;

// ── Node & edge kinds ────────────────────────────────────────────────────

/// Closed-set taxonomy of nodes that may appear in an active graph.
#[non_exhaustive]
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
pub enum ActiveGraphNodeKind {
    Workflow,
    WorkflowRun,
    DecisionMemory,
    DecisionMemoryRef,
    Assurance,
    Uat,
    WorkflowLab,
    LabPromotion,
    Runbook,
    HumanDecision,
}

impl ActiveGraphNodeKind {
    /// Canonical textual label (used as part of deterministic `NodeId`).
    #[must_use]
    pub const fn label(self) -> &'static str {
        match self {
            Self::Workflow => "workflow",
            Self::WorkflowRun => "workflow_run",
            Self::DecisionMemory => "decision_memory",
            Self::DecisionMemoryRef => "decision_memory_ref",
            Self::Assurance => "assurance",
            Self::Uat => "uat",
            Self::WorkflowLab => "workflow_lab",
            Self::LabPromotion => "lab_promotion",
            Self::Runbook => "runbook",
            Self::HumanDecision => "human_decision",
        }
    }
}

/// Closed-set taxonomy of typed relations between nodes.
#[non_exhaustive]
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
pub enum ActiveGraphEdgeKind {
    /// Workflow parent → child.
    ParentOf,
    /// Human/Secretary/Orchestrator → worker.
    DelegatedTo,
    /// Assurance/UAT evidence → gated workflow node.
    EvidenceOf,
    /// Lab promotion → workflow node.
    Gates,
    /// Memory ref → promoted workflow/lab output.
    Promotes,
    /// Runbook/handoff → upstream node.
    References,
}

impl ActiveGraphEdgeKind {
    /// Canonical textual label used in audit output.
    #[must_use]
    pub const fn label(self) -> &'static str {
        match self {
            Self::ParentOf => "parent_of",
            Self::DelegatedTo => "delegated_to",
            Self::EvidenceOf => "evidence_of",
            Self::Gates => "gates",
            Self::Promotes => "promotes",
            Self::References => "references",
        }
    }
}

// ── Graph data types ─────────────────────────────────────────────────────

/// One node in the projected graph.
#[non_exhaustive]
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ActiveGraphNode {
    pub id: NodeId,
    pub kind: ActiveGraphNodeKind,
    pub label: String,
    pub recorded_at: String,
    /// Optional source attribution. Carries the `ProvenanceRef` that
    /// was recorded in the upstream `ActiveGraphInput` for this node's
    /// `NodeId`. `None` when the input did not record provenance
    /// (e.g. legacy fixtures, plain struct literals).
    pub provenance: Option<ProvenanceRef>,
}

/// One typed edge in the projected graph.
#[non_exhaustive]
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ActiveGraphEdge {
    pub kind: ActiveGraphEdgeKind,
    pub source: NodeId,
    pub target: NodeId,
    /// Optional source attribution for the edge. Set when the input
    /// recorded provenance for this edge's `(source, target)` pair
    /// (M8.6 contract: same map shape as node provenance).
    pub provenance: Option<ProvenanceRef>,
}

/// Where a node or edge came from. Carried through `ActiveGraphInput`
/// so every projected entity is attributable back to the byte that
/// produced it.
///
/// The string `source_locator` is free-form but should be a stable,
/// reproducible token (e.g. `Commits:row_3` for a table row, or
/// `Bridges:bullet_2` for a bullet). Locators are intended to be
/// human-readable first and machine-parseable second; the audit
/// contract is "an operator can re-locate the source by hand from
/// the locator alone".
#[non_exhaustive]
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ProvenanceRef {
    pub source_kind: ProvenanceSourceKind,
    pub source_locator: String,
}

impl ProvenanceRef {
    /// Stable constructor. Used by callers (CLI parsers, fixture
    /// loaders) that need to mint a `ProvenanceRef` from outside the
    /// defining crate — `ProvenanceRef` is `#[non_exhaustive]` so a
    /// plain struct expression would be rejected by the borrow checker.
    #[must_use]
    pub const fn new(source_kind: ProvenanceSourceKind, source_locator: String) -> Self {
        Self {
            source_kind,
            source_locator,
        }
    }
}

/// Closed-set taxonomy of where a node or edge came from.
#[non_exhaustive]
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
pub enum ProvenanceSourceKind {
    /// A live cycle's archive manifest at
    /// `~/.sddk-knowledge/<project>/cycles/<id>/archive-manifest.md`
    /// (M8.4+; the parser lives in `sddk-cli`).
    CycleManifest,
    /// A static `ActiveGraphInput` JSON fixture on disk
    /// (M8.0+; loaded via `--from-input`).
    InputFixture,
    /// An operator-provided JSON document loaded at runtime.
    InputDocument,
}

impl ProvenanceSourceKind {
    /// Canonical textual label (used in JSON output and audit logs).
    #[must_use]
    pub const fn label(self) -> &'static str {
        match self {
            Self::CycleManifest => "cycle_manifest",
            Self::InputFixture => "input_fixture",
            Self::InputDocument => "input_document",
        }
    }
}

impl ActiveGraphEdge {
    /// Canonical sort key `(kind, source, target)`.
    #[must_use]
    pub fn sort_key(&self) -> (ActiveGraphEdgeKind, NodeId, NodeId) {
        (self.kind, self.source.clone(), self.target.clone())
    }
}

/// Output of an active-graph projection.
#[non_exhaustive]
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct ActiveGraphProjection {
    /// Every projected node, sorted by `NodeId`.
    pub nodes: BTreeMap<NodeId, ActiveGraphNode>,
    /// Every projected edge, sorted by `(kind, source, target)`.
    pub edges: Vec<ActiveGraphEdge>,
    /// Entry nodes (workflow roots + memory HEAD), sorted, deduplicated.
    pub roots: Vec<NodeId>,
    /// `nodes.len()` at projection time.
    pub node_count: usize,
    /// `edges.len()` at projection time.
    pub edge_count: usize,
}

// ── Input ────────────────────────────────────────────────────────────────

/// Plain-data input aggregating upstream cycle outputs.
///
/// Empty `Vec`s are valid: an empty input yields an empty projection
/// (see scenario S-1).
#[non_exhaustive]
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct ActiveGraphInput {
    /// Workflow nodes, in topological order (root first). May be empty.
    pub workflow_nodes: Vec<NodeId>,
    /// Workflow parent → child relations (parallel to `workflow_nodes` ordering).
    pub workflow_edges: Vec<(NodeId, NodeId)>,
    /// Optional workflow run id (becomes a `WorkflowRun` node).
    pub workflow_run_id: Option<String>,
    /// Decision memory HEAD label (becomes a `DecisionMemory` node + root).
    pub memory_head: Option<String>,
    /// Memory refs pointing at workflow nodes (`(ref_label, target_node)`).
    pub memory_refs: Vec<(String, NodeId)>,
    /// Assurance verdict labels (each becomes an `Assurance` node).
    pub assurance_labels: Vec<String>,
    /// UAT scenario labels (each becomes a `Uat` node).
    pub uat_labels: Vec<String>,
    /// Workflow-lab run labels (each becomes a `WorkflowLab` node).
    pub workflow_lab_labels: Vec<String>,
    /// Lab promotion decisions: `(gate_label, target_node, is_promote)`.
    /// `is_promote=true` ⇒ `Promotes` edge; `false` ⇒ `Gates` edge.
    pub lab_promotions: Vec<(String, NodeId, bool)>,
    /// Runbook labels (each becomes a `Runbook` node).
    pub runbook_labels: Vec<String>,
    /// Human decision labels (each becomes a `HumanDecision` node).
    pub human_decision_labels: Vec<String>,
    /// Delegation edges: `(delegator, delegate)` in node-id space.
    pub delegations: Vec<(NodeId, NodeId)>,
    /// Evidence links: `(evidence_label, gated_node)`.
    pub evidence_links: Vec<(String, NodeId)>,
    /// Per-node provenance (`NodeId` → `ProvenanceRef`). Empty by
    /// default for backward compatibility; M8.6 cycle-aware
    /// derivation populates this from the manifest's section + bullet
    /// positions. The projector copies these into the corresponding
    /// `ActiveGraphNode::provenance` field.
    pub node_provenance: BTreeMap<NodeId, ProvenanceRef>,
    /// Per-edge provenance. Keyed by `(source, target)` because the
    /// edge tuple is the canonical id in this graph. Empty by default
    /// for backward compatibility.
    pub edge_provenance: BTreeMap<(NodeId, NodeId), ProvenanceRef>,
}

// ── Trait + default evaluator ────────────────────────────────────────────

/// Evaluates an [`ActiveGraphInput`] into an [`ActiveGraphProjection`].
pub trait ActiveGraphProjector {
    fn project(&self, input: &ActiveGraphInput, recorded_at: &str) -> ActiveGraphProjection;
}

/// Default plain-data evaluator.
#[derive(Debug, Default, Clone, Copy, PartialEq, Eq)]
pub struct DefaultActiveGraphProjector;

impl ActiveGraphProjector for DefaultActiveGraphProjector {
    fn project(&self, input: &ActiveGraphInput, recorded_at: &str) -> ActiveGraphProjection {
        // Use BTreeSet internally to keep insertion order canonical.
        let mut nodes: BTreeMap<NodeId, ActiveGraphNode> = BTreeMap::new();
        let mut edges: Vec<ActiveGraphEdge> = Vec::new();
        let mut roots: BTreeSet<NodeId> = BTreeSet::new();

        // Helper: build a node with provenance lookup from
        // `input.node_provenance`. Missing entry ⇒ `provenance: None`.
        let prov = |id: &NodeId| input.node_provenance.get(id).cloned();
        // Helper: build an edge with provenance lookup keyed by
        // `(source, target)`. The same lookup happens for edges whose
        // direction the projector preserves (e.g. parent_of, evidence_of).
        let edge_prov = |src: &NodeId, dst: &NodeId| {
            input.edge_provenance.get(&(src.clone(), dst.clone())).cloned()
        };

        // Workflow nodes
        for nid in &input.workflow_nodes {
            let id = nid.clone();
            let provenance = prov(&id);
            nodes.entry(id.clone()).or_insert_with(|| ActiveGraphNode {
                id: id.clone(),
                kind: ActiveGraphNodeKind::Workflow,
                label: format!("workflow:{id}", id = id.0),
                recorded_at: recorded_at.to_string(),
                provenance,
            });
            roots.insert(id);
        }

        // Workflow parent → child edges
        for (parent, child) in &input.workflow_edges {
            edges.push(ActiveGraphEdge {
                kind: ActiveGraphEdgeKind::ParentOf,
                source: parent.clone(),
                target: child.clone(),
                provenance: edge_prov(parent, child),
            });
        }

        // Workflow run
        if let Some(run_id) = &input.workflow_run_id {
            let id = NodeId(format!("run:{run_id}"));
            let provenance = prov(&id);
            nodes.insert(
                id.clone(),
                ActiveGraphNode {
                    id: id.clone(),
                    kind: ActiveGraphNodeKind::WorkflowRun,
                    label: format!("run:{run_id}"),
                    recorded_at: recorded_at.to_string(),
                    provenance,
                },
            );
            roots.insert(id);
        }

        // Memory HEAD
        if let Some(head) = &input.memory_head {
            let id = NodeId(format!("memory:{head}"));
            let provenance = prov(&id);
            nodes.insert(
                id.clone(),
                ActiveGraphNode {
                    id: id.clone(),
                    kind: ActiveGraphNodeKind::DecisionMemory,
                    label: format!("memory:{head}"),
                    recorded_at: recorded_at.to_string(),
                    provenance,
                },
            );
            roots.insert(id);
        }

        // Memory refs
        for (label, target) in &input.memory_refs {
            let id = NodeId(format!("memoryref:{label}"));
            let provenance = prov(&id);
            nodes.insert(
                id.clone(),
                ActiveGraphNode {
                    id: id.clone(),
                    kind: ActiveGraphNodeKind::DecisionMemoryRef,
                    label: format!("memoryref:{label}"),
                    recorded_at: recorded_at.to_string(),
                    provenance,
                },
            );
            edges.push(ActiveGraphEdge {
                kind: ActiveGraphEdgeKind::References,
                source: id,
                target: target.clone(),
                provenance: edge_prov(&NodeId(format!("memoryref:{label}")), target),
            });
        }

        // Assurance + evidence edges
        for label in &input.assurance_labels {
            let id = NodeId(format!("assurance:{label}"));
            let provenance = prov(&id);
            nodes.insert(
                id.clone(),
                ActiveGraphNode {
                    id: id.clone(),
                    kind: ActiveGraphNodeKind::Assurance,
                    label: format!("assurance:{label}"),
                    recorded_at: recorded_at.to_string(),
                    provenance,
                },
            );
        }
        for (evidence_label, gated) in &input.evidence_links {
            let id = NodeId(format!("evidence:{evidence_label}"));
            let provenance = prov(&id);
            nodes.insert(
                id.clone(),
                ActiveGraphNode {
                    id: id.clone(),
                    kind: ActiveGraphNodeKind::Assurance,
                    label: format!("evidence:{evidence_label}"),
                    recorded_at: recorded_at.to_string(),
                    provenance,
                },
            );
            edges.push(ActiveGraphEdge {
                kind: ActiveGraphEdgeKind::EvidenceOf,
                source: id,
                target: gated.clone(),
                provenance: edge_prov(
                    &NodeId(format!("evidence:{evidence_label}")),
                    gated,
                ),
            });
        }

        // UAT
        for label in &input.uat_labels {
            let id = NodeId(format!("uat:{label}"));
            let provenance = prov(&id);
            nodes.insert(
                id.clone(),
                ActiveGraphNode {
                    id: id.clone(),
                    kind: ActiveGraphNodeKind::Uat,
                    label: format!("uat:{label}"),
                    recorded_at: recorded_at.to_string(),
                    provenance,
                },
            );
        }

        // Workflow-lab
        for label in &input.workflow_lab_labels {
            let id = NodeId(format!("lab:{label}"));
            let provenance = prov(&id);
            nodes.insert(
                id.clone(),
                ActiveGraphNode {
                    id: id.clone(),
                    kind: ActiveGraphNodeKind::WorkflowLab,
                    label: format!("lab:{label}"),
                    recorded_at: recorded_at.to_string(),
                    provenance,
                },
            );
        }

        // Lab promotion (Gates | Promotes)
        for (gate_label, target, is_promote) in &input.lab_promotions {
            let id = NodeId(format!("promotion:{gate_label}"));
            let provenance = prov(&id);
            nodes.insert(
                id.clone(),
                ActiveGraphNode {
                    id: id.clone(),
                    kind: ActiveGraphNodeKind::LabPromotion,
                    label: format!("promotion:{gate_label}"),
                    recorded_at: recorded_at.to_string(),
                    provenance,
                },
            );
            let edge_kind = if *is_promote {
                ActiveGraphEdgeKind::Promotes
            } else {
                ActiveGraphEdgeKind::Gates
            };
            let edge_provenance = edge_prov(&id, target);
            edges.push(ActiveGraphEdge {
                kind: edge_kind,
                source: id,
                target: target.clone(),
                provenance: edge_provenance,
            });
        }

        // Runbook
        for label in &input.runbook_labels {
            let id = NodeId(format!("runbook:{label}"));
            let provenance = prov(&id);
            nodes.insert(
                id.clone(),
                ActiveGraphNode {
                    id: id.clone(),
                    kind: ActiveGraphNodeKind::Runbook,
                    label: format!("runbook:{label}"),
                    recorded_at: recorded_at.to_string(),
                    provenance,
                },
            );
        }

        // Human decisions
        for label in &input.human_decision_labels {
            let id = NodeId(format!("human:{label}"));
            let provenance = prov(&id);
            nodes.insert(
                id.clone(),
                ActiveGraphNode {
                    id: id.clone(),
                    kind: ActiveGraphNodeKind::HumanDecision,
                    label: format!("human:{label}"),
                    recorded_at: recorded_at.to_string(),
                    provenance,
                },
            );
        }

        // Delegations
        for (delegator, delegate) in &input.delegations {
            edges.push(ActiveGraphEdge {
                kind: ActiveGraphEdgeKind::DelegatedTo,
                source: delegator.clone(),
                target: delegate.clone(),
                provenance: edge_prov(delegator, delegate),
            });
        }

        // Canonical edge ordering
        edges.sort_by_key(ActiveGraphEdge::sort_key);

        let node_count = nodes.len();
        let edge_count = edges.len();
        let roots: Vec<NodeId> = roots.into_iter().collect();

        ActiveGraphProjection {
            nodes,
            edges,
            roots,
            node_count,
            edge_count,
        }
    }
}

// ── Tests ────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    fn projector() -> DefaultActiveGraphProjector {
        DefaultActiveGraphProjector
    }

    // ── S-1: empty input ────────────────────────────────────────────────

    #[test]
    fn s1_empty_input_yields_empty_projection() {
        let p = projector().project(&ActiveGraphInput::default(), "t0");
        assert_eq!(p.node_count, 0);
        assert_eq!(p.edge_count, 0);
        assert!(p.nodes.is_empty());
        assert!(p.edges.is_empty());
        assert!(p.roots.is_empty());
    }

    // ── S-2: workflow + run + parent edge ───────────────────────────────

    #[test]
    fn s2_workflow_run_with_parent_edge() {
        let input = ActiveGraphInput {
            workflow_nodes: vec![NodeId("root".to_string()), NodeId("child".to_string())],
            workflow_edges: vec![(NodeId("root".to_string()), NodeId("child".to_string()))],
            workflow_run_id: Some("run-1".to_string()),
            ..Default::default()
        };
        let p = projector().project(&input, "t0");
        assert_eq!(p.node_count, 3); // 2 workflow + 1 run
        assert_eq!(p.edge_count, 1);
        assert_eq!(p.edges[0].kind, ActiveGraphEdgeKind::ParentOf);
        assert_eq!(p.edges[0].source, NodeId("root".to_string()));
        assert_eq!(p.edges[0].target, NodeId("child".to_string()));
        assert!(p.roots.contains(&NodeId("run:run-1".to_string())));
    }

    // ── S-3: memory ref → workflow node ─────────────────────────────────

    #[test]
    fn s3_memory_ref_references_workflow_node() {
        let input = ActiveGraphInput {
            workflow_nodes: vec![NodeId("wf-1".to_string())],
            memory_refs: vec![("ref-a".to_string(), NodeId("wf-1".to_string()))],
            ..Default::default()
        };
        let p = projector().project(&input, "t0");
        assert_eq!(p.node_count, 2);
        assert_eq!(p.edge_count, 1);
        assert_eq!(p.edges[0].kind, ActiveGraphEdgeKind::References);
        assert_eq!(p.edges[0].source, NodeId("memoryref:ref-a".to_string()));
        assert_eq!(p.edges[0].target, NodeId("wf-1".to_string()));
    }

    // ── S-4: lab promotion gates workflow node ──────────────────────────

    #[test]
    fn s4_lab_promotion_gates_workflow_node() {
        let input = ActiveGraphInput {
            workflow_nodes: vec![NodeId("wf-x".to_string())],
            lab_promotions: vec![("g1".to_string(), NodeId("wf-x".to_string()), true)],
            ..Default::default()
        };
        let p = projector().project(&input, "t0");
        assert_eq!(p.node_count, 2);
        assert_eq!(p.edge_count, 1);
        assert_eq!(p.edges[0].kind, ActiveGraphEdgeKind::Promotes);
        assert_eq!(p.edges[0].source, NodeId("promotion:g1".to_string()));
        assert_eq!(p.edges[0].target, NodeId("wf-x".to_string()));
    }

    #[test]
    fn s4b_lab_promotion_hold_yields_gates_edge() {
        let input = ActiveGraphInput {
            workflow_nodes: vec![NodeId("wf-y".to_string())],
            lab_promotions: vec![("g2".to_string(), NodeId("wf-y".to_string()), false)],
            ..Default::default()
        };
        let p = projector().project(&input, "t0");
        assert_eq!(p.edges[0].kind, ActiveGraphEdgeKind::Gates);
    }

    // ── S-5: closed-set audit guard ────────────────────────────────────

    #[test]
    fn s5_closed_set_audit_guards_compile() {
        // Bump these constants when variants are added.
        assert_eq!(ACTIVE_GRAPH_NODE_KIND_COUNT, 10);
        assert_eq!(ACTIVE_GRAPH_EDGE_KIND_COUNT, 6);

        // Walk every variant to ensure the macro/audit catches unused variants.
        let all_node_kinds = [
            ActiveGraphNodeKind::Workflow,
            ActiveGraphNodeKind::WorkflowRun,
            ActiveGraphNodeKind::DecisionMemory,
            ActiveGraphNodeKind::DecisionMemoryRef,
            ActiveGraphNodeKind::Assurance,
            ActiveGraphNodeKind::Uat,
            ActiveGraphNodeKind::WorkflowLab,
            ActiveGraphNodeKind::LabPromotion,
            ActiveGraphNodeKind::Runbook,
            ActiveGraphNodeKind::HumanDecision,
        ];
        assert_eq!(all_node_kinds.len(), ACTIVE_GRAPH_NODE_KIND_COUNT);

        let all_edge_kinds = [
            ActiveGraphEdgeKind::ParentOf,
            ActiveGraphEdgeKind::DelegatedTo,
            ActiveGraphEdgeKind::EvidenceOf,
            ActiveGraphEdgeKind::Gates,
            ActiveGraphEdgeKind::Promotes,
            ActiveGraphEdgeKind::References,
        ];
        assert_eq!(all_edge_kinds.len(), ACTIVE_GRAPH_EDGE_KIND_COUNT);
    }

    // ── S-6: deterministic regardless of `recorded_at` ──────────────────

    #[test]
    fn s6_recorded_at_does_not_change_projection() {
        let input = ActiveGraphInput {
            workflow_nodes: vec![NodeId("a".to_string()), NodeId("b".to_string())],
            workflow_edges: vec![(NodeId("a".to_string()), NodeId("b".to_string()))],
            workflow_run_id: Some("r".to_string()),
            memory_head: Some("head".to_string()),
            memory_refs: vec![("m".to_string(), NodeId("a".to_string()))],
            ..Default::default()
        };
        let p1 = projector().project(&input, "t-A");
        let p2 = projector().project(&input, "t-B");
        assert_eq!(p1.node_count, p2.node_count);
        assert_eq!(p1.edge_count, p2.edge_count);
        // IDs and edges identical; only the `recorded_at` strings inside
        // the nodes differ.
        let keys_a: Vec<_> = p1.nodes.keys().collect();
        let keys_b: Vec<_> = p2.nodes.keys().collect();
        assert_eq!(keys_a, keys_b);
        assert_eq!(p1.edges, p2.edges);
    }

    // ── Bonus: edge canonical ordering across kinds ─────────────────────

    #[test]
    fn s7_edges_sorted_by_kind_then_endpoints() {
        let input = ActiveGraphInput {
            workflow_nodes: vec![NodeId("n1".to_string()), NodeId("n2".to_string())],
            workflow_edges: vec![
                (NodeId("n1".to_string()), NodeId("n2".to_string())),
                (NodeId("n2".to_string()), NodeId("n1".to_string())),
            ],
            delegations: vec![(NodeId("n1".to_string()), NodeId("n2".to_string()))],
            ..Default::default()
        };
        let p = projector().project(&input, "t0");
        // Ordering: ParentOf (0) < DelegatedTo (1).
        // Among ParentOf entries: (n1→n2) < (n2→n1) lex.
        assert_eq!(p.edges.len(), 3);
        assert_eq!(p.edges[0].kind, ActiveGraphEdgeKind::ParentOf);
        assert_eq!(p.edges[0].source, NodeId("n1".to_string()));
        assert_eq!(p.edges[0].target, NodeId("n2".to_string()));
        assert_eq!(p.edges[1].kind, ActiveGraphEdgeKind::ParentOf);
        assert_eq!(p.edges[1].source, NodeId("n2".to_string()));
        assert_eq!(p.edges[1].target, NodeId("n1".to_string()));
        assert_eq!(p.edges[2].kind, ActiveGraphEdgeKind::DelegatedTo);
    }

    // ── Bonus: roots deduplicated ───────────────────────────────────────

    #[test]
    fn s8_roots_dedup_and_sorted() {
        let input = ActiveGraphInput {
            workflow_nodes: vec![NodeId("a".to_string()), NodeId("b".to_string())],
            memory_head: Some("h".to_string()),
            workflow_run_id: Some("r".to_string()),
            ..Default::default()
        };
        let p = projector().project(&input, "t0");
        // Sorted set semantics.
        assert_eq!(p.roots.len(), 4);
        assert_eq!(p.roots[0], NodeId("a".to_string()));
        assert_eq!(p.roots[1], NodeId("b".to_string()));
        assert_eq!(p.roots[2], NodeId("memory:h".to_string()));
        assert_eq!(p.roots[3], NodeId("run:r".to_string()));
    }

    // ── S-NEW: M8.6 — provenance is threaded through the projector ────

    #[test]
    fn provenance_node_is_attributed_when_input_records_it() {
        let mut input = ActiveGraphInput::default();
        input.workflow_nodes.push(NodeId("commit-1".to_string()));
        input.workflow_run_id = Some("run-7".to_string());
        input.node_provenance.insert(
            NodeId("commit-1".to_string()),
            ProvenanceRef {
                source_kind: ProvenanceSourceKind::CycleManifest,
                source_locator: "Commits:row_1".to_string(),
            },
        );
        input.node_provenance.insert(
            NodeId("run:run-7".to_string()),
            ProvenanceRef {
                source_kind: ProvenanceSourceKind::CycleManifest,
                source_locator: "Releases:line_5".to_string(),
            },
        );
        let p = projector().project(&input, "t0");
        let commit_node = p.nodes.get(&NodeId("commit-1".to_string())).unwrap();
        assert_eq!(
            commit_node.provenance.as_ref().map(|p| p.source_locator.as_str()),
            Some("Commits:row_1"),
            "node provenance must be copied into the projected node"
        );
        let run_node = p.nodes.get(&NodeId("run:run-7".to_string())).unwrap();
        assert_eq!(
            run_node.provenance.as_ref().map(|p| p.source_locator.as_str()),
            Some("Releases:line_5")
        );
    }

    #[test]
    fn provenance_node_is_none_when_input_does_not_record_it() {
        // No provenance recorded ⇒ every node has `provenance: None`.
        let mut input = ActiveGraphInput::default();
        input.workflow_nodes.push(NodeId("commit-2".to_string()));
        let p = projector().project(&input, "t0");
        let commit_node = p.nodes.get(&NodeId("commit-2".to_string())).unwrap();
        assert!(
            commit_node.provenance.is_none(),
            "node without provenance input must project with `provenance: None`"
        );
    }

    #[test]
    fn provenance_edge_is_attributed_when_input_records_it() {
        let mut input = ActiveGraphInput::default();
        input.workflow_nodes.push(NodeId("child".to_string()));
        input.workflow_nodes.push(NodeId("parent".to_string()));
        input.workflow_edges.push((
            NodeId("child".to_string()),
            NodeId("parent".to_string()),
        ));
        input.edge_provenance.insert(
            (NodeId("child".to_string()), NodeId("parent".to_string())),
            ProvenanceRef {
                source_kind: ProvenanceSourceKind::CycleManifest,
                source_locator: "Commit_parents:bullet_1".to_string(),
            },
        );
        let p = projector().project(&input, "t0");
        let edge = p
            .edges
            .iter()
            .find(|e| e.kind == ActiveGraphEdgeKind::ParentOf)
            .expect("parent_of edge must exist");
        assert_eq!(
            edge.provenance.as_ref().map(|p| p.source_locator.as_str()),
            Some("Commit_parents:bullet_1"),
            "edge provenance must be copied into the projected edge"
        );
    }

    #[test]
    fn provenance_source_kind_label_is_stable() {
        // The label() method is the audit contract — locked-down so a
        // rename does not silently break downstream JSON consumers.
        assert_eq!(ProvenanceSourceKind::CycleManifest.label(), "cycle_manifest");
        assert_eq!(ProvenanceSourceKind::InputFixture.label(), "input_fixture");
        assert_eq!(ProvenanceSourceKind::InputDocument.label(), "input_document");
        assert_eq!(PROVENANCE_SOURCE_KIND_COUNT, 3);
    }
}
