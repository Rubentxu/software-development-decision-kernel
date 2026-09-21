# ADR-025 — LLM as semantic evaluator

**Status:** Proposed  
**Evolution:** Software Alignment / Progressive Software Knowledge

## Context

The Semantic Core already defines canonical facts, objects, Decision Memory, SemanticGraph, Evidence, Authority, Packs and Agent Experience. The next evolution must reuse those authorities rather than create parallel stores.

## Decision

Use LLMs for ambiguity, responsibility, tradeoff and semantic design assessment after deterministic evidence. Do not use an LLM as RPC middleware or source of OBSERVED facts.

## Consequences

- implementation proceeds by strangler slices and UAT;
- new persistent state requires explicit state-class/authority/rebuild metadata;
- APIs may evolve internally as long as the semantic contract remains stable;
- assumptions discovered invalid during spikes require ADR amendment rather than additive workaround.
