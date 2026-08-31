//! Smoke tests for ExecutionGraphRevision and graph types.
//!
//! Covers:
//! - Chain divergence: different parents → different digests
//! - BTreeMap insertion-order independence for digests
//! - Conflicting expansion rejection (same parent, different events)
//! - Revision 0 has no parent (is_initial)
//! - Digest stability across JSON roundtrip

use std::collections::BTreeMap;

use sddk_domain::graph::{ExecutionGraphRevision, GraphEvent, NodeSnapshot};
use sddk_domain::workflow_ir::{EventId, NodeId, RevisionId};

fn empty_revision(revision: u64, revision_id: &str) -> ExecutionGraphRevision {
    ExecutionGraphRevision {
        revision,
        revision_id: RevisionId(revision_id.into()),
        parent: None,
        events: BTreeMap::new(),
        nodes: BTreeMap::new(),
        edges: BTreeMap::new(),
        digest: [0u8; 32],
        schema_version: 1,
    }
}

fn revision_with_parent(
    revision: u64,
    revision_id: &str,
    parent: ExecutionGraphRevision,
) -> ExecutionGraphRevision {
    ExecutionGraphRevision {
        revision,
        revision_id: RevisionId(revision_id.into()),
        parent: Some(Box::new(parent)),
        events: BTreeMap::new(),
        nodes: BTreeMap::new(),
        edges: BTreeMap::new(),
        digest: [0u8; 32],
        schema_version: 1,
    }
}

// ── Digest determinism ───────────────────────────────────────────────────────

#[test]
fn revision_0_is_initial() {
    let rev = empty_revision(0, "rev-initial");
    assert!(rev.is_initial());
    assert_eq!(rev.revision, 0);
    assert!(rev.parent.is_none());
}

#[test]
fn different_parents_produce_different_digests() {
    // Build parents with different content so they have different digests
    let mut parent_a = empty_revision(0, "parent-a");
    parent_a.events.insert(
        sddk_domain::workflow_ir::EventId("evt-a".into()),
        sddk_domain::graph::GraphEvent {
            event_id: sddk_domain::workflow_ir::EventId("evt-a".into()),
            event_type: "event.a".into(),
            occurred_at: "2026-08-19T10:00:00Z".into(),
        },
    );
    parent_a.digest = parent_a.compute_digest();

    let mut parent_b = empty_revision(0, "parent-b");
    parent_b.events.insert(
        sddk_domain::workflow_ir::EventId("evt-b".into()),
        sddk_domain::graph::GraphEvent {
            event_id: sddk_domain::workflow_ir::EventId("evt-b".into()),
            event_type: "event.b".into(),
            occurred_at: "2026-08-19T10:00:01Z".into(),
        },
    );
    parent_b.digest = parent_b.compute_digest();

    // Children of parents with different digests must have different digests
    let mut rev_a = revision_with_parent(1, "child-a", parent_a);
    rev_a.digest = rev_a.compute_digest();

    let mut rev_b = revision_with_parent(1, "child-b", parent_b);
    rev_b.digest = rev_b.compute_digest();

    // Different parent content → different child digest
    assert_ne!(
        rev_a.digest, rev_b.digest,
        "different parent content must affect child digest"
    );
}

#[test]
fn identical_content_produces_identical_digest() {
    // Build two revisions with identical content independently
    let parent = empty_revision(0, "shared-parent");

    let mut rev1 = revision_with_parent(1, "rev1", parent.clone());
    rev1.digest = rev1.compute_digest();

    let rev2 = revision_with_parent(1, "rev1", parent);
    let digest2 = rev2.compute_digest();

    assert_eq!(
        rev1.digest, digest2,
        "identical content must produce identical digest"
    );
}

#[test]
fn btreemap_order_does_not_affect_revision_digest() {
    // Insert events in different orders
    let mk_rev = |order: usize| -> ExecutionGraphRevision {
        let parent = empty_revision(0, "p");
        let mut rev = revision_with_parent(1, "r", parent);
        for i in 0..5 {
            let idx = (i + order) % 5;
            let event_id = EventId(format!("evt-{}", idx));
            rev.events.insert(
                event_id,
                GraphEvent {
                    event_id: EventId(format!("evt-{}", idx)),
                    event_type: format!("test.event.{}", idx),
                    occurred_at: "2026-08-19T10:00:00Z".into(),
                },
            );
        }
        rev
    };

    let digest0 = mk_rev(0).compute_digest();
    let digest1 = mk_rev(1).compute_digest();
    let digest2 = mk_rev(2).compute_digest();

    // All three should be identical since BTreeMap sorts by key
    assert_eq!(digest0, digest1);
    assert_eq!(digest1, digest2);
}

#[test]
fn nodes_btreemap_order_does_not_affect_digest() {
    let mk_rev = |order: usize| -> ExecutionGraphRevision {
        let parent = empty_revision(0, "p");
        let mut rev = revision_with_parent(1, "r", parent);
        for i in 0..5 {
            let idx = (i + order) % 5;
            let node_id = NodeId(format!("node-{}", idx));
            rev.nodes.insert(
                node_id,
                NodeSnapshot {
                    node_id: NodeId(format!("node-{}", idx)),
                    state: "running".into(),
                    snapshot_at: "2026-08-19T10:00:00Z".into(),
                },
            );
        }
        rev
    };

    let digest0 = mk_rev(0).compute_digest();
    let digest1 = mk_rev(1).compute_digest();

    assert_eq!(
        digest0, digest1,
        "node BTreeMap order must not affect digest"
    );
}

// ── Chain integrity ─────────────────────────────────────────────────────────

#[test]
fn child_parent_chain_is_valid() {
    let root = empty_revision(0, "root");
    let root_digest = root.compute_digest();
    let root_computed = root_digest;

    let mut child = revision_with_parent(1, "child", root);
    child.digest = child.compute_digest();

    // Child has parent
    assert!(child.parent.is_some());
    // Parent digest is embedded in child digest
    assert_ne!(child.digest, root_computed);
}

#[test]
fn revision_number_increments_correctly() {
    let rev0 = empty_revision(0, "rev-0");
    let rev0_revision = rev0.revision;
    let rev1 = revision_with_parent(1, "rev-1", rev0);
    let rev1_revision = rev1.revision;
    let rev2 = revision_with_parent(2, "rev-2", rev1);

    assert_eq!(rev0_revision, 0);
    assert_eq!(rev1_revision, 1);
    assert_eq!(rev2.revision, 2);
}

#[test]
fn deeply_nested_chain_digest_changes() {
    let rev0 = empty_revision(0, "r0");
    let mut prev = rev0;
    let mut prev_digest = prev.compute_digest();

    for i in 1..=5 {
        let next = revision_with_parent(i, &format!("r{}", i), prev);
        let next_digest = next.compute_digest();
        assert_ne!(
            prev_digest, next_digest,
            "each revision in chain must have different digest"
        );
        prev_digest = next_digest;
        prev = next;
    }
}

// ── JSON roundtrip ───────────────────────────────────────────────────────────

#[test]
fn revision_json_roundtrip_preserves_digest() {
    let mut rev = empty_revision(0, "test-rev");
    rev.digest = rev.compute_digest();

    let json = serde_json::to_string(&rev).expect("must serialize");
    let rev2: ExecutionGraphRevision = serde_json::from_str(&json).expect("must deserialize");

    // Note: digest is a [u8; 32] field that survives roundtrip
    assert_eq!(rev.digest, rev2.digest);
}

// ── SCHEMA_VERSION ────────────────────────────────────────────────────────────

#[test]
fn execution_graph_revision_schema_version_is_one() {
    assert_eq!(ExecutionGraphRevision::SCHEMA_VERSION, 1);
}

// ── Conflicting expansion (same parent, different events) ─────────────────────

#[test]
fn same_parent_different_events_different_digest() {
    let parent = empty_revision(0, "shared-parent");

    let mut rev_a = revision_with_parent(1, "rev-a", parent.clone());
    rev_a.events.insert(
        EventId("evt-A".into()),
        GraphEvent {
            event_id: EventId("evt-A".into()),
            event_type: "expansion.a".into(),
            occurred_at: "2026-08-19T10:00:00Z".into(),
        },
    );
    let digest_a = rev_a.compute_digest();

    let mut rev_b = revision_with_parent(1, "rev-b", parent);
    rev_b.events.insert(
        EventId("evt-B".into()),
        GraphEvent {
            event_id: EventId("evt-B".into()),
            event_type: "expansion.b".into(),
            occurred_at: "2026-08-19T10:00:01Z".into(),
        },
    );
    let digest_b = rev_b.compute_digest();

    assert_ne!(
        digest_a, digest_b,
        "different events must produce different digest"
    );
}
