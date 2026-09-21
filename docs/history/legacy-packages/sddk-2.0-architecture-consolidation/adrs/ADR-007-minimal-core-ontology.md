# ADR-007 — Minimal Universal Core Vocabulary

**Status:** Proposed  
**Date:** 2026-08-11

## Context

A universal graph ontology tends to absorb domain nouns and become rigid.

## Decision

Limit the universal vocabulary to actor, intent, work_item, artifact, observation, evidence and decision plus a small relation set. Domain nouns stay in packs.

## Consequences

### Positive

- Cross-pack lingua franca.
- Avoids ontology lock-in.

### Trade-offs / risks

- Mapping domain concepts to core can feel repetitive.
- Boundaries require discipline.

## Implementation notes

Add core ontology schema and bridge-pack convention. Reject new core nouns by default.

## Revisit trigger

Revisit a candidate noun only after at least three unrelated packs require identical semantics.
