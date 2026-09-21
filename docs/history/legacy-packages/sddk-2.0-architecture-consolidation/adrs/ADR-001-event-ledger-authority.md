# ADR-001 — Event Ledger as Operational Authority

**Status:** Proposed  
**Date:** 2026-08-11

## Context

SDDK stores workflow, receipts, UAT, knowledge and other operational facts across several mechanisms. A single durable history is needed for replay and audit.

## Decision

Adopt a versioned append-only event ledger as the source of truth for SDDK operational history. Mutable stores become projections. Git remains source authority for repository contents.

## Consequences

### Positive

- One causal history for replay and audit.
- Projections can be rebuilt.
- Fork/diff becomes feasible.

### Trade-offs / risks

- Requires event schema discipline.
- Migration from mutable-only state must be staged.

## Implementation notes

Introduce the CEP, SQLite EventStore port/adapter, projection checkpoints and compatibility fixtures. New domains write ledger-first; legacy state migrates incrementally.

## Revisit trigger

Revisit only if a proven workload cannot satisfy durability/performance needs with an append-only model; authority semantics should still remain event-oriented.
