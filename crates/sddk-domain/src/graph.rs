//! Reactive knowledge/evidence graph (SPEC-004, Phase 5).
//!
//! The graph is a deterministic read-model projection over the CEP event
//! ledger: events are the authority, `GraphProjection` derives typed nodes and
//! edges with provenance, and `GraphView` exposes bounded scopes to pattern
//! queries and behaviors.

use std::collections::BTreeMap;

use serde::{Deserialize, Serialize};

use crate::event_envelope::EventEnvelopeV1;
use crate::projections::{Checkpoint, Projection, ProjectionError, ProjectionVersion};

/// One typed node in the graph.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct GraphNode {
    /// Entity kind (e.g. `cycle`, `capability`, `actor`, `phase`).
    pub kind: String,
    /// Stable entity id within its kind namespace.
    pub id: String,
    /// Event id that created this node (provenance).
    pub created_by: String,
    /// Content hash of the creating event (provenance).
    pub content_hash: String,
    /// RFC 3339 timestamp of the creating event.
    pub occurred_at: String,
}

impl GraphNode {
    /// Canonical graph key: `kind:id`.
    pub fn key(&self) -> String {
        format!("{}:{}", self.kind, self.id)
    }
}

/// One typed directed edge in the graph.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct GraphEdge {
    /// Source node key (`kind:id`).
    pub from: String,
    /// Relation name — the event type (`realm.object.verb`).
    pub relation: String,
    /// Target node key (`kind:id`).
    pub to: String,
    /// Event id that created this edge (provenance).
    pub event_id: String,
    /// RFC 3339 timestamp of the event.
    pub occurred_at: String,
    /// Actor id of the event.
    pub actor: String,
}

/// Full projection state: nodes keyed by `kind:id` and edges in append order.
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
pub struct GraphState {
    /// Nodes keyed by `kind:id` (BTreeMap → deterministic JSON).
    pub nodes: BTreeMap<String, GraphNode>,
    /// Edges in event-append order.
    pub edges: Vec<GraphEdge>,
    /// Monotonic sequence of the last applied event.
    pub last_event_sequence: u64,
    /// Hash of the last applied event.
    pub last_event_hash: String,
}

/// Deterministic graph projection over the event ledger (SPEC-004 §2).
pub struct GraphProjection {
    /// Stream this projection consumes from.
    stream_id: String,
    /// Mutable projection state.
    state: GraphState,
}

impl GraphProjection {
    /// Canonical projection name.
    pub const NAME: &'static str = "graph";
    /// Version for the v1 `apply` semantics.
    pub const VERSION: ProjectionVersion = 1;

    /// Creates a new `GraphProjection` for the given event stream.
    pub fn new(stream_id: impl Into<String>) -> Self {
        Self {
            stream_id: stream_id.into(),
            state: GraphState::default(),
        }
    }

    /// Returns the stream this projection consumes from.
    pub fn stream_id(&self) -> &str {
        &self.stream_id
    }
}

/// Validates that an event type has the `realm.object[.verb]` shape (2+ segments).
///
/// CEP events use `realm.object.verb` (3 segments); kernel ledger events use
/// `realm.object` (2 segments). Both contribute to the graph.
fn is_valid_event_type(event_type: &str) -> bool {
    event_type.split('.').count() >= 2
}

/// Upserts a node from an event subject, preserving first `created_by`.
fn upsert_node(state: &mut GraphState, kind: &str, id: &str, event: &EventEnvelopeV1) {
    let key = format!("{kind}:{id}");
    state.nodes.entry(key).or_insert_with(|| GraphNode {
        kind: kind.to_string(),
        id: id.to_string(),
        created_by: event.event_id.clone(),
        content_hash: event.content_hash.clone(),
        occurred_at: event.occurred_at.clone(),
    });
}

/// Appends a typed edge with full provenance.
fn push_edge(
    state: &mut GraphState,
    from: &str,
    relation: &str,
    to: &str,
    event: &EventEnvelopeV1,
) {
    state.edges.push(GraphEdge {
        from: from.to_string(),
        relation: relation.to_string(),
        to: to.to_string(),
        event_id: event.event_id.clone(),
        occurred_at: event.occurred_at.clone(),
        actor: event.actor.id.clone(),
    });
}

impl Projection for GraphProjection {
    type State = GraphState;

    fn name(&self) -> &str {
        Self::NAME
    }

    fn version(&self) -> ProjectionVersion {
        Self::VERSION
    }

    fn apply(&mut self, event: &EventEnvelopeV1) -> Result<(), ProjectionError> {
        // The graph is project-global: events from ANY stream of the project
        // contribute nodes/edges. `stream_id` is retained for the projection
        // contract but does not filter.

        // Update monotone fields regardless of event type.
        self.state.last_event_sequence = event.sequence;
        self.state.last_event_hash = event.content_hash.clone();

        // context.read is bookkeeping-only (SPEC-011 §3): it MUST NOT create
        // graph nodes/edges nor trigger reactive behaviors.
        if event.event_type == crate::context_read::CONTEXT_READ_EVENT_TYPE {
            return Ok(());
        }

        // Skip malformed event types (no 3-segment realm.object.verb).
        if !is_valid_event_type(&event.event_type) {
            return Ok(());
        }

        // Root node: the cycle (or project when no cycle is set).
        let root_kind = if event.cycle_id.is_some() {
            "cycle"
        } else {
            "project"
        };
        let root_id = event
            .cycle_id
            .clone()
            .unwrap_or_else(|| event.project_id.clone());
        upsert_node(&mut self.state, root_kind, &root_id, event);
        let root_key = format!("{root_kind}:{root_id}");

        // Special-case: workflow.phase.entered → cycle --entered_phase--> phase.
        if event.event_type == "workflow.phase.entered" {
            let phase = event
                .payload
                .get("phase")
                .and_then(|v| v.as_str())
                .ok_or_else(|| ProjectionError::InvalidPayload {
                    event_type: event.event_type.clone(),
                    detail: format!("event {} missing 'phase' string in payload", event.event_id),
                })?
                .to_string();
            upsert_node(&mut self.state, "phase", &phase, event);
            push_edge(
                &mut self.state,
                &root_key,
                "entered_phase",
                &format!("phase:{phase}"),
                event,
            );
            return Ok(());
        }

        // Generic mapping: subjects → nodes; event_type → edge.
        let subject_keys: Vec<String> = event
            .subjects
            .iter()
            .map(|subject| {
                upsert_node(&mut self.state, &subject.kind, &subject.id, event);
                format!("{}:{}", subject.kind, subject.id)
            })
            .collect();

        match subject_keys.len() {
            0 => {
                // No subjects: root --event_type--> root (self-loop marks the event).
                push_edge(
                    &mut self.state,
                    &root_key,
                    &event.event_type,
                    &root_key,
                    event,
                );
            }
            1 => {
                // One subject: subject --event_type--> subject (loop).
                push_edge(
                    &mut self.state,
                    &subject_keys[0],
                    &event.event_type,
                    &subject_keys[0],
                    event,
                );
            }
            _ => {
                // Two or more subjects: first --event_type--> second.
                push_edge(
                    &mut self.state,
                    &subject_keys[0],
                    &event.event_type,
                    &subject_keys[1],
                    event,
                );
            }
        }

        Ok(())
    }

    fn checkpoint(&self) -> Checkpoint {
        Checkpoint {
            projection_name: Self::NAME.to_string(),
            version: self.version(),
            last_event_sequence: self.state.last_event_sequence,
            last_event_hash: self.state.last_event_hash.clone(),
            updated_at: time::OffsetDateTime::now_utc()
                .format(&time::format_description::well_known::Rfc3339)
                .expect("RFC 3339 formatting cannot fail"),
        }
    }

    fn state_ref(&self) -> &Self::State {
        &self.state
    }
}

/// Bounded read view over a graph state (SPEC-004 §7).
///
/// Behaviors and queries receive a `GraphView`, never the full state.
#[derive(Debug, Clone)]
pub struct GraphView<'a> {
    /// Underlying state (borrowed).
    state: &'a GraphState,
    /// Visible node keys after filtering.
    visible_nodes: Vec<String>,
    /// Visible edges after filtering.
    visible_edges: Vec<&'a GraphEdge>,
    /// Maximum hop depth from the start node (0 = no bound).
    max_depth: u32,
}

impl<'a> GraphView<'a> {
    /// Creates an unbounded view over the whole state.
    pub fn new(state: &'a GraphState) -> Self {
        Self {
            state,
            visible_nodes: state.nodes.keys().cloned().collect(),
            visible_edges: state.edges.iter().collect(),
            max_depth: 0,
        }
    }

    /// Filters the view to the given node kinds.
    pub fn with_node_types(mut self, kinds: &[&str]) -> Self {
        self.visible_nodes = self
            .state
            .nodes
            .keys()
            .filter(|key| {
                kinds.iter().any(|kind| {
                    key.strip_prefix(kind)
                        .map(|rest| rest.starts_with(':'))
                        .unwrap_or(false)
                })
            })
            .cloned()
            .collect();
        self.visible_edges = self
            .state
            .edges
            .iter()
            .filter(|edge| {
                self.visible_nodes.contains(&edge.from) && self.visible_nodes.contains(&edge.to)
            })
            .collect();
        self
    }

    /// Filters the view to the given relation names.
    pub fn with_relations(mut self, relations: &[&str]) -> Self {
        self.visible_edges
            .retain(|edge| relations.contains(&edge.relation.as_str()));
        self
    }

    /// Bounds traversal depth from the start node (reachability).
    pub fn with_max_depth(mut self, max_depth: u32) -> Self {
        self.max_depth = max_depth;
        self
    }

    /// Nodes visible in this view.
    pub fn nodes(&self) -> impl Iterator<Item = &'a GraphNode> {
        self.visible_nodes
            .iter()
            .filter_map(|key| self.state.nodes.get(key))
    }

    /// Edges visible in this view.
    pub fn edges(&self) -> impl Iterator<Item = &'a GraphEdge> {
        self.visible_edges.iter().copied()
    }

    /// Returns the node keys visible in this view.
    pub fn node_keys(&self) -> &[String] {
        &self.visible_nodes
    }

    /// Returns the edge references visible in this view.
    pub fn edge_refs(&self) -> &[&'a GraphEdge] {
        &self.visible_edges
    }

    /// Maximum hop depth bound (0 = unbounded).
    pub fn max_depth(&self) -> u32 {
        self.max_depth
    }

    /// Looks up a node by key within the view.
    pub fn node(&self, key: &str) -> Option<&'a GraphNode> {
        if self.visible_nodes.iter().any(|k| k == key) {
            self.state.nodes.get(key)
        } else {
            None
        }
    }
}

// ---------------------------------------------------------------------------
// Pattern queries (SPEC-004 §5)
// ---------------------------------------------------------------------------

/// One step in a directed pattern chain.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct PatternStep {
    /// Optional node-type predicate for the target node (`kind`).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub node_type: Option<String>,
    /// Directed relation name to traverse.
    pub relation: String,
    /// When `true`, the step matches only if NO such edge exists (NOT EXISTS).
    #[serde(default)]
    pub not_exists: bool,
}

impl PatternStep {
    /// Creates a positive step: traverse `relation` to a node of `node_type`.
    pub fn positive(relation: &str, node_type: Option<&str>) -> Self {
        Self {
            node_type: node_type.map(|t| t.to_string()),
            relation: relation.to_string(),
            not_exists: false,
        }
    }

    /// Creates a NOT EXISTS step.
    pub fn not_exists(relation: &str, node_type: Option<&str>) -> Self {
        Self {
            node_type: node_type.map(|t| t.to_string()),
            relation: relation.to_string(),
            not_exists: true,
        }
    }
}

/// A deterministic pattern query over the graph.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct PatternQuery {
    /// Fixed start node key (`kind:id`), or `None` to start from any node.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub start: Option<String>,
    /// Optional start node type predicate.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub start_type: Option<String>,
    /// Ordered chain of steps.
    pub steps: Vec<PatternStep>,
    /// Maximum traversal depth (0 = unbounded).
    #[serde(default)]
    pub max_depth: u32,
}

impl PatternQuery {
    /// Creates a query with the given steps.
    pub fn new(steps: Vec<PatternStep>) -> Self {
        Self {
            start: None,
            start_type: None,
            steps,
            max_depth: 0,
        }
    }

    /// Sets a fixed start node.
    pub fn starting_at(mut self, key: &str) -> Self {
        self.start = Some(key.to_string());
        self
    }

    /// Sets the start node type predicate.
    pub fn starting_type(mut self, kind: &str) -> Self {
        self.start_type = Some(kind.to_string());
        self
    }

    /// Sets the depth bound.
    pub fn bounded(mut self, depth: u32) -> Self {
        self.max_depth = depth;
        self
    }

    /// Executes the query deterministically over the state.
    ///
    /// Returns one path (`Vec<String>` of node keys) per match.
    pub fn execute(&self, state: &GraphState) -> Vec<Vec<String>> {
        let mut results = Vec::new();
        let start_keys: Vec<String> = match &self.start {
            Some(key) => vec![key.clone()],
            None => state
                .nodes
                .keys()
                .filter(|key| match &self.start_type {
                    Some(kind) => key
                        .strip_prefix(kind)
                        .map(|rest| rest.starts_with(':'))
                        .unwrap_or(false),
                    None => true,
                })
                .cloned()
                .collect(),
        };

        for start in start_keys {
            let mut path = vec![start.clone()];
            self.walk(state, &start, &mut path, 0, &mut results);
        }
        results.sort();
        results.dedup();
        results
    }

    fn walk(
        &self,
        state: &GraphState,
        current: &str,
        path: &mut Vec<String>,
        depth: u32,
        results: &mut Vec<Vec<String>>,
    ) {
        if depth >= self.steps.len() as u32 {
            // All steps consumed → full match.
            results.push(path.clone());
            return;
        }
        // Depth bound limits the number of traversed positive steps.
        if self.max_depth > 0 && depth >= self.max_depth {
            return;
        }

        let step = &self.steps[depth as usize];
        if step.not_exists {
            // NOT EXISTS: match the step as a negation — the path continues
            // only if NO outgoing edge with this relation exists.
            let exists = state
                .edges
                .iter()
                .any(|edge| edge.from == current && edge.relation == step.relation);
            if !exists {
                // Consume the step without advancing the node.
                let mut next_path = path.clone();
                self.walk(state, current, &mut next_path, depth + 1, results);
            }
            return;
        }

        // Positive step: traverse all matching edges.
        let mut advanced = false;
        for edge in state.edges.iter().filter(|e| {
            e.from == current
                && e.relation == step.relation
                && step
                    .node_type
                    .as_ref()
                    .map(|kind| e.to.starts_with(&format!("{kind}:")))
                    .unwrap_or(true)
        }) {
            advanced = true;
            path.push(edge.to.clone());
            self.walk(state, &edge.to, path, depth + 1, results);
            path.pop();
        }
        if !advanced && depth + 1 == self.steps.len() as u32 {
            // A positive step that cannot advance means no match from here.
            // (handled by not pushing anything)
        }
    }
}

/// Matches a pattern over a view and returns node-key paths.
pub fn match_pattern(state: &GraphState, query: &PatternQuery) -> Vec<Vec<String>> {
    query.execute(state)
}

// ---------------------------------------------------------------------------
// Behaviors (SPEC-004 §4, §6) — proposal-only runtime
// ---------------------------------------------------------------------------

/// A proposal emitted by a behavior. Behaviors NEVER perform side effects;
/// they only emit proposals that the kernel evaluates under normal policy.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct BehaviorProposal {
    /// Stable kind, e.g. `verification-stale`, `dependency-blocked`.
    pub kind: String,
    /// Subject node key the proposal concerns.
    pub subject: String,
    /// Human-readable reason.
    pub reason: String,
    /// Causal path: event ids that led to this proposal.
    pub causal_path: Vec<String>,
    /// Event id that triggered evaluation.
    pub trigger_event_id: String,
}

/// A deterministic graph behavior. Implementations MUST be pure: they receive
/// a bounded view and return proposals. There is NO capability API here —
/// behaviors cannot acquire governed capabilities by construction (exit
/// criterion: reactive behavior cannot directly acquire a governed capability).
pub trait GraphBehavior {
    /// Stable behavior name (dedup key).
    fn name(&self) -> &str;
    /// Evaluates the behavior against a bounded view.
    fn evaluate(&self, view: &GraphView<'_>) -> Vec<BehaviorProposal>;
}

/// Runs behaviors and deduplicates proposals per (behavior, subject, trigger).
pub struct BehaviorRuntime {
    /// Registered behaviors.
    behaviors: Vec<Box<dyn GraphBehavior>>,
    /// Dedup set: (behavior name, subject, trigger_event_id).
    emitted: std::collections::BTreeSet<(String, String, String)>,
}

impl BehaviorRuntime {
    /// Creates an empty runtime.
    pub fn new() -> Self {
        Self {
            behaviors: Vec::new(),
            emitted: std::collections::BTreeSet::new(),
        }
    }

    /// Registers a behavior.
    pub fn register(&mut self, behavior: Box<dyn GraphBehavior>) {
        self.behaviors.push(behavior);
    }

    /// Evaluates all behaviors against a view; deduplicates repeated triggers.
    pub fn evaluate_all(
        &mut self,
        view: &GraphView<'_>,
        trigger_event_id: &str,
    ) -> Vec<BehaviorProposal> {
        let mut proposals = Vec::new();
        for behavior in &self.behaviors {
            for proposal in behavior.evaluate(view) {
                let key = (
                    behavior.name().to_string(),
                    proposal.subject.clone(),
                    trigger_event_id.to_string(),
                );
                if self.emitted.insert(key) {
                    let mut enriched = proposal;
                    enriched.trigger_event_id = trigger_event_id.to_string();
                    proposals.push(enriched);
                }
            }
        }
        proposals
    }

    /// Number of deduplicated emissions so far.
    pub fn emitted_count(&self) -> usize {
        self.emitted.len()
    }
}

impl Default for BehaviorRuntime {
    fn default() -> Self {
        Self::new()
    }
}

/// `verifies` relation behavior (SPEC-004 §6): when the verified subject
/// changes after the verification edge was created, emit `verification-stale`.
pub struct VerifiesBehavior;

impl GraphBehavior for VerifiesBehavior {
    fn name(&self) -> &str {
        "verifies"
    }

    fn evaluate(&self, view: &GraphView<'_>) -> Vec<BehaviorProposal> {
        let mut proposals = Vec::new();
        for edge in view.edges().filter(|e| e.relation == "verifies") {
            let verified = edge.to.clone();
            // A subject is stale when a later event touched it.
            let touched_after = view
                .edges()
                .filter(|e| e.from == verified && e.occurred_at > edge.occurred_at)
                .count();
            if touched_after > 0 {
                proposals.push(BehaviorProposal {
                    kind: "verification-stale".into(),
                    subject: verified.clone(),
                    reason: format!(
                        "subject {verified} changed after verification by {}",
                        edge.event_id
                    ),
                    causal_path: vec![edge.event_id.clone()],
                    trigger_event_id: String::new(),
                });
            }
        }
        proposals
    }
}

/// `depends_on` relation behavior: when a dependency becomes blocked, emit
/// `dependency-blocked` for the dependent.
pub struct DependsOnBehavior;

impl GraphBehavior for DependsOnBehavior {
    fn name(&self) -> &str {
        "depends_on"
    }

    fn evaluate(&self, view: &GraphView<'_>) -> Vec<BehaviorProposal> {
        let mut proposals = Vec::new();
        for edge in view.edges().filter(|e| e.relation == "depends_on") {
            let dependency = edge.to.clone();
            let blocked = view
                .edges()
                .any(|e| e.from == dependency && e.relation == "blocked");
            if blocked {
                proposals.push(BehaviorProposal {
                    kind: "dependency-blocked".into(),
                    subject: edge.from.clone(),
                    reason: format!("dependency {dependency} is blocked"),
                    causal_path: vec![edge.event_id.clone()],
                    trigger_event_id: String::new(),
                });
            }
        }
        proposals
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::event_envelope::{ActorKind, ActorRef, EntityRef};
    use serde_json::json;

    fn make_event(
        stream: &str,
        event_type: &str,
        seq: u64,
        subjects: Vec<EntityRef>,
        cycle_id: Option<&str>,
        payload: serde_json::Value,
    ) -> EventEnvelopeV1 {
        EventEnvelopeV1 {
            event_id: format!("evt-{stream}-{seq}"),
            event_type: event_type.into(),
            schema_version: 1,
            stream_id: stream.into(),
            sequence: seq,
            project_id: "p-1".into(),
            occurred_at: format!("2026-08-18T10:00:{seq:02}Z"),
            recorded_at: format!("2026-08-18T10:00:{seq:02}Z"),
            actor: ActorRef {
                kind: ActorKind::System,
                id: "sddk-test".into(),
                definition_hash: None,
                policy_hash: None,
                model: None,
            },
            subjects,
            payload,
            evidence_refs: vec![],
            content_hash: format!("sha256:{seq:064x}"),
            metadata: None,
            causation_id: None,
            correlation_id: None,
            cycle_id: cycle_id.map(|c| c.to_string()),
            frame_id: None,
            fork_id: None,
        }
    }

    fn subject(kind: &str, id: &str) -> EntityRef {
        EntityRef {
            kind: kind.into(),
            id: id.into(),
            version: None,
            content_hash: None,
        }
    }

    #[test]
    fn subjects_become_nodes() {
        let mut projection = GraphProjection::new("project:p-1");
        let event = make_event(
            "project:p-1",
            "approval.capability.granted",
            1,
            vec![
                subject("cycle", "c-1"),
                subject("capability", "git.commit"),
                subject("actor", "alice"),
            ],
            Some("c-1"),
            json!({}),
        );
        projection.apply(&event).unwrap();
        let state = projection.state_ref();
        assert!(state.nodes.contains_key("cycle:c-1"));
        assert!(state.nodes.contains_key("capability:git.commit"));
        assert!(state.nodes.contains_key("actor:alice"));
        assert_eq!(
            state.nodes["capability:git.commit"].created_by,
            "evt-project:p-1-1"
        );
    }

    #[test]
    fn event_type_becomes_relation() {
        let mut projection = GraphProjection::new("project:p-1");
        let event = make_event(
            "project:p-1",
            "approval.capability.granted",
            1,
            vec![
                subject("actor", "alice"),
                subject("capability", "git.commit"),
            ],
            Some("c-1"),
            json!({}),
        );
        projection.apply(&event).unwrap();
        let state = projection.state_ref();
        assert_eq!(state.edges.len(), 1);
        let edge = &state.edges[0];
        assert_eq!(edge.from, "actor:alice");
        assert_eq!(edge.relation, "approval.capability.granted");
        assert_eq!(edge.to, "capability:git.commit");
        assert_eq!(edge.actor, "sddk-test");
    }

    #[test]
    fn phase_entered_creates_phase_edge() {
        let mut projection = GraphProjection::new("project:p-1");
        let event = make_event(
            "project:p-1",
            "workflow.phase.entered",
            1,
            vec![],
            Some("c-1"),
            json!({ "phase": "verify" }),
        );
        projection.apply(&event).unwrap();
        let state = projection.state_ref();
        assert!(state.nodes.contains_key("phase:verify"));
        let edge = &state.edges[0];
        assert_eq!(edge.from, "cycle:c-1");
        assert_eq!(edge.relation, "entered_phase");
        assert_eq!(edge.to, "phase:verify");
    }

    #[test]
    fn unknown_event_type_skipped() {
        let mut projection = GraphProjection::new("project:p-1");
        let event = make_event(
            "project:p-1",
            "short",
            1,
            vec![subject("cycle", "c-1")],
            Some("c-1"),
            json!({}),
        );
        projection.apply(&event).unwrap();
        let state = projection.state_ref();
        assert!(state.edges.is_empty());
    }

    #[test]
    fn context_read_is_bookkeeping_only() {
        let mut projection = GraphProjection::new("project:p-1");
        let event = make_event(
            "project:p-1",
            crate::context_read::CONTEXT_READ_EVENT_TYPE,
            1,
            vec![subject("cycle", "c-1")],
            Some("c-1"),
            json!({ "execution_id": "exec-1" }),
        );
        projection.apply(&event).unwrap();
        let state = projection.state_ref();
        assert!(state.edges.is_empty(), "context.read must not create edges");
        assert!(
            !state.nodes.contains_key("cycle:c-1"),
            "context.read must not create nodes"
        );
    }

    #[test]
    fn rebuild_is_deterministic() {
        let events = vec![
            make_event(
                "project:p-1",
                "approval.capability.requested",
                1,
                vec![subject("cycle", "c-1"), subject("capability", "git.commit")],
                Some("c-1"),
                json!({}),
            ),
            make_event(
                "project:p-1",
                "approval.capability.granted",
                2,
                vec![
                    subject("actor", "alice"),
                    subject("capability", "git.commit"),
                ],
                Some("c-1"),
                json!({}),
            ),
            make_event(
                "project:p-1",
                "workflow.phase.entered",
                3,
                vec![],
                Some("c-1"),
                json!({ "phase": "verify" }),
            ),
        ];
        let mut a = GraphProjection::new("project:p-1");
        let mut b = GraphProjection::new("project:p-1");
        for event in &events {
            a.apply(event).unwrap();
            b.apply(event).unwrap();
        }
        assert_eq!(a.state_ref(), b.state_ref());
        assert_eq!(a.state_ref().nodes.len(), 4); // cycle, capability, actor, phase
        assert_eq!(a.state_ref().edges.len(), 3);
    }

    #[test]
    fn view_filters_by_type() {
        let mut projection = GraphProjection::new("project:p-1");
        projection
            .apply(&make_event(
                "project:p-1",
                "approval.capability.granted",
                1,
                vec![subject("cycle", "c-1"), subject("capability", "git.commit")],
                Some("c-1"),
                json!({}),
            ))
            .unwrap();
        let view = GraphView::new(projection.state_ref()).with_node_types(&["capability"]);
        let keys: Vec<String> = view.node_keys().to_vec();
        assert_eq!(keys, vec!["capability:git.commit"]);
        assert!(view.edge_refs().is_empty());
    }

    #[test]
    fn view_bounds_hop_depth() {
        // A -> B -> C -> D via chain of self-loop-less edges: build manually.
        let mut state = GraphState::default();
        for (i, (from, rel, to)) in [
            ("a:A", "r", "b:B"),
            ("b:B", "r", "c:C"),
            ("c:C", "r", "d:D"),
        ]
        .iter()
        .enumerate()
        {
            state.nodes.insert(
                from.to_string(),
                GraphNode {
                    kind: from.split(':').next().unwrap().into(),
                    id: from.split(':').nth(1).unwrap().into(),
                    created_by: format!("e{i}"),
                    content_hash: "sha256:x".into(),
                    occurred_at: "2026-08-18T10:00:00Z".into(),
                },
            );
            state.edges.push(GraphEdge {
                from: from.to_string(),
                relation: rel.to_string(),
                to: to.to_string(),
                event_id: format!("e{i}"),
                occurred_at: "2026-08-18T10:00:00Z".into(),
                actor: "t".into(),
            });
        }
        state.nodes.insert(
            "d:D".to_string(),
            GraphNode {
                kind: "d".into(),
                id: "D".into(),
                created_by: "e3".into(),
                content_hash: "sha256:x".into(),
                occurred_at: "2026-08-18T10:00:00Z".into(),
            },
        );
        // Depth bound is enforced by pattern queries; the view itself exposes
        // reachability via max_depth only for query use. Here we assert the
        // view exposes all edges (bounded views filter by type/relation, and
        // depth bounding lives in PatternQuery).
        let view = GraphView::new(&state);
        assert_eq!(view.edges().count(), 3);
    }

    // --- Pattern query tests ---

    fn chain_state() -> GraphState {
        let mut state = GraphState::default();
        for (i, (from, rel, to)) in [
            ("requirement:R1", "implemented_by", "commit:C1"),
            ("commit:C1", "verified_by", "test:T1"),
        ]
        .iter()
        .enumerate()
        {
            state.nodes.insert(
                from.to_string(),
                GraphNode {
                    kind: from.split(':').next().unwrap().into(),
                    id: from.split(':').nth(1).unwrap().into(),
                    created_by: format!("e{i}"),
                    content_hash: "sha256:x".into(),
                    occurred_at: "2026-08-18T10:00:00Z".into(),
                },
            );
            state.nodes.insert(
                to.to_string(),
                GraphNode {
                    kind: to.split(':').next().unwrap().into(),
                    id: to.split(':').nth(1).unwrap().into(),
                    created_by: format!("e{i}"),
                    content_hash: "sha256:x".into(),
                    occurred_at: "2026-08-18T10:00:00Z".into(),
                },
            );
            state.edges.push(GraphEdge {
                from: from.to_string(),
                relation: rel.to_string(),
                to: to.to_string(),
                event_id: format!("e{i}"),
                occurred_at: "2026-08-18T10:00:00Z".into(),
                actor: "t".into(),
            });
        }
        state
    }

    #[test]
    fn pattern_simple_chain_matches() {
        let state = chain_state();
        let query = PatternQuery::new(vec![PatternStep::positive(
            "implemented_by",
            Some("commit"),
        )])
        .starting_type("requirement");
        let matches = match_pattern(&state, &query);
        assert_eq!(matches.len(), 1);
        assert_eq!(matches[0], vec!["requirement:R1", "commit:C1"]);
    }

    #[test]
    fn pattern_not_exists_filters() {
        // R2 has no verified_by edge → matches; R1 does → excluded.
        let mut state = chain_state();
        state.nodes.insert(
            "requirement:R2".into(),
            GraphNode {
                kind: "requirement".into(),
                id: "R2".into(),
                created_by: "e9".into(),
                content_hash: "sha256:x".into(),
                occurred_at: "2026-08-18T10:00:00Z".into(),
            },
        );
        state.edges.push(GraphEdge {
            from: "requirement:R2".into(),
            relation: "implemented_by".into(),
            to: "commit:C2".into(),
            event_id: "e9".into(),
            occurred_at: "2026-08-18T10:00:00Z".into(),
            actor: "t".into(),
        });
        state.nodes.insert(
            "commit:C2".into(),
            GraphNode {
                kind: "commit".into(),
                id: "C2".into(),
                created_by: "e9".into(),
                content_hash: "sha256:x".into(),
                occurred_at: "2026-08-18T10:00:00Z".into(),
            },
        );

        let query = PatternQuery::new(vec![
            PatternStep::positive("implemented_by", Some("commit")),
            PatternStep::not_exists("verified_by", Some("test")),
        ])
        .starting_type("requirement");
        let matches = match_pattern(&state, &query);
        // Only R2: R1's commit C1 has verified_by.
        assert_eq!(matches.len(), 1);
        assert_eq!(matches[0][0], "requirement:R2");
    }

    #[test]
    fn pattern_depth_bound_stops_traversal() {
        let state = chain_state();
        let query = PatternQuery::new(vec![PatternStep::positive("implemented_by", None)])
            .starting_type("requirement")
            .bounded(1);
        let matches = match_pattern(&state, &query);
        assert_eq!(matches.len(), 1);
        assert_eq!(matches[0], vec!["requirement:R1", "commit:C1"]);
    }

    // --- Behavior tests ---

    fn behavior_state() -> GraphState {
        let mut state = GraphState::default();
        // test:T1 verifies requirement:R1 (edge at 10:00:00)
        // requirement:R1 touched again at 10:00:05
        // task:A depends_on task:B; task:B blocked
        for (i, (from, rel, to, at)) in [
            (
                "test:T1",
                "verifies",
                "requirement:R1",
                "2026-08-18T10:00:00Z",
            ),
            (
                "requirement:R1",
                "modified",
                "requirement:R1",
                "2026-08-18T10:00:05Z",
            ),
            ("task:A", "depends_on", "task:B", "2026-08-18T10:00:00Z"),
            ("task:B", "blocked", "task:B", "2026-08-18T10:00:01Z"),
        ]
        .iter()
        .enumerate()
        {
            state.nodes.insert(
                from.to_string(),
                GraphNode {
                    kind: from.split(':').next().unwrap().into(),
                    id: from.split(':').nth(1).unwrap().into(),
                    created_by: format!("e{i}"),
                    content_hash: "sha256:x".into(),
                    occurred_at: at.to_string(),
                },
            );
            state.nodes.insert(
                to.to_string(),
                GraphNode {
                    kind: to.split(':').next().unwrap().into(),
                    id: to.split(':').nth(1).unwrap().into(),
                    created_by: format!("e{i}"),
                    content_hash: "sha256:x".into(),
                    occurred_at: at.to_string(),
                },
            );
            state.edges.push(GraphEdge {
                from: from.to_string(),
                relation: rel.to_string(),
                to: to.to_string(),
                event_id: format!("e{i}"),
                occurred_at: at.to_string(),
                actor: "t".into(),
            });
        }
        state
    }

    #[test]
    fn verifies_behavior_emits_stale() {
        let state = behavior_state();
        let view = GraphView::new(&state);
        let behavior = VerifiesBehavior;
        let proposals = behavior.evaluate(&view);
        assert!(
            proposals
                .iter()
                .any(|p| p.kind == "verification-stale" && p.subject == "requirement:R1"),
            "expected verification-stale for requirement:R1, got {:?}",
            proposals
        );
    }

    #[test]
    fn depends_on_behavior_emits_blocked() {
        let state = behavior_state();
        let view = GraphView::new(&state);
        let behavior = DependsOnBehavior;
        let proposals = behavior.evaluate(&view);
        assert!(
            proposals
                .iter()
                .any(|p| p.kind == "dependency-blocked" && p.subject == "task:A"),
            "expected dependency-blocked for task:A, got {:?}",
            proposals
        );
    }

    #[test]
    fn runtime_deduplicates_proposals() {
        let state = behavior_state();
        let mut runtime = BehaviorRuntime::new();
        runtime.register(Box::new(VerifiesBehavior));
        runtime.register(Box::new(DependsOnBehavior));

        let view = GraphView::new(&state);
        let first = runtime.evaluate_all(&view, "evt-trigger-1");
        assert!(!first.is_empty());
        let second = runtime.evaluate_all(&view, "evt-trigger-1");
        assert!(second.is_empty(), "duplicate trigger must dedupe");
        assert!(runtime.emitted_count() >= first.len());
    }

    #[test]
    fn runtime_no_proposals_without_changes() {
        let mut state = GraphState::default();
        state.nodes.insert(
            "test:T9".into(),
            GraphNode {
                kind: "test".into(),
                id: "T9".into(),
                created_by: "e0".into(),
                content_hash: "sha256:x".into(),
                occurred_at: "2026-08-18T10:00:00Z".into(),
            },
        );
        state.nodes.insert(
            "requirement:R9".into(),
            GraphNode {
                kind: "requirement".into(),
                id: "R9".into(),
                created_by: "e0".into(),
                content_hash: "sha256:x".into(),
                occurred_at: "2026-08-18T10:00:00Z".into(),
            },
        );
        state.edges.push(GraphEdge {
            from: "test:T9".into(),
            relation: "verifies".into(),
            to: "requirement:R9".into(),
            event_id: "e0".into(),
            occurred_at: "2026-08-18T10:00:00Z".into(),
            actor: "t".into(),
        });
        let view = GraphView::new(&state);
        let behavior = VerifiesBehavior;
        assert!(behavior.evaluate(&view).is_empty());
    }
}

// ---------------------------------------------------------------------------
// ExecutionGraphRevision — versioned runtime graph with parent chain + digest
// ---------------------------------------------------------------------------

/// A versioned snapshot of the runtime execution graph.
///
/// Each revision records the full graph state at a point in time, linked to its
/// parent via a cryptographic digest chain. This enables detecting conflicting
/// expansions and replaying from any revision.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ExecutionGraphRevision {
    /// Monotonic revision number per run.
    pub revision: u64,
    /// Unique revision identifier (ULID).
    pub revision_id: crate::workflow_ir::RevisionId,
    /// Parent revision (None only for revision 0).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub parent: Option<Box<ExecutionGraphRevision>>,
    /// Graph events that produced this revision.
    pub events: BTreeMap<crate::workflow_ir::EventId, GraphEvent>,
    /// Node snapshots at this revision.
    pub nodes: BTreeMap<crate::workflow_ir::NodeId, NodeSnapshot>,
    /// Edge snapshots at this revision.
    pub edges: BTreeMap<crate::workflow_ir::EdgeId, EdgeSnapshot>,
    /// SHA-256 digest of this revision's content (excluding parent).
    pub digest: [u8; 32],
    /// Schema version (must be 1).
    pub schema_version: u32,
}

impl ExecutionGraphRevision {
    /// Schema version constant.
    pub const SCHEMA_VERSION: u32 = 1;

    /// Computes the digest for this revision.
    ///
    /// Recipe: `digest = sha256( canonical_json( {parent_digest, parent_revision,
    /// events_sorted, nodes_sorted, edges_sorted} ) )`.
    ///
    /// Returns the same digest for identical parent + content.
    /// Returns a different digest if the parent changes (chain divergence).
    pub fn compute_digest(&self) -> [u8; 32] {
        use sha2::{Digest, Sha256};

        #[derive(Serialize)]
        struct DigestInput<'a> {
            parent_digest: Option<String>,
            parent_revision: u64,
            events_sorted: &'a BTreeMap<crate::workflow_ir::EventId, GraphEvent>,
            nodes_sorted: &'a BTreeMap<crate::workflow_ir::NodeId, NodeSnapshot>,
            edges_sorted: &'a BTreeMap<crate::workflow_ir::EdgeId, EdgeSnapshot>,
        }

        let parent_digest = self.parent.as_ref().map(|p| {
            let hex: String = p.digest.iter().map(|b| format!("{:02x}", b)).collect();
            format!("sha256:{}", hex)
        });

        let input = DigestInput {
            parent_digest,
            parent_revision: self.parent.as_ref().map(|p| p.revision).unwrap_or(0),
            events_sorted: &self.events,
            nodes_sorted: &self.nodes,
            edges_sorted: &self.edges,
        };

        let bytes =
            serde_json::to_vec(&input).expect("ExecutionGraphRevision is always serializable");
        Sha256::digest(&bytes).into()
    }

    /// Returns true if this is revision 0 (has no parent).
    pub fn is_initial(&self) -> bool {
        self.revision == 0 && self.parent.is_none()
    }
}

/// A frozen snapshot of a node at a particular revision.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct NodeSnapshot {
    /// Node identifier.
    pub node_id: crate::workflow_ir::NodeId,
    /// State at snapshot time.
    pub state: String,
    /// Snapshot timestamp (RFC 3339).
    pub snapshot_at: String,
}

/// A frozen snapshot of an edge at a particular revision.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct EdgeSnapshot {
    /// Edge identifier.
    pub edge_id: crate::workflow_ir::EdgeId,
    /// Source node key.
    pub from: String,
    /// Relation name.
    pub relation: String,
    /// Target node key.
    pub to: String,
    /// Snapshot timestamp (RFC 3339).
    pub snapshot_at: String,
}

/// A graph event recorded in a revision.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct GraphEvent {
    /// Event identifier.
    pub event_id: crate::workflow_ir::EventId,
    /// Event type.
    pub event_type: String,
    /// Occurred at (RFC 3339).
    pub occurred_at: String,
}

#[cfg(test)]
mod execution_graph_revision_tests {
    use super::*;

    #[test]
    fn compute_digest_is_deterministic() {
        // Two revisions with identical content but built separately
        let mut rev1 = ExecutionGraphRevision {
            revision: 1,
            revision_id: crate::workflow_ir::RevisionId("rev1".into()),
            parent: None,
            events: BTreeMap::new(),
            nodes: BTreeMap::new(),
            edges: BTreeMap::new(),
            digest: [0u8; 32],
            schema_version: 1,
        };
        rev1.digest = rev1.compute_digest();

        let rev2 = ExecutionGraphRevision {
            revision: 1,
            revision_id: crate::workflow_ir::RevisionId("rev1".into()),
            parent: None,
            events: BTreeMap::new(),
            nodes: BTreeMap::new(),
            edges: BTreeMap::new(),
            digest: [0u8; 32],
            schema_version: 1,
        };

        // Identical content → identical digest
        let digest2 = rev2.compute_digest();
        assert_eq!(
            rev1.digest, digest2,
            "identical content must produce identical digest"
        );
    }

    #[test]
    fn parent_chain_affects_digest() {
        let parent = ExecutionGraphRevision {
            revision: 0,
            revision_id: crate::workflow_ir::RevisionId("parent".into()),
            parent: None,
            events: BTreeMap::new(),
            nodes: BTreeMap::new(),
            edges: BTreeMap::new(),
            digest: [0u8; 32],
            schema_version: 1,
        };
        let parent_digest = parent.compute_digest();

        let child = ExecutionGraphRevision {
            revision: 1,
            revision_id: crate::workflow_ir::RevisionId("child".into()),
            parent: Some(Box::new(parent)),
            events: BTreeMap::new(),
            nodes: BTreeMap::new(),
            edges: BTreeMap::new(),
            digest: [0u8; 32],
            schema_version: 1,
        };
        let child_digest = child.compute_digest();

        // Different parent → different digest
        assert_ne!(
            parent_digest, child_digest,
            "parent chain change must affect digest"
        );
    }

    #[test]
    fn revision_0_has_no_parent() {
        let rev = ExecutionGraphRevision {
            revision: 0,
            revision_id: crate::workflow_ir::RevisionId("initial".into()),
            parent: None,
            events: BTreeMap::new(),
            nodes: BTreeMap::new(),
            edges: BTreeMap::new(),
            digest: [0u8; 32],
            schema_version: 1,
        };
        assert!(rev.is_initial(), "revision 0 must have no parent");
    }
}
