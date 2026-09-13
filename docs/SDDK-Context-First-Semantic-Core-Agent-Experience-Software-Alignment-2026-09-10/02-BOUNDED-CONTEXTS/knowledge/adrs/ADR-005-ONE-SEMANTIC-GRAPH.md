# ADR-005 — One semantic graph projection, many typed views

**Status:** Proposed

## Decision

`GraphProjection` evolves into the canonical rebuildable **SemanticGraphProjection** over canonical facts and typed object references.

`ActiveGraphProjection` becomes a compatibility/typed view, not a second graph authority. Vault wiki graph remains an internal adapter/index. Pack-specific graph concepts use namespaced kinds/relations.

## Core relations

Relations distinguish semantics explicitly: `depends_on`, `caused_by`, `supported_by`, `contradicted_by`, `justified_by`, `selected_over`, `supersedes`, `produced_by`, `gates`, `references`.

## Consequences

Why/impact queries gain consistent semantics; graph storage can later change without changing authority.

## 2026-09-10 amendment

Software Alignment's Knowledge Dependency Overlay MUST be a typed view/relationship set over the existing SemanticGraph. It MUST NOT introduce another GraphStore. The Knowledge Merkle Tree is not a competing graph: it owns hierarchical fingerprints/freshness only.
