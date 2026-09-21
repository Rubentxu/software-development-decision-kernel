# ADR-021-EVENT-LEDGER-AUTHORITY — Use an append-only Event Ledger as operational authority

**Status:** Accepted


## Context
Sessions, agents, tools, UAT and providers emit important state changes. Without a durable common history, recovery and explanation depend on volatile IDE state.

## Decision
All relevant state transitions are represented by canonical events in an append-only ledger. Operational projections are derived and rebuildable.

## Invariants
- Events are immutable after append.
- `correlation_id` groups a logical workflow/session.
- `causation_id` links cause/effect where known.
- External host events preserve provenance to the raw host event type/id when available.
- Rebuilding a projection never replays side effects.

## Consequences
Enables replay, causal traces, journal, graph, metrics, forks and reliable recovery at the cost of event schema/versioning discipline.
