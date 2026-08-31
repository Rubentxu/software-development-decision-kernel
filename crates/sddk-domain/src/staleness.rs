//! Universal staleness derivation over the reactive graph (SPEC-012, Phase 6).
//!
//! A fact/view/decision becomes stale when a version-bound dependency changes.
//! `derive_staleness` walks the graph deterministically: edges `verifies` /
//! `governs` establish the verification relationship, and later events that
//! touch the verified subject produce a causal path. The derivation is
//! conservative (SPEC-012 §4): uncertainty maps to `PossiblyStale` and
//! requires explicit revalidation.

use serde::{Deserialize, Serialize};

use crate::graph::{GraphEdge, GraphState};

/// Universal staleness state (SPEC-012 §2).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum StalenessState {
    /// No version-bound dependency changed since verification.
    Fresh,
    /// A version-bound dependency changed after verification; conservative
    /// state that requires explicit revalidation.
    PossiblyStale,
    /// A dependency changed AND invalidation evidence exists.
    Stale,
    /// Explicitly invalidated by an `invalidated_by` edge.
    Invalidated,
    /// No provenance is available to derive staleness.
    Unknown,
}

crate::assert_variant_count_eq!(
    StalenessState,
    5,
    [
        StalenessState::Fresh,
        StalenessState::PossiblyStale,
        StalenessState::Stale,
        StalenessState::Invalidated,
        StalenessState::Unknown,
    ]
);

/// Result of staleness derivation for one entity.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct StalenessResult {
    /// Derived state.
    pub state: StalenessState,
    /// Event ids that touched the subject after verification, ordered by
    /// `occurred_at` ascending.
    pub causal_path: Vec<String>,
    /// Event id of the verification edge, when one exists.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub verified_by: Option<String>,
}

/// Relations that establish a verification/governance link.
///
/// Matches the canonical relation names (`verifies`, `governs`) as well as
/// verb-form event types that end in `.verifies` / `.verified` / `.governs`
/// (e.g. `uat.acceptance.verified`), since the graph maps event types to
/// relation names verbatim.
const VERIFICATION_RELATIONS: &[&str] = &["verifies", "governs"];

/// Relations that explicitly invalidate an entity.
const INVALIDATION_RELATIONS: &[&str] = &["invalidated_by"];

/// Whether a relation name is a verification/governance link.
fn is_verification_relation(relation: &str) -> bool {
    VERIFICATION_RELATIONS.contains(&relation)
        || relation.ends_with(".verifies")
        || relation.ends_with(".verified")
        || relation.ends_with(".governs")
}

/// Whether a relation name is an explicit invalidation.
fn is_invalidation_relation(relation: &str) -> bool {
    INVALIDATION_RELATIONS.contains(&relation) || relation.ends_with(".invalidated")
}

/// Derives staleness for `entity` from the graph.
///
/// Deterministic for a fixed `GraphState` and entity key.
pub fn derive_staleness(state: &GraphState, entity: &str) -> StalenessResult {
    // Find verification edges where `entity` is the verified subject.
    let verification_edges: Vec<&GraphEdge> = state
        .edges
        .iter()
        .filter(|edge| edge.to == entity && is_verification_relation(&edge.relation))
        .collect();

    // Explicit invalidation without any verification context → Invalidated.
    // (With verification context, invalidation escalates to Stale below.)
    if verification_edges.is_empty() {
        let invalidated = state
            .edges
            .iter()
            .find(|edge| edge.from == entity && is_invalidation_relation(&edge.relation));
        if let Some(edge) = invalidated {
            return StalenessResult {
                state: StalenessState::Invalidated,
                causal_path: vec![edge.event_id.clone()],
                verified_by: None,
            };
        }
        // No verification provenance → unknown.
        return StalenessResult {
            state: StalenessState::Unknown,
            causal_path: Vec::new(),
            verified_by: None,
        };
    }

    // Latest verification edge (by occurred_at).
    let latest = verification_edges
        .iter()
        .max_by(|a, b| a.occurred_at.cmp(&b.occurred_at))
        .copied()
        .expect("non-empty by construction");

    // Events that touched the entity after the verification.
    let mut changes: Vec<(String, String)> = state
        .edges
        .iter()
        .filter(|edge| {
            edge.from == entity
                && edge.occurred_at > latest.occurred_at
                && edge.event_id != latest.event_id
        })
        .map(|edge| (edge.occurred_at.clone(), edge.event_id.clone()))
        .collect();
    // Deterministic ordering by occurred_at, then event_id.
    changes.sort();

    let causal_path: Vec<String> = changes.into_iter().map(|(_, id)| id).collect();

    if causal_path.is_empty() {
        return StalenessResult {
            state: StalenessState::Fresh,
            causal_path: Vec::new(),
            verified_by: Some(latest.event_id.clone()),
        };
    }

    // Subject changed after verification: conservative PossiblyStale. If an
    // invalidation edge exists after the latest change, escalate to Stale.
    let has_later_invalidation = state.edges.iter().any(|edge| {
        edge.from == entity
            && is_invalidation_relation(&edge.relation)
            && edge.occurred_at >= latest.occurred_at
    });

    let state = if has_later_invalidation {
        StalenessState::Stale
    } else {
        StalenessState::PossiblyStale
    };

    StalenessResult {
        state,
        causal_path,
        verified_by: Some(latest.event_id.clone()),
    }
}

/// Returns the staleness state for every entity in the graph that has
/// verification/governance provenance, in deterministic key order.
pub fn all_staleness(state: &GraphState) -> Vec<(String, StalenessResult)> {
    let mut entities: Vec<String> = state
        .edges
        .iter()
        .filter(|edge| is_verification_relation(&edge.relation))
        .map(|edge| edge.to.clone())
        .collect();
    entities.sort();
    entities.dedup();
    entities
        .into_iter()
        .map(|entity| {
            let result = derive_staleness(state, &entity);
            (entity, result)
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::graph::{GraphNode, GraphState};

    fn node(state: &mut GraphState, key: &str, at: &str) {
        let (kind, id) = key.split_once(':').unwrap();
        state.nodes.insert(
            key.to_string(),
            GraphNode {
                kind: kind.to_string(),
                id: id.to_string(),
                created_by: format!("evt-{key}"),
                content_hash: "sha256:x".into(),
                occurred_at: at.to_string(),
            },
        );
    }

    fn edge(state: &mut GraphState, from: &str, rel: &str, to: &str, event_id: &str, at: &str) {
        state.edges.push(GraphEdge {
            from: from.to_string(),
            relation: rel.to_string(),
            to: to.to_string(),
            event_id: event_id.to_string(),
            occurred_at: at.to_string(),
            actor: "t".into(),
        });
    }

    #[test]
    fn fresh_when_verifier_unchanged() {
        let mut state = GraphState::default();
        node(&mut state, "test:T1", "2026-08-18T10:00:00Z");
        node(&mut state, "requirement:R1", "2026-08-18T10:00:00Z");
        edge(
            &mut state,
            "test:T1",
            "verifies",
            "requirement:R1",
            "e-verify",
            "2026-08-18T10:00:00Z",
        );
        let result = derive_staleness(&state, "requirement:R1");
        assert_eq!(result.state, StalenessState::Fresh);
        assert!(result.causal_path.is_empty());
        assert_eq!(result.verified_by.as_deref(), Some("e-verify"));
    }

    #[test]
    fn possibly_stale_after_subject_change() {
        let mut state = GraphState::default();
        node(&mut state, "test:T1", "2026-08-18T10:00:00Z");
        node(&mut state, "requirement:R1", "2026-08-18T10:00:00Z");
        edge(
            &mut state,
            "test:T1",
            "verifies",
            "requirement:R1",
            "e-verify",
            "2026-08-18T10:00:00Z",
        );
        edge(
            &mut state,
            "requirement:R1",
            "modified",
            "requirement:R1",
            "e-change",
            "2026-08-18T11:00:00Z",
        );
        let result = derive_staleness(&state, "requirement:R1");
        assert_eq!(result.state, StalenessState::PossiblyStale);
        assert_eq!(result.causal_path, vec!["e-change"]);
    }

    #[test]
    fn invalidated_marks_invalidated() {
        let mut state = GraphState::default();
        node(&mut state, "requirement:R1", "2026-08-18T10:00:00Z");
        edge(
            &mut state,
            "requirement:R1",
            "invalidated_by",
            "requirement:R1",
            "e-inv",
            "2026-08-18T12:00:00Z",
        );
        let result = derive_staleness(&state, "requirement:R1");
        assert_eq!(result.state, StalenessState::Invalidated);
        assert_eq!(result.causal_path, vec!["e-inv"]);
    }

    #[test]
    fn stale_when_change_and_invalidation() {
        let mut state = GraphState::default();
        node(&mut state, "test:T1", "2026-08-18T10:00:00Z");
        node(&mut state, "requirement:R1", "2026-08-18T10:00:00Z");
        edge(
            &mut state,
            "test:T1",
            "verifies",
            "requirement:R1",
            "e-verify",
            "2026-08-18T10:00:00Z",
        );
        edge(
            &mut state,
            "requirement:R1",
            "modified",
            "requirement:R1",
            "e-change",
            "2026-08-18T11:00:00Z",
        );
        edge(
            &mut state,
            "requirement:R1",
            "invalidated_by",
            "requirement:R1",
            "e-inv",
            "2026-08-18T12:00:00Z",
        );
        let result = derive_staleness(&state, "requirement:R1");
        assert_eq!(result.state, StalenessState::Stale);
    }

    #[test]
    fn unknown_without_provenance() {
        let mut state = GraphState::default();
        node(&mut state, "requirement:R9", "2026-08-18T10:00:00Z");
        let result = derive_staleness(&state, "requirement:R9");
        assert_eq!(result.state, StalenessState::Unknown);
        assert!(result.causal_path.is_empty());
    }

    #[test]
    fn causal_path_is_ordered() {
        let mut state = GraphState::default();
        node(&mut state, "test:T1", "2026-08-18T10:00:00Z");
        node(&mut state, "requirement:R1", "2026-08-18T10:00:00Z");
        edge(
            &mut state,
            "test:T1",
            "verifies",
            "requirement:R1",
            "e-verify",
            "2026-08-18T10:00:00Z",
        );
        edge(
            &mut state,
            "requirement:R1",
            "modified",
            "requirement:R1",
            "e-late",
            "2026-08-18T13:00:00Z",
        );
        edge(
            &mut state,
            "requirement:R1",
            "modified",
            "requirement:R1",
            "e-early",
            "2026-08-18T11:00:00Z",
        );
        let result = derive_staleness(&state, "requirement:R1");
        assert_eq!(result.causal_path, vec!["e-early", "e-late"]);
    }

    #[test]
    fn all_staleness_lists_verified_entities() {
        let mut state = GraphState::default();
        node(&mut state, "test:T1", "2026-08-18T10:00:00Z");
        node(&mut state, "requirement:R1", "2026-08-18T10:00:00Z");
        node(&mut state, "requirement:R2", "2026-08-18T10:00:00Z");
        edge(
            &mut state,
            "test:T1",
            "verifies",
            "requirement:R1",
            "e-verify",
            "2026-08-18T10:00:00Z",
        );
        edge(
            &mut state,
            "test:T1",
            "verifies",
            "requirement:R2",
            "e-verify2",
            "2026-08-18T10:00:00Z",
        );
        let list = all_staleness(&state);
        assert_eq!(list.len(), 2);
        assert_eq!(list[0].0, "requirement:R1");
        assert_eq!(list[1].0, "requirement:R2");
    }
}
