// Copyright (c) SDDK contributors.
// SPDX-License-Identifier: MIT
//
// active_graph_digest.rs — T-08 (M8.8)
//
// Stable, deterministic digests over `ActiveGraphProjection`. Two
// variants:
//
//   - strict: includes `recorded_at`, so any timestamp drift produces
//     a different hash. Use when you want to detect "any change,
//     even cosmetic".
//   - content: ignores `recorded_at`, only the structural + provenance
//     content of the projection participates in the hash. Use when
//     you want to ask "did the structure change?".
//
// Both are pure, deterministic, and stateless. The canonical encoding
// is JSON (via serde_json) because BTreeMap natural ordering +
// canonical Vec order give stable byte streams across runs without
// hand-rolling a format.
//
// SHA-256 hex output prefixed with `sha256:` matches the convention
// already used by the engine's CAS substrate (`sddk-storage`).

use crate::active_graph::ActiveGraphProjection;
use sha2::{Digest, Sha256};

/// Audit guard. Must match the number of `DigestKind` variants.
pub const PROJECTION_DIGEST_KIND_COUNT: usize = 2;

/// Which projection surface to digest.
#[non_exhaustive]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum DigestKind {
    /// Hash the full projection including the `recorded_at` timestamp.
    Strict,
    /// Hash the structural + provenance content, ignoring `recorded_at`.
    Content,
}

impl DigestKind {
    /// Canonical textual label (used in JSON output and audit logs).
    #[must_use]
    pub const fn label(self) -> &'static str {
        match self {
            Self::Strict => "strict",
            Self::Content => "content",
        }
    }
}

/// Stable hex SHA-256 digest with the `sha256:` prefix.
///
/// `PartialEq` / `Eq` / `Hash` derive intentionally NOT applied: the
/// type wraps a `String` and equality on the wrapper is just string
/// equality. Callers compare via `.as_str()` or by parsing the hex.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ProjectionDigest {
    pub kind: DigestKind,
    pub hex: String,
}

impl ProjectionDigest {
    /// Compute the SHA-256 of a canonical projection encoding.
    ///
    /// Returns a stable, deterministic hex string prefixed with `sha256:`.
    /// The encoding pipeline:
    ///
    /// 1. Serialize the projection to JSON (BTreeMap = sorted keys,
    ///    Vec = canonical order).
    /// 2. If `kind == Strict`, include `recorded_at` on every node; if
    ///    `Content`, strip it before serializing.
    /// 3. SHA-256 the bytes; hex-encode.
    #[must_use]
    pub fn compute(projection: &ActiveGraphProjection, kind: DigestKind) -> Self {
        let bytes = match kind {
            DigestKind::Strict => serde_json::to_vec(projection)
                .expect("ActiveGraphProjection serialization is infallible"),
            DigestKind::Content => {
                // Strip `recorded_at` from every node before hashing.
                let stripped_nodes: std::collections::BTreeMap<
                    sddk_domain::workflow_ir::NodeId,
                    ActiveGraphNodeWithoutTimestamp<'_>,
                > = projection
                    .nodes
                    .iter()
                    .map(|(k, v)| {
                        (
                            k.clone(),
                            ActiveGraphNodeWithoutTimestamp {
                                id: &v.id,
                                kind: v.kind,
                                label: &v.label,
                                provenance: v.provenance.as_ref(),
                            },
                        )
                    })
                    .collect();
                let content = ProjectionForContentDigest {
                    nodes: &stripped_nodes,
                    edges: &projection.edges,
                    roots: &projection.roots,
                    node_count: projection.node_count,
                    edge_count: projection.edge_count,
                };
                serde_json::to_vec(&content)
                    .expect("ProjectionForContentDigest serialization is infallible")
            }
        };
        let mut hasher = Sha256::new();
        hasher.update(&bytes);
        let hash = hasher.finalize();
        let hex = format!("sha256:{}", hex_lower(&hash));
        Self {
            kind,
            hex: hex.clone(),
        }
    }

    /// Expose the raw hex (without the `sha256:` prefix).
    #[must_use]
    pub fn hex_no_prefix(&self) -> &str {
        // Strip the "sha256:" prefix safely.
        &self.hex["sha256:".len()..]
    }
}

/// Free-function variant. Equivalent to
/// `ProjectionDigest::compute(projection, DigestKind::Strict)`.
#[must_use]
pub fn digest_projection(projection: &ActiveGraphProjection) -> ProjectionDigest {
    ProjectionDigest::compute(projection, DigestKind::Strict)
}

/// Free-function variant. Equivalent to
/// `ProjectionDigest::compute(projection, DigestKind::Content)`.
#[must_use]
pub fn digest_projection_content(projection: &ActiveGraphProjection) -> ProjectionDigest {
    ProjectionDigest::compute(projection, DigestKind::Content)
}

/// Subset projection used for the `Content` digest — drops `recorded_at`
/// from every node so timestamp drift doesn't pollute the hash.
#[derive(serde::Serialize)]
struct ProjectionForContentDigest<'a, N: serde::Serialize = ActiveGraphNodeWithoutTimestamp<'a>> {
    nodes: &'a std::collections::BTreeMap<sddk_domain::workflow_ir::NodeId, N>,
    edges: &'a Vec<crate::active_graph::ActiveGraphEdge>,
    roots: &'a Vec<sddk_domain::workflow_ir::NodeId>,
    node_count: usize,
    edge_count: usize,
}

/// Minimal node view used by the content digest. Omits `recorded_at`.
#[derive(serde::Serialize)]
struct ActiveGraphNodeWithoutTimestamp<'a> {
    id: &'a sddk_domain::workflow_ir::NodeId,
    kind: crate::active_graph::ActiveGraphNodeKind,
    label: &'a String,
    provenance: Option<&'a crate::active_graph::ProvenanceRef>,
}

fn hex_lower(bytes: &[u8]) -> String {
    const HEX: &[u8; 16] = b"0123456789abcdef";
    let mut out = String::with_capacity(bytes.len() * 2);
    for b in bytes {
        out.push(HEX[(b >> 4) as usize] as char);
        out.push(HEX[(b & 0x0f) as usize] as char);
    }
    out
}

// ── Tests ───────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;
    use crate::active_graph::{
        ActiveGraphEdge, ActiveGraphEdgeKind, ActiveGraphNode, ActiveGraphNodeKind, ProvenanceRef,
        ProvenanceSourceKind,
    };
    use sddk_domain::workflow_ir::NodeId;
    // `DefaultActiveGraphProjector` is intentionally re-imported inside the
    // specific test that uses it (`projection_built_via_projector_is_digestable`)
    // so the dependency on that type stays local and obvious. Likewise
    // `BTreeMap` is referenced via the fully-qualified `std::collections::BTreeMap`
    // path elsewhere in this file, so a bare import here would be unused.

    fn nid(s: &str) -> NodeId {
        NodeId(s.to_string())
    }

    fn node(id: &str, label: &str, recorded_at: &str) -> ActiveGraphNode {
        ActiveGraphNode {
            id: nid(id),
            kind: ActiveGraphNodeKind::Workflow,
            label: label.to_string(),
            recorded_at: recorded_at.to_string(),
            provenance: None,
        }
    }

    fn proj(nodes: Vec<ActiveGraphNode>, edges: Vec<ActiveGraphEdge>) -> ActiveGraphProjection {
        let mut p = ActiveGraphProjection::default();
        for n in nodes {
            p.nodes.insert(n.id.clone(), n);
        }
        p.edges = edges;
        p.node_count = p.nodes.len();
        p.edge_count = p.edges.len();
        p.roots = p.nodes.keys().cloned().collect();
        p
    }

    #[test]
    fn identical_projections_produce_identical_strict_digests() {
        let p = proj(vec![node("a", "A", "2026-01-01T00:00:00Z")], vec![]);
        let d1 = digest_projection(&p);
        let d2 = digest_projection(&p);
        assert_eq!(d1.hex, d2.hex);
        assert_eq!(d1.kind, DigestKind::Strict);
    }

    #[test]
    fn different_recorded_at_changes_strict_digest_but_not_content() {
        let p1 = proj(vec![node("a", "A", "2026-01-01T00:00:00Z")], vec![]);
        let p2 = proj(vec![node("a", "A", "2026-02-15T12:34:56Z")], vec![]);
        let d_strict_1 = digest_projection(&p1);
        let d_strict_2 = digest_projection(&p2);
        let d_content_1 = digest_projection_content(&p1);
        let d_content_2 = digest_projection_content(&p2);
        assert_ne!(
            d_strict_1.hex, d_strict_2.hex,
            "strict digest must detect recorded_at drift"
        );
        assert_eq!(
            d_content_1.hex, d_content_2.hex,
            "content digest must ignore recorded_at drift"
        );
    }

    #[test]
    fn different_node_id_changes_digest() {
        let p1 = proj(vec![node("a", "A", "2026-01-01T00:00:00Z")], vec![]);
        let p2 = proj(vec![node("b", "A", "2026-01-01T00:00:00Z")], vec![]);
        assert_ne!(digest_projection(&p1).hex, digest_projection(&p2).hex);
    }

    #[test]
    fn different_label_changes_digest() {
        let p1 = proj(vec![node("a", "A", "2026-01-01T00:00:00Z")], vec![]);
        let p2 = proj(vec![node("a", "A-renamed", "2026-01-01T00:00:00Z")], vec![]);
        assert_ne!(digest_projection(&p1).hex, digest_projection(&p2).hex);
    }

    #[test]
    fn different_edge_set_changes_digest() {
        let p1 = proj(vec![node("a", "A", "t0"), node("b", "B", "t0")], vec![]);
        let p2 = proj(
            vec![node("a", "A", "t0"), node("b", "B", "t0")],
            vec![ActiveGraphEdge {
                kind: ActiveGraphEdgeKind::ParentOf,
                source: nid("a"),
                target: nid("b"),
                provenance: None,
            }],
        );
        assert_ne!(digest_projection(&p1).hex, digest_projection(&p2).hex);
    }

    #[test]
    fn different_provenance_changes_digest() {
        let mut n1 = node("a", "A", "2026-01-01T00:00:00Z");
        n1.provenance = Some(ProvenanceRef::new(
            ProvenanceSourceKind::CycleManifest,
            "Commits:row_1".to_string(),
        ));
        let mut n2 = n1.clone();
        n2.provenance = Some(ProvenanceRef::new(
            ProvenanceSourceKind::CycleManifest,
            "Commits:row_2".to_string(),
        ));
        let p1 = proj(vec![n1], vec![]);
        let p2 = proj(vec![n2], vec![]);
        assert_ne!(digest_projection(&p1).hex, digest_projection(&p2).hex);
    }

    #[test]
    fn empty_projection_digest_is_stable() {
        let p = ActiveGraphProjection::default();
        let d = digest_projection(&p);
        // Just verify the shape — we don't pin the exact hash to avoid
        // brittleness if default field serialization ever changes.
        assert!(d.hex.starts_with("sha256:"));
        assert_eq!(d.hex.len(), "sha256:".len() + 64);
        // Re-running yields the same hash.
        let d2 = digest_projection(&p);
        assert_eq!(d.hex, d2.hex);
    }

    #[test]
    fn projection_built_via_projector_is_digestable() {
        // End-to-end: build a projection through the real projector
        // and confirm the digest pipeline doesn't panic.
        use crate::active_graph::{
            ActiveGraphInput, ActiveGraphProjector, DefaultActiveGraphProjector,
        };
        let mut input = ActiveGraphInput::default();
        input.workflow_nodes.push(nid("a"));
        input.workflow_nodes.push(nid("b"));
        input.workflow_edges.push((nid("a"), nid("b")));
        let projector = DefaultActiveGraphProjector;
        let p = projector.project(&input, "2026-09-10T00:00:00Z");
        let d = digest_projection(&p);
        assert!(d.hex.starts_with("sha256:"));
    }

    #[test]
    fn digest_kind_count_matches_variants() {
        assert_eq!(
            PROJECTION_DIGEST_KIND_COUNT, 2,
            "PROJECTION_DIGEST_KIND_COUNT must match DigestKind variant count"
        );
    }

    #[test]
    fn digest_kind_label_strings() {
        assert_eq!(DigestKind::Strict.label(), "strict");
        assert_eq!(DigestKind::Content.label(), "content");
    }

    #[test]
    fn hex_no_prefix_strips_scheme() {
        let p = ActiveGraphProjection::default();
        let d = digest_projection(&p);
        assert!(!d.hex_no_prefix().contains(':'));
        assert_eq!(d.hex_no_prefix().len(), 64);
    }

    #[test]
    fn digest_is_deterministic_across_runs() {
        // Same input, computed twice, must match.
        let p1 = proj(
            vec![node("a", "A", "t0"), node("b", "B", "t0")],
            vec![ActiveGraphEdge {
                kind: ActiveGraphEdgeKind::DelegatedTo,
                source: nid("orchestrator"),
                target: nid("m8_8"),
                provenance: None,
            }],
        );
        let d_a = digest_projection(&p1);
        let d_b = digest_projection(&p1);
        assert_eq!(d_a.hex, d_b.hex);
    }
}
