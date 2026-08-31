//! Moldable explorer views (SPEC-013, Phase 8).
//!
//! Declarative view descriptors over the reactive graph: the same
//! `GraphState` is projected into task-specific views (graph, timeline,
//! verification, evidence, release) without duplicating domain data.

use std::collections::{BTreeMap, BTreeSet};

use serde::{Deserialize, Serialize};

use crate::graph::GraphState;
use crate::staleness::{StalenessResult, derive_staleness};

/// Layout hint for a view.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ViewLayout {
    /// Node/edge graph with progressive disclosure.
    Graph,
    /// Ordered causal trace.
    Timeline,
    /// Filtered list.
    List,
}

/// Declarative view descriptor (SPEC-013 §3).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ViewDescriptor {
    /// Stable view identifier.
    pub view_id: String,
    /// Human title.
    pub title: String,
    /// Node-kind filter (empty = all).
    #[serde(default)]
    pub node_types: Vec<String>,
    /// Layout hint.
    pub layout: ViewLayout,
    /// Whether to include the provenance panel.
    #[serde(default)]
    pub provenance: bool,
    /// Progressive-disclosure depth bound (0 = unbounded).
    #[serde(default)]
    pub max_depth: u32,
}

/// Built-in task-specific views (SPEC-013 §3 initial views).
pub fn builtin_views() -> Vec<ViewDescriptor> {
    vec![
        ViewDescriptor {
            view_id: "overview".into(),
            title: "Overview".into(),
            node_types: vec![],
            layout: ViewLayout::List,
            provenance: true,
            max_depth: 0,
        },
        ViewDescriptor {
            view_id: "graph".into(),
            title: "Graph".into(),
            node_types: vec![],
            layout: ViewLayout::Graph,
            provenance: true,
            max_depth: 2,
        },
        ViewDescriptor {
            view_id: "timeline".into(),
            title: "Timeline".into(),
            node_types: vec![],
            layout: ViewLayout::Timeline,
            provenance: true,
            max_depth: 0,
        },
        ViewDescriptor {
            view_id: "verification".into(),
            title: "Verification".into(),
            node_types: vec!["requirement".into(), "test".into(), "task".into()],
            layout: ViewLayout::List,
            provenance: true,
            max_depth: 0,
        },
        ViewDescriptor {
            view_id: "evidence".into(),
            title: "Evidence".into(),
            node_types: vec!["evidence".into()],
            layout: ViewLayout::List,
            provenance: true,
            max_depth: 0,
        },
        ViewDescriptor {
            view_id: "release".into(),
            title: "Release assurance".into(),
            node_types: vec!["release".into(), "acceptance".into()],
            layout: ViewLayout::List,
            provenance: true,
            max_depth: 0,
        },
    ]
}

/// Looks up a builtin view by id.
pub fn find_builtin_view(view_id: &str) -> Option<ViewDescriptor> {
    builtin_views().into_iter().find(|v| v.view_id == view_id)
}

/// A node in the view model.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ViewModelNode {
    /// Node key (`kind:id`).
    pub key: String,
    /// Entity kind.
    pub kind: String,
    /// Entity id.
    pub id: String,
    /// Creating event id (provenance).
    pub created_by: String,
    /// Content hash of the creating event.
    pub content_hash: String,
    /// Whether the node has neighbors beyond the current depth (graph view).
    #[serde(default)]
    pub expandable: bool,
}

/// An edge in the view model.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ViewModelEdge {
    /// Source node key.
    pub from: String,
    /// Relation name.
    pub relation: String,
    /// Target node key.
    pub to: String,
    /// Creating event id (provenance).
    pub event_id: String,
}

/// One timeline entry (causal trace).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct TimelineEvent {
    /// Stream sequence.
    pub sequence: u64,
    /// Event id.
    pub event_id: String,
    /// Event type.
    pub event_type: String,
    /// RFC 3339 occurrence timestamp.
    pub occurred_at: String,
    /// Content hash.
    pub content_hash: String,
}

/// Serialized view payload (rendered by the CLI template).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ViewModel {
    /// View identifier.
    pub view_id: String,
    /// View title.
    pub title: String,
    /// Focus entity, when given.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub entity: Option<String>,
    /// Visible nodes.
    pub nodes: Vec<ViewModelNode>,
    /// Visible edges.
    pub edges: Vec<ViewModelEdge>,
    /// Timeline entries (timeline view).
    pub timeline: Vec<TimelineEvent>,
    /// Staleness info (verification view).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub staleness: Option<StalenessResult>,
}

/// Renders a view model from the graph for the given descriptor.
pub fn render_view_model(
    state: &GraphState,
    descriptor: &ViewDescriptor,
    entity: Option<&str>,
) -> ViewModel {
    match descriptor.layout {
        ViewLayout::Graph => render_graph(state, descriptor, entity),
        ViewLayout::Timeline => render_timeline(state, descriptor),
        ViewLayout::List => render_list(state, descriptor, entity),
    }
}

fn render_graph(
    state: &GraphState,
    descriptor: &ViewDescriptor,
    entity: Option<&str>,
) -> ViewModel {
    let mut nodes: BTreeMap<String, ViewModelNode> = BTreeMap::new();
    let mut edges: Vec<ViewModelEdge> = Vec::new();
    let mut seen: BTreeSet<String> = BTreeSet::new();

    // Start set: the focus entity or all nodes matching node_types (or all).
    let start_keys: Vec<String> = match entity {
        Some(key) => vec![key.to_string()],
        None => state
            .nodes
            .keys()
            .filter(|key| {
                descriptor.node_types.is_empty()
                    || descriptor
                        .node_types
                        .iter()
                        .any(|t| key.starts_with(&format!("{t}:")))
            })
            .cloned()
            .collect(),
    };

    for start in &start_keys {
        if let Some(node) = state.nodes.get(start) {
            seen.insert(start.clone());
            nodes.insert(
                start.clone(),
                ViewModelNode {
                    key: start.clone(),
                    kind: node.kind.clone(),
                    id: node.id.clone(),
                    created_by: node.created_by.clone(),
                    content_hash: node.content_hash.clone(),
                    expandable: false,
                },
            );
        }
    }

    // BFS up to max_depth.
    let max_depth = if descriptor.max_depth == 0 {
        u32::MAX
    } else {
        descriptor.max_depth
    };
    let mut frontier: Vec<(String, u32)> = start_keys.iter().cloned().map(|k| (k, 0)).collect();
    while let Some((current, depth)) = frontier.pop() {
        if depth >= max_depth {
            continue;
        }
        for edge in state.edges.iter().filter(|e| e.from == current) {
            edges.push(ViewModelEdge {
                from: edge.from.clone(),
                relation: edge.relation.clone(),
                to: edge.to.clone(),
                event_id: edge.event_id.clone(),
            });
            if !seen.contains(&edge.to)
                && let Some(node) = state.nodes.get(&edge.to)
            {
                seen.insert(edge.to.clone());
                let expandable = state
                    .edges
                    .iter()
                    .any(|e| e.from == edge.to && depth + 1 < max_depth);
                nodes.insert(
                    edge.to.clone(),
                    ViewModelNode {
                        key: edge.to.clone(),
                        kind: node.kind.clone(),
                        id: node.id.clone(),
                        created_by: node.created_by.clone(),
                        content_hash: node.content_hash.clone(),
                        expandable,
                    },
                );
                frontier.push((edge.to.clone(), depth + 1));
            }
        }
    }

    // Deduplicate edges.
    let mut edge_set: BTreeSet<String> = BTreeSet::new();
    edges.retain(|e| edge_set.insert(format!("{}|{}|{}", e.from, e.relation, e.to)));

    ViewModel {
        view_id: descriptor.view_id.clone(),
        title: descriptor.title.clone(),
        entity: entity.map(|s| s.to_string()),
        nodes: nodes.into_values().collect(),
        edges,
        timeline: Vec::new(),
        staleness: None,
    }
}

fn render_timeline(state: &GraphState, descriptor: &ViewDescriptor) -> ViewModel {
    // Derive timeline entries from edge provenance (event order by occurred_at).
    let mut timeline: Vec<TimelineEvent> = state
        .edges
        .iter()
        .map(|e| TimelineEvent {
            sequence: 0,
            event_id: e.event_id.clone(),
            event_type: e.relation.clone(),
            occurred_at: e.occurred_at.clone(),
            content_hash: String::new(),
        })
        .collect();
    // Deterministic order: occurred_at then event_id.
    timeline.sort_by(|a, b| {
        a.occurred_at
            .cmp(&b.occurred_at)
            .then(a.event_id.cmp(&b.event_id))
    });
    // Assign pseudo-sequences in order.
    for (i, event) in timeline.iter_mut().enumerate() {
        event.sequence = (i + 1) as u64;
    }

    ViewModel {
        view_id: descriptor.view_id.clone(),
        title: descriptor.title.clone(),
        entity: None,
        nodes: Vec::new(),
        edges: Vec::new(),
        timeline,
        staleness: None,
    }
}

fn render_list(state: &GraphState, descriptor: &ViewDescriptor, entity: Option<&str>) -> ViewModel {
    let mut nodes: Vec<ViewModelNode> = state
        .nodes
        .values()
        .filter(|node| {
            descriptor.node_types.is_empty() || descriptor.node_types.contains(&node.kind)
        })
        .filter(|node| {
            entity
                .map(|e| format!("{}:{}", node.kind, node.id) == e)
                .unwrap_or(true)
        })
        .map(|node| ViewModelNode {
            key: node.key(),
            kind: node.kind.clone(),
            id: node.id.clone(),
            created_by: node.created_by.clone(),
            content_hash: node.content_hash.clone(),
            expandable: false,
        })
        .collect();
    nodes.sort_by(|a, b| a.key.cmp(&b.key));

    let staleness = if descriptor.view_id == "verification" {
        entity
            .and_then(|e| state.nodes.get(e))
            .map(|_| derive_staleness(state, entity.unwrap()))
    } else {
        None
    };

    ViewModel {
        view_id: descriptor.view_id.clone(),
        title: descriptor.title.clone(),
        entity: entity.map(|s| s.to_string()),
        nodes,
        edges: Vec::new(),
        timeline: Vec::new(),
        staleness,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::graph::{GraphEdge, GraphNode};

    fn sample_state() -> GraphState {
        let mut state = GraphState::default();
        for (i, key) in [
            "cycle:c-1",
            "capability:git.commit",
            "actor:alice",
            "phase:verify",
        ]
        .iter()
        .enumerate()
        {
            let (kind, id) = key.split_once(':').unwrap();
            state.nodes.insert(
                key.to_string(),
                GraphNode {
                    kind: kind.into(),
                    id: id.into(),
                    created_by: format!("e{i}"),
                    content_hash: format!("sha256:{i}"),
                    occurred_at: format!("2026-08-18T10:00:0{i}Z"),
                },
            );
        }
        state.edges.push(GraphEdge {
            from: "actor:alice".into(),
            relation: "approval.capability.granted".into(),
            to: "capability:git.commit".into(),
            event_id: "e-approve".into(),
            occurred_at: "2026-08-18T10:00:05Z".into(),
            actor: "alice".into(),
        });
        state.edges.push(GraphEdge {
            from: "cycle:c-1".into(),
            relation: "entered_phase".into(),
            to: "phase:verify".into(),
            event_id: "e-phase".into(),
            occurred_at: "2026-08-18T10:00:01Z".into(),
            actor: "sys".into(),
        });
        state
    }

    #[test]
    fn builtin_views_registered() {
        let views = builtin_views();
        assert!(views.len() >= 6);
        let ids: BTreeSet<String> = views.iter().map(|v| v.view_id.clone()).collect();
        assert_eq!(ids.len(), views.len(), "view ids must be unique");
        for id in [
            "overview",
            "graph",
            "timeline",
            "verification",
            "evidence",
            "release",
        ] {
            assert!(ids.contains(id), "missing view {id}");
        }
    }

    #[test]
    fn descriptor_serde_roundtrip() {
        let view = find_builtin_view("graph").unwrap();
        let json = serde_json::to_string(&view).unwrap();
        let back: ViewDescriptor = serde_json::from_str(&json).unwrap();
        assert_eq!(back, view);
        assert_eq!(back.layout, ViewLayout::Graph);
        assert_eq!(back.max_depth, 2);
    }

    #[test]
    fn graph_view_filters_by_entity_depth() {
        let state = sample_state();
        let view = find_builtin_view("graph").unwrap();
        let model = render_view_model(&state, &view, Some("cycle:c-1"));
        // cycle:c-1 → phase:verify at depth 1; actor/capability not reachable from cycle.
        assert!(model.nodes.iter().any(|n| n.key == "cycle:c-1"));
        assert!(model.nodes.iter().any(|n| n.key == "phase:verify"));
        assert!(model.edges.iter().any(|e| e.relation == "entered_phase"));
    }

    #[test]
    fn graph_view_unbounded_when_max_depth_zero() {
        let state = sample_state();
        let view = ViewDescriptor {
            view_id: "graph".into(),
            title: "Graph".into(),
            node_types: vec![],
            layout: ViewLayout::Graph,
            provenance: true,
            max_depth: 0,
        };
        let model = render_view_model(&state, &view, Some("actor:alice"));
        // actor:alice → capability:git.commit reachable at depth 1.
        assert!(model.nodes.iter().any(|n| n.key == "capability:git.commit"));
    }

    #[test]
    fn timeline_orders_events() {
        let state = sample_state();
        let view = find_builtin_view("timeline").unwrap();
        let model = render_view_model(&state, &view, None);
        assert_eq!(model.timeline.len(), 2);
        // e-phase (10:00:01) before e-approve (10:00:05).
        assert_eq!(model.timeline[0].event_id, "e-phase");
        assert_eq!(model.timeline[1].event_id, "e-approve");
        assert_eq!(model.timeline[0].sequence, 1);
        assert_eq!(model.timeline[1].sequence, 2);
    }

    #[test]
    fn verification_reuses_staleness() {
        let mut state = sample_state();
        // Add verification edge + later change on capability.
        state.edges.push(GraphEdge {
            from: "test:T1".into(),
            relation: "verifies".into(),
            to: "requirement:R1".into(),
            event_id: "e-verify".into(),
            occurred_at: "2026-08-18T10:00:00Z".into(),
            actor: "t".into(),
        });
        state.nodes.insert(
            "test:T1".into(),
            GraphNode {
                kind: "test".into(),
                id: "T1".into(),
                created_by: "e-t".into(),
                content_hash: "sha256:t".into(),
                occurred_at: "2026-08-18T10:00:00Z".into(),
            },
        );
        state.nodes.insert(
            "requirement:R1".into(),
            GraphNode {
                kind: "requirement".into(),
                id: "R1".into(),
                created_by: "e-r".into(),
                content_hash: "sha256:r".into(),
                occurred_at: "2026-08-18T10:00:00Z".into(),
            },
        );
        state.edges.push(GraphEdge {
            from: "requirement:R1".into(),
            relation: "modified".into(),
            to: "requirement:R1".into(),
            event_id: "e-change".into(),
            occurred_at: "2026-08-18T11:00:00Z".into(),
            actor: "t".into(),
        });

        let view = find_builtin_view("verification").unwrap();
        let model = render_view_model(&state, &view, Some("requirement:R1"));
        let staleness = model.staleness.expect("staleness present");
        assert!(staleness.causal_path.contains(&"e-change".to_string()));
    }

    #[test]
    fn same_entity_multiple_views_no_duplication() {
        let state = sample_state();
        let graph_view = find_builtin_view("graph").unwrap();
        let list_view = find_builtin_view("overview").unwrap();
        let graph_model = render_view_model(&state, &graph_view, Some("cycle:c-1"));
        let list_model = render_view_model(&state, &list_view, Some("cycle:c-1"));
        // Both derive from the same state; the graph view is a projection.
        assert!(graph_model.nodes.iter().any(|n| n.key == "cycle:c-1"));
        assert!(list_model.nodes.iter().any(|n| n.key == "cycle:c-1"));
        // No mutation of the source state happened.
        assert_eq!(state.nodes.len(), 4);
    }
}
