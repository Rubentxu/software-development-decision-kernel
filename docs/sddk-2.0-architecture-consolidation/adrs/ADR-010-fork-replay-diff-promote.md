# ADR-010 — Fork, Replay, Diff and Promote

**Status:** Proposed  
**Date:** 2026-08-11

## Context

Agent/policy/process changes need controlled experimentation against the same history.

## Decision

Add durable event-stream forks, replay, structural/semantic diff and policy-gated promote.

## Consequences

### Positive

- Counterfactual engineering experiments.
- Model/prompt A/B testing.
- Safer self-improvement.

### Trade-offs / risks

- Event/cache/version semantics are non-trivial.

## Implementation notes

Start with state-only forks over SQLite streams. Add recorded LLM/tool cache. Add Git/worktree promotion later through capabilities.

## Revisit trigger

Revisit implementation complexity after a spike proves deterministic shared-prefix replay.
