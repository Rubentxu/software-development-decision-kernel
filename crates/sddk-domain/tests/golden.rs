//! Golden dataset runner (SPEC-014, Phase 9).
//!
//! Loads `fixtures/golden/*.yaml`, applies each case's events to a fresh
//! `GraphProjection`, and asserts the resulting node/edge counts match the
//! expectation. This is the deterministic regression ratchet for the graph.

use sddk_domain::{ActorKind, ActorRef, EntityRef, EventEnvelopeV1, GraphProjection, Projection};
use serde::Deserialize;
use serde_json::Value;
use std::path::PathBuf;

#[derive(Debug, Deserialize)]
struct GoldenCase {
    name: String,
    #[serde(default)]
    events: Vec<GoldenEvent>,
    expect: GoldenExpect,
}

#[derive(Debug, Deserialize)]
struct GoldenEvent {
    event_type: String,
    #[serde(default)]
    subjects: Vec<Vec<String>>,
    #[serde(default)]
    cycle_id: Option<String>,
    #[serde(default)]
    payload: Value,
}

#[derive(Debug, Deserialize)]
struct GoldenExpect {
    nodes: usize,
    edges: usize,
    /// Verify these specific node keys exist in the graph (optional).
    #[serde(default)]
    node_keys: Vec<String>,
    /// Verify these specific edges exist with exact from/relation/to (optional).
    #[serde(default)]
    edges_match: Vec<EdgeMatch>,
}

#[derive(Debug, Deserialize)]
struct EdgeMatch {
    from: String,
    relation: String,
    to: String,
}

fn golden_dir() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../fixtures/golden")
}

fn load_cases() -> Vec<GoldenCase> {
    let mut cases = Vec::new();
    let dir = golden_dir();
    let mut entries: Vec<_> = std::fs::read_dir(&dir)
        .unwrap_or_else(|e| panic!("golden dir {dir:?}: {e}"))
        .filter_map(|entry| entry.ok())
        .filter(|entry| {
            entry
                .path()
                .extension()
                .map(|e| e == "yaml")
                .unwrap_or(false)
        })
        .collect();
    entries.sort_by_key(|entry| entry.file_name());
    for entry in entries {
        let content = std::fs::read_to_string(entry.path())
            .unwrap_or_else(|e| panic!("read {}: {e}", entry.path().display()));
        let case: GoldenCase = serde_saphyr::from_str(&content)
            .unwrap_or_else(|e| panic!("parse {}: {e}", entry.path().display()));
        cases.push(case);
    }
    cases
}

fn apply_case(case: &GoldenCase) -> (usize, usize, sddk_domain::GraphState) {
    let mut projection = GraphProjection::new("project:golden");
    for (i, event) in case.events.iter().enumerate() {
        let envelope = EventEnvelopeV1 {
            event_id: format!("golden-{}-{i}", case.name),
            event_type: event.event_type.clone(),
            schema_version: 1,
            stream_id: "project:golden".into(),
            sequence: (i + 1) as u64,
            project_id: "project:golden".into(),
            occurred_at: format!("2026-08-18T10:00:{i:02}Z"),
            recorded_at: format!("2026-08-18T10:00:{i:02}Z"),
            actor: ActorRef {
                kind: ActorKind::System,
                id: "golden".into(),
                definition_hash: None,
                policy_hash: None,
                model: None,
            },
            subjects: event
                .subjects
                .iter()
                .map(|s| EntityRef {
                    kind: s[0].clone(),
                    id: s[1].clone(),
                    version: None,
                    content_hash: None,
                })
                .collect(),
            payload: if event.payload.is_null() {
                Value::Null
            } else {
                event.payload.clone()
            },
            evidence_refs: vec![],
            content_hash: format!("sha256:{i:064x}"),
            metadata: None,
            causation_id: None,
            correlation_id: None,
            cycle_id: event.cycle_id.clone(),
            frame_id: None,
            fork_id: None,
        };
        projection.apply(&envelope).unwrap();
    }
    let state = projection.state_ref().clone();
    (state.nodes.len(), state.edges.len(), state)
}

#[test]
fn golden_dataset_matches_expectations() {
    let cases = load_cases();
    assert!(
        cases.len() >= 10,
        "golden dataset must have at least 10 cases, got {}",
        cases.len()
    );
    for case in &cases {
        let (nodes, edges, state) = apply_case(case);
        assert_eq!(
            nodes, case.expect.nodes,
            "case '{}': expected {} nodes, got {}",
            case.name, case.expect.nodes, nodes
        );
        assert_eq!(
            edges, case.expect.edges,
            "case '{}': expected {} edges, got {}",
            case.name, case.expect.edges, edges
        );

        // Verify specific node keys exist
        for key in &case.expect.node_keys {
            assert!(
                state.nodes.contains_key(key),
                "case '{}': expected node '{}' not found. Available: {:?}",
                case.name,
                key,
                state.nodes.keys().collect::<Vec<_>>()
            );
        }

        // Verify specific edges exist with exact from/relation/to
        for em in &case.expect.edges_match {
            assert!(
                state
                    .edges
                    .iter()
                    .any(|e| e.from == em.from && e.relation == em.relation && e.to == em.to),
                "case '{}': expected edge '{} -->[{}]--> {}' not found",
                case.name,
                em.from,
                em.relation,
                em.to
            );
        }
    }
}

#[test]
fn golden_dataset_is_deterministic() {
    let cases = load_cases();
    for case in &cases {
        let (a_nodes, a_edges, a_state) = apply_case(case);
        let (b_nodes, b_edges, b_state) = apply_case(case);
        assert_eq!(
            (a_nodes, a_edges),
            (b_nodes, b_edges),
            "case '{}' nondeterministic count",
            case.name
        );
        assert_eq!(
            a_state, b_state,
            "case '{}' nondeterministic state",
            case.name
        );
    }
}
