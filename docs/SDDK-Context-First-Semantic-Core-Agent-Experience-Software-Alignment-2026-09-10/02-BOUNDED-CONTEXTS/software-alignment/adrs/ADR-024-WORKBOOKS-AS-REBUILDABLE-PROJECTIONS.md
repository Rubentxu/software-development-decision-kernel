# ADR-024 — Workbooks as rebuildable projections

**Status:** Proposed  
**Evolution:** Software Alignment / Progressive Software Knowledge

## Context

The Semantic Core already defines canonical facts, objects, Decision Memory, SemanticGraph, Evidence, Authority, Packs and Agent Experience. The next evolution must reuse those authorities rather than create parallel stores.

## Decision

Excel-like tables, matrices and charts are read models over canonical facts/objects/decisions/assertions. UI edits emit semantic commands rather than mutating cells.

## Consequences

- implementation proceeds by strangler slices and UAT;
- new persistent state requires explicit state-class/authority/rebuild metadata;
- APIs may evolve internally as long as the semantic contract remains stable;
- assumptions discovered invalid during spikes require ADR amendment rather than additive workaround.
