//! Fork model, structural diff and fail-closed promotion (SPEC-009, Phase 7).
//!
//! A fork is a durable branch from a specific ledger event/sequence, with a
//! shared prefix hash that gates promotion: the parent must be unchanged at
//! the fork point for promotion to proceed.

use std::collections::BTreeMap;

use serde::{Deserialize, Serialize};

use crate::StorageError;
use crate::graph::GraphState;

/// Replay policy for a fork (SPEC-009 §4).
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ReplayPolicy {
    /// Rebuild state without invoking nondeterministic behaviors.
    #[default]
    Reconstruct,
    /// Re-execute deterministic behavior, verify hashes, serve recorded
    /// LLM/tool responses from cache.
    Strict,
}

/// Input for creating a fork.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ForkInput {
    /// Stable fork identifier.
    pub fork_id: String,
    /// Parent stream the fork branches from.
    pub parent_stream_id: String,
    /// Inclusive event sequence at the fork point.
    pub at_sequence: u64,
    /// Optional human label.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub label: Option<String>,
    /// Overrides applied on top of the parent (SPEC-009 §3).
    #[serde(default)]
    pub overrides: BTreeMap<String, String>,
    /// Replay policy.
    #[serde(default)]
    pub replay_policy: ReplayPolicy,
}

/// A durable fork record (SPEC-009 §3).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ForkRecord {
    /// Stable fork identifier.
    pub fork_id: String,
    /// Parent stream the fork branches from.
    pub parent_stream_id: String,
    /// Inclusive event sequence at the fork point.
    pub at_sequence: u64,
    /// Content hash of the event at `at_sequence` — the shared prefix head.
    pub shared_prefix_hash: String,
    /// Optional human label.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub label: Option<String>,
    /// Overrides applied on top of the parent.
    #[serde(default)]
    pub overrides: BTreeMap<String, String>,
    /// Creator actor.
    pub creator: String,
    /// RFC 3339 creation timestamp.
    pub created_at: String,
    /// Replay policy.
    pub replay_policy: ReplayPolicy,
}

/// Fork store port (SPEC-009 §3).
pub trait ForkStore {
    /// Creates a fork record. Rejects duplicate fork ids.
    fn create_fork(
        &mut self,
        input: ForkInput,
        creator: &str,
        created_at: &str,
        prefix_hash: &str,
    ) -> Result<ForkRecord, StorageError>;
    /// Loads a fork by id.
    fn load_fork(&self, fork_id: &str) -> Result<Option<ForkRecord>, StorageError>;
    /// Lists all forks ordered by creation time.
    fn list_forks(&self) -> Result<Vec<ForkRecord>, StorageError>;
}

/// Cached LLM/tool response (SPEC-009 §4).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct CachedResponse {
    /// Deterministic request hash.
    pub request_hash: String,
    /// Serialized response payload.
    pub response_json: String,
    /// Model that produced the response, when known.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub model: Option<String>,
    /// RFC 3339 creation timestamp.
    pub created_at: String,
}

/// Response cache port (SPEC-009 §4).
pub trait ResponseCachePort {
    /// Retrieves a cached response by request hash.
    fn get_response(&self, request_hash: &str) -> Result<Option<CachedResponse>, StorageError>;
    /// Stores (or replaces) a cached response.
    fn put_response(&mut self, entry: CachedResponse) -> Result<(), StorageError>;
}

/// Errors emitted by fork operations.
#[derive(Debug, thiserror::Error, PartialEq, Eq)]
pub enum ForkError {
    /// A fork with this id already exists.
    #[error("fork already exists: {0}")]
    Duplicate(String),
    /// The fork is not known.
    #[error("fork not found: {0}")]
    NotFound(String),
    /// Underlying storage failure.
    #[error("storage: {0}")]
    Storage(String),
}

/// Errors emitted by fork promotion (fail-closed).
#[derive(Debug, thiserror::Error, PartialEq, Eq)]
pub enum ForkPromoteError {
    /// The parent changed after the fork point — promotion is rejected.
    #[error("parent changed after fork: expected {expected}, actual {actual}")]
    ParentChanged {
        /// Hash recorded at fork creation.
        expected: String,
        /// Current parent head hash.
        actual: String,
    },
    /// The fork is not known.
    #[error("fork not found: {0}")]
    ForkNotFound(String),
}

/// Structural diff between two graph states (SPEC-009 §5).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct DiffReport {
    /// Event ids present only in the parent.
    pub events_only_in_parent: Vec<String>,
    /// Event ids present only in the fork.
    pub events_only_in_fork: Vec<String>,
    /// Node keys added in the fork.
    pub nodes_added: Vec<String>,
    /// Node keys removed in the fork.
    pub nodes_removed: Vec<String>,
    /// Edge keys (`from|relation|to`) present in only one side.
    pub edges_changed: Vec<String>,
    /// SHA-256 of the parent's canonical state JSON.
    pub parent_checksum: String,
    /// SHA-256 of the fork's canonical state JSON.
    pub fork_checksum: String,
}

/// Computes the canonical state checksum (deterministic: BTreeMap ordering).
pub fn state_checksum(state: &GraphState) -> String {
    use sha2::{Digest, Sha256};
    let canonical = serde_json::to_string(state).unwrap_or_default();
    let mut hasher = Sha256::new();
    hasher.update(canonical.as_bytes());
    format!("sha256:{:x}", hasher.finalize())
}

/// Computes a structural diff between parent and fork states.
pub fn structural_diff(parent: &GraphState, fork: &GraphState) -> DiffReport {
    let parent_edges: std::collections::BTreeSet<String> = parent
        .edges
        .iter()
        .map(|e| format!("{}|{}|{}", e.from, e.relation, e.to))
        .collect();
    let fork_edges: std::collections::BTreeSet<String> = fork
        .edges
        .iter()
        .map(|e| format!("{}|{}|{}", e.from, e.relation, e.to))
        .collect();

    let parent_nodes: std::collections::BTreeSet<&String> = parent.nodes.keys().collect();
    let fork_nodes: std::collections::BTreeSet<&String> = fork.nodes.keys().collect();

    DiffReport {
        events_only_in_parent: parent_edges.difference(&fork_edges).cloned().collect(),
        events_only_in_fork: fork_edges.difference(&parent_edges).cloned().collect(),
        nodes_added: fork_nodes
            .difference(&parent_nodes)
            .map(|k| k.to_string())
            .collect(),
        nodes_removed: parent_nodes
            .difference(&fork_nodes)
            .map(|k| k.to_string())
            .collect(),
        edges_changed: {
            let mut changed: Vec<String> = parent_edges
                .symmetric_difference(&fork_edges)
                .cloned()
                .collect();
            changed.sort();
            changed
        },
        parent_checksum: state_checksum(parent),
        fork_checksum: state_checksum(fork),
    }
}

/// Fail-closed promotion check (SPEC-009 §6).
///
/// Promotion is allowed only when the parent's current head hash equals the
/// shared prefix hash recorded at fork creation.
pub fn promote_check(fork: &ForkRecord, parent_last_hash: &str) -> Result<(), ForkPromoteError> {
    if parent_last_hash == fork.shared_prefix_hash {
        Ok(())
    } else {
        Err(ForkPromoteError::ParentChanged {
            expected: fork.shared_prefix_hash.clone(),
            actual: parent_last_hash.to_string(),
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn sample_state(nodes: &[&str], edges: &[(&str, &str, &str)]) -> GraphState {
        let mut state = GraphState::default();
        for (i, key) in nodes.iter().enumerate() {
            let (kind, id) = key.split_once(':').unwrap();
            state.nodes.insert(
                key.to_string(),
                crate::graph::GraphNode {
                    kind: kind.to_string(),
                    id: id.to_string(),
                    created_by: format!("e{i}"),
                    content_hash: "sha256:x".into(),
                    occurred_at: "2026-08-18T10:00:00Z".into(),
                },
            );
        }
        for (i, (from, rel, to)) in edges.iter().enumerate() {
            state.edges.push(crate::graph::GraphEdge {
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
    fn fork_record_serde_roundtrip() {
        let record = ForkRecord {
            fork_id: "f-1".into(),
            parent_stream_id: "project:p-1".into(),
            at_sequence: 3,
            shared_prefix_hash: "sha256:abc".into(),
            label: Some("experiment".into()),
            overrides: BTreeMap::from([("model".into(), "gpt-x".into())]),
            creator: "alice".into(),
            created_at: "2026-08-18T10:00:00Z".into(),
            replay_policy: ReplayPolicy::Strict,
        };
        let json = serde_json::to_string(&record).unwrap();
        let back: ForkRecord = serde_json::from_str(&json).unwrap();
        assert_eq!(back, record);
    }

    #[test]
    fn replay_policy_default_is_reconstruct() {
        assert_eq!(ReplayPolicy::default(), ReplayPolicy::Reconstruct);
    }

    #[test]
    fn diff_detects_added_node() {
        let parent = sample_state(&["requirement:R1"], &[]);
        let fork = sample_state(&["requirement:R1", "test:T1"], &[]);
        let report = structural_diff(&parent, &fork);
        assert!(report.nodes_added.contains(&"test:T1".to_string()));
        assert!(report.nodes_removed.is_empty());
        assert_ne!(report.parent_checksum, report.fork_checksum);
    }

    #[test]
    fn diff_detects_changed_edge() {
        let parent = sample_state(&["a:A", "b:B"], &[("a:A", "r", "b:B")]);
        let mut fork = parent.clone();
        fork.edges[0].relation = "r2".into();
        let report = structural_diff(&parent, &fork);
        assert!(
            report
                .edges_changed
                .iter()
                .any(|e| e.contains("|r|") || e.contains("|r2|")),
            "got: {:?}",
            report.edges_changed
        );
    }

    #[test]
    fn diff_identical_states_empty() {
        let state = sample_state(&["a:A"], &[("a:A", "r", "a:A")]);
        let report = structural_diff(&state, &state);
        assert!(report.nodes_added.is_empty());
        assert!(report.nodes_removed.is_empty());
        assert!(report.edges_changed.is_empty());
        assert_eq!(report.parent_checksum, report.fork_checksum);
    }

    #[test]
    fn promote_passes_on_unchanged_parent() {
        let fork = ForkRecord {
            fork_id: "f-1".into(),
            parent_stream_id: "s".into(),
            at_sequence: 3,
            shared_prefix_hash: "sha256:abc".into(),
            label: None,
            overrides: BTreeMap::new(),
            creator: "alice".into(),
            created_at: "2026-08-18T10:00:00Z".into(),
            replay_policy: ReplayPolicy::Reconstruct,
        };
        assert!(promote_check(&fork, "sha256:abc").is_ok());
    }

    #[test]
    fn promote_fails_on_changed_parent() {
        let fork = ForkRecord {
            fork_id: "f-1".into(),
            parent_stream_id: "s".into(),
            at_sequence: 3,
            shared_prefix_hash: "sha256:abc".into(),
            label: None,
            overrides: BTreeMap::new(),
            creator: "alice".into(),
            created_at: "2026-08-18T10:00:00Z".into(),
            replay_policy: ReplayPolicy::Reconstruct,
        };
        let error = promote_check(&fork, "sha256:changed").unwrap_err();
        assert!(matches!(error, ForkPromoteError::ParentChanged { .. }));
    }
}
