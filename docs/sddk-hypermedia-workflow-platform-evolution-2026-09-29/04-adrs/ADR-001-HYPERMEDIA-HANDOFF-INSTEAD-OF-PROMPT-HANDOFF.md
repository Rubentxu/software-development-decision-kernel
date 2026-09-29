# ADR-001 — Hypermedia handoff instead of prompt-defined handoff

**Status:** Proposed

## Context

Current agent handoff uses typed pieces (ContextDelta, ContextLease, Contribution envelope) but operational continuity is still too dependent on launch packets, prompt recipes and summaries. A restarted/new agent must know too much of the lifecycle protocol.

## Decision

Adopt a transport-independent hypermedia resource model. Every agent-facing resource exposes current state, semantic relations and typed legal affordances derived from kernel state/policy/authority.

A handoff is therefore not a bespoke message from agent A to B. It is the next resource representation of a StepRun/Run plus ContextBasis/Delta and affordances.

## Consequences

### Positive
- agents can recover without transcript replay;
- protocol becomes discoverable;
- state and next actions stay aligned;
- CLI/MCP/HTTP can share semantics;
- prompts lose lifecycle recipes.

### Negative
- requires stable resource IDs and representation versioning;
- action derivation becomes a security-sensitive service;
- stale-basis handling becomes mandatory.

## Rejected

1. **Keep prose handoffs but improve template** — still duplicates state/protocol.
2. **Persist every link/action** — creates stale second authority.
3. **REST-only API** — over-couples architecture to transport.

## Guardrails

Affordances are derived and basis-bound; authority is rechecked at invocation. Hypermedia text never becomes instruction authority.
