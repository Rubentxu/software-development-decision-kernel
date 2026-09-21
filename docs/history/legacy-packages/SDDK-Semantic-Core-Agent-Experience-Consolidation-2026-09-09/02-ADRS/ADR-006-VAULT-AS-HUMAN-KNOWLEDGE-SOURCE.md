# ADR-006 — Vault is curated human knowledge, not authority

**Status:** Proposed

## Decision

Keep `sddk-vault` as a human-readable knowledge source: Markdown + frontmatter + wikilinks + FTS/HTML browsing.

It may feed Context Compiler and Semantic Graph through adapters. Its local graph/search index is rebuildable and non-authoritative.

Decision Memory never stores “truth” merely because text appears in the Vault; durable decisions require canonical decision facts/memory objects with provenance.

## Rationale

The Vault is valuable precisely because it optimizes explanation and human maintenance, not deterministic authority.
