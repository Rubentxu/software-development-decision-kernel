# ADR-032 — Declared vs static vs runtime alignment

**Status:** Proposed  
**Evolution:** Software Alignment / Progressive Software Knowledge

## Context

The Semantic Core already defines canonical facts, objects, Decision Memory, SemanticGraph, Evidence, Authority, Packs and Agent Experience. The next evolution must reuse those authorities rather than create parallel stores.

## Decision

Allow static providers and runtime providers to contribute separate observations so runtime architecture drift can be detected without conflating static and dynamic evidence.

## Consequences

- implementation proceeds by strangler slices and UAT;
- new persistent state requires explicit state-class/authority/rebuild metadata;
- APIs may evolve internally as long as the semantic contract remains stable;
- assumptions discovered invalid during spikes require ADR amendment rather than additive workaround.
