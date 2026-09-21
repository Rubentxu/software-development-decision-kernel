# ADR-022-ACTIVE-GRAPH-PROJECTION — Represent operational knowledge as an event-derived active graph

**Status:** Accepted


## Context
A flat event stream answers "what happened" but poorly answers "what is connected to what" and "why was this affected".

## Decision
Maintain typed graph projections derived from the Event Ledger. Behaviors may query the graph and react to newly appended events, but all resulting state changes return to the ledger as events.

## Rules
- Graph is not authority.
- Graph nodes/edges carry provenance to events/artifacts.
- Behavior execution is idempotent.
- Workflow control is not encoded only as graph topology; the Workflow Runtime remains explicit.

## Consequences
Provides ActiveGraph-like reactivity, causal exploration and moldable views without sacrificing deterministic workflow execution.
